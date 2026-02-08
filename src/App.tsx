import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
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
  const [startingGateway, setStartingGateway] = useState(false);

  // Track if we've already opened dashboard this session
  const [dashboardOpened, setDashboardOpened] = useState(false);

  // Log panel state
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<string>("");

  // Check status on load and every 3 seconds
  useEffect(() => {
    checkStatus();
    const interval = setInterval(checkStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  // Poll logs when log panel is open
  useEffect(() => {
    if (!showLogs) return;

    async function fetchLogs() {
      try {
        const logContent = await invoke<string>("get_gateway_logs", { lines: 200 });
        setLogs(logContent);
      } catch (e) {
        console.error("Failed to fetch logs:", e);
      }
    }

    fetchLogs();
    const interval = setInterval(fetchLogs, 1000);
    return () => clearInterval(interval);
  }, [showLogs]);

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

      // Auto-open dashboard if gateway is running and we haven't opened it yet
      if (gatewayStatus.running && !dashboardOpened) {
        setDashboardOpened(true);
        setStartingGateway(false);
        // Auto-open the dashboard window
        await invoke("open_dashboard_window");
      } else if (gatewayStatus.running) {
        setStartingGateway(false);
      }
    } catch (e) {
      console.error("Status check failed:", e);
      setPage("dashboard");
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
    if (startingGateway) return;
    setStartingGateway(true);
    try {
      await invoke("start_gateway");
      // Keep checking status
      setTimeout(checkStatus, 2000);
    } catch (e) {
      console.error("Failed to start gateway:", e);
      setStartingGateway(false);
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

  async function handleRestartGateway() {
    try {
      await invoke("restart_gateway");
      // Reset dashboard opened so it re-opens after restart
      setDashboardOpened(false);
      setTimeout(checkStatus, 3000);
    } catch (e) {
      console.error("Failed to restart gateway:", e);
    }
  }

  async function openKofi() {
    await openUrl("https://ko-fi.com/ai_dev_2024");
  }

  async function openDashboard() {
    await invoke("open_dashboard_window");
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
          <p className="credits">Made with ❤️ by ai-dev-2024</p>
        </footer>
      </main>
    );
  }

  // Dashboard view - Gateway running or stopped
  return (
    <main className="container dashboard">
      {/* Status Bar */}
      <header className="status-bar">
        <div className="status-left">
          <span className="logo-small">🦞</span>
          <span className="app-title">OpenClaw Desktop</span>
        </div>

        <div className="status-center">
          <span className={`status-indicator ${status?.running ? "running" : startingGateway ? "starting" : "stopped"}`}>
            {status?.running ? "🟢 Gateway Running" : startingGateway ? "🟡 Starting..." : "🔴 Gateway Stopped"}
          </span>
        </div>

        <div className="status-right">
          {!status?.running ? (
            <button
              className="control-btn start"
              onClick={handleStartGateway}
              disabled={startingGateway}
            >
              {startingGateway ? "Starting..." : "▶ Start"}
            </button>
          ) : (
            <>
              <button className="control-btn restart" onClick={handleRestartGateway} title="Restart Gateway">
                🔄 Restart
              </button>
              <button className="control-btn stop" onClick={handleStopGateway}>
                ⏹ Stop
              </button>
            </>
          )}
          <button
            className="control-btn dashboard-btn"
            onClick={openDashboard}
            disabled={!status?.running}
            title="Open Dashboard"
          >
            🌐 Dashboard
          </button>
          <button
            className={`control-btn logs-btn ${showLogs ? "active" : ""}`}
            onClick={() => setShowLogs(!showLogs)}
            title={showLogs ? "Hide Logs" : "Show Logs"}
          >
            📋 {showLogs ? "Hide Logs" : "Logs"}
          </button>
          <button className="support-btn-small" onClick={openKofi} title="Support on Ko-fi">
            ☕
          </button>
        </div>
      </header>

      {/* Log Panel */}
      {showLogs && (
        <div className="log-panel">
          <div className="log-header">
            <span>📋 Gateway Logs</span>
            <button
              className="log-close-btn"
              onClick={() => setShowLogs(false)}
              title="Close logs"
            >
              ✕
            </button>
          </div>
          <pre className="log-content">{logs || "No logs available yet..."}</pre>
        </div>
      )}

      {/* Main Content */}
      {status?.running ? (
        <div className="gateway-running">
          <div className="running-content">
            <span className="running-emoji">🦞</span>
            <h2>Gateway is Running</h2>
            <p>Your OpenClaw gateway is active and ready to use.</p>

            <div className="dashboard-info">
              <p className="dashboard-url">
                <strong>Dashboard:</strong> <a href="#" onClick={(e) => { e.preventDefault(); openDashboard(); }}>http://127.0.0.1:18789/</a>
              </p>
              <p className="info-note">
                ℹ️ The OpenClaw dashboard opens in your browser for security reasons.
              </p>
            </div>

            <button className="primary-btn large" onClick={openDashboard}>
              🌐 Open Dashboard
            </button>
          </div>

          <div className="support-section">
            <p>Enjoying OpenClaw Desktop?</p>
            <button className="support-btn" onClick={openKofi}>
              ☕ Buy me a coffee on Ko-fi
            </button>
          </div>
        </div>
      ) : (
        <div className="gateway-offline">
          <span className="offline-emoji">🦞</span>
          <h2>{startingGateway ? "Starting Gateway..." : "Gateway is not running"}</h2>
          <p>{startingGateway ? "Please wait, this may take a few seconds..." : "Click \"Start\" to launch the OpenClaw gateway"}</p>
          {!startingGateway && (
            <button className="primary-btn" onClick={handleStartGateway}>
              ▶ Start Gateway
            </button>
          )}

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
