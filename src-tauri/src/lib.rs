mod rich_presence;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use rich_presence::{now_millis, NowPlaying, PlaybackState, RichPresenceManager};

/// Maximum connection attempts before giving up, like ytmdesktop.
const CONNECTION_MAX_ATTEMPTS: u32 = 30;
/// Delay between connection attempts, like ytmdesktop.
const CONNECTION_RETRY_DELAY_SECS: u64 = 5;
/// While paused/buffering, Discord keeps showing the track for this long
/// before the activity is cleared (ytmdesktop behaviour).
const PAUSE_CLEAR_DELAY_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    rich_presence: RichPresenceSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            rich_presence: RichPresenceSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RichPresenceSettings {
    enabled: bool,
    client_id: String,
}

impl Default for RichPresenceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RichPresenceStatus {
    enabled: bool,
    has_client_id: bool,
    connected: bool,
    error: Option<String>,
}

struct AppState {
    settings: Mutex<AppSettings>,
    rich_presence: Mutex<Option<RichPresenceManager>>,
    now_playing: Mutex<Option<NowPlaying>>,
    last_error: Mutex<Option<String>>,
    /// True while a background task is (re)connecting to Discord, to avoid
    /// spawning concurrent connection attempts.
    connecting: AtomicBool,
    /// Monotonic counter bumped on every pause/play transition. A pending
    /// "clear activity after 30s of pause" task only acts if its generation
    /// is still the latest one.
    pause_generation: AtomicU64,
}

impl AppState {
    fn status(&self) -> RichPresenceStatus {
        let settings = self.settings.lock().unwrap();
        let connected = self.rich_presence.lock().unwrap().is_some();
        let error = self.last_error.lock().unwrap().clone();
        RichPresenceStatus {
            enabled: settings.rich_presence.enabled,
            has_client_id: !settings.rich_presence.client_id.trim().is_empty(),
            connected,
            error,
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join("settings.json"))
}

fn load_settings_from_disk(app: &AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Reports the current track to the `set_now_playing` command (used by the
/// Discord Rich Presence).
///
/// Modeled after ytmdesktop: it reads the player API (`playerApi` /
/// `getPlayerResponse`) for reliable metadata, listens to `<video>` events
/// for accurate playback state and position, and falls back to the DOM
/// selectors and the document title if the player API is not available yet.
const NOW_PLAYING_POLLER: &str = r#"
(function () {
  if (window.__yteNowPlayingPoller) return;
  window.__yteNowPlayingPoller = true;

  var lastKey = "";
  var lastState = "";
  var lastDuration = 0;

  function send(payload) {
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
      window.__TAURI_INTERNALS__.invoke("set_now_playing", payload).catch(function () {});
    }
  }

  function videoEl() {
    return document.querySelector("video") || null;
  }

  function playerResponse() {
    try {
      var bar = document.querySelector("ytmusic-player-bar");
      if (bar && bar.playerApi && typeof bar.playerApi.getPlayerResponse === "function") {
        return bar.playerApi.getPlayerResponse();
      }
    } catch (e) {}
    return null;
  }

  function titleFromDocument() {
    var t = document.title || "";
    t = t.replace(/\s*-\s*YouTube Music\s*$/i, "").trim();
    if (!t || t === "YouTube Music") return { title: "", artist: "" };
    var idx = t.lastIndexOf(" - ");
    if (idx > 0) {
      return { title: t.slice(0, idx).trim(), artist: t.slice(idx + 3).trim() };
    }
    return { title: t, artist: "" };
  }

  function readNowPlaying(force) {
    try {
      var video = videoEl();
      var pr = playerResponse();
      var videoDetails = pr && pr.videoDetails ? pr.videoDetails : null;

      var title = "";
      var artist = "";
      var videoId = "";
      var durationSeconds = 0;
      var thumbnailUrl = "";
      var album = "";

      if (videoDetails) {
        title = videoDetails.title || "";
        artist = videoDetails.author || "";
        videoId = videoDetails.videoId || "";
        durationSeconds = parseInt(videoDetails.lengthSeconds || "0", 10) || 0;
        var thumbs = videoDetails.thumbnail && videoDetails.thumbnail.thumbnails;
        if (thumbs && thumbs.length) {
          var best = null;
          for (var i = 0; i < thumbs.length; i++) {
            var th = thumbs[i];
            if (!best || (th.width * th.height > best.width * best.height)) best = th;
          }
          if (best && best.url) thumbnailUrl = best.url;
        }
      }

      // Byline ("Artist • Album • Year") gives us the album and a DOM
      // fallback for the artist when the player API is not ready yet.
      var bylineEl = document.querySelector("ytmusic-player-bar .byline");
      var byline = bylineEl && bylineEl.textContent ? bylineEl.textContent.trim() : "";
      var bylineParts = byline.split("\u{2022}").map(function (s) { return s.trim(); }).filter(Boolean);

      if (!title) {
        var songEl = document.querySelector("ytmusic-player-bar .title");
        title = songEl && songEl.textContent ? songEl.textContent.trim() : "";
        if (!title) {
          var fallback = titleFromDocument();
          title = fallback.title;
          artist = fallback.artist;
        }
        if (!artist) artist = bylineParts[0] || "";
        var link = document.querySelector("ytmusic-player-bar a[href*='watch?v=']");
        if (link) {
          var m = link.href.match(/[?&]v=([\w-]{11})/);
          if (m) videoId = m[1];
        }
        var img = document.querySelector("ytmusic-player-bar img");
        if (img && img.src) thumbnailUrl = img.src;
      }

      if (!album && bylineParts.length >= 2) album = bylineParts[1];

      var position = 0;
      if (video) {
        position = video.currentTime || 0;
        if (!durationSeconds && video.duration && isFinite(video.duration)) {
          durationSeconds = Math.floor(video.duration);
        }
      }

      var state = "unknown";
      if (video) {
        if (video.paused) state = "paused";
        else if (video.readyState < 3) state = "buffering";
        else state = "playing";
      }

      var key = videoId || (title + "\u0000" + artist);
      if (!force && key === lastKey && state === lastState && durationSeconds === lastDuration) return;
      lastKey = key;
      lastState = state;
      lastDuration = durationSeconds;

      send({
        videoId: videoId || null,
        title: title,
        artist: artist || null,
        album: album || null,
        thumbnailUrl: thumbnailUrl || null,
        durationSeconds: durationSeconds || null,
        positionSeconds: position,
        state: state
      });
    } catch (e) {}
  }

  function hookVideo() {
    var video = videoEl();
    if (!video || video.__yteHooked) return;
    video.__yteHooked = true;
    video.addEventListener("play", function () { readNowPlaying(); });
    video.addEventListener("pause", function () { readNowPlaying(); });
    video.addEventListener("ended", function () { readNowPlaying(); });
    // A seek changes the position, so force a refresh of the end timestamp.
    video.addEventListener("seeked", function () { readNowPlaying(true); });
    video.addEventListener("durationchange", function () { readNowPlaying(); });
    video.addEventListener("loadedmetadata", function () { readNowPlaying(); });
  }

  // Track changes: watch the player bar DOM (or the whole document while the
  // bar is not rendered yet). Debounced because YouTube Music mutates the DOM
  // very frequently and playerApi.getPlayerResponse() is expensive.
  var debounceTimer = null;
  var observer = new MutationObserver(function () {
    if (debounceTimer) return;
    debounceTimer = setTimeout(function () {
      debounceTimer = null;
      readNowPlaying();
      hookVideo();
    }, 150);
  });
  observer.observe(document.body || document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });

  // Safety net: re-run periodically in case the player API or the video
  // element appears after the observer was attached.
  setInterval(function () {
    readNowPlaying();
    hookVideo();
  }, 2000);

  readNowPlaying();
  hookVideo();
})();
"#;

#[tauri::command]
fn inject_now_playing_poller(app: AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("ytmusic")
        .ok_or_else(|| "La webview YouTube Music n'est pas prête".to_string())?;

    webview.eval(NOW_PLAYING_POLLER).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("YouTube Music Evolved — Settings")
        .inner_size(500.0, 680.0)
        .min_inner_size(440.0, 560.0)
        .decorations(false)
        .build()
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
fn load_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

/// Connects to Discord in a background thread, retrying every few seconds
/// (up to 30 attempts) like ytmdesktop. Stops if the feature gets disabled.
fn spawn_rich_presence_connection(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let state = handle.state::<AppState>();

        // Only one connection attempt at a time.
        if state.connecting.swap(true, Ordering::SeqCst) {
            return;
        }

        let client_id = {
            let settings = state.settings.lock().unwrap();
            if !settings.rich_presence.enabled {
                state.connecting.store(false, Ordering::SeqCst);
                return;
            }
            settings.rich_presence.client_id.trim().to_string()
        };

        if client_id.is_empty() {
            state.connecting.store(false, Ordering::SeqCst);
            return;
        }

        for attempt in 0..CONNECTION_MAX_ATTEMPTS {
            // The user may have disabled Rich Presence while we were waiting.
            let enabled = {
                let settings = state.settings.lock().unwrap();
                settings.rich_presence.enabled
            };
            if !enabled {
                break;
            }

            match RichPresenceManager::connect(&client_id) {
                Ok(client) => {
                    // The user may have disabled Rich Presence while we were
                    // blocked in connect(): drop the client in that case.
                    let enabled = {
                        let settings = state.settings.lock().unwrap();
                        settings.rich_presence.enabled
                    };
                    if !enabled {
                        break;
                    }
                    *state.rich_presence.lock().unwrap() = Some(client);
                    *state.last_error.lock().unwrap() = None;
                    state.connecting.store(false, Ordering::SeqCst);
                    push_now_playing(&handle);
                    return;
                }
                Err(error) => {
                    if attempt + 1 >= CONNECTION_MAX_ATTEMPTS {
                        *state.last_error.lock().unwrap() = Some(error);
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(CONNECTION_RETRY_DELAY_SECS));
        }

        state.connecting.store(false, Ordering::SeqCst);
    });
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<RichPresenceStatus, String> {
    let path = settings_path(&app)?;
    let json = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| error.to_string())?;

    *state.settings.lock().unwrap() = settings.clone();

    // Drop any existing connection, cancel pending pause-clears and reset the
    // last error before (re)connecting.
    *state.rich_presence.lock().unwrap() = None;
    *state.last_error.lock().unwrap() = None;
    state.connecting.store(false, Ordering::SeqCst);
    state.pause_generation.fetch_add(1, Ordering::SeqCst);

    let client_id = settings.rich_presence.client_id.trim().to_string();
    if settings.rich_presence.enabled && !client_id.is_empty() {
        spawn_rich_presence_connection(&app);
    }

    Ok(state.status())
}

/// Re-publishes the latest known track to Discord after a (re)connection.
fn push_now_playing(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(now_playing) = state.now_playing.lock().unwrap().clone() else {
        return;
    };

    let update_failed = {
        let mut guard = state.rich_presence.lock().unwrap();
        match guard.as_mut() {
            Some(client) => client.update(&now_playing).is_err(),
            None => false,
        }
    };

    if update_failed {
        *state.rich_presence.lock().unwrap() = None;
        spawn_rich_presence_connection(app);
    }
}

#[tauri::command]
fn set_now_playing(
    app: AppHandle,
    state: State<'_, AppState>,
    video_id: Option<String>,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    thumbnail_url: Option<String>,
    duration_seconds: Option<i64>,
    position_seconds: Option<f64>,
    playback_state: PlaybackState,
) {
    let artist = artist.filter(|value| !value.trim().is_empty());
    let album = album.filter(|value| !value.trim().is_empty());

    // No track playing: forget the current one and hide the presence.
    if title.trim().is_empty() {
        *state.now_playing.lock().unwrap() = None;
        state.pause_generation.fetch_add(1, Ordering::SeqCst);
        if let Some(client) = state.rich_presence.lock().unwrap().as_mut() {
            let _ = client.clear();
        }
        return;
    }

    let mut last = state.now_playing.lock().unwrap();
    let started_at = match last.as_ref() {
        Some(previous)
            if previous.video_id == video_id
                && previous.title == title
                && previous.artist == artist =>
        {
            previous.started_at
        }
        _ => now_millis(),
    };
    let now_playing = NowPlaying {
        video_id: video_id.filter(|value| !value.trim().is_empty()),
        title: title.trim().to_string(),
        artist,
        album,
        thumbnail_url: thumbnail_url.filter(|value| !value.trim().is_empty()),
        duration_seconds: duration_seconds.filter(|duration| *duration > 0),
        position_seconds: position_seconds.unwrap_or(0.0),
        state: playback_state,
        started_at,
    };
    *last = Some(now_playing.clone());
    drop(last);

    let update_failed = {
        let mut guard = state.rich_presence.lock().unwrap();
        match guard.as_mut() {
            Some(client) => client.update(&now_playing).is_err(),
            None => false,
        }
    };

    // While paused/buffering, keep the track visible for a while, then hide
    // it (ytmdesktop behaviour). Starting playback cancels the pending clear.
    let is_paused = matches!(
        now_playing.state,
        PlaybackState::Paused | PlaybackState::Buffering | PlaybackState::Unknown
    );
    if is_paused {
        let generation = state.pause_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(PAUSE_CLEAR_DELAY_SECS));
            let state = handle.state::<AppState>();
            if state.pause_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            {
                let mut guard = state.rich_presence.lock().unwrap();
                if let Some(client) = guard.as_mut() {
                    let _ = client.clear();
                }
                // Forget the track too, so a later reconnect does not
                // re-publish a presence we deliberately hid.
                *state.now_playing.lock().unwrap() = None;
            }
        });
    } else {
        state.pause_generation.fetch_add(1, Ordering::SeqCst);
    }

    // The IPC socket died (e.g. Discord was closed): drop the client and let
    // the background task reconnect, like ytmdesktop's retry logic.
    if update_failed {
        *state.rich_presence.lock().unwrap() = None;
        spawn_rich_presence_connection(&app);
    }
}

#[tauri::command]
fn get_rich_presence_status(state: State<'_, AppState>) -> RichPresenceStatus {
    state.status()
}

#[tauri::command]
fn get_now_playing(state: State<'_, AppState>) -> Option<NowPlaying> {
    state.now_playing.lock().unwrap().clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            settings: Mutex::new(AppSettings::default()),
            rich_presence: Mutex::new(None),
            now_playing: Mutex::new(None),
            last_error: Mutex::new(None),
            connecting: AtomicBool::new(false),
            pause_generation: AtomicU64::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            open_settings,
            load_settings,
            save_settings,
            set_now_playing,
            get_rich_presence_status,
            get_now_playing,
            inject_now_playing_poller
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let settings = load_settings_from_disk(app.handle());
            *state.settings.lock().unwrap() = settings.clone();

            if settings.rich_presence.enabled
                && !settings.rich_presence.client_id.trim().is_empty()
            {
                spawn_rich_presence_connection(app.handle());
            }

            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("YouTube Music Evolved")
                .inner_size(1280.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .decorations(false)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
