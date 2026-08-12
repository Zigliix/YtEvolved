import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { Webview } from "@tauri-apps/api/webview";
import "./styles.css";

const TOPBAR_HEIGHT = 40;
const appWindow = getCurrentWindow();

const topbar = document.createElement("header");
topbar.id = "titlebar";
topbar.innerHTML = `
  <div class="branding">
    <span class="logo" aria-hidden="true">
      <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <circle cx="12" cy="12" r="10" fill="#ff0000" />
        <path d="M10 8.25L16 12L10 15.75V8.25Z" fill="white" />
      </svg>
    </span>
    <span class="title">YouTube Music Evolved</span>
  </div>
  <div class="drag-space"></div>
  <div class="controls">
    <button id="settings" class="window-button" title="Settings" aria-label="Settings">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" /><path d="M19.4 15a1.8 1.8 0 0 0 .36 1.99l.05.05-1.41 1.41-.05-.05a1.8 1.8 0 0 0-1.99-.36 1.8 1.8 0 0 0-1.09 1.65V20h-2v-.31a1.8 1.8 0 0 0-1.09-1.65 1.8 1.8 0 0 0-1.99.36l-.05.05-1.41-1.41.05-.05A1.8 1.8 0 0 0 9.14 15a1.8 1.8 0 0 0-1.65-1.09H7v-2h.49A1.8 1.8 0 0 0 9.14 11a1.8 1.8 0 0 0-.36-1.99l-.05-.05 1.41-1.41.05.05a1.8 1.8 0 0 0 1.99.36 1.8 1.8 0 0 0 1.09-1.65V6h2v.31a1.8 1.8 0 0 0 1.09 1.65 1.8 1.8 0 0 0 1.99-.36l.05-.05 1.41 1.41-.05.05A1.8 1.8 0 0 0 19.4 11a1.8 1.8 0 0 0 1.65 1.09h.31v2h-.31A1.8 1.8 0 0 0 19.4 15Z" /></svg>
    </button>
    <span class="control-separator" aria-hidden="true"></span>
    <button id="minimize" class="window-button" title="Minimize" aria-label="Minimize">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
    </button>
    <button id="maximize" class="window-button" title="Maximize" aria-label="Maximize">
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="1.5" /></svg>
    </button>
    <button id="close" class="window-button close-button" title="Close" aria-label="Close">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </div>
`;
document.body.appendChild(topbar);

const webviewStatus = document.createElement("div");
webviewStatus.id = "webview-status";
webviewStatus.textContent = "Chargement de YouTube Music…";
document.body.appendChild(webviewStatus);

const remoteWebview = new Webview(appWindow, "ytmusic", {
  url: "https://music.youtube.com",
  x: 0,
  y: TOPBAR_HEIGHT,
  width: Math.max(window.innerWidth, 1),
  height: Math.max(window.innerHeight - TOPBAR_HEIGHT, 1),
  userAgent:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
});

remoteWebview.once("tauri://error", (event) => {
  const message = String(event.payload ?? "erreur inconnue");
  webviewStatus.textContent = `Impossible de charger YouTube Music : ${message}`;
  webviewStatus.classList.add("error");
  console.error("Unable to create the YouTube Music webview", event.payload);
});

async function syncRemoteBounds(): Promise<void> {
  const width = Math.max(window.innerWidth, 1);
  const height = Math.max(window.innerHeight - TOPBAR_HEIGHT, 1);

  await Promise.all([
    remoteWebview.setPosition(new LogicalPosition(0, TOPBAR_HEIGHT)),
    remoteWebview.setSize(new LogicalSize(width, height)),
  ]);
}

remoteWebview.once("tauri://created", () => {
  void (async () => {
    await remoteWebview.show();
    await syncRemoteBounds();
    await remoteWebview.setFocus();
    webviewStatus.remove();
  })().catch((error) => {
    webviewStatus.textContent = `Impossible d’afficher YouTube Music : ${String(error)}`;
    webviewStatus.classList.add("error");
    console.error("Unable to show the YouTube Music webview", error);
  });
});

void appWindow
  .onResized(() => {
    void syncRemoteBounds().catch((error) => {
      console.error("Unable to resize the YouTube Music webview", error);
    });
  })
  .catch((error) => {
    console.error("Unable to listen for window resize events", error);
  });

document.getElementById("settings")?.addEventListener("click", () => {
  void invoke("open_settings").catch((error: unknown) => {
    console.error("Unable to open YouTube Music settings", error);
  });
});

document.getElementById("minimize")?.addEventListener("click", () => {
  void appWindow.minimize();
});

document.getElementById("maximize")?.addEventListener("click", () => {
  void appWindow.toggleMaximize();
});

document.getElementById("close")?.addEventListener("click", () => {
  void appWindow.close();
});

topbar.addEventListener("mousedown", (event) => {
  const target = event.target;
  if (!(target instanceof Element) || target.closest("button")) return;
  if (event.buttons !== 1) return;

  if (event.detail === 2) {
    void appWindow.toggleMaximize();
  } else {
    void appWindow.startDragging();
  }
});
