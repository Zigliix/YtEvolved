use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("ytmusic")
        .ok_or_else(|| "YouTube Music n’est pas encore prêt".to_string())?;

    webview
        .eval("window.location.href = 'https://music.youtube.com/settings'")
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_settings])
        .setup(|app| {
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
