use crate::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub static SCREENSHOT_IMAGE: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
static LATEST_SCREENSHOT_PAYLOAD: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();
static SCREENSHOT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_screenshot_session_id() -> String {
    format!(
        "ss-{}",
        SCREENSHOT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn now_epoch_millis_local() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn ensure_png_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
    {
        path
    } else {
        let mut next = path;
        next.set_extension("png");
        next
    }
}

fn log_screenshot_baseline(session_id: &str, phase: &str, started_at: &Instant, detail: &str) {
    println!(
        "[screenshot-baseline] session={} phase={} elapsed_ms={} {}",
        session_id,
        phase,
        started_at.elapsed().as_millis(),
        detail
    );
}

fn get_screenshot_image() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_IMAGE.get_or_init(|| Mutex::new(None))
}

fn get_latest_screenshot_payload_store() -> &'static Mutex<Option<serde_json::Value>> {
    LATEST_SCREENSHOT_PAYLOAD.get_or_init(|| Mutex::new(None))
}

fn set_latest_screenshot_payload(payload: serde_json::Value) {
    if let Ok(mut guard) = get_latest_screenshot_payload_store().lock() {
        *guard = Some(payload);
    }
}

fn clear_latest_screenshot_payload() {
    if let Ok(mut guard) = get_latest_screenshot_payload_store().lock() {
        *guard = None;
    }
}

fn capture_current_monitor_png() -> Result<(Vec<u8>, (i32, i32, u32, u32)), String> {
    match capture_current_monitor_png_xcap() {
        Ok(result) => Ok(result),
        Err(xcap_error) => {
            eprintln!(
                "[screenshot] xcap capture failed, falling back to screenshots crate: {xcap_error}"
            );
            capture_current_monitor_png_legacy().map_err(|legacy_error| {
                format!("xcap capture failed: {xcap_error}; legacy capture failed: {legacy_error}")
            })
        }
    }
}

fn capture_current_monitor_png_xcap() -> Result<(Vec<u8>, (i32, i32, u32, u32)), String> {
    let monitors =
        xcap::Monitor::all().map_err(|error| format!("xcap enumerate displays failed: {error}"))?;
    if monitors.is_empty() {
        return Err("xcap detected no display".to_string());
    }
    let monitor = if let Some((cx, cy)) = get_cursor_position() {
        xcap::Monitor::from_point(cx, cy).unwrap_or_else(|_| monitors[0].clone())
    } else {
        monitors[0].clone()
    };
    let x = monitor
        .x()
        .map_err(|error| format!("xcap display x failed: {error}"))?;
    let y = monitor
        .y()
        .map_err(|error| format!("xcap display y failed: {error}"))?;
    let width = monitor
        .width()
        .map_err(|error| format!("xcap display width failed: {error}"))?;
    let height = monitor
        .height()
        .map_err(|error| format!("xcap display height failed: {error}"))?;
    let image = monitor
        .capture_image()
        .map_err(|error| format!("xcap screenshot failed: {error}"))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    let encoder = xcap::image::codecs::png::PngEncoder::new_with_quality(
        &mut buffer,
        xcap::image::codecs::png::CompressionType::Fast,
        xcap::image::codecs::png::FilterType::NoFilter,
    );
    xcap::image::ImageEncoder::write_image(
        encoder,
        image.as_raw(),
        width,
        height,
        xcap::image::ColorType::Rgba8.into(),
    )
    .map_err(|error| format!("xcap encode PNG failed: {error}"))?;
    Ok((buffer.into_inner(), (x, y, width, height)))
}

fn capture_current_monitor_png_legacy() -> Result<(Vec<u8>, (i32, i32, u32, u32)), String> {
    let screens =
        Screen::all().map_err(|error| format!("Failed to enumerate displays: {error}"))?;
    if screens.is_empty() {
        return Err("No display detected".to_string());
    }
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
    } else {
        screens[0]
    };
    let info = screen.display_info;
    let screen_info = (info.x, info.y, info.width, info.height);
    let image = screen
        .capture()
        .map_err(|error| format!("Screenshot failed: {error}"))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    let encoder = screenshots::image::codecs::png::PngEncoder::new_with_quality(
        &mut buffer,
        screenshots::image::codecs::png::CompressionType::Fast,
        screenshots::image::codecs::png::FilterType::NoFilter,
    );
    screenshots::image::ImageEncoder::write_image(
        encoder,
        image.as_raw(),
        info.width,
        info.height,
        screenshots::image::ColorType::Rgba8,
    )
    .map_err(|error| format!("Encode PNG failed: {error}"))?;
    Ok((buffer.into_inner(), screen_info))
}

fn write_fullscreen_capture_backup(png_bytes: Vec<u8>) -> Result<PathBuf, String> {
    let write_dir = app_data_dir();
    let write_path = write_dir.join("fullscreen_temp.png");
    let legacy_write_path = write_dir.join("fullscreen_temp.jpg");
    if let Some(parent) = write_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Create screenshot temp directory failed: {}", e))?;
        }
    }
    fs::write(&write_path, &png_bytes)
        .map_err(|e| format!("Write screenshot temp image failed: {}", e))?;
    let _ = fs::remove_file(&legacy_write_path);
    Ok(write_path)
}

fn persist_fullscreen_capture_backup(session_id: String, started_at: Instant, png_bytes: Vec<u8>) {
    tauri::async_runtime::spawn(async move {
        let backup_started_at = started_at.elapsed().as_millis();
        log_screenshot_baseline(&session_id, "backup_write_start", &started_at, "background=true");
        match tokio::task::spawn_blocking(move || write_fullscreen_capture_backup(png_bytes)).await {
            Ok(Ok(path)) => log_screenshot_baseline(
                &session_id,
                "backup_write_end",
                &started_at,
                &format!(
                    "background=true write_ms={} path={}",
                    started_at.elapsed().as_millis().saturating_sub(backup_started_at),
                    path.to_string_lossy()
                ),
            ),
            Ok(Err(error)) => eprintln!("[screenshot] failed to write fullscreen backup: {error}"),
            Err(error) => eprintln!("[screenshot] fullscreen backup task failed: {error}"),
        }
    });
}

pub fn ensure_screenshot_window(
    app: &tauri::AppHandle,
    reason: &str,
) -> Result<tauri::WebviewWindow, String> {
    if let Some(win) = app.get_webview_window("screenshot") {
        let _ = win.set_skip_taskbar(true);
        return Ok(win);
    }

    println!("[screenshot-trace] ensure_screenshot_window: creating hidden window reason={reason}");
    let win = tauri::WebviewWindowBuilder::new(
        app,
        "screenshot",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("YSN Screenshot Helper")
    .decorations(false)
    .transparent(true)
    .always_on_top(false)
    .visible(false)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .focused(false)
    .build()
    .map_err(|e| format!("Create screenshot window failed: {}", e))?;
    let _ = win.set_skip_taskbar(true);
    disable_windows_transition(&win);
    hide_window_without_activation(&win);
    Ok(win)
}

pub fn prewarm_screenshot_window(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if let Err(error) = ensure_screenshot_window(&app, "startup-prewarm") {
            eprintln!("[screenshot] failed to prewarm screenshot window: {error}");
        }
    });
}

pub async fn start_screenshot_impl(
    app: tauri::AppHandle,
    mode: Option<String>,
) -> Result<(), String> {
    let started_at = Instant::now();
    let session_id = next_screenshot_session_id();
    log_screenshot_baseline(
        &session_id,
        "hotkey_received",
        &started_at,
        &format!("mode={}", mode.as_deref().unwrap_or("normal")),
    );
    println!(
        "[screenshot-trace] enter start_screenshot_impl, mode={:?}",
        mode
    );
    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());
    let _ = crate::window_lifecycle::set_webview_capture_excluded(&app, "main", false);
    start_text_source_snapshot_capture(&app);

    let main_hidden_for_capture = crate::window_lifecycle::prepare_main_window_for_screenshot(&app);
    log_screenshot_baseline(
        &session_id,
        "main_window_prepared",
        &started_at,
        &format!("hidden_for_capture={}", main_hidden_for_capture),
    );

    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        if screenshot_win.is_visible().unwrap_or(false) {
            let _ = screenshot_win.set_always_on_top(false);
            crate::window_lifecycle::hide_window_without_activation(&screenshot_win);
        }
    }
    close_screenshot_windows(&app, false);
    if main_hidden_for_capture && crate::window_lifecycle::current_screenshot_capture_needs_settle() {
        crate::window_lifecycle::wait_for_hidden_main_capture_settle().await;
        log_screenshot_baseline(&session_id, "main_hidden_settled", &started_at, "");
    }

    // Capture and encode on a blocking thread to avoid blocking the async runtime.
    log_screenshot_baseline(&session_id, "capture_start", &started_at, "");
    let (png_bytes, screen_info) = match tokio::task::spawn_blocking(capture_current_monitor_png)
        .await
        .map_err(|error| format!("Screenshot task failed: {error}"))
        .and_then(|result| result)
    {
        Ok(result) => result,
        Err(error) => {
            crate::window_lifecycle::restore_main_window_after_screenshot(&app, "capture-error");
            return Err(error);
        }
    };
    log_screenshot_baseline(
        &session_id,
        "capture_end",
        &started_at,
        &format!(
            "bytes={} screen={}x{}@{},{}",
            png_bytes.len(),
            screen_info.2,
            screen_info.3,
            screen_info.0,
            screen_info.1
        ),
    );
    println!(
        "[screenshot-perf] capture ready {}ms bytes={}",
        started_at.elapsed().as_millis(),
        png_bytes.len()
    );

    // Store lossless screenshot bytes in memory for OCR/cropping quality and speed.
    if let Ok(mut guard) = get_screenshot_image().lock() {
        *guard = Some(png_bytes.clone());
    }

    // Write backup in the background only. Do not block payload emission or overlay readiness.
    persist_fullscreen_capture_backup(session_id.clone(), started_at, png_bytes.clone());

    let screenshot_win = if let Some(win) = app.get_webview_window("screenshot") {
        let is_visible = win.is_visible().unwrap_or(false);
        log_screenshot_baseline(
            &session_id,
            "window_reused",
            &started_at,
            &format!("visible={}", is_visible),
        );
        println!("[screenshot-trace] start_screenshot_impl: reusing screenshot window, visible={is_visible}");
        let _ = win.set_skip_taskbar(true);
        win
    } else {
        ensure_screenshot_window(&app, "start-screenshot").map_err(|e| {
            crate::window_lifecycle::restore_main_window_after_screenshot(
                &app,
                "create-screenshot-window-error",
            );
            e
        })?
    };

    // Disable transition animation to avoid windows rendering delay/flicker
    disable_windows_transition(&screenshot_win);

    let (x, y, width, height) = screen_info;

    // Position and configure the window while still hidden
    println!(
        "[screenshot-trace] start_screenshot_impl: configuring window, url={:?}, title={:?}",
        screenshot_win.url(),
        screenshot_win.title()
    );
    let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
    let _ = screenshot_win.set_always_on_top(true);

    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let payload = serde_json::json!({
        "kind": "memory",
        "bytes": png_bytes.len(),
        "mode": screenshot_mode,
        "sessionId": session_id.clone(),
    });
    set_latest_screenshot_payload(payload.clone());
    let _ = screenshot_win.emit("screenshot-updated", payload);
    log_screenshot_baseline(
        &session_id,
        "payload_emit",
        &started_at,
        "event=screenshot-updated",
    );
    println!(
        "[screenshot-perf] screenshot payload emitted {}ms",
        started_at.elapsed().as_millis()
    );

    Ok(())
}

#[tauri::command]
pub fn get_screenshot_pointer_state(
    app: tauri::AppHandle,
    label: Option<String>,
) -> Result<serde_json::Value, String> {
    let target_label = label.unwrap_or_else(|| "screenshot".to_string());
    if target_label != "screenshot" && !target_label.starts_with("screenshot_") {
        return Ok(serde_json::json!({
            "leftDown": false,
            "x": 0,
            "y": 0,
            "globalX": 0,
            "globalY": 0
        }));
    }
    let (global_x, global_y) = get_cursor_position().unwrap_or((0, 0));
    let mut window_x = 0;
    let mut window_y = 0;
    if let Some(window) = app.get_webview_window(&target_label) {
        if let Ok(position) = window.outer_position() {
            window_x = position.x;
            window_y = position.y;
        }
    }
    #[cfg(target_os = "windows")]
    let left_down = unsafe { (win32::GetAsyncKeyState(0x01) & i16::MIN) != 0 };
    #[cfg(not(target_os = "windows"))]
    let left_down = false;
    Ok(serde_json::json!({
        "leftDown": left_down,
        "x": global_x - window_x,
        "y": global_y - window_y,
        "globalX": global_x,
        "globalY": global_y
    }))
}

#[tauri::command]
pub fn get_latest_screenshot_payload() -> Result<Option<serde_json::Value>, String> {
    get_latest_screenshot_payload_store()
        .lock()
        .map(|guard| guard.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_save_feedback_toast(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let label = format!("save_toast_{}", now_epoch_millis_local());
    let encoded_path = encode_query_component(&path);
    let toast_width = 168_i32;
    let toast_height = 46_i32;
    let cursor = get_cursor_position().unwrap_or((0, 0));
    let screen = match Screen::from_point(cursor.0, cursor.1) {
        Ok(screen) => screen,
        Err(_) => {
            let screens = Screen::all()
                .map_err(|error| format!("Resolve toast display failed: {error}"))?;
            *screens
                .first()
                .ok_or_else(|| "Resolve toast display failed: no screen".to_string())?
        }
    };
    let screen_info = screen.display_info;
    let x = screen_info.x + ((screen_info.width as i32 - toast_width) / 2).max(12);
    let y = screen_info.y + 28;
    let toast = tauri::WebviewWindowBuilder::new(
        &app,
        label.clone(),
        tauri::WebviewUrl::App(format!("index.html?save_toast=1&path={encoded_path}").into()),
    )
    .title("截图已保存")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .visible(false)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .focused(false)
    .inner_size(toast_width as f64, toast_height as f64)
    .position(x as f64, y as f64)
    .build()
    .map_err(|error| format!("Create save toast failed: {error}"))?;
    disable_windows_transition(&toast);
    let _ = toast.show();
    let toast_clone = toast.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1700)).await;
        let _ = toast_clone.close();
    });
    Ok(())
}

fn encode_query_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[tauri::command]
pub async fn start_screenshot(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    println!("[screenshot-trace] enter start_screenshot, mode={:?}", mode);
    let is_recording = {
        let guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        guard.is_some()
    };
    if is_recording {
        return Err("Recording is already running".to_string());
    }

    // Restart cleanly on repeated hotkey presses instead of racing two overlay sessions.
    if CAPTURING.swap(true, Ordering::SeqCst) {
        println!("[screenshot-trace] start_screenshot: CAPTURING was already true, closing existing windows");
        close_screenshot_windows(&app, true);
    }
    println!("[screenshot-trace] start_screenshot: CAPTURING is now true");

    match start_screenshot_impl(app, mode).await {
        Ok(()) => Ok(()),
        Err(e) => {
            CAPTURING.store(false, Ordering::SeqCst);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn force_close_screenshots(app: tauri::AppHandle) -> Result<(), String> {
    println!("[screenshot-trace] enter force_close_screenshots");
    close_screenshot_windows(&app, true);
    CAPTURING.store(false, Ordering::SeqCst);
    crate::window_lifecycle::restore_main_window_after_screenshot(&app, "force-close-screenshots");
    println!("[screenshot-trace] force_close_screenshots: CAPTURING is now false");
    Ok(())
}

#[tauri::command]
pub fn quick_fullscreen_capture() -> Result<(), String> {
    let (png_bytes, _) = capture_current_monitor_png()?;
    let decoded = image::load_from_memory(&png_bytes)
        .map_err(|error| format!("Decode fullscreen capture failed: {error}"))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Initialize clipboard failed: {error}"))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(decoded.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|error| format!("Copy image to clipboard failed: {error}"))?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_screenshot(
    app: tauri::AppHandle,
    label: Option<String>,
    restore_main: Option<bool>,
) -> Result<(), String> {
    let should_restore_main = restore_main.unwrap_or(true);
    if !should_restore_main {
        crate::window_lifecycle::suppress_next_screenshot_restore();
    }
    if let Some(target_label) = label {
        if target_label == "screenshot" || target_label.starts_with("screenshot_") {
            if let Some(screenshot_win) = app.get_webview_window(&target_label) {
                let _ = screenshot_win.set_always_on_top(false);
                crate::window_lifecycle::prepare_focus_for_screenshot_overlay_close(
                    &app,
                    "cancel-screenshot-target",
                );
                crate::window_lifecycle::hide_window_without_activation(&screenshot_win);
            }
            close_screenshot_windows(&app, false);
        }
    } else {
        close_screenshot_windows(&app, true);
    }
    CAPTURING.store(false, Ordering::SeqCst);
    clear_latest_screenshot_payload();
    if should_restore_main {
        crate::window_lifecycle::restore_main_window_after_screenshot(&app, "cancel-screenshot");
    }
    Ok(())
}

#[tauri::command]
pub fn get_fullscreen_image() -> Result<String, String> {
    // Try memory first (fast), fall back to disk
    if let Ok(guard) = get_screenshot_image().lock() {
        if let Some(ref bytes) = *guard {
            return Ok(BASE64_STANDARD.encode(bytes));
        }
    }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() {
        return Err("No display detected".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
pub fn get_fullscreen_image_bytes() -> Result<tauri::ipc::Response, String> {
    if let Ok(guard) = get_screenshot_image().lock() {
        if let Some(ref bytes) = *guard {
            return Ok(tauri::ipc::Response::new(bytes.clone()));
        }
    }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() {
        return Err("No display detected".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection region".to_string());
    }

    // Try memory first (fast), fall back to disk
    let screenshot_bytes = {
        let guard = get_screenshot_image().lock().map_err(|e| e.to_string())?;
        if let Some(ref bytes) = *guard {
            bytes.clone()
        } else {
            let mut path = app_data_dir();
            path.push("fullscreen_temp.png");
            if !path.exists() {
                path = app_data_dir();
                path.push("fullscreen_temp.jpg");
            }
            if !path.exists() {
                return Err("No display detected".to_string());
            }
            fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))?
        }
    };

    let img = screenshots::image::load_from_memory(&screenshot_bytes)
        .map_err(|e| format!("Load fullscreen image failed: {}", e))?;
    let iw = img.width() as i32;
    let ih = img.height() as i32;
    let sx = x.clamp(0, iw.saturating_sub(1));
    let sy = y.clamp(0, ih.saturating_sub(1));
    let sw = w.clamp(1, iw - sx);
    let sh = h.clamp(1, ih - sy);
    let cropped = img.crop_imm(sx as u32, sy as u32, sw as u32, sh as u32);
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("Encode PNG failed: {}", e))?;
    let bytes = buffer.into_inner();
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    let _ = fs::write(&cropped_path, &bytes);
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
pub fn capture_live_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection area".to_string());
    }
    let (origin_x, origin_y, _, _) = current_screen_origin();
    let global_x = origin_x + x;
    let global_y = origin_y + y;
    let center_x = global_x + w / 2;
    let center_y = global_y + h / 2;
    let screen = Screen::from_point(center_x, center_y)
        .map_err(|e| format!("Failed to locate screen for scroll capture: {}", e))?;
    let rel_x = global_x - screen.display_info.x;
    let rel_y = global_y - screen.display_info.y;
    let image = screen
        .capture_area(rel_x, rel_y, w as u32, h as u32)
        .map_err(|e| format!("Failed to capture live region: {}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    screenshots::image::DynamicImage::ImageRgba8(image)
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    Ok(BASE64_STANDARD.encode(buffer.into_inner()))
}

#[tauri::command]
pub fn scroll_mouse_at(x: i32, y: i32, delta: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const MOUSEEVENTF_WHEEL: u32 = 0x0800;
        let (origin_x, origin_y, _, _) = current_screen_origin();
        let global_x = origin_x + x;
        let global_y = origin_y + y;
        unsafe {
            let _ = win32::SetCursorPos(global_x, global_y);
            win32::mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta as u32, 0);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (x, y, delta);
        Err("Automatic scrolling is not supported on this platform".to_string())
    }
}

#[tauri::command]
pub fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Decode base64 failed: {}", e))?;
    let img = screenshots::image::load_from_memory_with_format(
        &bytes,
        screenshots::image::ImageFormat::Png,
    )
    .map_err(|e| format!("Parse cropped image data failed: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Initialize clipboard failed: {}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Copy image to clipboard failed: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn save_image_to_file(image_base64: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Decode base64 failed: {}", e))?;
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_file_name("screenshot.png");
    if let Some(desktop_dir) = dirs::desktop_dir() {
        dialog = dialog.set_directory(desktop_dir);
    }
    let file_path = dialog.save_file().await;
    if let Some(file_handle) = file_path {
        let path = file_handle.path();
        fs::write(path, &bytes).map_err(|e| format!("Write file failed: {}", e))?;
        if !path.exists() {
            return Err("No display detected".to_string());
        }
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Save cancelled by user".to_string())
    }
}

#[tauri::command]
pub async fn choose_image_save_path() -> Result<Option<String>, String> {
    let started_at = Instant::now();
    println!("[screenshot-baseline] session=save-as phase=dialog_open_start elapsed_ms=0");
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_file_name("screenshot.png");
    if let Some(desktop_dir) = dirs::desktop_dir() {
        dialog = dialog.set_directory(desktop_dir);
    }
    let file_path = dialog.save_file().await;
    let result = file_path.map(|file_handle| {
        ensure_png_extension(file_handle.path().to_path_buf())
            .to_string_lossy()
            .to_string()
    });
    println!(
        "[screenshot-baseline] session=save-as phase=dialog_open_end elapsed_ms={} cancelled={}",
        started_at.elapsed().as_millis(),
        result.is_none()
    );
    Ok(result)
}

#[tauri::command]
pub fn write_image_to_file(
    app: tauri::AppHandle,
    image_base64: String,
    path: String,
) -> Result<String, String> {
    let started_at = Instant::now();
    println!(
        "[screenshot-baseline] session=save-as phase=file_write_start elapsed_ms=0 bytes_estimate={} path={}",
        image_base64.len().saturating_mul(3) / 4,
        path
    );
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Decode base64 failed: {}", e))?;
    let path = PathBuf::from(path);
    let path = ensure_png_extension(PathBuf::from(path));
    fs::write(&path, &bytes).map_err(|e| format!("Write file failed: {}", e))?;
    let saved_path = path.to_string_lossy().to_string();
    let toast_app = app.clone();
    let toast_path = saved_path.clone();
    tauri::async_runtime::spawn(async move {
        let _ = show_save_feedback_toast(toast_app, toast_path);
    });
    println!(
        "[screenshot-baseline] session=save-as phase=file_write_end elapsed_ms={} path={}",
        started_at.elapsed().as_millis(),
        saved_path
    );
    Ok(saved_path)
}

#[tauri::command]
pub fn log_screenshot_perf(message: String) {
    println!("[screenshot-perf] {message}");
}
