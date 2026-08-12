import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const LOGO = `<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="10" fill="#ff0000" /><path d="M10 8.25L16 12L10 15.75V8.25Z" fill="white" /></svg>`;

const ICON_SETTINGS = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" /><path d="M19.4 15a1.8 1.8 0 0 0 .36 1.99l.05.05-1.41 1.41-.05-.05a1.8 1.8 0 0 0-1.99-.36 1.8 1.8 0 0 0-1.09 1.65V20h-2v-.31a1.8 1.8 0 0 0-1.09-1.65 1.8 1.8 0 0 0-1.99.36l-.05.05-1.41-1.41.05-.05A1.8 1.8 0 0 0 9.14 15a1.8 1.8 0 0 0-1.65-1.09H7v-2h.49A1.8 1.8 0 0 0 9.14 11a1.8 1.8 0 0 0-.36-1.99l-.05-.05 1.41-1.41.05.05a1.8 1.8 0 0 0 1.99.36 1.8 1.8 0 0 0 1.09-1.65V6h2v.31a1.8 1.8 0 0 0 1.09 1.65 1.8 1.8 0 0 0 1.99-.36l.05-.05 1.41 1.41-.05.05A1.8 1.8 0 0 0 19.4 11a1.8 1.8 0 0 0 1.65 1.09h.31v2h-.31A1.8 1.8 0 0 0 19.4 15Z" /></svg>`;

const ICON_MIN = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>`;

const ICON_MAX = `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="1.5" /></svg>`;

const ICON_CLOSE = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>`;

/**
 * Builds the custom window topbar (shared by the main window and the
 * settings window) and wires up the window controls and dragging.
 */
export function createTopbar(title: string, withSettingsButton = false): HTMLElement {
  const settingsPart = withSettingsButton
    ? `<button id="settings" class="window-button" title="Settings" aria-label="Settings">${ICON_SETTINGS}</button><span class="control-separator" aria-hidden="true"></span>`
    : "";

  const topbar = document.createElement("header");
  topbar.id = "titlebar";
  topbar.innerHTML = `
    <div class="branding">
      <span class="logo" aria-hidden="true">${LOGO}</span>
      <span class="title">${title}</span>
    </div>
    <div class="drag-space"></div>
    <div class="controls">
      ${settingsPart}
      <button id="minimize" class="window-button" title="Minimize" aria-label="Minimize">${ICON_MIN}</button>
      <button id="maximize" class="window-button" title="Maximize" aria-label="Maximize">${ICON_MAX}</button>
      <button id="close" class="window-button close-button" title="Close" aria-label="Close">${ICON_CLOSE}</button>
    </div>
  `;

  topbar.querySelector("#minimize")?.addEventListener("click", () => {
    void appWindow.minimize();
  });

  topbar.querySelector("#maximize")?.addEventListener("click", () => {
    void appWindow.toggleMaximize();
  });

  topbar.querySelector("#close")?.addEventListener("click", () => {
    void appWindow.close();
  });

  if (withSettingsButton) {
    topbar.querySelector("#settings")?.addEventListener("click", () => {
      void invoke("open_settings").catch((error: unknown) => {
        console.error("Unable to open the settings window", error);
      });
    });
  }

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

  return topbar;
}
