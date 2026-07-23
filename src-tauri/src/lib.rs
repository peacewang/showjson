use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

const SHOW_SHORTCUT: &str = "CommandOrControl+Shift+J";
const MAX_HISTORY_ITEMS: usize = 50;
const MAX_HISTORY_ENTRY_BYTES: u64 = 5 * 1024 * 1024;
const MAX_HISTORY_TOTAL_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClipboardPayload {
    text: String,
    source: &'static str,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryEntry {
    id: String,
    created_at: u64,
    preview: String,
    bytes: u64,
    kind: String,
    valid: bool,
    text: String,
    fingerprint: String,
}

#[derive(Clone, Default)]
struct HistoryState {
    lock: Arc<Mutex<()>>,
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("clipboard-history.json"))
        .map_err(|error| format!("无法定位历史目录：{error}"))
}

fn load_history_file(app: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(&path).map_err(|error| format!("无法读取剪贴板历史：{error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("剪贴板历史文件已损坏：{error}"))
}

fn write_history_file(app: &AppHandle, entries: &[HistoryEntry]) -> Result<(), String> {
    let path = history_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "剪贴板历史路径无效".to_string())?;
    fs::create_dir_all(directory).map_err(|error| format!("无法创建历史目录：{error}"))?;

    let json =
        serde_json::to_vec(entries).map_err(|error| format!("无法序列化剪贴板历史：{error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json).map_err(|error| format!("无法写入剪贴板历史：{error}"))?;

    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("无法更新剪贴板历史：{error}"))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("无法保存剪贴板历史：{error}"))
}

fn reveal_window(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

#[tauri::command]
async fn load_history(
    app: AppHandle,
    state: State<'_, HistoryState>,
) -> Result<Vec<HistoryEntry>, String> {
    let lock = state.lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|error| format!("无法锁定剪贴板历史：{error}"))?;
        load_history_file(&app)
    })
    .await
    .map_err(|error| format!("读取历史任务失败：{error}"))?
}

#[tauri::command]
async fn save_history_entry(
    app: AppHandle,
    state: State<'_, HistoryState>,
    entry: HistoryEntry,
) -> Result<Vec<HistoryEntry>, String> {
    let lock = state.lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|error| format!("无法锁定剪贴板历史：{error}"))?;
        if entry.bytes > MAX_HISTORY_ENTRY_BYTES {
            return Err("单条内容超过 5 MB，未加入剪贴板历史。".to_string());
        }

        let mut entries = load_history_file(&app)?;
        entries.retain(|existing| {
            existing.id != entry.id && existing.fingerprint != entry.fingerprint
        });
        entries.insert(0, entry);

        let mut total_bytes = 0_u64;
        entries.retain(|item| {
            if total_bytes.saturating_add(item.bytes) > MAX_HISTORY_TOTAL_BYTES {
                return false;
            }
            total_bytes += item.bytes;
            true
        });
        entries.truncate(MAX_HISTORY_ITEMS);

        write_history_file(&app, &entries)?;
        Ok(entries)
    })
    .await
    .map_err(|error| format!("保存历史任务失败：{error}"))?
}

#[tauri::command]
async fn delete_history_entry(
    app: AppHandle,
    state: State<'_, HistoryState>,
    id: String,
) -> Result<Vec<HistoryEntry>, String> {
    let lock = state.lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|error| format!("无法锁定剪贴板历史：{error}"))?;
        let mut entries = load_history_file(&app)?;
        entries.retain(|entry| entry.id != id);
        write_history_file(&app, &entries)?;
        Ok(entries)
    })
    .await
    .map_err(|error| format!("删除历史任务失败：{error}"))?
}

#[tauri::command]
async fn clear_history(app: AppHandle, state: State<'_, HistoryState>) -> Result<(), String> {
    let lock = state.lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|error| format!("无法锁定剪贴板历史：{error}"))?;
        let path = history_path(&app)?;
        if path.exists() {
            fs::remove_file(path).map_err(|error| format!("无法清空剪贴板历史：{error}"))?;
        }
        Ok(())
    })
    .await
    .map_err(|error| format!("清空历史任务失败：{error}"))?
}

fn reveal_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        reveal_window(&window);
    }
}

fn read_clipboard(app: &AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|error| format!("无法读取剪贴板：{error}"))
}

fn capture_clipboard(app: AppHandle, source: &'static str) {
    tauri::async_runtime::spawn(async move {
        let clipboard_app = app.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || read_clipboard(&clipboard_app)).await;

        match result {
            Ok(Ok(text)) => {
                reveal_main_window(&app);
                let _ = app.emit("showjson://clipboard", ClipboardPayload { text, source });
            }
            Ok(Err(message)) => {
                reveal_main_window(&app);
                let _ = app.emit("showjson://error", message);
            }
            Err(error) => {
                reveal_main_window(&app);
                let _ = app.emit("showjson://error", format!("剪贴板任务失败：{error}"));
            }
        }
    });
}

#[tauri::command]
async fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.clipboard()
            .write_text(text)
            .map_err(|error| format!("无法写入剪贴板：{error}"))
    })
    .await
    .map_err(|error| format!("剪贴板任务失败：{error}"))?
}

#[tauri::command]
fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window
        .hide()
        .map_err(|error| format!("无法隐藏窗口：{error}"))
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示 ShowJSON", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "隐藏窗口", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 ShowJSON", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("showjson-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("ShowJSON")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => reveal_main_window(app),
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                reveal_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HistoryState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            reveal_main_window(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            build_tray(app)?;

            #[cfg(desktop)]
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcut(SHOW_SHORTCUT)?
                    .with_handler(|app, shortcut, event| {
                        if event.state == ShortcutState::Pressed
                            && shortcut.matches(
                                if cfg!(target_os = "macos") {
                                    Modifiers::SUPER | Modifiers::SHIFT
                                } else {
                                    Modifiers::CONTROL | Modifiers::SHIFT
                                },
                                Code::KeyJ,
                            )
                        {
                            capture_clipboard(app.clone(), "shortcut");
                        }
                    })
                    .build(),
            )?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            } else if let WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            copy_text,
            hide_window,
            load_history,
            save_history_entry,
            delete_history_entry,
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running ShowJSON");
}
