use std::process::Command;
use std::net::TcpStream;
use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
    Manager, Runtime,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const GATEWAY_PORT: u16 = 18789;

/// Read the gateway token from OpenClaw config file
fn read_gateway_token() -> Option<String> {
    // Try to find the config file
    let home = dirs::home_dir()?;
    let config_path = home.join(".openclaw").join("openclaw.json");
    
    if !config_path.exists() {
        // Try legacy path
        let legacy_path = home.join(".clawdbot").join("clawdbot.json");
        if legacy_path.exists() {
            return read_token_from_file(&legacy_path);
        }
        return None;
    }
    
    read_token_from_file(&config_path)
}

fn read_token_from_file(path: &PathBuf) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    json.get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get("token"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub running: bool,
    pub port: u16,
    pub dashboard_url: String,
}

/// Check if the OpenClaw gateway is running by probing the port
fn is_gateway_running() -> bool {
    TcpStream::connect(format!("127.0.0.1:{}", GATEWAY_PORT)).is_ok()
}

/// Get the current gateway status
#[tauri::command]
fn get_gateway_status() -> GatewayStatus {
    GatewayStatus {
        running: is_gateway_running(),
        port: GATEWAY_PORT,
        dashboard_url: format!("http://127.0.0.1:{}/", GATEWAY_PORT),
    }
}

/// Start the OpenClaw gateway
#[tauri::command]
fn start_gateway() -> Result<String, String> {
    if is_gateway_running() {
        return Ok("Gateway is already running".to_string());
    }

    // Run gateway directly with output redirected to log file (no visible CMD window)
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir().ok_or("Could not find home directory")?;
        let log_path = home.join(".openclaw").join("gateway.log");
        
        // Clear the log file
        let _ = std::fs::write(&log_path, "");
        
        // Run gateway directly with output redirected to log file
        // Using powershell to redirect output properly
        let cmd = format!(
            "Start-Process -WindowStyle Hidden -FilePath 'node' -ArgumentList '{} gateway --port {}' -RedirectStandardOutput '{}' -RedirectStandardError '{}'",
            home.join("AppData\\Roaming\\npm\\node_modules\\openclaw\\dist\\index.js").display(),
            GATEWAY_PORT,
            log_path.display(),
            home.join(".openclaw").join("gateway_error.log").display()
        );
        
        Command::new("powershell")
            .args(["-WindowStyle", "Hidden", "-Command", &cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to start gateway: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().ok_or("Could not find home directory")?;
        let log_path = home.join(".openclaw").join("gateway.log");
        
        Command::new("sh")
            .args(["-c", &format!(
                "nohup openclaw gateway --port {} > '{}' 2>&1 &",
                GATEWAY_PORT,
                log_path.display()
            )])
            .spawn()
            .map_err(|e| format!("Failed to start gateway: {}", e))?;
    }

    Ok("Gateway starting...".to_string())
}

/// Stop the OpenClaw gateway
#[tauri::command]
fn stop_gateway() -> Result<String, String> {
    // Use 'openclaw gateway stop' which stops the registered service
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "openclaw", "gateway", "stop"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to stop gateway: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-c", "openclaw gateway stop"])
            .spawn()
            .map_err(|e| format!("Failed to stop gateway: {}", e))?;
    }

    Ok("Gateway stopped".to_string())
}

/// Restart the OpenClaw gateway
#[tauri::command]
fn restart_gateway() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "openclaw", "gateway", "restart"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to restart gateway: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-c", "openclaw gateway restart"])
            .spawn()
            .map_err(|e| format!("Failed to restart gateway: {}", e))?;
    }

    Ok("Gateway restarting...".to_string())
}

/// Check if OpenClaw is installed
#[tauri::command]
fn is_openclaw_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg("openclaw")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg("openclaw")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Install OpenClaw via npm
#[tauri::command]
async fn install_openclaw() -> Result<String, String> {
    let output = Command::new("npm")
        .args(["install", "-g", "openclaw"])
        .output()
        .map_err(|e| format!("Failed to install: {}", e))?;

    if output.status.success() {
        Ok("OpenClaw installed successfully".to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Navigate main window to the dashboard
#[tauri::command]
async fn open_dashboard_window(app: tauri::AppHandle) -> Result<(), String> {
    // Get the main window and navigate it to the dashboard
    if let Some(window) = app.get_webview_window("main") {
        // Build tokenized URL for authentication
        let base_url = format!("http://127.0.0.1:{}/", GATEWAY_PORT);
        let dashboard_url = match read_gateway_token() {
            Some(token) => format!("{}?token={}", base_url, urlencoding::encode(&token)),
            None => base_url,
        };
        window.navigate(dashboard_url.parse().unwrap())
            .map_err(|e| format!("Failed to navigate: {}", e))?;
    }
    Ok(())
}

/// Get the gateway logs from the log file
#[tauri::command]
fn get_gateway_logs(lines: Option<usize>) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_path = home.join(".openclaw").join("gateway.log");
    
    if !log_path.exists() {
        return Ok("No logs available yet. Start the gateway to see logs.".to_string());
    }
    
    let content = fs::read_to_string(&log_path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;
    
    let max_lines = lines.unwrap_or(100);
    let log_lines: Vec<&str> = content.lines().collect();
    let start = if log_lines.len() > max_lines { log_lines.len() - max_lines } else { 0 };
    
    Ok(log_lines[start..].join("\n"))
}

/// Clear the gateway logs
#[tauri::command]
fn clear_gateway_logs() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_path = home.join(".openclaw").join("gateway.log");
    
    if log_path.exists() {
        fs::write(&log_path, "").map_err(|e| format!("Failed to clear logs: {}", e))?;
    }
    
    Ok(())
}

fn create_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let status = if is_gateway_running() { "🟢 Running" } else { "🔴 Stopped" };
    
    let menu = Menu::with_items(app, &[
        &MenuItem::with_id(app, "status", status, false, None::<&str>)?,
        &MenuItem::with_id(app, "separator", "─────────", false, None::<&str>)?,
        &MenuItem::with_id(app, "start", "▶ Start Gateway", true, None::<&str>)?,
        &MenuItem::with_id(app, "stop", "⏹ Stop Gateway", true, None::<&str>)?,
        &MenuItem::with_id(app, "dashboard", "🌐 Open Dashboard", true, None::<&str>)?,
        &MenuItem::with_id(app, "separator2", "─────────", false, None::<&str>)?,
        &MenuItem::with_id(app, "quit", "✖ Quit", true, None::<&str>)?,
    ])?;
    
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Create system tray
            let menu = create_tray_menu(app.handle())?;
            
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "start" => {
                            let _ = start_gateway();
                        }
                        "stop" => {
                            let _ = stop_gateway();
                        }
                        "dashboard" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle window close - minimize to tray instead of quitting
            let main_window = app.get_webview_window("main").unwrap();
            let main_window_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent the window from closing
                    api.prevent_close();
                    // Hide the window instead
                    let _ = main_window_clone.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_gateway_status,
            start_gateway,
            stop_gateway,
            restart_gateway,
            is_openclaw_installed,
            install_openclaw,
            open_dashboard_window,
            get_gateway_logs,
            clear_gateway_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
