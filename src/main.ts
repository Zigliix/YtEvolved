import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { Webview } from "@tauri-apps/api/webview";
import { createTopbar } from "./titlebar";
import "./styles.css";

const TOPBAR_HEIGHT = 40;
const appWindow = getCurrentWindow();

document.body.appendChild(createTopbar("YouTube Music Evolved", true));

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
    await invoke("inject_now_playing_poller");
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

// Re-inject the now playing poller periodically. An eval-based script is
// destroyed when the webview navigates (e.g. when YouTube Music finishes
// loading after the webview is created), so we re-inject it to make sure it
// is always active in the loaded page.
setInterval(() => {
  void invoke("inject_now_playing_poller").catch((error: unknown) => {
    console.error("Unable to re-inject the now playing poller", error);
  });
}, 3000);
