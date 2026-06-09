use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Flag set by tray icon thread or second-instance wake listener to request
/// that the GUI window be restored and brought to the foreground.
#[cfg(windows)]
pub static SHOW_GUI: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::printer::{self, PrinterInfo};
use crate::websocket;

#[cfg(windows)]
static CONFIRM_QUIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Server,
    Origins,
    Printers,
    Logs,
}

pub struct PxlApp {
    config_path: PathBuf,
    config: Config,
    port_input: String,
    new_origin_input: String,
    printers: Vec<PrinterInfo>,
    selected_printer: String,
    logs: Arc<Mutex<Vec<String>>>,
    server_status: String,
    server_running: bool,
    rt: Arc<Runtime>,
    server_task: Option<JoinHandle<()>>,
    tls_config: Arc<tokio_rustls::rustls::ServerConfig>,
    status_message: String,
    status_timer: Option<std::time::Instant>,
    confirm_quit: bool,
    active_tab: Tab,
    #[cfg(windows)]
    _tray_icon: Option<tray_icon::TrayIcon>,
    #[cfg(windows)]
    show_menu_id: Option<tray_icon::menu::MenuId>,
    #[cfg(windows)]
    quit_menu_id: Option<tray_icon::menu::MenuId>,
    #[cfg(windows)]
    _tray_menu: Option<tray_icon::menu::Menu>,
    #[cfg(windows)]
    _show_item: Option<tray_icon::menu::MenuItem>,
    #[cfg(windows)]
    _quit_item: Option<tray_icon::menu::MenuItem>,
}

impl PxlApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config_path: PathBuf,
        config: Config,
        logs: Arc<Mutex<Vec<String>>>,
        rt: Arc<Runtime>,
        tls_config: Arc<tokio_rustls::rustls::ServerConfig>,
    ) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Make the egui context discoverable by other processes (single-instance wake)
        if let Ok(mut guard) = crate::EGUI_CTX.lock() {
            *guard = Some(cc.egui_ctx.clone());
        }

        #[cfg(windows)]
        let (
            tray_icon_opt,
            show_menu_id_opt,
            quit_menu_id_opt,
            tray_menu_opt,
            show_item_opt,
            quit_item_opt,
        ) = {
            use tray_icon::{
                menu::{Menu, MenuItem, PredefinedMenuItem},
                TrayIconBuilder,
            };

            let tray_menu = Menu::new();
            let show_item = MenuItem::new("Show PXL", true, None);
            let quit_item = MenuItem::new("Quit", true, None);

            let show_id = show_item.id().clone();
            let quit_id = quit_item.id().clone();

            let _ =
                tray_menu.append_items(&[&show_item, &PredefinedMenuItem::separator(), &quit_item]);

            // Load the PXL icon from embedded PNG bytes
            let icon_bytes = include_bytes!("../icons/PXL Icon.png");
            let icon_image =
                image::load_from_memory_with_format(icon_bytes, image::ImageFormat::Png)
                    .expect("Failed to decode embedded PXL icon");
            let rgba_image = icon_image.to_rgba8();
            let (width, height) = rgba_image.dimensions();
            let rgba = rgba_image.into_raw();

            let tray = if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, width, height) {
                TrayIconBuilder::new()
                    .with_menu(Box::new(tray_menu.clone()))
                    .with_tooltip("PXL Print Client")
                    .with_icon(icon)
                    .build()
                    .ok()
            } else {
                None
            };

            (
                tray,
                Some(show_id),
                Some(quit_id),
                Some(tray_menu),
                Some(show_item),
                Some(quit_item),
            )
        };

        let mut app = Self {
            config_path,
            port_input: config.server.port.to_string(),
            new_origin_input: String::new(),
            config,
            printers: Vec::new(),
            selected_printer: String::new(),
            logs,
            server_status: "Initializing...".to_string(),
            server_running: false,
            rt,
            server_task: None,
            tls_config,
            status_message: String::new(),
            status_timer: None,
            confirm_quit: false,
            active_tab: Tab::Server,
            #[cfg(windows)]
            _tray_icon: tray_icon_opt,
            #[cfg(windows)]
            show_menu_id: show_menu_id_opt,
            #[cfg(windows)]
            quit_menu_id: quit_menu_id_opt,
            #[cfg(windows)]
            _tray_menu: tray_menu_opt,
            #[cfg(windows)]
            _show_item: show_item_opt,
            #[cfg(windows)]
            _quit_item: quit_item_opt,
        };

        #[cfg(windows)]
        {
            let ctx = cc.egui_ctx.clone();
            let show_id = app.show_menu_id.clone();
            let quit_id = app.quit_menu_id.clone();

            std::thread::spawn(move || {
                loop {
                    // Process menu events
                    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                        if let Some(ref s_id) = show_id {
                            if event.id == *s_id {
                                // Set the flag; the update() loop will restore the window
                                SHOW_GUI.store(true, Ordering::SeqCst);
                                ctx.request_repaint();
                            }
                        }
                        if let Some(ref q_id) = quit_id {
                            if event.id == *q_id {
                                std::process::exit(0);
                            }
                        }
                    }

                    // Process tray events (left-click on tray icon)
                    while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                        match event {
                            tray_icon::TrayIconEvent::Click {
                                button: tray_icon::MouseButton::Left,
                                button_state: tray_icon::MouseButtonState::Up,
                                ..
                            } => {
                                SHOW_GUI.store(true, Ordering::SeqCst);
                                ctx.request_repaint();
                            }
                            _ => {}
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });
        }

        app.refresh_printers();
        app.start_server();
        app
    }

    fn refresh_printers(&mut self) {
        if let Ok(list) = printer::list_printers() {
            self.printers = list;
            if let Some(p) = self.printers.iter().find(|p| p.is_default) {
                self.selected_printer = p.name.clone();
            } else if !self.printers.is_empty() {
                self.selected_printer = self.printers[0].name.clone();
            }
        }
    }

    fn start_server(&mut self) {
        self.stop_server();

        let config = Arc::new(self.config.clone());
        let tls_config = self.tls_config.clone();
        let logs = self.logs.clone();

        let port = config.server.port;
        let host = config.server.host.clone();

        let handle = self.rt.spawn(async move {
            match websocket::run_server(config, tls_config).await {
                Ok(_) => {
                    let mut lock = logs.lock().unwrap();
                    lock.push("[INFO] WebSocket server stopped gracefully.".to_string());
                }
                Err(e) => {
                    let mut lock = logs.lock().unwrap();
                    lock.push(format!("[ERROR] WebSocket server error: {}", e));
                }
            }
        });

        self.server_task = Some(handle);
        self.server_status = format!("Running on wss://{}:{}", host, port);
        self.server_running = true;
        self.show_status(format!("Server started on port {}", port));
    }

    fn stop_server(&mut self) {
        if let Some(task) = self.server_task.take() {
            task.abort();
            self.server_status = "Stopped".to_string();
            self.server_running = false;
        }
    }

    fn show_status(&mut self, msg: String) {
        self.status_message = msg;
        self.status_timer = Some(std::time::Instant::now());
    }

    fn trigger_test_print(&mut self) {
        if self.selected_printer.is_empty() {
            self.show_status("No printer selected!".to_string());
            return;
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x1B, 0x40]);
        bytes.extend_from_slice(&[0x1B, 0x61, 1]);
        bytes.extend_from_slice(&[0x1D, 0x21, 0x11]);
        bytes.extend_from_slice(b"PXL RECEIPT\n\n");
        bytes.extend_from_slice(&[0x1D, 0x21, 0x00]);
        bytes.extend_from_slice(b"--- TEST PRINT SUCCESSFUL ---\n");
        bytes.extend_from_slice(&[0x1B, 0x61, 0]);
        bytes.extend_from_slice(b"Item 1: Test Burger      x1\n");
        bytes.extend_from_slice(b"Item 2: Golden Fries     x1\n");
        bytes.extend_from_slice(b"-----------------------------\n");
        bytes.extend_from_slice(b"PXL Agent is running smoothly\n");
        bytes.extend_from_slice(b"Port configuration: OK\n");
        bytes.extend_from_slice(b"TLS cert handshake: OK\n\n\n\n");
        bytes.extend_from_slice(&[0x1D, 0x56, 0x42, 0x00]);

        match printer::print_raw(&self.selected_printer, &bytes) {
            Ok(_) => self.show_status(format!("Test print sent to '{}'", self.selected_printer)),
            Err(e) => self.show_status(format!("Print failed: {}", e)),
        }
    }

    // ── Tab renderers ──

    fn render_server_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);

        // Status card
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let (dot, color, label) = if self.server_running {
                    ("●", egui::Color32::from_rgb(46, 204, 113), "ONLINE")
                } else {
                    ("●", egui::Color32::from_rgb(231, 76, 60), "OFFLINE")
                };
                ui.colored_label(color, egui::RichText::new(dot).size(18.0));
                ui.colored_label(color, egui::RichText::new(label).strong());
                ui.separator();
                ui.label(&self.server_status);
            });
        });

        ui.add_space(12.0);

        // Port config
        ui.label(egui::RichText::new("WebSocket Port").strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.port_input).desired_width(100.0));
            if ui.button("Save & Restart").clicked() {
                if let Ok(new_port) = self.port_input.trim().parse::<u16>() {
                    self.config.server.port = new_port;
                    if let Err(e) = self.config.save(&self.config_path) {
                        self.show_status(format!("Failed to save: {}", e));
                    } else {
                        self.start_server();
                    }
                } else {
                    self.show_status("Invalid port number!".to_string());
                }
            }
        });

        ui.add_space(16.0);

        // Server controls
        ui.label(egui::RichText::new("Server Controls").strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if self.server_running {
                if ui.button("Stop Server").clicked() {
                    self.stop_server();
                    self.show_status("Server stopped.".to_string());
                }
            } else {
                if ui.button("Start Server").clicked() {
                    self.start_server();
                }
            }
        });
    }

    fn render_origins_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Allowed Origins (CORS)").strong());
        ui.label(
            egui::RichText::new("Websites that are allowed to connect to PXL.")
                .weak()
                .size(11.0),
        );
        ui.add_space(8.0);

        // Add new origin
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_origin_input)
                    .hint_text("e.g. https://myapp.com")
                    .desired_width(ui.available_width() - 60.0),
            );
            if ui.button("Add").clicked() {
                let origin = self.new_origin_input.trim().to_string();
                if !origin.is_empty() && !self.config.server.allowed_origins.contains(&origin) {
                    self.config.server.allowed_origins.push(origin);
                    if let Err(e) = self.config.save(&self.config_path) {
                        self.show_status(format!("Save failed: {}", e));
                    } else {
                        self.new_origin_input.clear();
                        self.show_status("Origin added. Restart server to apply.".to_string());
                    }
                } else if origin.is_empty() {
                    self.show_status("Enter an origin URL.".to_string());
                } else {
                    self.show_status("Origin already exists!".to_string());
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Origins list
        let origins = self.config.server.allowed_origins.clone();
        let mut to_remove: Option<usize> = None;

        let available = ui.available_height() - 30.0;
        egui::ScrollArea::vertical()
            .id_source("origins_list")
            .max_height(available.max(100.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if origins.is_empty() {
                    ui.label(
                        egui::RichText::new("No origins configured. Add one above.")
                            .weak()
                            .italics(),
                    );
                }
                for (i, origin) in origins.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(origin).monospace());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Remove")
                                        .color(egui::Color32::from_rgb(231, 76, 60))
                                        .size(11.0),
                                )
                                .clicked()
                            {
                                to_remove = Some(i);
                            }
                        });
                    });
                    ui.separator();
                }
            });

        if let Some(idx) = to_remove {
            self.config.server.allowed_origins.remove(idx);
            if let Err(e) = self.config.save(&self.config_path) {
                self.show_status(format!("Save failed: {}", e));
            } else {
                self.show_status("Origin removed. Restart server to apply.".to_string());
            }
        }
    }

    fn render_printers_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("System Printers").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.refresh_printers();
                    self.show_status(format!("Found {} printer(s)", self.printers.len()));
                }
            });
        });
        ui.add_space(8.0);

        // Printer list
        egui::ScrollArea::vertical()
            .id_source("printers_list")
            .max_height(150.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if self.printers.is_empty() {
                    ui.label(
                        egui::RichText::new("No printers found on this machine.")
                            .weak()
                            .italics(),
                    );
                }
                for p in &self.printers {
                    let is_selected = self.selected_printer == p.name;
                    let text = if p.is_default {
                        format!("{}  (default)", p.name)
                    } else {
                        p.name.clone()
                    };

                    let response = ui.selectable_label(is_selected, &text);
                    if response.clicked() {
                        self.selected_printer = p.name.clone();
                    }
                }
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Test print section
        ui.label(egui::RichText::new("Test Print").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Selected:");
            if self.selected_printer.is_empty() {
                ui.label(
                    egui::RichText::new("None — click a printer above")
                        .weak()
                        .italics(),
                );
            } else {
                ui.label(
                    egui::RichText::new(&self.selected_printer)
                        .monospace()
                        .strong(),
                );
            }
        });

        ui.add_space(4.0);

        if ui
            .add_enabled(
                !self.selected_printer.is_empty(),
                egui::Button::new("Send Test Receipt"),
            )
            .clicked()
        {
            self.trigger_test_print();
        }
    }

    fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Live Logs").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    if let Ok(mut logs) = self.logs.lock() {
                        logs.clear();
                    }
                }
            });
        });
        ui.add_space(4.0);

        let mut log_text = {
            let lock = self.logs.lock().unwrap();
            if lock.is_empty() {
                "Waiting for activity...".to_string()
            } else {
                lock.join("\n")
            }
        };

        let available = ui.available_height() - 8.0;
        egui::ScrollArea::vertical()
            .id_source("logs_scroll")
            .max_height(available.max(100.0))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut log_text)
                        .font(egui::TextStyle::Monospace)
                        .text_color(egui::Color32::from_rgb(200, 200, 200))
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .lock_focus(true),
                );
            });
    }
}

impl eframe::App for PxlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        // If another instance signalled us to wake, restore and focus the window
        if crate::WAKE_GUI.load(Ordering::SeqCst) {
            crate::WAKE_GUI.store(false, Ordering::SeqCst);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // If tray icon or second instance requested showing the GUI
        #[cfg(windows)]
        if SHOW_GUI.load(Ordering::SeqCst) {
            SHOW_GUI.store(false, Ordering::SeqCst);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // Minimize on close instead of quitting (keeps the event loop alive)
        if ctx.input(|i| i.viewport().close_requested()) {
            #[cfg(windows)]
            let bypass = CONFIRM_QUIT.load(Ordering::SeqCst) || self.confirm_quit;
            #[cfg(not(windows))]
            let bypass = self.confirm_quit;

            if !bypass {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                // Minimize instead of hiding — Visible(false) kills the event loop
                // so the tray icon can never bring the window back
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            } else {
                std::process::exit(0);
            }
        }

        // ── Top panel: header + tabs ──
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("PXL").strong().size(20.0));

                // Status dot
                let (dot_color, tip) = if self.server_running {
                    (egui::Color32::from_rgb(46, 204, 113), "Server is running")
                } else {
                    (egui::Color32::from_rgb(231, 76, 60), "Server is stopped")
                };
                ui.colored_label(dot_color, egui::RichText::new("●").size(14.0))
                    .on_hover_text(tip);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Quit")
                                .color(egui::Color32::from_rgb(231, 76, 60))
                                .size(12.0),
                        )
                        .clicked()
                    {
                        std::process::exit(0);
                    }
                });
            });

            ui.add_space(2.0);

            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Server, "Server");
                ui.selectable_value(&mut self.active_tab, Tab::Origins, "Origins");
                ui.selectable_value(&mut self.active_tab, Tab::Printers, "Printers");
                ui.selectable_value(&mut self.active_tab, Tab::Logs, "Logs");
            });
        });

        // ── Bottom panel: status bar ──
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(2.0);
            if let Some(timer) = self.status_timer {
                if timer.elapsed().as_secs() < 5 {
                    ui.colored_label(egui::Color32::from_rgb(100, 180, 255), &self.status_message);
                } else {
                    self.status_timer = None;
                    ui.label(egui::RichText::new("Ready").weak().size(11.0));
                }
            } else {
                ui.label(egui::RichText::new("Ready").weak().size(11.0));
            }
            ui.add_space(2.0);
        });

        // ── Central panel: active tab content ──
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            Tab::Server => self.render_server_tab(ui),
            Tab::Origins => self.render_origins_tab(ui),
            Tab::Printers => self.render_printers_tab(ui),
            Tab::Logs => self.render_logs_tab(ui),
        });
    }
}
