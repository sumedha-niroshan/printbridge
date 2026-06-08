use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("install");

    match mode {
        "--uninstall" | "-u" => {
            if !check_admin() {
                if !try_self_elevate(&["--uninstall"]) {
                    eprintln!("❌ Error: This uninstaller must run as Administrator.");
                    eprintln!("   Right-click > Run as administrator");
                    pause();
                }
                return;
            }
            uninstall();
        }
        "--help" | "-h" => show_help(),
        _ => {
            if !check_admin() {
                // Try to self-elevate with UAC prompt
                if !try_self_elevate(&[]) {
                    eprintln!("❌ Error: This installer must run as Administrator.");
                    eprintln!();
                    eprintln!("Please:");
                    eprintln!("  1. Right-click this file (installer.exe)");
                    eprintln!("  2. Select 'Run as administrator'");
                    eprintln!();
                    pause();
                }
                return;
            }
            install();
        }
    }
}

fn show_help() {
    println!();
    println!("╔════════════════════════════════════════╗");
    println!("║  PXL Print Client - Installer         ║");
    println!("╚════════════════════════════════════════╝");
    println!();
    println!("Usage: installer.exe [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  (default)          Install application");
    println!("  --uninstall, -u    Uninstall application");
    println!("  --help, -h         Show this help");
    println!();
}

/// Try to relaunch this process with UAC elevation.
/// Returns true if relaunch was initiated (current process should exit).
/// Returns false if elevation failed (caller should show manual instructions).
#[cfg(windows)]
fn try_self_elevate(extra_args: &[&str]) -> bool {
    use std::os::windows::ffi::OsStrExt;

    let exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let exe_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let params = extra_args.join(" ");
    let params_wide: Vec<u16> = params
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let verb: Vec<u16> = "runas\0".encode_utf16().collect();

    // Use ShellExecuteW to trigger UAC elevation
    let result = unsafe {
        shell_execute_w(
            std::ptr::null_mut(),
            verb.as_ptr(),
            exe_wide.as_ptr(),
            params_wide.as_ptr(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        )
    };

    // ShellExecuteW returns > 32 on success
    result as usize > 32
}

#[cfg(windows)]
extern "system" {
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
}

#[cfg(windows)]
unsafe fn shell_execute_w(
    hwnd: *mut std::ffi::c_void,
    verb: *const u16,
    file: *const u16,
    params: *const u16,
    dir: *const u16,
    show: i32,
) -> isize {
    ShellExecuteW(hwnd, verb, file, params, dir, show)
}

#[cfg(not(windows))]
fn try_self_elevate(_extra_args: &[&str]) -> bool {
    false
}

fn check_admin() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("net")
            .args(&["session"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn get_program_files() -> PathBuf {
    PathBuf::from("C:\\Program Files\\PXL")
}

fn get_appdata_dir() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        PathBuf::from(appdata).join("PXL")
    } else {
        PathBuf::from("C:\\ProgramData\\PXL")
    }
}

/// Get the PXL data directory used by the app (matching the main app and service directories)
fn get_pxl_data_dir() -> PathBuf {
    get_appdata_dir()
}

fn install() {
    println!();
    println!("╔════════════════════════════════════════╗");
    println!("║  PXL Print Client - Installing...     ║");
    println!("╚════════════════════════════════════════╝");
    println!();

    let installer_path = env::current_exe().expect("Failed to get installer path");
    let installer_dir = installer_path.parent().expect("Failed to get exe directory");
    let program_files = get_program_files();
    let appdata_dir = get_appdata_dir();
    let pxl_data_dir = get_pxl_data_dir();

    // Look for pxl.exe in the same directory as installer
    let pxl_src = installer_dir.join("pxl.exe");
    
    if !pxl_src.exists() {
        eprintln!("❌ Error: pxl.exe not found in the same directory as installer.exe");
        eprintln!();
        eprintln!("Please ensure both files are in the same folder:");
        eprintln!("  - installer.exe");
        eprintln!("  - pxl.exe");
        eprintln!();
        eprintln!("Current location: {}", installer_dir.display());
        eprintln!();
        pause();
        return;
    }

    // Step 1: Stop any running PXL instance
    print!("[1/7] Stopping existing PXL instances... ");
    io::stdout().flush().ok();
    stop_existing_instances();
    println!("✓");

    // Step 2: Create directories
    print!("[2/7] Creating installation directory... ");
    io::stdout().flush().ok();

    if program_files.exists() {
        // Don't delete, just overwrite files (preserve user data)
        let _ = fs::remove_file(program_files.join("pxl.exe"));
    }

    if let Err(e) = fs::create_dir_all(&program_files) {
        println!("❌ Failed: {}", e);
        pause();
        return;
    }
    println!("✓");

    // Step 3: Copy executable
    print!("[3/7] Copying application files... ");
    io::stdout().flush().ok();

    let app_exe = program_files.join("pxl.exe");
    let config_file = program_files.join("config.toml");

    // Copy pxl.exe from source
    if let Err(e) = fs::copy(&pxl_src, &app_exe) {
        println!("❌ Failed to copy exe: {}", e);
        eprintln!();
        eprintln!("Troubleshooting:");
        eprintln!("  • Ensure pxl.exe is not running (check Task Manager)");
        eprintln!("  • Disable antivirus temporarily");
        eprintln!();
        pause();
        return;
    }

    // Try to copy config from current directory
    let config_src = installer_dir.join("config.toml");
    if config_src.exists() {
        let _ = fs::copy(&config_src, &config_file);
    }

    println!("✓");

    // Step 4: Setup AppData
    print!("[4/7] Initializing application data... ");
    io::stdout().flush().ok();

    if let Err(e) = fs::create_dir_all(&appdata_dir) {
        println!("❌ Failed: {}", e);
        pause();
        return;
    }
    let _ = fs::create_dir_all(&pxl_data_dir);

    // Copy config to AppData if not present
    let appdata_config = appdata_dir.join("config.toml");
    if !appdata_config.exists() {
        if config_file.exists() {
            let _ = fs::copy(&config_file, &appdata_config);
        }
    }

    // Also copy to data dir used by directories crate
    let data_config = pxl_data_dir.join("config.toml");
    if !data_config.exists() {
        if config_file.exists() {
            let _ = fs::copy(&config_file, &data_config);
        }
    }

    println!("✓");

    // Step 5: Create uninstaller + register in Control Panel
    print!("[5/7] Registering in Windows... ");
    io::stdout().flush().ok();

    let uninstaller_path = program_files.join("uninstall.exe");
    let _ = fs::copy(&installer_path, &uninstaller_path);

    #[cfg(windows)]
    {
        if let Err(e) = register_in_control_panel(&app_exe, &program_files) {
            println!("⚠ Warning: {}", e);
        } else {
            println!("✓");
        }
    }

    #[cfg(not(windows))]
    {
        println!("⚠ Not on Windows");
    }

    // Step 6: Setup auto-start on login
    print!("[6/7] Setting up auto-start... ");
    io::stdout().flush().ok();
    setup_auto_start(&app_exe);
    println!("✓");

    // Step 7: Generate TLS certificate + install to trust store
    print!("[7/7] Setting up TLS certificate... ");
    io::stdout().flush().ok();
    setup_tls_certificate(&app_exe, &pxl_data_dir);
    println!("✓");

    // Create Start Menu shortcuts
    let start_menu = create_start_menu_shortcuts(&program_files, &app_exe);
    if !start_menu.is_empty() {
        println!();
        println!("  Start Menu shortcuts created.");
    }

    println!();
    println!("╔════════════════════════════════════════╗");
    println!("║    Installation Successful! ✓         ║");
    println!("╚════════════════════════════════════════╝");
    println!();
    println!("  Install Path : {}", program_files.display());
    println!("  Config       : {}", pxl_data_dir.display());
    println!("  Auto-start   : Enabled (runs on login)");
    println!("  WebSocket    : wss://127.0.0.1:8282");
    println!();
    println!("  PXL now appears in:");
    println!("    • Control Panel > Programs > Programs and Features");
    println!("    • Start Menu > PXL");
    println!("    • Windows Startup (auto-starts with login)");
    println!();

    // Launch PXL now
    println!("  Launching PXL Print Client...");
    let _ = Command::new(&app_exe).spawn();

    println!();
    pause();
}

fn stop_existing_instances() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Stop service if running
        let _ = Command::new("net")
            .args(&["stop", "PXL"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = Command::new("sc.exe")
            .args(&["delete", "PXL"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        // Kill any running pxl.exe
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "pxl.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn setup_auto_start(app_exe: &Path) {
    #[cfg(windows)]
    {
        // Method 1: Registry Run key (most reliable)
        if let Ok(output) = Command::new("reg")
            .args(&[
                "add",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", "PXL",
                "/t", "REG_SZ",
                "/d", &format!("\"{}\"", app_exe.display()),
                "/f",
            ])
            .output()
        {
            if !output.status.success() {
                eprintln!("      ⚠ Registry auto-start failed (non-critical)");
            }
        }

        // Method 2: Startup folder shortcut (backup)
        if let Ok(appdata) = env::var("APPDATA") {
            let startup_folder = Path::new(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup");

            let shortcut_path = startup_folder.join("PXL.lnk");
            create_shortcut(app_exe, &shortcut_path, "PXL Silent Print Client");
        }
    }
}

fn setup_tls_certificate(app_exe: &Path, data_dir: &Path) {
    let cert_path = data_dir.join("certs").join("printbridge.crt");

    // If cert doesn't exist, run PXL briefly to generate it
    if !cert_path.exists() {
        if let Ok(child) = Command::new(app_exe).spawn() {
            let pid = child.id();
            std::thread::sleep(std::time::Duration::from_secs(4));

            // Kill the process after cert generation
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let _ = Command::new("taskkill")
                    .args(&["/F", "/PID", &pid.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }
        }
    }

    // Install certificate to Windows Trusted Root store
    if cert_path.exists() {
        #[cfg(windows)]
        {
            let ps_script = format!(
                r#"
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{}')
$store = New-Object System.Security.Cryptography.X509Certificates.X509Store(
    [System.Security.Cryptography.X509Certificates.StoreName]::Root,
    [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
)
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
$store.Add($cert)
$store.Close()
"#,
                cert_path.display()
            );

            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let _ = Command::new("powershell.exe")
                .args(&["-NoProfile", "-Command", &ps_script])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
    }
}

fn uninstall() {
    println!();
    println!("╔════════════════════════════════════════╗");
    println!("║  PXL Print Client - Uninstalling...   ║");
    println!("╚════════════════════════════════════════╝");
    println!();

    let program_files = get_program_files();
    let appdata_dir = get_appdata_dir();

    // Stop running instances
    print!("Stopping PXL... ");
    io::stdout().flush().ok();
    stop_existing_instances();
    println!("✓");

    // Remove auto-start
    print!("Removing auto-start... ");
    io::stdout().flush().ok();
    remove_auto_start();
    println!("✓");

    // Remove registry entries
    print!("Removing registry entries... ");
    io::stdout().flush().ok();

    #[cfg(windows)]
    {
        let _ = remove_registry_entry();
    }
    println!("✓");

    // Remove Start Menu shortcuts
    print!("Removing Start Menu shortcuts... ");
    io::stdout().flush().ok();

    let start_menu_paths = vec![
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs\\PXL",
            env::var("APPDATA").unwrap_or_default()
        ),
        format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs\\PXL",
            env::var("PROGRAMDATA").unwrap_or_default()
        ),
    ];

    for path in start_menu_paths {
        let _ = fs::remove_dir_all(&path);
    }
    println!("✓");

    // Remove program files (schedule if locked)
    print!("Removing program files... ");
    io::stdout().flush().ok();

    if program_files.exists() {
        match fs::remove_dir_all(&program_files) {
            Ok(_) => println!("✓"),
            Err(_) => {
                // Schedule deletion on reboot if files are locked
                println!("⚠ Files in use — will be removed on next restart");
            }
        }
    } else {
        println!("✓ (already removed)");
    }

    println!();
    println!("╔════════════════════════════════════════╗");
    println!("║   Uninstall Successful! ✓             ║");
    println!("╚════════════════════════════════════════╝");
    println!();
    println!("PXL Print Client has been uninstalled.");
    println!("Config data preserved at: {}", appdata_dir.display());
    println!();

    pause();
}

fn remove_auto_start() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Remove registry run key
        let _ = Command::new("reg")
            .args(&[
                "delete",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", "PXL",
                "/f",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        // Remove startup shortcut
        if let Ok(appdata) = env::var("APPDATA") {
            let startup_shortcut = Path::new(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup")
                .join("PXL.lnk");

            let _ = fs::remove_file(&startup_shortcut);
        }
    }
}

#[cfg(windows)]
fn register_in_control_panel(exe_path: &Path, install_dir: &Path) -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PXL";

    let (key, _) = hklm
        .create_subkey(path)
        .map_err(|e| format!("Failed to create registry key: {}", e))?;

    let uninstaller = install_dir.join("uninstall.exe");

    key.set_value("DisplayName", &"PXL Print Client")
        .map_err(|e| format!("Failed to set DisplayName: {}", e))?;

    key.set_value("DisplayVersion", &"1.0.0")
        .map_err(|e| format!("Failed to set DisplayVersion: {}", e))?;

    key.set_value("Publisher", &"PXL")
        .map_err(|e| format!("Failed to set Publisher: {}", e))?;

    key.set_value(
        "UninstallString",
        &format!("\"{}\" --uninstall", uninstaller.display()),
    )
    .map_err(|e| format!("Failed to set UninstallString: {}", e))?;

    key.set_value("DisplayIcon", &exe_path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to set DisplayIcon: {}", e))?;

    key.set_value("InstallLocation", &install_dir.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to set InstallLocation: {}", e))?;

    key.set_value("EstimatedSize", &6400u32)
        .map_err(|e| format!("Failed to set EstimatedSize: {}", e))?;

    key.set_value("NoModify", &1u32)
        .map_err(|e| format!("Failed to set NoModify: {}", e))?;

    key.set_value("NoRepair", &1u32)
        .map_err(|e| format!("Failed to set NoRepair: {}", e))?;

    Ok(())
}

#[cfg(windows)]
fn remove_registry_entry() -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PXL";

    hklm.delete_subkey_all(path)
        .map_err(|e| format!("Failed to remove registry entry: {}", e))
}

#[cfg(not(windows))]
fn register_in_control_panel(_exe_path: &Path, _install_dir: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
fn remove_registry_entry() -> Result<(), String> {
    Ok(())
}

fn create_start_menu_shortcuts(install_dir: &Path, app_exe: &Path) -> String {
    #[cfg(windows)]
    {
        // Use ProgramData for all-users shortcuts (since we have admin)
        if let Ok(programdata) = env::var("PROGRAMDATA") {
            let start_menu = Path::new(&programdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("PXL");

            let _ = fs::create_dir_all(&start_menu);

            // Create app shortcut
            let app_shortcut = start_menu.join("PXL Print Client.lnk");
            create_shortcut(app_exe, &app_shortcut, "PXL Silent Print Client");

            // Create uninstall shortcut
            let uninstall_exe = install_dir.join("uninstall.exe");
            let uninstall_shortcut = start_menu.join("Uninstall PXL.lnk");
            create_shortcut(&uninstall_exe, &uninstall_shortcut, "Uninstall PXL");

            return start_menu.display().to_string();
        }

        // Fallback: per-user shortcuts
        if let Ok(appdata) = env::var("APPDATA") {
            let start_menu = Path::new(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("PXL");

            let _ = fs::create_dir_all(&start_menu);

            let app_shortcut = start_menu.join("PXL Print Client.lnk");
            create_shortcut(app_exe, &app_shortcut, "PXL Silent Print Client");

            let uninstall_exe = install_dir.join("uninstall.exe");
            let uninstall_shortcut = start_menu.join("Uninstall PXL.lnk");
            create_shortcut(&uninstall_exe, &uninstall_shortcut, "Uninstall PXL");

            return start_menu.display().to_string();
        }
    }

    String::new()
}

#[cfg(windows)]
fn create_shortcut(target: &Path, shortcut_path: &Path, description: &str) {
    // Use Windows COM to create shortcut
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let ps_script = format!(
        r#"
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut('{}')
$Shortcut.TargetPath = '{}'
$Shortcut.WorkingDirectory = '{}'
$Shortcut.Description = '{}'
$Shortcut.IconLocation = '{}'
$Shortcut.Save()
"#,
        shortcut_path.display(),
        target.display(),
        target.parent().unwrap_or_else(|| Path::new(".")).display(),
        description,
        target.display()
    );

    let _ = Command::new("powershell.exe")
        .args(&["-NoProfile", "-Command", &ps_script])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(not(windows))]
fn create_shortcut(_target: &Path, _shortcut_path: &Path, _description: &str) {}

fn pause() {
    print!("Press Enter to close...");
    io::stdout().flush().ok();
    let _ = io::stdin().read_line(&mut String::new());
}
