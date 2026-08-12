use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn window_minimize(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn window_close(window: tauri::Window) {
    let _ = window.close();
}

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
        .invoke_handler(tauri::generate_handler![
            set_window_title,
            window_minimize,
            window_toggle_maximize,
            window_close
        ])
        .setup(|app| {
            let init_script = r###"
(function () {
    function injectTopbar() {
        if (document.getElementById('yte-topbar')) return;

        var style = document.createElement('style');
        style.id = 'yte-style';
        style.textContent = [
            '#yte-topbar {',
            '    position: fixed; top: 0; left: 0; right: 0; height: 40px;',
            '    background: #0d0d0d;',
            '    border-bottom: 1px solid rgba(255,255,255,0.07);',
            '    z-index: 2147483647;',
            '    display: flex; align-items: center;',
            '    padding: 0 6px 0 14px; box-sizing: border-box;',
            '    -webkit-app-region: drag;',
            '    font-family: -apple-system, "Segoe UI", system-ui, sans-serif;',
            '}',
            '#yte-branding {',
            '    display: flex; align-items: center; gap: 8px;',
            '    flex-shrink: 0; pointer-events: none;',
            '}',
            '#yte-branding svg { width: 16px; height: 16px; flex-shrink: 0; }',
            '#yte-title {',
            '    font-size: 11.5px; font-weight: 600;',
            '    letter-spacing: 0.25px; color: rgba(255,255,255,0.7); white-space: nowrap;',
            '}',
            '#yte-spacer { flex: 1; }',
            '#yte-controls {',
            '    display: flex; align-items: center; gap: 1px;',
            '    -webkit-app-region: no-drag; flex-shrink: 0;',
            '}',
            '.yte-sep { width: 1px; height: 14px; background: rgba(255,255,255,0.09); margin: 0 5px; }',
            '.yte-btn {',
            '    width: 34px; height: 30px; background: transparent; border: none;',
            '    border-radius: 5px; color: rgba(255,255,255,0.55); cursor: pointer;',
            '    display: flex; align-items: center; justify-content: center;',
            '    transition: background 0.12s ease, color 0.12s ease;',
            '    -webkit-app-region: no-drag; flex-shrink: 0; padding: 0; outline: none;',
            '}',
            '.yte-btn:hover { background: rgba(255,255,255,0.08); color: rgba(255,255,255,0.9); }',
            '.yte-btn svg { width: 13px; height: 13px; display: block; }',
            '#yte-btn-close:hover { background: #c42b1c; color: #fff; }',
            'ytmusic-app { margin-top: 40px !important; height: calc(100% - 40px) !important; }'
        ].join('\n');
        document.head.appendChild(style);

        var ytmLogo = '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">'
            + '<circle cx="12" cy="12" r="10" fill="#ff0000"/>'
            + '<polygon points="10,8 16,12 10,16" fill="white"/>'
            + '</svg>';

        var iconSettings = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">'
            + '<circle cx="12" cy="12" r="3"/>'
            + '<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06-.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>'
            + '</svg>';

        var iconMin = '<svg viewBox="0 0 24 24" fill="currentColor">'
            + '<rect x="4" y="11.25" width="16" height="1.5" rx="0.75"/>'
            + '</svg>';

        var iconMax = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">'
            + '<rect x="4.5" y="4.5" width="15" height="15" rx="2"/>'
            + '</svg>';

        var iconClose = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">'
            + '<line x1="18" y1="6" x2="6" y2="18"/>'
            + '<line x1="6" y1="6" x2="18" y2="18"/>'
            + '</svg>';

        var bar = document.createElement('div');
        bar.id = 'yte-topbar';
        bar.innerHTML = '<div id="yte-branding">' + ytmLogo + '<span id="yte-title">YouTube Music Evolved</span></div>'
            + '<div id="yte-spacer"></div>'
            + '<div id="yte-controls">'
            + '<button class="yte-btn" id="yte-btn-settings" title="Settings">' + iconSettings + '</button>'
            + '<div class="yte-sep"></div>'
            + '<button class="yte-btn" id="yte-btn-min" title="Minimize">' + iconMin + '</button>'
            + '<button class="yte-btn" id="yte-btn-max" title="Maximize">' + iconMax + '</button>'
            + '<button class="yte-btn" id="yte-btn-close" title="Close">' + iconClose + '</button>'
            + '</div>';

        document.body.prepend(bar);

        document.getElementById('yte-btn-min').addEventListener('click', function () {
            window.__TAURI_INTERNALS__.invoke('window_minimize');
        });
        document.getElementById('yte-btn-max').addEventListener('click', function () {
            window.__TAURI_INTERNALS__.invoke('window_toggle_maximize');
        });
        document.getElementById('yte-btn-close').addEventListener('click', function () {
            window.__TAURI_INTERNALS__.invoke('window_close');
        });
    }

    function updateTitle() {
        try {
            var songEl = document.querySelector('ytmusic-player-bar .title');
            var artistEl = document.querySelector('ytmusic-player-bar .byline');
            var songTitle = '';

            if (songEl && songEl.textContent && songEl.textContent.trim()) {
                var titleText = songEl.textContent.trim();
                var artistText = artistEl && artistEl.textContent
                    ? artistEl.textContent.trim().split('\u{2022}')[0].trim()
                    : '';
                songTitle = artistText ? titleText + ' \u{2022} ' + artistText : titleText;
            } else if (document.title && document.title !== 'YouTube Music') {
                songTitle = document.title.replace(/\s*-\s*YouTube Music$/i, '').trim();
            }

            var target = songTitle ? songTitle + ' - YouTube Music Evolved' : 'YouTube Music Evolved';
            if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                window.__TAURI_INTERNALS__.invoke('plugin:window|set_title', { value: target }).catch(function () {});
            }
        } catch (e) {}
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', injectTopbar);
    } else {
        injectTopbar();
    }

    setInterval(updateTitle, 1000);
    updateTitle();
})();
            "###;

            let url = WebviewUrl::External("https://music.youtube.com".parse().unwrap());

            WebviewWindowBuilder::new(app, "main", url)
                .title("YouTube Music Evolved")
                .inner_size(1280.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .decorations(false)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
                .initialization_script(init_script)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
