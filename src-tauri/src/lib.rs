use std::process::Command;
use std::net::TcpStream;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
    Manager, Runtime,
};
use serde::{Deserialize, Serialize};

const GATEWAY_PORT: u16 = 18789;

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

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "/min", "cmd", "/c", "openclaw", "gateway", "--port", "18789"])
            .spawn()
            .map_err(|e| format!("Failed to start gateway: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-c", "openclaw gateway --port 18789 &"])
            .spawn()
            .map_err(|e| format!("Failed to start gateway: {}", e))?;
    }

    Ok("Gateway starting...".to_string())
}

/// Stop the OpenClaw gateway
#[tauri::command]
fn stop_gateway() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(["/F", "/IM", "node.exe"])
            .output()
            .map_err(|e| format!("Failed to stop gateway: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("pkill")
            .args(["-f", "openclaw.*gateway"])
            .output()
            .map_err(|e| format!("Failed to stop gateway: {}", e))?;
    }

    Ok("Gateway stopped".to_string())
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

/// Open the dashboard in the default browser
#[tauri::command]
fn open_dashboard() -> Result<(), String> {
    opener::open(format!("http://127.0.0.1:{}/", GATEWAY_PORT))
        .map_err(|e| format!("Failed to open dashboard: {}", e))
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_gateway_status,
            start_gateway,
            stop_gateway,
            is_openclaw_installed,
            install_openclaw,
            open_dashboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
