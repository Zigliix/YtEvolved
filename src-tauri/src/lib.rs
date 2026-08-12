use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn set_window_title(window: tauri::Window, title: String) {
    let clean = title.trim();
    let formatted = if clean.is_empty() || clean == "YouTube Music" {
        "YouTube Music Evolved".to_string()
    } else {
        clean.replace(" - YouTube Music", "")
    };
    let _ = window.set_title(&formatted);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_window_title])
        .setup(|app| {
            let init_script = r#"
                (function() {
                    function updateWindow() {
                        try {
                            let songTitle = '';
                            let songElement = document.querySelector('ytmusic-player-bar .title');
                            let artistElement = document.querySelector('ytmusic-player-bar .byline');
                            
                            if (songElement && songElement.textContent && songElement.textContent.trim()) {
                                let titleText = songElement.textContent.trim();
                                let artistText = artistElement && artistElement.textContent ? artistElement.textContent.trim() : '';
                                if (artistText) {
                                    artistText = artistText.split('•')[0].trim();
                                }
                                if (artistText) {
                                    songTitle = titleText + ' • ' + artistText;
                                } else {
                                    songTitle = titleText;
                                }
                            } else if (document.title && document.title !== 'YouTube Music') {
                                songTitle = document.title.replace(/\s*-\s*YouTube Music$/i, '').trim();
                            }

                            let targetTitle = songTitle ? (songTitle + ' - YouTube Music Evolved') : 'YouTube Music Evolved';
                            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                                window.__TAURI_INTERNALS__.invoke('plugin:window|set_title', { value: targetTitle })
                                    .catch(function(e) { console.error("Erreur IPC Tauri :", e); });
                            } else {
                                console.warn("Tauri IPC non détecté sur music.youtube.com");
                            }
                        } catch(e) {}
                    }

                    if (document.readyState === 'loading') {
                        window.addEventListener('DOMContentLoaded', function() {
                            setInterval(updateWindow, 1000);
                            updateWindow();
                        });
                    } else {
                        setInterval(updateWindow, 1000);
                        updateWindow();
                    }
                })();
            "#;

            let url = WebviewUrl::External("https://music.youtube.com".parse().unwrap());
            
            let builder = WebviewWindowBuilder::new(app, "main", url)
                .title("YouTube Music Evolved")
                .inner_size(1280.0, 800.0)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
                .initialization_script(init_script);

            builder.build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
