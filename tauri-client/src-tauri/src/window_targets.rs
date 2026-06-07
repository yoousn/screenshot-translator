use crate::*;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
const WINDOW_EDGE_HIT_SLOP_PX: i32 = 10;

#[cfg(target_os = "windows")]
pub fn get_cursor_position() -> Option<(i32, i32)> {
    let mut point = win32::POINT { x: 0, y: 0 };
    // SAFETY: Calling Win32 API GetCursorPos with a valid mutable pointer to a POINT struct.
    unsafe {
        if win32::GetCursorPos(&mut point) != 0 {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_cursor_position() -> Option<(i32, i32)> {
    None
}

#[cfg(target_os = "windows")]
pub fn current_screen_work_area() -> Option<(i32, i32, i32, i32)> {
    let (cx, cy) = get_cursor_position()?;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    let point = win32::POINT { x: cx, y: cy };
    let monitor = unsafe { win32::MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    if monitor == 0 {
        return None;
    }
    let mut info = win32::MONITORINFO {
        cbSize: std::mem::size_of::<win32::MONITORINFO>() as u32,
        rcMonitor: win32::RECT { left: 0, top: 0, right: 0, bottom: 0 },
        rcWork: win32::RECT { left: 0, top: 0, right: 0, bottom: 0 },
        dwFlags: 0,
    };
    let ok = unsafe { win32::GetMonitorInfoW(monitor, &mut info as *mut win32::MONITORINFO) };
    if ok == 0 || info.rcWork.right <= info.rcWork.left || info.rcWork.bottom <= info.rcWork.top {
        return None;
    }
    Some((
        info.rcWork.left,
        info.rcWork.top,
        info.rcWork.right - info.rcWork.left,
        info.rcWork.bottom - info.rcWork.top,
    ))
}

#[cfg(target_os = "windows")]
pub fn current_screen_origin() -> (i32, i32, i32, i32) {
    if let Some((cx, cy)) = get_cursor_position() {
        if let Ok(screen) = Screen::from_point(cx, cy) {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
        if let Ok(screens) = Screen::all() {
            if let Some(screen) = nearest_screen_for_point(&screens, cx, cy) {
                let info = screen.display_info;
                return (info.x, info.y, info.width as i32, info.height as i32);
            }
        }
    }
    if let Ok(screens) = Screen::all() {
        if let Some(screen) = screens.first() {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
    }
    (0, 0, i32::MAX, i32::MAX)
}

#[cfg(target_os = "windows")]
fn nearest_screen_for_point(screens: &[Screen], x: i32, y: i32) -> Option<Screen> {
    screens
        .iter()
        .min_by_key(|screen| {
            let info = screen.display_info;
            let left = info.x;
            let top = info.y;
            let right = info.x + info.width as i32;
            let bottom = info.y + info.height as i32;
            let dx = if x < left {
                left - x
            } else if x > right {
                x - right
            } else {
                0
            };
            let dy = if y < top {
                top - y
            } else if y > bottom {
                y - bottom
            } else {
                0
            };
            dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
        })
        .copied()
}

#[cfg(target_os = "windows")]
pub fn hwnd_rect(hwnd: isize, prefer_dwm_bounds: bool) -> Option<win32::RECT> {
    if hwnd == 0 {
        return None;
    }
    if prefer_dwm_bounds {
        let mut rect = win32::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: DwmGetWindowAttribute is called with a valid HWND and RECT buffer.
        let hr = unsafe {
            win32::DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut win32::RECT as *mut std::ffi::c_void,
                std::mem::size_of::<win32::RECT>() as u32,
            )
        };
        if hr == 0 && rect.right > rect.left && rect.bottom > rect.top {
            return Some(rect);
        }
    }
    let mut rect = win32::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: GetWindowRect is called with a valid HWND and RECT buffer.
    let ok = unsafe { win32::GetWindowRect(hwnd, &mut rect) };
    if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
        Some(rect)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn rect_size(rect: win32::RECT) -> (i32, i32) {
    (rect.right - rect.left, rect.bottom - rect.top)
}

#[cfg(target_os = "windows")]
pub fn rect_contains_point(rect: win32::RECT, x: i32, y: i32, slop: i32) -> bool {
    x >= rect.left.saturating_sub(slop)
        && x <= rect.right.saturating_add(slop)
        && y >= rect.top.saturating_sub(slop)
        && y <= rect.bottom.saturating_add(slop)
}

#[cfg(target_os = "windows")]
pub fn hwnd_hit_test_rect(hwnd: isize) -> Option<win32::RECT> {
    let dwm_rect = hwnd_rect(hwnd, true);
    let window_rect = hwnd_rect(hwnd, false);
    match (dwm_rect, window_rect) {
        (Some(mut visible), Some(outer)) => {
            visible.left = visible.left.min(outer.left);
            visible.top = visible.top.min(outer.top);
            visible.right = visible.right.max(outer.right);
            visible.bottom = visible.bottom.max(outer.bottom);
            Some(visible)
        }
        (Some(rect), None) | (None, Some(rect)) => Some(rect),
        (None, None) => None,
    }
}

#[cfg(target_os = "windows")]
pub fn hwnd_contains_cursor(hwnd: isize, cursor_x: i32, cursor_y: i32, min_size: i32) -> bool {
    let Some(rect) = hwnd_hit_test_rect(hwnd) else {
        return false;
    };
    let (w, h) = rect_size(rect);
    w >= min_size
        && h >= min_size
        && rect_contains_point(rect, cursor_x, cursor_y, WINDOW_EDGE_HIT_SLOP_PX)
}

#[cfg(target_os = "windows")]
pub fn window_class_name(hwnd: isize) -> String {
    let mut buffer = vec![0u16; 256];
    let copied = unsafe { win32::GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

#[cfg(target_os = "windows")]
pub fn is_system_capture_window(hwnd: isize) -> bool {
    matches!(
        window_class_name(hwnd).as_str(),
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Button"
            | "Progman"
            | "WorkerW"
            | "Windows.UI.Core.CoreWindow"
    )
}

#[cfg(target_os = "windows")]
pub fn push_rect_candidate(
    rects: &mut Vec<serde_json::Value>,
    rect: win32::RECT,
    kind: &str,
    screen: (i32, i32, i32, i32),
    clip: (i32, i32, i32, i32),
    min_size: i32,
) {
    let (screen_x, screen_y, _, _) = screen;
    let (clip_x, clip_y, clip_w, clip_h) = clip;
    let left = rect.left.max(clip_x);
    let top = rect.top.max(clip_y);
    let right = rect.right.min(clip_x + clip_w);
    let bottom = rect.bottom.min(clip_y + clip_h);
    let w = right - left;
    let h = bottom - top;
    if w < min_size || h < min_size {
        return;
    }
    let json_rect = serde_json::json!({
        "x": left - screen_x,
        "y": top - screen_y,
        "w": w,
        "h": h,
        "kind": kind,
    });
    let duplicate = rects.iter().any(|item| {
        item.get("x") == json_rect.get("x")
            && item.get("y") == json_rect.get("y")
            && item.get("w") == json_rect.get("w")
            && item.get("h") == json_rect.get("h")
    });
    if !duplicate {
        rects.push(json_rect);
    }
}

#[cfg(target_os = "windows")]
pub struct WindowSearchContext {
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub excluded_hwnds: Vec<isize>,
    pub matches: Vec<isize>,
    pub min_size: i32,
}

#[cfg(target_os = "windows")]
pub fn excluded_app_hwnds(app: &tauri::AppHandle) -> Vec<isize> {
    let mut excluded = Vec::new();
    for label in ["screenshot", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            if let Ok(hwnd) = window.hwnd() {
                excluded.push(hwnd.0 as isize);
            }
        }
    }
    excluded
}

#[cfg(target_os = "windows")]
pub fn top_level_windows_at_cursor(
    cursor_x: i32,
    cursor_y: i32,
    excluded_hwnds: Vec<isize>,
) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds,
        matches: Vec::new(),
        min_size: 50,
    };
    // SAFETY: EnumWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumWindows(
            Some(enum_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[cfg(target_os = "windows")]
pub fn child_windows_at_cursor(root: isize, cursor_x: i32, cursor_y: i32) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds: Vec::new(),
        matches: Vec::new(),
        min_size: 12,
    };
    // SAFETY: EnumChildWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumChildWindows(
            root,
            Some(enum_child_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[tauri::command]
pub fn get_window_rects(
    app: tauri::AppHandle,
    include_controls: Option<bool>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut rects: Vec<serde_json::Value> = Vec::new();
        let screen = current_screen_origin();
        let window_clip = current_screen_work_area().unwrap_or(screen);
        let include_controls = include_controls.unwrap_or(false);
        if let Some((cx, cy)) = get_cursor_position() {
            let excluded_hwnds = excluded_app_hwnds(&app);
            let windows = top_level_windows_at_cursor(cx, cy, excluded_hwnds);
            if let Some(hwnd) = windows.first().copied() {
                if include_controls {
                    for child in child_windows_at_cursor(hwnd, cx, cy)
                        .into_iter()
                        .rev()
                        .take(1)
                    {
                        if let Some(rect) = hwnd_rect(child, false) {
                            push_rect_candidate(&mut rects, rect, "control", screen, window_clip, 12);
                        }
                    }
                }
                if let Some(rect) = hwnd_rect(hwnd, true) {
                    push_rect_candidate(&mut rects, rect, "window", screen, window_clip, 50);
                }
            }
        }
        Ok(serde_json::to_string(&rects).unwrap_or_else(|_| "[]".to_string()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("[]".to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn window_title(hwnd: isize) -> String {
    let len = unsafe { win32::GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; (len + 1) as usize];
    let copied = unsafe { win32::GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

#[cfg(target_os = "windows")]
pub fn process_path_for_hwnd(hwnd: isize) -> Option<PathBuf> {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let mut pid: u32 = 0;
    unsafe {
        win32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
    }
    if pid == 0 {
        return None;
    }
    let handle = unsafe { win32::OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    let ok = unsafe {
        win32::QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size as *mut u32)
    };
    unsafe {
        let _ = win32::CloseHandle(handle);
    }
    if ok == 0 || size == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..size as usize],
    )))
}

#[cfg(target_os = "windows")]
pub fn exe_name_from_path(path: Option<&PathBuf>) -> String {
    path.and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("app.exe")
        .to_string()
}

#[tauri::command]
pub fn get_recording_targets(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let displays = Screen::all()
        .map_err(|e| format!("Failed to enumerate displays: {}", e))?
        .into_iter()
        .enumerate()
        .map(|(index, screen)| {
            let info = screen.display_info;
            serde_json::json!({
                "id": index.to_string(),
                "title": format!("Display {} ({}x{})", index + 1, info.width, info.height),
                "x": info.x,
                "y": info.y,
                "w": info.width,
                "h": info.height,
            })
        })
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    let windows = {
        let mut ctx = RecordingWindowListContext {
            excluded_hwnds: excluded_app_hwnds(&app),
            windows: Vec::new(),
        };
        unsafe {
            win32::EnumWindows(
                Some(enum_recording_windows),
                &mut ctx as *mut RecordingWindowListContext as isize,
            );
        }
        ctx.windows
    };
    #[cfg(not(target_os = "windows"))]
    let windows: Vec<serde_json::Value> = Vec::new();

    Ok(serde_json::json!({
        "windows": windows,
        "displays": displays,
    }))
}

