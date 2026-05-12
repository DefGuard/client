use tauri::{webview::WebviewWindowBuilder, AppHandle, Manager, WebviewUrl};

#[tauri::command]
pub async fn open_new_ui_window(app: AppHandle) {
    let url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/index.html".into())
    };

    let _window = WebviewWindowBuilder::new(&app, "new-ui", url)
        .title("New UI")
        .inner_size(1000.0, 800.0)
        .build()
        .unwrap();
}

#[tauri::command]
pub async fn open_old_ui_window(app: AppHandle) {
    let url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    };

    let _window = WebviewWindowBuilder::new(&app, "old-ui", url)
        .title("Old UI")
        .inner_size(1000.0, 800.0)
        .build()
        .unwrap();
}

#[tauri::command]
pub async fn swap_to_old_ui(app: AppHandle) {
    let url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:5071".parse().unwrap())
    } else {
        WebviewUrl::App("old-ui/index.html".into())
    };
    WebviewWindowBuilder::new(&app, "old-ui", url)
        .title("Old UI")
        .inner_size(1000.0, 800.0)
        .build()
        .unwrap();
    if let Some(w) = app.get_webview_window("new-ui") {
        w.close().unwrap();
    }
}

#[tauri::command]
pub async fn swap_to_new_ui(app: AppHandle) {
    let url = if cfg!(debug_assertions) {
        WebviewUrl::External("http://localhost:5072".parse().unwrap())
    } else {
        WebviewUrl::App("new-ui/index.html".into())
    };
    WebviewWindowBuilder::new(&app, "new-ui", url)
        .title("New UI")
        .inner_size(1000.0, 800.0)
        .build()
        .unwrap();
    if let Some(w) = app.get_webview_window("old-ui") {
        w.close().unwrap();
    }
}
