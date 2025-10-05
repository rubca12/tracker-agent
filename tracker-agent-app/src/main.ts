import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// UI Elements
let statusIndicator: HTMLElement;
let statusText: HTMLElement;
let currentApp: HTMLElement;
let currentActivity: HTMLElement;
let currentTask: HTMLElement;
let trackingSince: HTMLElement;
let logContainer: HTMLElement;
let startButton: HTMLButtonElement;
let stopButton: HTMLButtonElement;
let saveSettingsButton: HTMLButtonElement;

// Settings inputs
let intervalInput: HTMLInputElement;
let openrouterKeyInput: HTMLInputElement;
let freeloEmailInput: HTMLInputElement;
let freeloKeyInput: HTMLInputElement;

// Initialize app
window.addEventListener("DOMContentLoaded", async () => {
  // Get UI elements
  statusIndicator = document.getElementById("status-indicator")!;
  statusText = document.getElementById("status-text")!;
  currentApp = document.getElementById("current-app")!;
  currentActivity = document.getElementById("current-activity")!;
  currentTask = document.getElementById("current-task")!;
  trackingSince = document.getElementById("tracking-since")!;
  logContainer = document.getElementById("log-container")!;
  startButton = document.getElementById("start-tracking") as HTMLButtonElement;
  stopButton = document.getElementById("stop-tracking") as HTMLButtonElement;
  saveSettingsButton = document.getElementById("save-settings") as HTMLButtonElement;

  intervalInput = document.getElementById("interval") as HTMLInputElement;
  openrouterKeyInput = document.getElementById("openrouter-key") as HTMLInputElement;
  freeloEmailInput = document.getElementById("freelo-email") as HTMLInputElement;
  freeloKeyInput = document.getElementById("freelo-key") as HTMLInputElement;

  // Event listeners
  startButton.addEventListener("click", startTracking);
  stopButton.addEventListener("click", stopTracking);
  saveSettingsButton.addEventListener("click", saveSettings);

  // Listen for backend events
  await listen("log-event", (event: any) => {
    addLogEntry(event.payload.level, event.payload.message);
  });

  await listen("tracking-update", (event: any) => {
    updateTrackingInfo(event.payload);
  });

  // Load saved settings
  loadSettings();

  addLogEntry("info", "Aplikace inicializována");
  updateStatus("inactive", "Připraveno");
});

// Start tracking
async function startTracking() {
  try {
    await invoke("start_tracking");
    startButton.disabled = true;
    stopButton.disabled = false;
    updateStatus("active", "Tracking aktivní");
    addLogEntry("success", "Tracking spuštěn");
  } catch (error) {
    addLogEntry("error", `Chyba při spuštění: ${error}`);
  }
}

// Stop tracking
async function stopTracking() {
  try {
    await invoke("stop_tracking");
    startButton.disabled = false;
    stopButton.disabled = true;
    updateStatus("inactive", "Zastaveno");
    addLogEntry("warning", "Tracking zastaven");
  } catch (error) {
    addLogEntry("error", `Chyba při zastavení: ${error}`);
  }
}

// Save settings
async function saveSettings() {
  const settings = {
    interval: parseInt(intervalInput.value),
    openrouter_key: openrouterKeyInput.value,
    freelo_email: freeloEmailInput.value,
    freelo_key: freeloKeyInput.value,
  };

  try {
    await invoke("save_settings", { settings });
    addLogEntry("success", "Nastavení uloženo");

    // Save to localStorage
    localStorage.setItem("tracker-settings", JSON.stringify(settings));
  } catch (error) {
    addLogEntry("error", `Chyba při ukládání: ${error}`);
  }
}

// Load settings from localStorage
function loadSettings() {
  const saved = localStorage.getItem("tracker-settings");
  if (saved) {
    try {
      const settings = JSON.parse(saved);
      intervalInput.value = settings.interval || "10";
      openrouterKeyInput.value = settings.openrouter_key || "";
      freeloEmailInput.value = settings.freelo_email || "";
      freeloKeyInput.value = settings.freelo_key || "";
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  }
}

// Update status indicator
function updateStatus(status: "active" | "inactive" | "warning", text: string) {
  statusIndicator.className = `status-indicator ${status}`;
  statusText.textContent = text;
}

// Update tracking info
function updateTrackingInfo(info: any) {
  currentApp.textContent = info.application || "-";
  currentActivity.textContent = info.activity || "-";
  currentTask.textContent = info.task || "Žádný";
  trackingSince.textContent = info.since || "-";
}

// Add log entry
function addLogEntry(level: string, message: string) {
  const time = new Date().toLocaleTimeString("cs-CZ");
  const entry = document.createElement("div");
  entry.className = `log-entry ${level}`;
  entry.innerHTML = `
    <span class="log-time">${time}</span>
    <span class="log-message">${message}</span>
  `;

  logContainer.appendChild(entry);

  // Auto-scroll to bottom
  logContainer.scrollTop = logContainer.scrollHeight;

  // Keep only last 100 entries
  while (logContainer.children.length > 100) {
    logContainer.removeChild(logContainer.firstChild!);
  }
}
