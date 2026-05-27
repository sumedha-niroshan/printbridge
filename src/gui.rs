use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::egui;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::printer::{self, PrinterInfo};
use crate::websocket;

pub struct PxlApp {
    config_path: PathBuf,
    config: Config,
    port_input: String,
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
        // Setup initial styling to look premium (Dark Theme)
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let mut app = Self {
            config_path,
            port_input: config.server.port.to_string(),
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
        };

        app.refresh_printers();
        app.start_server();
        app
    }

    fn refresh_printers(&mut self) {
        if let Ok(list) = printer::list_printers() {
            self.printers = list;
            // Select default printer if available
            if let Some(p) = self.printers.iter().find(|p| p.is_default) {
                self.selected_printer = p.name.clone();
            } else if !self.printers.is_empty() {
                self.selected_printer = self.printers[0].name.clone();
            }
        }
    }

    fn start_server(&mut self) {
        // Stop server if running
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
        self.server_status = format!("Active (wss://{}:{})", host, port);
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
            self.show_status("Error: No printer selected!".to_string());
            return;
        }

        let mut bytes = Vec::new();
        
        // ESC @ (Initialize)
        bytes.extend_from_slice(&[0x1B, 0x40]);
        // ESC a 1 (Align Center)
        bytes.extend_from_slice(&[0x1B, 0x61, 1]);
        // Double-height double-width font
        bytes.extend_from_slice(&[0x1D, 0x21, 0x11]);
        bytes.extend_from_slice(b"PXL RECEIPT\n\n");
        // Normal text
        bytes.extend_from_slice(&[0x1D, 0x21, 0x00]);
        bytes.extend_from_slice(b"--- TEST PRINT SUCCESSFUL ---\n");
        
        // Left alignment
        bytes.extend_from_slice(&[0x1B, 0x61, 0]);
        bytes.extend_from_slice(b"Item 1: Test Burger      x1\n");
        bytes.extend_from_slice(b"Item 2: Golden Fries     x1\n");
        bytes.extend_from_slice(b"-----------------------------\n");
        bytes.extend_from_slice(b"PXL Agent is running smoothly\n");
        bytes.extend_from_slice(b"Port configuration: OK\n");
        bytes.extend_from_slice(b"TLS cert handshake: OK\n\n\n\n");
        
        // GS V 66 0 (Cut paper)
        bytes.extend_from_slice(&[0x1D, 0x56, 0x42, 0x00]);

        match printer::print_raw(&self.selected_printer, &bytes) {
            Ok(_) => self.show_status(format!("Sent test print to {}", self.selected_printer)),
            Err(e) => self.show_status(format!("Print failed: {}", e)),
        }
    }
}

impl eframe::App for PxlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous repaint to get real-time log updates
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading("PXL Print Agent");
                ui.label(egui::RichText::new("Enterprise Silent Printing Client").weak());
                ui.add_space(8.0);
            });

            ui.separator();

            // Status Bar
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    let color = if self.server_running {
                        egui::Color32::from_rgb(46, 204, 113) // Green
                    } else {
                        egui::Color32::from_rgb(231, 76, 60)  // Red
                    };
                    ui.colored_label(color, &self.server_status);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.server_running {
                            if ui.button("Stop Server").clicked() {
                                self.stop_server();
                            }
                        } else {
                            if ui.button("Start Server").clicked() {
                                self.start_server();
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Left / Right panels layout
            ui.columns(2, |columns| {
                // Column 1: Config
                columns[0].vertical(|ui| {
                    ui.strong("Configuration");
                    ui.add_space(4.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("WebSocket Port:");
                        ui.text_edit_singleline(&mut self.port_input);
                    });

                    ui.add_space(6.0);

                    if ui.button("Save & Restart Port").clicked() {
                        if let Ok(new_port) = self.port_input.trim().parse::<u16>() {
                            self.config.server.port = new_port;
                            if let Err(e) = self.config.save(&self.config_path) {
                                self.show_status(format!("Failed to save config: {}", e));
                            } else {
                                self.start_server();
                            }
                        } else {
                            self.show_status("Invalid Port Number!".to_string());
                        }
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Test printing
                    ui.strong("Test Print Tool");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Printer:");
                        egui::ComboBox::from_id_source("printer_combo")
                            .selected_text(&self.selected_printer)
                            .show_ui(ui, |ui| {
                                for p in &self.printers {
                                    ui.selectable_value(&mut self.selected_printer, p.name.clone(), &p.name);
                                }
                            });
                    });

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui.button("Refresh Printers").clicked() {
                            self.refresh_printers();
                        }
                        if ui.button("Run Test Print").clicked() {
                            self.trigger_test_print();
                        }
                    });
                });

                // Column 2: Logs
                columns[1].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Live Printing Logs");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                if let Ok(mut logs) = self.logs.lock() {
                                    logs.clear();
                                }
                            }
                        });
                    });
                    ui.add_space(4.0);

                    let log_text = {
                        let lock = self.logs.lock().unwrap();
                        lock.join("\n")
                    };

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut log_text.as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .text_color(egui::Color32::from_rgb(220, 220, 220))
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10)
                                    .lock_focus(true)
                            );
                        });
                });
            });

            // Status message banner at bottom
            if let Some(timer) = self.status_timer {
                if timer.elapsed().as_secs() < 4 {
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status_message);
                } else {
                    self.status_timer = None;
                }
            }
        });
    }
}
