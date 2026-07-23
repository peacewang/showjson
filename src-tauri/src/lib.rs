mod selection;

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const DEFAULT_SHOW_SHORTCUT: &str = "CommandOrControl+Shift+J";
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

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    shortcut: String,
    capture_selection: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHOW_SHORTCUT.to_string(),
            capture_selection: false,
        }
    }
}

#[derive(Clone)]
struct SettingsState {
    current: Arc<Mutex<AppSettings>>,
}

impl SettingsState {
    fn new(settings: AppSettings) -> Self {
        Self {
            current: Arc::new(Mutex::new(settings)),
        }
    }
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("clipboard-history.json"))
        .map_err(|error| format!("无法定位历史目录：{error}"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("settings.json"))
        .map_err(|error| format!("无法定位设置目录：{error}"))
}

fn load_settings_file(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let text = fs::read_to_string(&path).map_err(|error| format!("无法读取设置：{error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("设置文件已损坏：{error}"))
}

fn write_settings_file(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let directory = path.parent().ok_or_else(|| "设置路径无效".to_string())?;
    fs::create_dir_all(directory).map_err(|error| format!("无法创建设置目录：{error}"))?;

    let json =
        serde_json::to_vec_pretty(settings).map_err(|error| format!("无法序列化设置：{error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json).map_err(|error| format!("无法写入设置：{error}"))?;

    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("无法更新设置：{error}"))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("无法保存设置：{error}"))
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
async fn clear_history(
    app: AppHandle,
    state: State<'_, HistoryState>,
) -> Result<Vec<HistoryEntry>, String> {
    let lock = state.lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock
            .lock()
            .map_err(|error| format!("无法锁定剪贴板历史：{error}"))?;
        let path = history_path(&app)?;
        if path.exists() {
            fs::remove_file(path).map_err(|error| format!("无法清空剪贴板历史：{error}"))?;
        }
        Ok(Vec::new())
    })
    .await
    .map_err(|error| format!("清空历史任务失败：{error}"))?
}

#[tauri::command]
fn get_settings(state: State<'_, SettingsState>) -> Result<AppSettings, String> {
    state
        .current
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| format!("无法读取设置：{error}"))
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    shortcut: String,
    capture_selection: bool,
) -> Result<AppSettings, String> {
    let normalized_shortcut = shortcut.trim();
    if normalized_shortcut.is_empty() {
        return Err("快捷键不能为空".to_string());
    }

    let mut current = state
        .current
        .lock()
        .map_err(|error| format!("无法锁定设置：{error}"))?;
    let previous = current.clone();
    let updated = AppSettings {
        shortcut: normalized_shortcut.to_string(),
        capture_selection,
    };

    let shortcut_changed = previous.shortcut != updated.shortcut;
    let shortcuts = app.global_shortcut();
    let previous_registered = shortcuts.is_registered(previous.shortcut.as_str());

    if shortcut_changed {
        shortcuts
            .register(updated.shortcut.as_str())
            .map_err(|error| format!("无法注册快捷键，可能已被其他应用占用：{error}"))?;

        if previous_registered {
            if let Err(error) = shortcuts.unregister(previous.shortcut.as_str()) {
                let _ = shortcuts.unregister(updated.shortcut.as_str());
                return Err(format!("无法替换旧快捷键：{error}"));
            }
        }
    }

    if let Err(error) = write_settings_file(&app, &updated) {
        if shortcut_changed {
            let _ = shortcuts.unregister(updated.shortcut.as_str());
            if previous_registered {
                let _ = shortcuts.register(previous.shortcut.as_str());
            }
        }
        return Err(error);
    }

    *current = updated.clone();
    Ok(updated)
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

struct CaptureOutcome {
    text: String,
    source: &'static str,
    notice: Option<String>,
}

fn capture_clipboard(app: AppHandle, source: &'static str, capture_selection: bool) {
    tauri::async_runtime::spawn(async move {
        let clipboard_app = app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let original = read_clipboard(&clipboard_app);
            if !capture_selection {
                return original.map(|text| CaptureOutcome {
                    text,
                    source,
                    notice: None,
                });
            }

            // Wait for the physical shortcut keys to be released before injecting Copy.
            thread::sleep(Duration::from_millis(120));
            if let Err(message) = selection::simulate_copy() {
                return original.map(|text| CaptureOutcome {
                    text,
                    source: "selection-fallback",
                    notice: Some(message),
                });
            }

            let baseline = original.as_deref().unwrap_or_default().to_string();
            let mut latest = original.ok();
            for _ in 0..12 {
                thread::sleep(Duration::from_millis(25));
                if let Ok(text) = read_clipboard(&clipboard_app) {
                    let changed = text != baseline;
                    latest = Some(text);
                    if changed {
                        break;
                    }
                }
            }

            latest
                .ok_or_else(|| "自动复制后仍无法读取剪贴板".to_string())
                .map(|text| CaptureOutcome {
                    text,
                    source: "selection",
                    notice: None,
                })
        })
        .await;

        match result {
            Ok(Ok(outcome)) => {
                reveal_main_window(&app);
                let _ = app.emit(
                    "showjson://clipboard",
                    ClipboardPayload {
                        text: outcome.text,
                        source: outcome.source,
                    },
                );
                if let Some(message) = outcome.notice {
                    let _ = app.emit("showjson://notice", message);
                }
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
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 ShowJSON", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &hide_item,
            &settings_item,
            &separator,
            &quit_item,
        ],
    )?;

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
            "settings" => {
                reveal_main_window(app);
                let _ = app.emit("showjson://open-settings", ());
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

            let initial_settings = load_settings_file(app.handle()).unwrap_or_default();
            app.manage(SettingsState::new(initial_settings.clone()));

            #[cfg(desktop)]
            {
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(|app, _shortcut, event| {
                            if event.state == ShortcutState::Released {
                                let capture_selection = app
                                    .state::<SettingsState>()
                                    .current
                                    .lock()
                                    .map(|settings| settings.capture_selection)
                                    .unwrap_or(false);
                                capture_clipboard(app.clone(), "shortcut", capture_selection);
                            }
                        })
                        .build(),
                )?;

                if let Err(error) = app
                    .global_shortcut()
                    .register(initial_settings.shortcut.as_str())
                {
                    eprintln!("无法注册快捷键 {}：{error}", initial_settings.shortcut);
                    if initial_settings.shortcut != DEFAULT_SHOW_SHORTCUT
                        && app
                            .global_shortcut()
                            .register(DEFAULT_SHOW_SHORTCUT)
                            .is_ok()
                    {
                        let fallback = AppSettings::default();
                        if let Ok(mut settings) = app.state::<SettingsState>().current.lock() {
                            *settings = fallback.clone();
                        }
                        let _ = write_settings_file(app.handle(), &fallback);
                    }
                }
            }

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
            clear_history,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running ShowJSON");
}
