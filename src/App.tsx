import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-opener";
import "./App.css";

interface GatewayStatus {
  running: boolean;
  port: number;
  dashboard_url: string;
}

type Page = "loading" | "setup" | "dashboard";

function App() {
  const [page, setPage] = useState<Page>("loading");
  const [status, setStatus] = useState<GatewayStatus | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check status on load and every 5 seconds
  useEffect(() => {
    checkStatus();
    const interval = setInterval(checkStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  async function checkStatus() {
    try {
      const isInstalled = await invoke<boolean>("is_openclaw_installed");
      if (!isInstalled) {
        setPage("setup");
        return;
      }

      const gatewayStatus = await invoke<GatewayStatus>("get_gateway_status");
      setStatus(gatewayStatus);
      setPage("dashboard");
    } catch (e) {
      console.error("Status check failed:", e);
    }
  }

  async function handleInstall() {
    setInstalling(true);
    setError(null);
    try {
      await invoke("install_openclaw");
      await checkStatus();
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  }

  async function handleStartGateway() {
    try {
      await invoke("start_gateway");
      setTimeout(checkStatus, 3000);
    } catch (e) {
      console.error("Failed to start gateway:", e);
    }
  }

  async function handleStopGateway() {
    try {
      await invoke("stop_gateway");
      setTimeout(checkStatus, 1000);
    } catch (e) {
      console.error("Failed to stop gateway:", e);
    }
  }

  async function openKofi() {
    await open("https://ko-fi.com/ai_dev_2024");
  }

  async function openDashboardExternal() {
    await open("http://127.0.0.1:18789/");
  }

  // Loading screen
  if (page === "loading") {
    return (
      <main className="container loading">
        <div className="logo-container">
          <span className="logo-emoji">🦞</span>
        </div>
        <h1>OpenClaw Desktop</h1>
        <p className="loading-text">Checking installation...</p>
      </main>
    );
  }

  // Setup wizard
  if (page === "setup") {
    return (
      <main className="container setup">
        <div className="logo-container">
          <span className="logo-emoji">🦞</span>
        </div>
        <h1>Welcome to OpenClaw Desktop</h1>
        <p className="subtitle">Your AI assistant, now with a beautiful UI</p>

        <div className="setup-card">
          <h2>🚀 Quick Setup</h2>
          <p>OpenClaw needs to be installed on your system. This will:</p>
          <ul>
            <li>Install OpenClaw via npm (requires Node.js)</li>
            <li>Set up the gateway service</li>
            <li>Get you ready to use your AI assistant</li>
          </ul>
          
          {error && <p className="error">{error}</p>}
          
          <button 
            className="primary-btn" 
            onClick={handleInstall}
            disabled={installing}
          >
            {installing ? "Installing..." : "Install OpenClaw"}
          </button>
        </div>

        <footer className="setup-footer">
          <button className="support-btn" onClick={openKofi}>
            ☕ Support Development
          </button>
          <p className="credits">Made with ❤️ by Muhib</p>
        </footer>
      </main>
    );
  }

  // Dashboard view
  return (
    <main className="container dashboard">
      {/* Status Bar */}
      <header className="status-bar">
        <div className="status-left">
          <span className="logo-small">🦞</span>
          <span className="app-title">OpenClaw Desktop</span>
        </div>
        
        <div className="status-center">
          <span className={`status-indicator ${status?.running ? "running" : "stopped"}`}>
            {status?.running ? "🟢 Gateway Running" : "🔴 Gateway Stopped"}
          </span>
        </div>

        <div className="status-right">
          {!status?.running ? (
            <button className="control-btn start" onClick={handleStartGateway}>
              ▶ Start
            </button>
          ) : (
            <button className="control-btn stop" onClick={handleStopGateway}>
              ⏹ Stop
            </button>
          )}
          <button className="control-btn external" onClick={openDashboardExternal}>
            🌐
          </button>
          <button className="support-btn-small" onClick={openKofi}>
            ☕
          </button>
        </div>
      </header>

      {/* Main Content */}
      {status?.running ? (
        <iframe 
          src={status.dashboard_url}
          className="dashboard-frame"
          title="OpenClaw Dashboard"
        />
      ) : (
        <div className="gateway-offline">
          <span className="offline-emoji">🦞</span>
          <h2>Gateway is not running</h2>
          <p>Click "Start" to launch the OpenClaw gateway</p>
          <button className="primary-btn" onClick={handleStartGateway}>
            ▶ Start Gateway
          </button>
          
          <div className="support-section">
            <p>Enjoying OpenClaw Desktop?</p>
            <button className="support-btn" onClick={openKofi}>
              ☕ Buy me a coffee on Ko-fi
            </button>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
