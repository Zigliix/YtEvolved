use discord_rich_presence::{
    activity::{Activity, ActivityType, Assets, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use serde::{Deserialize, Serialize};

/// Playback state of the current track, as reported by the YouTube Music
/// webview. Mirrors the states used by ytmdesktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
    Buffering,
    Unknown,
}

/// The currently playing track, as reported by the YouTube Music webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub video_id: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub thumbnail_url: Option<String>,
    /// Total duration of the track, in seconds.
    pub duration_seconds: Option<i64>,
    /// Current playback position, in seconds.
    pub position_seconds: f64,
    pub state: PlaybackState,
    /// Unix timestamp (milliseconds) when this track started playing.
    pub started_at: i64,
}

/// Truncates a string to Discord's 128 characters limit, appending "..." like
/// ytmdesktop does (see `stringLimit` in ytmdesktop's discord-presence).
fn string_limit(value: &str, limit: usize) -> String {
    if value.chars().count() > limit {
        let mut truncated: String = value.chars().take(limit.saturating_sub(3)).collect();
        truncated.push_str("...");
        truncated
    } else {
        value.to_string()
    }
}

/// Owns the Discord IPC connection and exposes simple high-level operations.
pub struct RichPresenceManager(DiscordIpcClient);

impl RichPresenceManager {
    /// Creates a client and connects to the local Discord IPC socket.
    pub fn connect(client_id: &str) -> Result<Self, String> {
        let mut client = DiscordIpcClient::new(client_id);
        client.connect().map_err(|error| error.to_string())?;
        Ok(Self(client))
    }

    /// Publishes the current track as the Discord activity, modeled after
    /// ytmdesktop's `DiscordPresence.playerStateChanged`:
    /// - `details` = song title, `state` = artist
    /// - progress bar via start/end timestamps while playing (no timestamps
    ///   while paused so the elapsed time does not keep counting)
    /// - album cover as the large image, album name as the hover text
    /// - a "Play on YouTube Music" button linking to the track
    pub fn update(&mut self, now_playing: &NowPlaying) -> Result<(), String> {
        if now_playing.title.trim().is_empty() {
            return self.clear();
        }

        let details = string_limit(now_playing.title.trim(), 128);
        let state = string_limit(
            now_playing.artist.as_deref().unwrap_or("").trim(),
            128,
        );
        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            .details(details)
            .state(state);

        // Progress bar (Discord requires both start and end timestamps for a
        // Listening activity to render one). When paused we omit them so the
        // elapsed time freezes, exactly like ytmdesktop.
        if let Some(duration) = now_playing.duration_seconds.filter(|d| *d > 0) {
            if now_playing.state == PlaybackState::Playing {
                let now = now_millis() / 1000;
                let position = now_playing.position_seconds.max(0.0) as i64;
                let remaining = (duration - position).max(0);
                activity = activity.timestamps(
                    Timestamps::new()
                        .start(now - position)
                        .end(now + remaining),
                );
            }
        }

        // Album cover. Discord accepts a direct image URL here (max 256 chars),
        // which is what ytmdesktop does with the highest resolution thumbnail.
        if let Some(url) = now_playing.thumbnail_url.as_deref() {
            if url.len() <= 256 && (url.starts_with("http://") || url.starts_with("https://")) {
                let mut assets = Assets::new().large_image(url);
                if let Some(album) = now_playing
                    .album
                    .as_deref()
                    .filter(|album| !album.trim().is_empty())
                {
                    let album_text = string_limit(album.trim(), 128);
                    assets = assets.large_text(album_text);
                }
                activity = activity.assets(assets);
            }
        }

        if let Some(video_id) = now_playing.video_id.as_deref() {
            if !video_id.is_empty() {
                activity = activity.buttons(vec![Button::new(
                    "Play on YouTube Music",
                    format!("https://music.youtube.com/watch?v={video_id}"),
                )]);
            }
        }

        self.0.set_activity(activity).map_err(|error| error.to_string())
    }

    /// Hides the activity from the Discord profile.
    pub fn clear(&mut self) -> Result<(), String> {
        self.0.clear_activity().map_err(|error| error.to_string())
    }
}

impl Drop for RichPresenceManager {
    fn drop(&mut self) {
        let _ = self.0.close();
    }
}

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
