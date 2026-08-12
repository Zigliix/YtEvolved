import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { createTopbar } from "./titlebar";
import "./styles.css";

interface RichPresenceSettings {
  enabled: boolean;
  clientId: string;
}

interface AppSettings {
  richPresence: RichPresenceSettings;
}

interface RichPresenceStatus {
  enabled: boolean;
  hasClientId: boolean;
  connected: boolean;
  error: string | null;
}

interface NowPlaying {
  videoId: string | null;
  title: string;
  artist: string | null;
  album: string | null;
  thumbnailUrl: string | null;
  durationSeconds: number | null;
  positionSeconds: number;
  state: "playing" | "paused" | "buffering" | "unknown";
  startedAt: number;
}

document.body.appendChild(createTopbar("Settings"));

const content = document.createElement("main");
content.id = "settings-content";
content.innerHTML = `
  <section class="settings-card">
    <div class="settings-card-header">
      <div>
        <h1>Rich Presence Discord</h1>
        <p>Partage la chanson que tu écoutes dans ton profil Discord.</p>
      </div>
      <label class="switch" title="Activer la Rich Presence">
        <input type="checkbox" id="rp-toggle" />
        <span class="slider"></span>
      </label>
    </div>

    <label class="field">
      <span class="field-label">Client ID de l'application Discord</span>
      <input
        type="text"
        id="rp-client-id"
        class="text-input"
        placeholder="1234567890123456789"
        autocomplete="off"
        spellcheck="false"
      />
      <span class="field-hint">
        Crée une application sur le portail développeur Discord, puis copie son
        Application ID ici.
      </span>
    </label>

    <div class="rp-actions">
      <button id="rp-open-portal" class="link-button">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" /><path d="M15 3h6v6" /><path d="M10 14L21 3" /></svg>
        Ouvrir le portail Discord
      </button>
    </div>

    <div class="rp-status" id="rp-status">
      <span class="rp-status-dot"></span>
      <span class="rp-status-text">En attente…</span>
    </div>

    <div class="rp-now-playing" id="rp-now-playing">Aucune chanson détectée</div>
  </section>
`;
document.body.appendChild(content);

const toggle = document.getElementById("rp-toggle") as HTMLInputElement;
const clientIdInput = document.getElementById("rp-client-id") as HTMLInputElement;
const statusDot = document.querySelector<HTMLElement>(".rp-status-dot")!;
const statusText = document.querySelector<HTMLElement>(".rp-status-text")!;

let saveTimer: ReturnType<typeof setTimeout> | undefined;

function currentSettings(): AppSettings {
  return {
    richPresence: {
      enabled: toggle.checked,
      clientId: clientIdInput.value.trim(),
    },
  };
}

function renderStatus(status: RichPresenceStatus): void {
  statusDot.className = "rp-status-dot";
  if (!status.enabled) {
    statusText.textContent = "Rich Presence désactivée";
  } else if (!status.hasClientId) {
    statusDot.classList.add("warning");
    statusText.textContent = "Ajoute ton Client ID pour activer la Rich Presence";
  } else if (status.connected) {
    statusDot.classList.add("connected");
    statusText.textContent = "Connecté à Discord — ta chanson en cours est partagée";
  } else {
    statusDot.classList.add("error");
    statusText.textContent =
      status.error && status.error.trim()
        ? `Non connecté : ${status.error}`
        : "Non connecté — vérifie que Discord est lancé";
  }
}

async function saveSettings(): Promise<void> {
  try {
    const status = await invoke<RichPresenceStatus>("save_settings", {
      settings: currentSettings(),
    });
    renderStatus(status);
  } catch (error) {
    console.error("Unable to save settings", error);
  }
}

function scheduleSave(): void {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => void saveSettings(), 350);
}

void (async () => {
  try {
    const settings = await invoke<AppSettings>("load_settings");
    toggle.checked = settings.richPresence.enabled;
    clientIdInput.value = settings.richPresence.clientId;

    const status = await invoke<RichPresenceStatus>("get_rich_presence_status");
    renderStatus(status);
  } catch (error) {
    console.error("Unable to load settings", error);
  }
})();

toggle.addEventListener("change", scheduleSave);
clientIdInput.addEventListener("input", scheduleSave);

document.getElementById("rp-open-portal")?.addEventListener("click", () => {
  void openUrl("https://discord.com/developers/applications").catch((error: unknown) => {
    console.error("Unable to open the Discord developer portal", error);
  });
});

const nowPlayingEl = document.getElementById("rp-now-playing")!;

function stateIcon(state: NowPlaying["state"]): string {
  switch (state) {
    case "playing":
      return "▶";
    case "paused":
      return "⏸";
    case "buffering":
      return "⋯";
    default:
      return "🎵";
  }
}

function renderNowPlaying(nowPlaying: NowPlaying | null): void {
  if (nowPlaying && nowPlaying.title) {
    const icon = stateIcon(nowPlaying.state);
    const artist = nowPlaying.artist ? ` — ${nowPlaying.artist}` : "";
    const album = nowPlaying.album ? ` (${nowPlaying.album})` : "";
    nowPlayingEl.textContent = `${icon} ${nowPlaying.title}${artist}${album}`;
    nowPlayingEl.classList.add("active");
  } else {
    nowPlayingEl.textContent = "Aucune chanson détectée pour le moment";
    nowPlayingEl.classList.remove("active");
  }
}

setInterval(() => {
  void Promise.all([
    invoke<RichPresenceStatus>("get_rich_presence_status"),
    invoke<NowPlaying | null>("get_now_playing"),
  ])
    .then(([status, nowPlaying]) => {
      renderStatus(status);
      renderNowPlaying(nowPlaying);
    })
    .catch((error: unknown) => {
      console.error("Unable to refresh the rich presence status", error);
    });
}, 3000);
