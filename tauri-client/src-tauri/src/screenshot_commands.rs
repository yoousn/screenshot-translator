use crate::screenshot_diagnostics_json::*;
pub use crate::screenshot_diagnostics_requests::*;
pub use crate::screenshot_wgc_diagnostic_commands::*;
use crate::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub static SCREENSHOT_IMAGE: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
static SCREENSHOT_RGBA: OnceLock<Mutex<Option<SessionScreenshotRgba>>> = OnceLock::new();
static LATEST_SCREENSHOT_PAYLOAD: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();
static LATEST_SCREENSHOT_SHELL_PAYLOAD: OnceLock<Mutex<Option<serde_json::Value>>> =
    OnceLock::new();
static CANCELLED_SCREENSHOT_SESSIONS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static SCREENSHOT_POINTER_PRE_CAPTURE: OnceLock<Mutex<Option<ScreenshotPointerPreCapture>>> =
    OnceLock::new();
static LAST_SCREENSHOT_WINDOW_BOUNDS: OnceLock<Mutex<Option<(i32, i32, u32, u32)>>> =
    OnceLock::new();
type ScreenshotRgba = Arc<crate::screenshot_native::RgbaFrame>;

#[derive(Debug, Clone)]
struct SessionScreenshotRgba {
    session_id: String,
    frame: ScreenshotRgba,
}

#[derive(Debug, Clone)]
struct ScreenshotPointerPreCapture {
    session_id: String,
    origin_x: i32,
    origin_y: i32,
    started_at: Instant,
    updated_at: Instant,
    was_down_at_start: bool,
    left_down: bool,
    completed: bool,
    down_global: Option<(i32, i32)>,
    latest_global: Option<(i32, i32)>,
    max_drag_distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScreenshotPointerPreCaptureActivity {
    pub left_down: bool,
    pub completed: bool,
    pub has_drag: bool,
    pub drag_distance: f64,
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

const DEFAULT_IMAGE_SAVE_NAME_PREFIX: &str = "Ysn_";
const DEFAULT_IMAGE_SAVE_NAME_FORMAT: &str = "yyyyMMdd_HHmmss";

fn sanitize_windows_file_name(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()
            {
                '-'
            } else {
                ch
            }
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches('.').trim().to_string();
    if trimmed.is_empty() {
        format!("{DEFAULT_IMAGE_SAVE_NAME_PREFIX}screenshot")
    } else if is_reserved_windows_file_name(&trimmed) {
        format!("_{trimmed}")
    } else {
        trimmed
    }
}

fn is_reserved_windows_file_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .and_then(|suffix| suffix.parse::<u8>().ok())
            .is_some_and(|number| (1..=9).contains(&number))
        || stem
            .strip_prefix("LPT")
            .and_then(|suffix| suffix.parse::<u8>().ok())
            .is_some_and(|number| (1..=9).contains(&number))
}

fn render_image_save_datetime_format(format: &str) -> String {
    let now = chrono::Local::now();
    let mut rendered = if format.trim().is_empty() {
        DEFAULT_IMAGE_SAVE_NAME_FORMAT.to_string()
    } else {
        format.trim().to_string()
    };
    for (token, value) in [
        ("yyyy", now.format("%Y").to_string()),
        ("yy", now.format("%y").to_string()),
        ("MM", now.format("%m").to_string()),
        ("dd", now.format("%d").to_string()),
        ("HH", now.format("%H").to_string()),
        ("mm", now.format("%M").to_string()),
        ("ss", now.format("%S").to_string()),
    ] {
        rendered = rendered.replace(token, &value);
    }
    rendered
}

fn default_image_save_file_name() -> String {
    let prefix = crate::config_store::config_value_string("imageSaveNamePrefix")
        .unwrap_or_else(|| DEFAULT_IMAGE_SAVE_NAME_PREFIX.to_string());
    let format = crate::config_store::config_value_string("imageSaveNameFormat")
        .unwrap_or_else(|| DEFAULT_IMAGE_SAVE_NAME_FORMAT.to_string());
    let format = if format == "yyyyMMdd_HHmm" {
        DEFAULT_IMAGE_SAVE_NAME_FORMAT.to_string()
    } else {
        format
    };
    let file_stem = sanitize_windows_file_name(&format!(
        "{}{}",
        prefix,
        render_image_save_datetime_format(&format)
    ));
    format!("{file_stem}.png")
}

fn usable_directory(value: Option<String>) -> Option<PathBuf> {
    let value = value?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

fn default_image_save_directory() -> Option<PathBuf> {
    let remember_last =
        crate::config_store::config_value_bool("imageSaveRememberLastDir").unwrap_or(false);
    if remember_last {
        if let Some(last_dir) =
            usable_directory(crate::config_store::config_value_string("imageSaveLastDir"))
        {
            return Some(last_dir);
        }
    }
    usable_directory(crate::config_store::config_value_string(
        "imageSaveDefaultDir",
    ))
    .or_else(dirs::desktop_dir)
}

fn remember_image_save_directory(path: &std::path::Path) {
    if !crate::config_store::config_value_bool("imageSaveRememberLastDir").unwrap_or(false) {
        return;
    }
    let Some(parent) = path.parent().filter(|parent| parent.is_dir()) else {
        return;
    };
    let next_dir = parent.to_string_lossy().to_string();
    if crate::config_store::config_value_string("imageSaveLastDir").as_deref()
        == Some(next_dir.as_str())
    {
        return;
    }
    let _ = crate::config_store::set_config_value_if_changed(
        "imageSaveLastDir",
        serde_json::Value::String(next_dir),
    );
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

fn log_native_overlay_launch_plan(
    session_id: &str,
    started_at: &Instant,
    plan: crate::screenshot_native::NativeOverlayLaunchPlan,
) {
    let fallback_reason = plan
        .fallback_reason
        .map(|reason| reason.as_str())
        .unwrap_or("none");
    log_screenshot_baseline(
        session_id,
        "native_overlay_capability",
        started_at,
        &format!(
            "runtime={} enabled={} fallback_reason={}",
            plan.runtime.as_str(),
            plan.capability.is_enabled(),
            fallback_reason
        ),
    );
}

fn native_overlay_mvp_fallback_plan() -> crate::screenshot_native::NativeOverlayLaunchPlan {
    crate::screenshot_native::NativeOverlayLaunchPlan::fallback(
        crate::screenshot_native::NativeOverlayFallbackReason::MvpNotWired,
    )
}

fn native_first_frame_session_enabled() -> bool {
    std::env::var("YSN_NATIVE_FIRST_FRAME_SESSION")
        .ok()
        .as_deref()
        == Some("1")
}

fn log_cpu_native_overlay_diagnostics(
    session_id: &str,
    started_at: &Instant,
    phase: &str,
    diagnostics: crate::screenshot_native::NativeOverlaySessionDiagnostics,
) {
    log_screenshot_baseline(
        session_id,
        phase,
        started_at,
        &format!(
            "active={} state={} runtime={} hwnd={:?} rendered={} visible={} reason={}",
            diagnostics.active,
            diagnostics.state.as_str(),
            diagnostics
                .runtime
                .map(|runtime| runtime.as_str())
                .unwrap_or("none"),
            diagnostics.hwnd,
            diagnostics.rendered,
            diagnostics.visible,
            diagnostics.fallback_reason.as_deref().unwrap_or("none")
        ),
    );
}

fn parse_output_action(
    action: Option<String>,
) -> Result<crate::screenshot_native::OutputAction, String> {
    match action
        .as_deref()
        .unwrap_or("copy")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "copy" | "clipboard" => Ok(crate::screenshot_native::OutputAction::Copy),
        "save" | "saveas" | "save_as" | "file" => {
            Ok(crate::screenshot_native::OutputAction::SaveAs)
        }
        "ocr" => Ok(crate::screenshot_native::OutputAction::Ocr),
        "translate" | "translation" => Ok(crate::screenshot_native::OutputAction::Translate),
        "record" => Ok(crate::screenshot_native::OutputAction::Record),
        other => Err(format!("Unsupported screenshot output action: {other}")),
    }
}

fn get_screenshot_image() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_IMAGE.get_or_init(|| Mutex::new(None))
}

fn get_screenshot_rgba() -> &'static Mutex<Option<SessionScreenshotRgba>> {
    SCREENSHOT_RGBA.get_or_init(|| Mutex::new(None))
}

fn get_matching_screenshot_rgba(session_id: Option<&str>) -> Result<ScreenshotRgba, String> {
    let cached = get_screenshot_rgba()
        .lock()
        .map_err(|_| "latest screenshot RGBA cache is poisoned".to_string())?
        .clone()
        .ok_or_else(|| {
            "no latest screenshot RGBA frame is available; start a screenshot first".to_string()
        })?;

    if let Some(expected_session_id) = session_id {
        if !expected_session_id.is_empty() && cached.session_id != expected_session_id {
            return Err(format!(
                "stale screenshot RGBA cache rejected: requested session {} but cached session is {}",
                expected_session_id, cached.session_id
            ));
        }
    }

    Ok(cached.frame)
}

fn get_latest_screenshot_payload_store() -> &'static Mutex<Option<serde_json::Value>> {
    LATEST_SCREENSHOT_PAYLOAD.get_or_init(|| Mutex::new(None))
}

fn get_latest_screenshot_shell_payload_store() -> &'static Mutex<Option<serde_json::Value>> {
    LATEST_SCREENSHOT_SHELL_PAYLOAD.get_or_init(|| Mutex::new(None))
}

fn get_screenshot_pointer_pre_capture_store() -> &'static Mutex<Option<ScreenshotPointerPreCapture>>
{
    SCREENSHOT_POINTER_PRE_CAPTURE.get_or_init(|| Mutex::new(None))
}

fn get_cancelled_screenshot_sessions_store() -> &'static Mutex<Vec<String>> {
    CANCELLED_SCREENSHOT_SESSIONS.get_or_init(|| Mutex::new(Vec::new()))
}

fn get_last_screenshot_window_bounds_store() -> &'static Mutex<Option<(i32, i32, u32, u32)>> {
    LAST_SCREENSHOT_WINDOW_BOUNDS.get_or_init(|| Mutex::new(None))
}

fn clear_screenshot_window_bounds_cache() {
    if let Ok(mut guard) = get_last_screenshot_window_bounds_store().lock() {
        *guard = None;
    }
}

fn set_latest_screenshot_payload(payload: serde_json::Value) {
    if let Ok(mut guard) = get_latest_screenshot_payload_store().lock() {
        *guard = Some(payload);
    }
}

fn set_latest_screenshot_shell_payload(payload: serde_json::Value) {
    if let Ok(mut guard) = get_latest_screenshot_shell_payload_store().lock() {
        *guard = Some(payload);
    }
}

fn clear_latest_screenshot_payload() {
    if let Ok(mut guard) = get_latest_screenshot_payload_store().lock() {
        *guard = None;
    }
    if let Ok(mut guard) = get_latest_screenshot_shell_payload_store().lock() {
        *guard = None;
    }
    if let Ok(mut guard) = get_screenshot_pointer_pre_capture_store().lock() {
        *guard = None;
    }
}

fn latest_screenshot_session_id() -> Option<String> {
    let from_payload = get_latest_screenshot_payload_store()
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .and_then(|payload| payload.get("sessionId"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
    if from_payload.is_some() {
        return from_payload;
    }

    get_latest_screenshot_shell_payload_store()
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .and_then(|payload| payload.get("sessionId"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
}

fn mark_screenshot_session_cancelled(session_id: &str) {
    if let Ok(mut guard) = get_cancelled_screenshot_sessions_store().lock() {
        if !guard.iter().any(|existing| existing == session_id) {
            guard.push(session_id.to_string());
        }
        let keep_from = guard.len().saturating_sub(16);
        if keep_from > 0 {
            guard.drain(0..keep_from);
        }
    }
}

pub(crate) fn is_screenshot_session_cancelled(session_id: &str) -> bool {
    get_cancelled_screenshot_sessions_store()
        .lock()
        .map(|guard| guard.iter().any(|cancelled| cancelled == session_id))
        .unwrap_or(false)
}

fn notify_screenshot_session_cancelled(app: &tauri::AppHandle, reason: &str) {
    let session_id = latest_screenshot_session_id();
    if let Some(session_id) = session_id.as_deref() {
        mark_screenshot_session_cancelled(session_id);
    }
    let payload = serde_json::json!({
        "sessionId": session_id,
        "reason": reason,
    });
    if let Some(window) = app.get_webview_window("screenshot") {
        let _ = window.emit("screenshot-session-cancelled", payload);
    }
}

pub(crate) fn latest_or_request_physical_bounds(
    request_bounds: Option<NativeDxgiSelectedReadbackSmokeRequest>,
) -> Result<
    (
        &'static str,
        crate::screenshot_native::MonitorCaptureBounds,
        Option<serde_json::Value>,
    ),
    serde_json::Value,
> {
    let latest_payload = get_latest_screenshot_payload_store()
        .lock()
        .map_err(|error| {
            serde_json::json!({
                "ok": false,
                "valid": false,
                "boundsSource": "error",
                "error": error.to_string(),
            })
        })?
        .clone();
    if let Some(bounds) = request_bounds.map(|bounds| {
        crate::screenshot_native::MonitorCaptureBounds::new(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        )
    }) {
        if bounds.is_empty() || bounds.right().is_none() || bounds.bottom().is_none() {
            return Err(serde_json::json!({
                "ok": false,
                "valid": false,
                "boundsSource": "request",
                "latestPayloadPresent": latest_payload.is_some(),
                "error": "request bounds must be non-empty and within i32 desktop coordinates",
            }));
        }
        return Ok(("request", bounds, latest_payload));
    }
    let Some(payload) = latest_payload.as_ref() else {
        return Err(serde_json::json!({
            "ok": false,
            "valid": false,
            "boundsSource": "missing",
            "latestPayloadPresent": false,
            "error": "latest screenshot payload is not available; pass explicit bounds or run screenshot first",
        }));
    };
    match parse_latest_screenshot_physical_bounds(payload) {
        Ok(bounds) => Ok(("latestPayload", bounds, latest_payload)),
        Err(error) => Err(serde_json::json!({
            "ok": false,
            "valid": false,
            "boundsSource": "latestPayload",
            "latestPayloadPresent": true,
            "sessionId": payload.get("sessionId").and_then(serde_json::Value::as_str),
            "error": error,
        })),
    }
}

fn dxgi_acquire_comparison_disabled_response(
    request: &NativeDxgiDefaultVsSelectedAcquireComparisonRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    let disabled_path = |path: &str| {
        dxgi_acquire_path_json(
            path,
            false,
            false,
            "disabled",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            Some(error.to_string()),
        )
    };
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "defaultOutput": disabled_path("default-output"),
        "selectedOutput": disabled_path("selected-output"),
        "comparison": {
            "defaultFrameConfirmed": false,
            "selectedFrameConfirmed": false,
            "bothTimedOut": false,
            "defaultOnlySucceeded": false,
            "selectedOnlySucceeded": false,
            "sameFailureClass": true,
        },
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "error": error,
        "scope": "diagnostic-only; compares default DXGI output and selected DXGI output acquire without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

fn dxgi_selected_output_bridge_disabled_response(
    request: &NativeDxgiSelectedOutputBridgeDryRunRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "elapsedMs": 0,
        "frameId": serde_json::Value::Null,
        "x": request.x,
        "y": request.y,
        "width": serde_json::Value::Null,
        "height": serde_json::Value::Null,
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "outputBounds": serde_json::Value::Null,
        "adapterIndex": serde_json::Value::Null,
        "outputIndex": serde_json::Value::Null,
        "outputRanking": serde_json::Value::Null,
        "crop": serde_json::Value::Null,
        "selectedReadbackPlan": selected_readback_plan_error_json(
            crate::screenshot_native::SelectedReadbackPlanBackend::DxgiOutput,
            requested_bounds,
            "guard-disabled-ranked-dxgi-output-bridge",
            "dxgi-selected-output-bridge-disabled",
            error.to_string(),
        ),
        "selectedOutputReadyPlanningOnly": false,
        "format": serde_json::Value::Null,
        "selectedOnly": false,
        "pngSignatureValid": false,
        "releasedFrame": false,
        "stopped": false,
        "bridge": serde_json::Value::Null,
        "actions": [],
        "bridgeValidated": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "error": error,
        "scope": "diagnostic-only; DXGI selected-output bridge dry-run is disabled unless explicitOptIn and allowRealDxgiApi are both true; does not perform clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}
fn dxgi_selected_readback_disabled_response(
    request: &NativeDxgiSelectedReadbackSmokeRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "elapsedMs": 0,
        "frameId": serde_json::Value::Null,
        "x": request.x,
        "y": request.y,
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "outputBounds": serde_json::Value::Null,
        "adapterIndex": serde_json::Value::Null,
        "outputIndex": serde_json::Value::Null,
        "crop": serde_json::Value::Null,
        "selectedReadbackPlan": selected_readback_plan_error_json(
            crate::screenshot_native::SelectedReadbackPlanBackend::DxgiOutput,
            requested_bounds,
            "guard-disabled-ranked-dxgi-output-desktop-coordinates",
            "dxgi-selected-readback-disabled",
            error.to_string(),
        ),
        "selectedOutputReadyPlanningOnly": false,
        "width": serde_json::Value::Null,
        "height": serde_json::Value::Null,
        "format": serde_json::Value::Null,
        "selectedOnly": false,
        "boundedCropValid": false,
        "copySubresourceRegion": false,
        "bgraToRgba": false,
        "pngSignatureValid": false,
        "releasedFrame": false,
        "stopped": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "error": error,
        "scope": "diagnostic-only; DXGI selected-region readback is disabled unless explicitOptIn and allowRealDxgiApi are both true; does not mark presenter, native overlay, Alt+A, or C/E readiness complete"
    })
}

fn encode_rgba_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let encoder = screenshots::image::codecs::png::PngEncoder::new_with_quality(
        &mut buffer,
        screenshots::image::codecs::png::CompressionType::Fast,
        screenshots::image::codecs::png::FilterType::NoFilter,
    );
    screenshots::image::ImageEncoder::write_image(
        encoder,
        rgba,
        width,
        height,
        screenshots::image::ColorType::Rgba8,
    )
    .map_err(|error| format!("Encode PNG failed: {error}"))?;
    Ok(buffer.into_inner())
}

fn capture_current_monitor_rgba() -> Result<(ScreenshotRgba, (i32, i32, u32, u32)), String> {
    match capture_current_monitor_rgba_xcap() {
        Ok(result) => Ok(result),
        Err(xcap_error) => {
            eprintln!(
                "[screenshot] xcap capture failed, falling back to screenshots crate: {xcap_error}"
            );
            capture_current_monitor_rgba_legacy().map_err(|legacy_error| {
                format!("xcap capture failed: {xcap_error}; legacy capture failed: {legacy_error}")
            })
        }
    }
}

fn capture_current_monitor_png() -> Result<(Vec<u8>, (i32, i32, u32, u32)), String> {
    let (rgba, screen_info) = capture_current_monitor_rgba()?;
    let png = encode_rgba_png(&rgba.bytes, rgba.width, rgba.height)?;
    Ok((png, screen_info))
}

fn capture_current_monitor_rgba_xcap() -> Result<(ScreenshotRgba, (i32, i32, u32, u32)), String> {
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
    Ok((
        Arc::new(crate::screenshot_native::RgbaFrame {
            bytes: image.as_raw().to_vec(),
            width,
            height,
        }),
        (x, y, width, height),
    ))
}

fn capture_current_monitor_rgba_legacy() -> Result<(ScreenshotRgba, (i32, i32, u32, u32)), String> {
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
    Ok((
        Arc::new(crate::screenshot_native::RgbaFrame {
            bytes: image.into_raw(),
            width: info.width,
            height: info.height,
        }),
        screen_info,
    ))
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
        log_screenshot_baseline(
            &session_id,
            "backup_write_start",
            &started_at,
            "background=true",
        );
        match tokio::task::spawn_blocking(move || write_fullscreen_capture_backup(png_bytes)).await
        {
            Ok(Ok(path)) => log_screenshot_baseline(
                &session_id,
                "backup_write_end",
                &started_at,
                &format!(
                    "background=true write_ms={} path={}",
                    started_at
                        .elapsed()
                        .as_millis()
                        .saturating_sub(backup_started_at),
                    path.to_string_lossy()
                ),
            ),
            Ok(Err(error)) => eprintln!("[screenshot] failed to write fullscreen backup: {error}"),
            Err(error) => eprintln!("[screenshot] fullscreen backup task failed: {error}"),
        }
    });
}

fn encode_and_store_fullscreen_png(session_id: String, started_at: Instant, rgba: ScreenshotRgba) {
    tauri::async_runtime::spawn(async move {
        let encode_started_at = started_at.elapsed().as_millis();
        log_screenshot_baseline(
            &session_id,
            "png_encode_start",
            &started_at,
            "background=true",
        );
        match tokio::task::spawn_blocking(move || {
            encode_rgba_png(&rgba.bytes, rgba.width, rgba.height)
        })
        .await
        {
            Ok(Ok(png_bytes)) => {
                log_screenshot_baseline(
                    &session_id,
                    "png_encode_end",
                    &started_at,
                    &format!(
                        "background=true encode_ms={} bytes={}",
                        started_at
                            .elapsed()
                            .as_millis()
                            .saturating_sub(encode_started_at),
                        png_bytes.len()
                    ),
                );
                if let Ok(mut guard) = get_screenshot_image().lock() {
                    *guard = Some(png_bytes.clone());
                }
                persist_fullscreen_capture_backup(session_id, started_at, png_bytes);
            }
            Ok(Err(error)) => eprintln!("[screenshot] failed to encode fullscreen PNG: {error}"),
            Err(error) => eprintln!("[screenshot] fullscreen PNG encode task failed: {error}"),
        }
    });
}

fn get_or_encode_screenshot_png() -> Result<Vec<u8>, String> {
    if let Ok(guard) = get_screenshot_image().lock() {
        if let Some(ref bytes) = *guard {
            return Ok(bytes.clone());
        }
    }
    if let Ok(guard) = get_screenshot_rgba().lock() {
        if let Some(ref rgba) = *guard {
            let rgba = &rgba.frame;
            let png = encode_rgba_png(&rgba.bytes, rgba.width, rgba.height)?;
            if let Ok(mut image_guard) = get_screenshot_image().lock() {
                *image_guard = Some(png.clone());
            }
            return Ok(png);
        }
    }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() {
        return Err("No display detected".to_string());
    }
    fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))
}

pub fn ensure_screenshot_window(
    app: &tauri::AppHandle,
    reason: &str,
) -> Result<tauri::WebviewWindow, String> {
    let transparent = screenshot_window_transparency_enabled();
    if let Some(win) = app.get_webview_window("screenshot") {
        if transparent {
            println!(
                "[screenshot-trace] ensure_screenshot_window: transparent screenshot helper active reason={reason}"
            );
        }
        let _ = win.set_skip_taskbar(true);
        crate::window_lifecycle::apply_screenshot_overlay_window_styles(&win, true);
        return Ok(win);
    }

    clear_screenshot_window_bounds_cache();
    println!("[screenshot-trace] ensure_screenshot_window: creating hidden window reason={reason}");
    let win = tauri::WebviewWindowBuilder::new(
        app,
        "screenshot",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("YSN Screenshot Helper")
    .decorations(false)
    .transparent(transparent)
    .always_on_top(false)
    .visible(false)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .focused(false)
    .build()
    .map_err(|e| format!("Create screenshot window failed: {}", e))?;
    let _ = win.set_skip_taskbar(true);
    crate::window_lifecycle::apply_screenshot_overlay_window_styles(&win, true);
    disable_windows_transition(&win);
    hide_window_without_activation(&win);
    Ok(win)
}

fn screenshot_window_transparency_enabled() -> bool {
    if std::env::var("YSN_SCREENSHOT_OPAQUE_WINDOW")
        .ok()
        .as_deref()
        == Some("1")
    {
        return false;
    }
    match std::env::var("YSN_SCREENSHOT_TRANSPARENT_WINDOW")
        .ok()
        .as_deref()
    {
        Some("0") => false,
        Some("1") => true,
        _ => true,
    }
}

fn screenshot_capture_exclusion_enabled() -> bool {
    matches!(
        std::env::var("YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE")
            .ok()
            .as_deref(),
        Some("1")
    )
}

#[cfg(test)]
mod screenshot_window_transparency_tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn screenshot_window_transparency_is_default_with_opaque_rollback() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_SCREENSHOT_TRANSPARENT_WINDOW");
        std::env::remove_var("YSN_SCREENSHOT_OPAQUE_WINDOW");
        assert!(screenshot_window_transparency_enabled());

        std::env::set_var("YSN_SCREENSHOT_TRANSPARENT_WINDOW", "0");
        assert!(!screenshot_window_transparency_enabled());

        std::env::set_var("YSN_SCREENSHOT_TRANSPARENT_WINDOW", "1");
        assert!(screenshot_window_transparency_enabled());

        std::env::set_var("YSN_SCREENSHOT_OPAQUE_WINDOW", "1");
        assert!(!screenshot_window_transparency_enabled());

        std::env::remove_var("YSN_SCREENSHOT_TRANSPARENT_WINDOW");
        std::env::remove_var("YSN_SCREENSHOT_OPAQUE_WINDOW");
    }

    #[test]
    fn transparent_input_shell_is_default_with_rollbacks() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_SCREENSHOT_DEFER_VISIBLE_SHELL");
        std::env::remove_var("YSN_SCREENSHOT_EARLY_VISIBLE_SHELL");
        assert!(screenshot_early_visible_shell_enabled());

        std::env::set_var("YSN_SCREENSHOT_EARLY_VISIBLE_SHELL", "0");
        assert!(!screenshot_early_visible_shell_enabled());

        std::env::set_var("YSN_SCREENSHOT_EARLY_VISIBLE_SHELL", "1");
        assert!(screenshot_early_visible_shell_enabled());

        std::env::set_var("YSN_SCREENSHOT_DEFER_VISIBLE_SHELL", "1");
        assert!(!screenshot_early_visible_shell_enabled());
        std::env::remove_var("YSN_SCREENSHOT_DEFER_VISIBLE_SHELL");
        std::env::remove_var("YSN_SCREENSHOT_EARLY_VISIBLE_SHELL");
    }

    #[test]
    fn offscreen_prewarm_show_is_opt_in() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW");
        assert!(!screenshot_offscreen_prewarm_enabled());

        std::env::set_var("YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW", "0");
        assert!(!screenshot_offscreen_prewarm_enabled());

        std::env::set_var("YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW", "1");
        assert!(screenshot_offscreen_prewarm_enabled());
        std::env::remove_var("YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW");
    }

    #[test]
    fn native_first_frame_session_is_opt_in_until_visual_artifacts_are_fixed() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_NATIVE_FIRST_FRAME_SESSION");
        assert!(!native_first_frame_session_enabled());

        std::env::set_var("YSN_NATIVE_FIRST_FRAME_SESSION", "1");
        assert!(native_first_frame_session_enabled());

        std::env::set_var("YSN_NATIVE_FIRST_FRAME_SESSION", "0");
        assert!(!native_first_frame_session_enabled());
        std::env::remove_var("YSN_NATIVE_FIRST_FRAME_SESSION");
    }

    #[test]
    fn screenshot_capture_exclusion_is_opt_in_for_visual_qa() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE");
        assert!(!screenshot_capture_exclusion_enabled());

        std::env::set_var("YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE", "1");
        assert!(screenshot_capture_exclusion_enabled());

        std::env::set_var("YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE", "0");
        assert!(!screenshot_capture_exclusion_enabled());
        std::env::remove_var("YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE");
    }
}

pub fn prewarm_screenshot_window(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        match ensure_screenshot_window(&app, "startup-prewarm") {
            Ok(window) => pulse_screenshot_window_for_webview_prewarm(window).await,
            Err(error) => eprintln!("[screenshot] failed to prewarm screenshot window: {error}"),
        }
    });
}

fn screenshot_offscreen_prewarm_enabled() -> bool {
    std::env::var("YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW")
        .ok()
        .as_deref()
        == Some("1")
}

async fn pulse_screenshot_window_for_webview_prewarm(window: tauri::WebviewWindow) {
    if !screenshot_offscreen_prewarm_enabled() {
        println!("[screenshot-trace] startup offscreen screenshot prewarm show disabled; hidden WebView prewarm only");
        return;
    }
    clear_screenshot_window_bounds_cache();
    let _ = window.set_position(tauri::PhysicalPosition::new(-32000, -32000));
    let _ = window.set_size(tauri::PhysicalSize::new(1_u32, 1_u32));
    let _ = window.set_skip_taskbar(true);
    crate::window_lifecycle::show_screenshot_overlay_window(&window);
    println!("[screenshot-trace] startup offscreen screenshot prewarm shown");
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    if CAPTURING.load(Ordering::SeqCst) {
        println!("[screenshot-trace] startup offscreen screenshot prewarm kept visible because capture started");
        return;
    }
    crate::window_lifecycle::hide_window_without_activation(&window);
    clear_screenshot_window_bounds_cache();
    println!("[screenshot-trace] startup offscreen screenshot prewarm hidden");
}

fn set_screenshot_window_bounds_if_changed(
    window: &tauri::WebviewWindow,
    session_id: &str,
    started_at: &Instant,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    let next = (x, y, width, height);
    let unchanged = get_last_screenshot_window_bounds_store()
        .lock()
        .map(|guard| guard.as_ref() == Some(&next))
        .unwrap_or(false);

    if unchanged {
        log_screenshot_baseline(
            session_id,
            "overlay_bounds_reused",
            started_at,
            &format!("screen={}x{}@{},{}", width, height, x, y),
        );
        return;
    }

    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = window.set_size(tauri::PhysicalSize::new(width.max(1), height.max(1)));
    if let Ok(mut guard) = get_last_screenshot_window_bounds_store().lock() {
        *guard = Some(next);
    }
    log_screenshot_baseline(
        session_id,
        "overlay_bounds_updated",
        started_at,
        &format!("screen={}x{}@{},{}", width, height, x, y),
    );
}

fn prepare_screenshot_overlay_window(
    app: &tauri::AppHandle,
    session_id: &str,
    started_at: &Instant,
    screenshot_mode: &str,
) -> Result<tauri::WebviewWindow, String> {
    let screenshot_win = ensure_screenshot_window(app, "ready-overlay").map_err(|error| {
        crate::window_lifecycle::restore_main_window_after_screenshot(
            app,
            "create-screenshot-overlay-error",
        );
        error
    })?;
    disable_windows_transition(&screenshot_win);
    let (x, y, width, height) = current_screen_origin();
    let safe_width = width.max(1) as u32;
    let safe_height = height.max(1) as u32;
    set_screenshot_window_bounds_if_changed(
        &screenshot_win,
        session_id,
        started_at,
        x,
        y,
        safe_width,
        safe_height,
    );
    let _ = screenshot_win.set_always_on_top(true);
    let _ = screenshot_win.set_skip_taskbar(true);
    let capture_exclusion = screenshot_capture_exclusion_enabled();
    let _ =
        crate::window_lifecycle::set_webview_capture_excluded(app, "screenshot", capture_exclusion);
    log_screenshot_baseline(
        session_id,
        "overlay_capture_exclusion",
        started_at,
        &format!(
            "excluded={} env=YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE",
            capture_exclusion
        ),
    );
    let transparent = screenshot_window_transparency_enabled();
    let native_first_frame = native_first_frame_session_enabled();
    let show_shell_before_ready = screenshot_early_visible_shell_enabled() && !native_first_frame;
    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.to_string());
    let shell_payload = serde_json::json!({
            "mode": screenshot_mode,
            "sessionId": session_id,
            "transparent": transparent,
            "nativeVisible": false,
            "nativeFirstFrame": native_first_frame,
            "showOnShellReady": show_shell_before_ready,
            "deferredShowUntilReady": !show_shell_before_ready,
            "screen": {
                "x": x,
                "y": y,
                "width": safe_width,
                "height": safe_height
            }
    });
    set_latest_screenshot_shell_payload(shell_payload.clone());
    if show_shell_before_ready {
        let _ = screenshot_win.emit("screenshot-shell", shell_payload);
        log_screenshot_baseline(
            session_id,
            "visible_shell_show_delegated",
            started_at,
            &format!("screen={}x{}@{},{}", safe_width, safe_height, x, y),
        );
    } else {
        crate::window_lifecycle::hide_window_without_activation(&screenshot_win);
        let _ = screenshot_win.emit("screenshot-shell", shell_payload);
        log_screenshot_baseline(
            session_id,
            "shell_deferred_until_ready",
            started_at,
            &format!("screen={}x{}@{},{}", safe_width, safe_height, x, y),
        );
    }
    Ok(screenshot_win)
}

fn screenshot_early_visible_shell_enabled() -> bool {
    if std::env::var("YSN_SCREENSHOT_DEFER_VISIBLE_SHELL")
        .ok()
        .as_deref()
        == Some("1")
    {
        return false;
    }
    match std::env::var("YSN_SCREENSHOT_EARLY_VISIBLE_SHELL")
        .ok()
        .as_deref()
    {
        Some("0") => false,
        Some("1") => true,
        _ => true,
    }
}

fn screenshot_left_button_down() -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        (win32::GetAsyncKeyState(0x01) & i16::MIN) != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn update_pointer_pre_capture_distance(state: &mut ScreenshotPointerPreCapture) {
    let (Some((down_x, down_y)), Some((latest_x, latest_y))) =
        (state.down_global, state.latest_global)
    else {
        return;
    };
    let dx = f64::from(latest_x - down_x);
    let dy = f64::from(latest_y - down_y);
    state.max_drag_distance = state.max_drag_distance.max((dx * dx + dy * dy).sqrt());
}

fn start_screenshot_pointer_pre_capture(session_id: &str, origin_x: i32, origin_y: i32) {
    let session_id = session_id.to_string();
    let started_at = Instant::now();
    let initial_down = screenshot_left_button_down();
    let initial_cursor = get_cursor_position();
    if let Ok(mut guard) = get_screenshot_pointer_pre_capture_store().lock() {
        *guard = Some(ScreenshotPointerPreCapture {
            session_id: session_id.clone(),
            origin_x,
            origin_y,
            started_at,
            updated_at: started_at,
            was_down_at_start: initial_down,
            left_down: initial_down,
            completed: false,
            down_global: initial_cursor.filter(|_| initial_down),
            latest_global: initial_cursor.filter(|_| initial_down),
            max_drag_distance: 0.0,
        });
    }

    std::thread::spawn(move || {
        for _ in 0..450 {
            std::thread::sleep(std::time::Duration::from_millis(4));
            let left_down = screenshot_left_button_down();
            let cursor = get_cursor_position();
            let mut should_stop = false;
            if let Ok(mut guard) = get_screenshot_pointer_pre_capture_store().lock() {
                let Some(state) = guard.as_mut() else {
                    break;
                };
                if state.session_id != session_id {
                    break;
                }
                if started_at.elapsed() > std::time::Duration::from_millis(1800) {
                    should_stop = true;
                }
                state.updated_at = Instant::now();
                if left_down {
                    if state.down_global.is_none() {
                        state.down_global = cursor;
                    }
                    state.latest_global = cursor.or(state.latest_global);
                    state.left_down = true;
                    state.completed = false;
                    update_pointer_pre_capture_distance(state);
                } else {
                    if state.left_down && state.down_global.is_some() {
                        state.latest_global = cursor.or(state.latest_global);
                        state.completed = true;
                        update_pointer_pre_capture_distance(state);
                    }
                    state.left_down = false;
                }
            } else {
                break;
            }
            if should_stop {
                break;
            }
        }
    });
}

pub(crate) fn read_screenshot_pointer_pre_capture_selection(
    expected_session_id: &str,
) -> Option<(i32, i32, i32, i32)> {
    let Ok(guard) = get_screenshot_pointer_pre_capture_store().lock() else {
        return None;
    };
    let Some(state) = guard.as_ref() else {
        return None;
    };
    if state.session_id != expected_session_id {
        return None;
    }
    let (Some((down_x, down_y)), Some((latest_x, latest_y))) =
        (state.down_global, state.latest_global)
    else {
        return None;
    };
    let left = down_x.min(latest_x);
    let top = down_y.min(latest_y);
    let right = down_x.max(latest_x);
    let bottom = down_y.max(latest_y);
    let w = right - left;
    let h = bottom - top;
    if w > 5 && h > 5 {
        Some((left, top, w, h))
    } else {
        None
    }
}

pub(crate) fn read_screenshot_pointer_pre_capture_activity(
    expected_session_id: Option<&str>,
) -> Option<ScreenshotPointerPreCaptureActivity> {
    let Ok(guard) = get_screenshot_pointer_pre_capture_store().lock() else {
        return None;
    };
    let Some(state) = guard.as_ref() else {
        return None;
    };
    if let Some(expected_session_id) = expected_session_id {
        if state.session_id != expected_session_id {
            return None;
        }
    }
    if state.started_at.elapsed() > std::time::Duration::from_millis(2500) {
        return None;
    }
    Some(ScreenshotPointerPreCaptureActivity {
        left_down: state.left_down,
        completed: state.completed,
        has_drag: state.max_drag_distance >= 3.0,
        drag_distance: state.max_drag_distance,
    })
}

fn screenshot_pointer_pre_capture_json(session_id: Option<&str>) -> serde_json::Value {
    let Ok(guard) = get_screenshot_pointer_pre_capture_store().lock() else {
        return serde_json::Value::Null;
    };
    let Some(state) = guard.as_ref() else {
        return serde_json::Value::Null;
    };
    if let Some(expected_session_id) = session_id {
        if state.session_id != expected_session_id {
            return serde_json::Value::Null;
        }
    }
    if state.started_at.elapsed() > std::time::Duration::from_millis(2500) {
        return serde_json::Value::Null;
    }
    let Some((down_global_x, down_global_y)) = state.down_global else {
        return serde_json::json!({
            "sessionId": state.session_id,
            "available": false,
            "leftDown": state.left_down,
            "completed": state.completed,
            "wasDownAtStart": state.was_down_at_start,
            "startedAgeMs": state.started_at.elapsed().as_millis(),
            "updatedAgeMs": state.updated_at.elapsed().as_millis()
        });
    };
    let (latest_global_x, latest_global_y) = state
        .latest_global
        .unwrap_or((down_global_x, down_global_y));
    serde_json::json!({
        "sessionId": state.session_id,
        "available": true,
        "leftDown": state.left_down,
        "completed": state.completed,
        "wasDownAtStart": state.was_down_at_start,
        "x": down_global_x - state.origin_x,
        "y": down_global_y - state.origin_y,
        "currentX": latest_global_x - state.origin_x,
        "currentY": latest_global_y - state.origin_y,
        "globalX": down_global_x,
        "globalY": down_global_y,
        "currentGlobalX": latest_global_x,
        "currentGlobalY": latest_global_y,
        "dragDistance": state.max_drag_distance,
        "startedAgeMs": state.started_at.elapsed().as_millis(),
        "updatedAgeMs": state.updated_at.elapsed().as_millis()
    })
}

fn native_overlay_selection_json(session_id: Option<&str>) -> serde_json::Value {
    let Some(snapshot) =
        crate::screenshot_native::cpu_native_overlay_selection_snapshot(session_id)
    else {
        return serde_json::Value::Null;
    };
    let base = serde_json::json!({
        "sessionId": session_id.unwrap_or("unknown"),
        "inputStarted": snapshot.input_started,
        "leftDown": snapshot.mouse_captured,
        "completed": snapshot.completed,
        "cancelled": snapshot.cancelled,
        "phase": snapshot.phase.as_str(),
        "eventSeq": snapshot.event_seq,
        "handoffReady": snapshot.phase.handoff_ready(),
        "source": "native-overlay",
        "hwnd": snapshot.hwnd
    });
    let Some(selection) = snapshot.selection else {
        let mut value = base;
        if let Some(object) = value.as_object_mut() {
            object.insert("available".to_string(), serde_json::Value::Bool(false));
        }
        return value;
    };
    let left = selection.left.min(selection.right);
    let top = selection.top.min(selection.bottom);
    let right = selection.left.max(selection.right);
    let bottom = selection.top.max(selection.bottom);
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    if width <= 0 || height <= 0 {
        let mut value = base;
        if let Some(object) = value.as_object_mut() {
            object.insert("available".to_string(), serde_json::Value::Bool(false));
        }
        return value;
    }
    let drag_distance = (f64::from(width).powi(2) + f64::from(height).powi(2)).sqrt();
    let mut value = base;
    if let Some(object) = value.as_object_mut() {
        object.insert("available".to_string(), serde_json::Value::Bool(true));
        object.insert("x".to_string(), serde_json::json!(left));
        object.insert("y".to_string(), serde_json::json!(top));
        object.insert("currentX".to_string(), serde_json::json!(right));
        object.insert("currentY".to_string(), serde_json::json!(bottom));
        object.insert(
            "globalX".to_string(),
            serde_json::json!(snapshot.bounds.origin_x + left),
        );
        object.insert(
            "globalY".to_string(),
            serde_json::json!(snapshot.bounds.origin_y + top),
        );
        object.insert(
            "currentGlobalX".to_string(),
            serde_json::json!(snapshot.bounds.origin_x + right),
        );
        object.insert(
            "currentGlobalY".to_string(),
            serde_json::json!(snapshot.bounds.origin_y + bottom),
        );
        object.insert("dragDistance".to_string(), serde_json::json!(drag_distance));
        object.insert(
            "rect".to_string(),
            serde_json::json!({
                "x": left,
                "y": top,
                "w": width,
                "h": height
            }),
        );
    }
    value
}

pub async fn start_screenshot_impl(
    app: tauri::AppHandle,
    mode: Option<String>,
    run_generation: crate::screenshot_native::ScreenshotRunGeneration,
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
    let (pointer_origin_x, pointer_origin_y, _, _) = current_screen_origin();
    start_screenshot_pointer_pre_capture(&session_id, pointer_origin_x, pointer_origin_y);
    let native_overlay_plan = crate::screenshot_native::default_native_overlay_launch_plan();
    log_native_overlay_launch_plan(&session_id, &started_at, native_overlay_plan);
    if native_overlay_plan.uses_native_overlay() {
        log_native_overlay_launch_plan(
            &session_id,
            &started_at,
            native_overlay_mvp_fallback_plan(),
        );
    }
    crate::window_lifecycle::remember_pre_screenshot_foreground("start-screenshot");
    let _ = crate::window_lifecycle::set_webview_capture_excluded(&app, "main", false);
    start_text_source_snapshot_capture(&app);
    let cleanup =
        crate::screenshot_native::cancel_cpu_native_overlay_session("new-screenshot-start");
    if cleanup.active
        || !matches!(
            cleanup.state,
            crate::screenshot_native::NativeOverlaySessionState::Empty
        )
    {
        log_cpu_native_overlay_diagnostics(
            &session_id,
            &started_at,
            "native_overlay_previous_cleanup",
            cleanup,
        );
    }

    let main_hidden_for_capture = crate::window_lifecycle::prepare_main_window_for_screenshot(&app);
    log_screenshot_baseline(
        &session_id,
        "main_window_prepared",
        &started_at,
        &format!("hidden_for_capture={}", main_hidden_for_capture),
    );

    if main_hidden_for_capture && crate::window_lifecycle::current_screenshot_capture_needs_settle()
    {
        crate::window_lifecycle::wait_for_hidden_main_capture_settle().await;
        log_screenshot_baseline(&session_id, "main_hidden_settled", &started_at, "");
    }

    log_screenshot_baseline(&session_id, "capture_start", &started_at, "");
    let capture_task = tokio::task::spawn_blocking(capture_current_monitor_rgba);

    close_screenshot_windows(&app, false);
    let screenshot_win =
        prepare_screenshot_overlay_window(&app, &session_id, &started_at, &screenshot_mode)?;
    log_screenshot_baseline(
        &session_id,
        "overlay_window_prepared",
        &started_at,
        &format!(
            "generation={} transparent_input_shell={}",
            run_generation,
            screenshot_early_visible_shell_enabled()
        ),
    );

    let (rgba_image, screen_info) = match capture_task
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
            "format=rgba bytes={} screen={}x{}@{},{}",
            rgba_image.bytes.len(),
            screen_info.2,
            screen_info.3,
            screen_info.0,
            screen_info.1
        ),
    );
    println!(
        "[screenshot-perf] capture ready {}ms format=rgba bytes={}",
        started_at.elapsed().as_millis(),
        rgba_image.bytes.len()
    );

    if crate::screenshot_native::is_stale_generation(run_generation) {
        let cleanup =
            crate::screenshot_native::cancel_cpu_native_overlay_session("capture-stale-generation");
        log_cpu_native_overlay_diagnostics(
            &session_id,
            &started_at,
            "native_overlay_capture_stale_cleanup",
            cleanup,
        );
        log_screenshot_baseline(
            &session_id,
            "capture_discarded_stale_generation",
            &started_at,
            &format!("generation={}", run_generation),
        );
        return Ok(());
    }

    if let Ok(mut guard) = get_screenshot_rgba().lock() {
        *guard = Some(SessionScreenshotRgba {
            session_id: session_id.clone(),
            frame: rgba_image.clone(),
        });
    }
    if let Ok(mut guard) = get_screenshot_image().lock() {
        *guard = None;
    }

    match crate::screenshot_shared_buffer::post_rgba_frame_to_webview(
        screenshot_win.as_ref().clone(),
        session_id.clone(),
        &rgba_image,
    ) {
        Ok(posted) if posted.posted => log_screenshot_baseline(
            &session_id,
            "shared_buffer_direct_posted",
            &started_at,
            &format!(
                "bytes={} size={}x{} transfer_type={}",
                posted.bytes, posted.width, posted.height, posted.transfer_type
            ),
        ),
        Ok(posted) => log_screenshot_baseline(
            &session_id,
            "shared_buffer_direct_unavailable",
            &started_at,
            posted.reason.as_deref().unwrap_or("unknown"),
        ),
        Err(error) => log_screenshot_baseline(
            &session_id,
            "shared_buffer_direct_failed",
            &started_at,
            &error,
        ),
    }

    if native_first_frame_session_enabled() {
        let bounds = crate::screenshot_native::MonitorCaptureBounds::new(
            screen_info.0,
            screen_info.1,
            screen_info.2,
            screen_info.3,
        );
        match crate::screenshot_native::begin_cpu_native_overlay_session(
            session_id.clone(),
            run_generation.value(),
            bounds,
            &rgba_image,
        ) {
            Ok(diagnostics) => log_cpu_native_overlay_diagnostics(
                &session_id,
                &started_at,
                "native_first_frame_visible",
                diagnostics,
            ),
            Err(error) => log_screenshot_baseline(
                &session_id,
                "native_first_frame_fallback",
                &started_at,
                &format!("reason={error}"),
            ),
        }
    } else {
        log_screenshot_baseline(
            &session_id,
            "native_first_frame_session_disabled",
            &started_at,
            "set YSN_NATIVE_FIRST_FRAME_SESSION=1 only for guarded native-session diagnostics; default remains WebView SharedBuffer/transparent-shell path",
        );
    }

    // Encode PNG in the background for compatibility and backup only. Do not block payload emission or overlay readiness.
    encode_and_store_fullscreen_png(session_id.clone(), started_at, rgba_image.clone());

    let (x, y, width, height) = screen_info;
    let physical_bounds = crate::screenshot_native::MonitorCaptureBounds::new(x, y, width, height);
    set_screenshot_window_bounds_if_changed(
        &screenshot_win,
        &session_id,
        &started_at,
        x,
        y,
        width.max(1),
        height.max(1),
    );
    let _ = screenshot_win.set_always_on_top(true);

    if crate::screenshot_native::is_stale_generation(run_generation) {
        let cleanup =
            crate::screenshot_native::cancel_cpu_native_overlay_session("payload-stale-generation");
        log_cpu_native_overlay_diagnostics(
            &session_id,
            &started_at,
            "native_overlay_payload_stale_cleanup",
            cleanup,
        );
        log_screenshot_baseline(
            &session_id,
            "payload_discarded_stale_generation",
            &started_at,
            &format!("generation={}", run_generation),
        );
        return Ok(());
    }

    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let payload = serde_json::json!({
        "kind": "rgba",
        "bytes": rgba_image.bytes.len(),
        "width": rgba_image.width,
        "height": rgba_image.height,
        "physicalBounds": screenshot_physical_bounds_json(physical_bounds),
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
    session_id: Option<String>,
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
    let left_down = screenshot_left_button_down();
    Ok(serde_json::json!({
        "leftDown": left_down,
        "x": global_x - window_x,
        "y": global_y - window_y,
        "globalX": global_x,
        "globalY": global_y,
        "preCapture": screenshot_pointer_pre_capture_json(session_id.as_deref()),
        "nativeOverlay": native_overlay_selection_json(session_id.as_deref())
    }))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSelectedImageBridgeRequest {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub action: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSelectedOutputCopyRequest {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub session_id: Option<String>,
    pub explicit_opt_in: bool,
}

#[tauri::command]
pub fn build_native_selected_image_bridge(
    request: NativeSelectedImageBridgeRequest,
) -> Result<serde_json::Value, String> {
    let frame = get_matching_screenshot_rgba(request.session_id.as_deref())?;
    let action = parse_output_action(request.action)?;
    if matches!(action, crate::screenshot_native::OutputAction::Record) {
        return Err("record is not a selected-image bridge action".to_string());
    }
    let selection = crate::screenshot_native::SelectionRect::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    let contract =
        crate::screenshot_native::selected_image_bridge::build_selected_image_bridge_contract(
            action, &frame, selection,
        )
        .map_err(|error| error.to_string())?;
    let diagnostics =
        crate::screenshot_native::selected_image_bridge::diagnose_selected_image_bridge(&contract);

    Ok(serde_json::json!({
        "pngBase64": contract.png_base64,
        "dataUrl": contract.data_url,
        "description": diagnostics.description,
        "diagnostics": {
            "pngSignatureValid": diagnostics.png_signature_valid,
            "dataUrlPrefixValid": diagnostics.data_url_prefix_valid,
            "base64MatchesPng": diagnostics.base64_matches_png,
            "selectedOnlyPng": diagnostics.selected_only_png,
            "isValidBridge": diagnostics.is_valid()
        }
    }))
}

#[tauri::command]
pub fn copy_native_selected_output_to_clipboard(
    request: NativeSelectedOutputCopyRequest,
) -> Result<serde_json::Value, String> {
    let mut sink =
        crate::screenshot_native::selected_output_clipboard::ArboardSelectedOutputEffectSink::new();
    copy_native_selected_output_to_clipboard_with_sink(request, &mut sink)
}

fn copy_native_selected_output_to_clipboard_with_sink(
    request: NativeSelectedOutputCopyRequest,
    sink: &mut impl crate::screenshot_native::selected_output_effects::SelectedOutputEffectSink,
) -> Result<serde_json::Value, String> {
    let frame = get_matching_screenshot_rgba(request.session_id.as_deref())?;
    let selection = crate::screenshot_native::SelectionRect::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    let contract =
        crate::screenshot_native::selected_image_bridge::build_selected_image_bridge_contract(
            crate::screenshot_native::OutputAction::Copy,
            &frame,
            selection,
        )
        .map_err(|error| error.to_string())?;
    let diagnostics =
        crate::screenshot_native::selected_image_bridge::diagnose_selected_image_bridge(&contract);
    let receipt = crate::screenshot_native::selected_output_effects::perform_selected_output_effect_with_sink(
        &contract,
        crate::screenshot_native::selected_output_effects::SelectedOutputEffectRequest {
            explicit_opt_in: request.explicit_opt_in,
        },
        sink,
    )
    .map_err(|error| error.to_string())?;

    Ok(serde_json::json!({
        "action": output_action_label(receipt.action),
        "target": output_bridge_target_label(receipt.target),
        "format": output_image_format_label(receipt.format),
        "selectedOnlyPng": receipt.selected_only_png,
        "pngByteLen": receipt.png_byte_len,
        "copiedToClipboard": receipt.copied_to_clipboard,
        "saveInvoked": receipt.save_invoked,
        "ocrInvoked": receipt.ocr_invoked,
        "translationInvoked": receipt.translation_invoked,
        "diagnostics": {
            "pngSignatureValid": diagnostics.png_signature_valid,
            "dataUrlPrefixValid": diagnostics.data_url_prefix_valid,
            "base64MatchesPng": diagnostics.base64_matches_png,
            "selectedOnlyPng": diagnostics.selected_only_png,
            "isValidBridge": diagnostics.is_valid()
        }
    }))
}

fn output_action_label(action: crate::screenshot_native::OutputAction) -> &'static str {
    match action {
        crate::screenshot_native::OutputAction::Copy => "copy",
        crate::screenshot_native::OutputAction::SaveAs => "saveAs",
        crate::screenshot_native::OutputAction::Ocr => "ocr",
        crate::screenshot_native::OutputAction::Translate => "translate",
        crate::screenshot_native::OutputAction::Record => "record",
    }
}

fn output_bridge_target_label(
    target: crate::screenshot_native::OutputBridgeTarget,
) -> &'static str {
    match target {
        crate::screenshot_native::OutputBridgeTarget::Clipboard => "clipboard",
        crate::screenshot_native::OutputBridgeTarget::File => "file",
        crate::screenshot_native::OutputBridgeTarget::Ocr => "ocr",
        crate::screenshot_native::OutputBridgeTarget::Translation => "translation",
    }
}

fn output_image_format_label(format: crate::screenshot_native::OutputImageFormat) -> &'static str {
    match format {
        crate::screenshot_native::OutputImageFormat::Png => "png",
    }
}

#[tauri::command]
pub fn get_native_screenshot_diagnostics_status() -> Result<serde_json::Value, String> {
    let d3d11 = crate::screenshot_native::gpu::probe_d3d11_gpu_capability();
    let wgc_api = crate::screenshot_native::wgc_probe::probe_wgc_native_api_support();
    let wgc_plan = crate::screenshot_native::wgc_probe::default_wgc_one_frame_probe_plan();
    let wgc_smoke = crate::screenshot_native::wgc_probe::planned_wgc_one_frame_smoke_report();
    let dxgi_api = crate::screenshot_native::dxgi_probe::probe_dxgi_native_api_support();
    let dxgi_contract =
        crate::screenshot_native::dxgi_capture::DxgiDesktopDuplicationContract::placeholder();
    let gpu_plan = crate::screenshot_native::gpu::d3d11_first_gpu_capture_plan();
    let native_overlay_launch_plan = crate::screenshot_native::default_native_overlay_launch_plan();
    let native_overlay_smoke =
        crate::screenshot_native::native_overlay_smoke::planned_native_overlay_smoke_report();
    let native_overlay_session = crate::screenshot_native::cpu_native_overlay_session_diagnostics();
    let pump_contract = crate::screenshot_native::win32_overlay_pump_contract();
    let native_route_readiness = crate::screenshot_native::default_native_route_readiness(
        native_overlay_launch_plan,
        pump_contract,
    );
    let native_route_readiness_blockers = native_route_readiness.blocker_labels();

    Ok(serde_json::json!({
        "d3d11": {
            "capability": {
                "backend": debug_value(d3d11.capability.backend),
                "textureInterop": debug_value(d3d11.capability.texture_interop),
                "status": debug_value(d3d11.capability.status),
                "missingRequirements": d3d11.capability.missing_requirements.iter().map(debug_value).collect::<Vec<_>>(),
                "fallback": debug_value(&d3d11.capability.fallback),
                "reason": d3d11.capability.reason.clone(),
                "usable": d3d11.capability.is_usable()
            },
            "diagnostics": {
                "adapterPreference": debug_value(d3d11.diagnostics.adapter_preference),
                "adapterLabel": d3d11.diagnostics.adapter_label,
                "featureLevel": debug_value(d3d11.diagnostics.feature_level),
                "debugLayerRequested": d3d11.diagnostics.debug_layer_requested,
                "usedDefaultAdapter": d3d11.diagnostics.used_default_adapter,
                "fallbackReason": d3d11.diagnostics.fallback_reason
            }
        },
        "wgc": {
            "nativeApi": {
                "isWindows": wgc_api.is_windows,
                "isSupported": wgc_api.is_supported,
                "reason": wgc_api.reason
            },
            "oneFrameProbe": {
                "contract": {
                    "default": debug_value(wgc_plan.contract.default),
                    "requiresExplicitOptIn": wgc_plan.contract.requires_explicit_opt_in,
                    "mayCallRealWgcApi": wgc_plan.contract.may_call_real_wgc_api,
                    "validatesNoDefaultEnable": wgc_plan.contract.validates_no_default_enable()
                },
                "status": debug_value(wgc_plan.status),
                "shouldAttemptProbe": wgc_plan.should_attempt_probe,
                "fallback": debug_value(wgc_plan.fallback),
                "error": wgc_plan.error.as_ref().map(debug_value),
                "reason": wgc_plan.reason.clone(),
                "usesFallback": wgc_plan.uses_fallback(),
                "runtimeSmokeRegistered": true,
                "runtimeSmokeCommand": "run_native_wgc_one_frame_probe_smoke",
                "wiredToAltA": false
            },
            "oneFrameSmoke": {
                "status": wgc_smoke.status.as_str(),
                "planStatus": debug_value(wgc_smoke.plan_status),
                "attemptedRealWgcApi": wgc_smoke.attempted_real_wgc_api,
                "frameCaptureAttempted": wgc_smoke.frame_capture_attempted,
                "frameCaptureConfirmed": wgc_smoke.frame_capture_confirmed,
                "shouldAttemptProbe": wgc_smoke.should_attempt_probe,
                "fallback": debug_value(wgc_smoke.fallback),
                "error": wgc_smoke.error.as_ref().map(debug_value),
                "reason": wgc_smoke.reason.clone()
            }
        },
        "dxgi": {
            "nativeApi": {
                "isWindows": dxgi_api.is_windows,
                "hasFactory": dxgi_api.has_factory,
                "hasAdapter": dxgi_api.has_adapter,
                "hasOutput": dxgi_api.has_output,
                "desktopCoordinates": dxgi_api.desktop_coordinates.map(dxgi_desktop_coordinates_json),
                "reason": dxgi_api.reason
            },
            "desktopDuplication": {
                "backend": debug_value(dxgi_contract.backend),
                "textureInterop": debug_value(dxgi_contract.texture_interop),
                "requiredCapabilities": dxgi_contract.required_capabilities.iter().map(debug_value).collect::<Vec<_>>(),
                "fallbackTarget": debug_value(dxgi_contract.fallback_target),
                "readiness": debug_value(dxgi_contract.readiness),
                "textureAcquisition": {
                    "contractReady": true,
                    "capturesWithoutImmediateReadback": true,
                    "returnsD3d11TextureFrame": true,
                    "runtimeProbeAttempted": false,
                    "presenterInteropReady": false,
                    "selectedRegionReadbackOnly": true,
                    "selectedReadback": {
                        "infrastructureReady": true,
                        "boundedCropValidation": true,
                        "copySubresourceRegion": true,
                        "bgraToRgba": true,
                        "bridgeAdapterReady": true,
                        "runtimeSmokeRegistered": true,
                        "runtimeSmokeCommand": "run_native_dxgi_selected_readback_smoke",
                        "selectedOutputBridgeDryRunRegistered": true,
                        "selectedOutputBridgeDryRunCommand": "run_native_dxgi_selected_output_bridge_dry_run",
                        "selectedOutputBridgeDryRunOutputSideEffects": false,
                        "wiredToAltA": false
                    },
                    "reason": "DXGI can describe acquired D3D11 textures and has a selected-region-only staging readback adapter; presenter interop, manual runtime acceptance, and Alt+A wiring remain pending."
                },
                "reason": dxgi_contract.reason,
                "capability": {
                    "status": debug_value(dxgi_contract.capability().status),
                    "fallback": debug_value(dxgi_contract.capability().fallback),
                    "reason": dxgi_contract.capability().reason
                }
            }
        },
        "gpuPlan": {
            "primaryStatus": debug_value(gpu_plan.primary.status),
            "primaryFallback": debug_value(&gpu_plan.primary.fallback),
            "selected": gpu_plan.selected().map(|capability| serde_json::json!({
                "backend": debug_value(capability.backend),
                "textureInterop": debug_value(capability.texture_interop),
                "status": debug_value(capability.status),
                "fallback": debug_value(&capability.fallback),
                "reason": capability.reason.clone()
            })),
            "fallbackCount": gpu_plan.fallbacks.len()
        },
        "nativeOverlay": {
            "launchPlan": {
                "runtime": debug_value(native_overlay_launch_plan.runtime),
                "capability": debug_value(native_overlay_launch_plan.capability),
                "fallbackReason": native_overlay_launch_plan.fallback_reason.as_ref().map(|reason| reason.as_str()),
                "usesNativeOverlay": native_overlay_launch_plan.uses_native_overlay(),
                "usesFallback": native_overlay_launch_plan.fallback_reason.is_some(),
                "captureRouteStatus": debug_value(native_overlay_launch_plan.capture_route.status),
                "captureRouteFallbackReason": debug_value(native_overlay_launch_plan.capture_route.fallback_reason),
                "captureRouteContract": debug_value(native_overlay_launch_plan.capture_route.contract)
            },
            "plannedSmoke": {
                "status": debug_value(native_overlay_smoke.status),
                "completedSteps": native_overlay_smoke.completed_steps,
                "failedStep": native_overlay_smoke.failed_step.map(debug_value),
                "recoveryAction": native_overlay_smoke.recovery_action.map(debug_value),
                "hasRuntimeSideEffects": native_overlay_smoke.plan.has_runtime_side_effects,
                "lifecycleChecksRequired": native_overlay_smoke.plan.lifecycle_checks_required(),
                "stepCount": native_overlay_smoke.plan.steps.len(),
                "requiresCandidateEdgeCaseCheck": native_overlay_smoke.plan.requires_candidate_edge_case_check,
                "requiresDpiBoundaryCheck": native_overlay_smoke.plan.requires_dpi_boundary_check
            },
            "activeSession": {
                "active": native_overlay_session.active,
                "state": native_overlay_session.state.as_str(),
                "runtime": native_overlay_session.runtime.map(|runtime| runtime.as_str()),
                "hwnd": native_overlay_session.hwnd,
                "rendered": native_overlay_session.rendered,
                "visible": native_overlay_session.visible,
                "fallbackReason": native_overlay_session.fallback_reason
            },
            "pumpReadiness": {
                "ownsThread": pump_contract.owns_thread,
                "dispatchesInput": pump_contract.dispatches_input,
                "blocksUntilTerminal": pump_contract.blocks_until_terminal,
                "supportsTimeout": pump_contract.supports_timeout,
                "restoresFocusOnExit": pump_contract.restores_focus_on_exit,
                "messageLoopDispatchReady": false,
                "wndProcInputDispatchReady": false,
                "readiness": "contract-only"
            },
            "mainRouteReadiness": {
                "recommendedRoute": native_route_readiness.recommended_route.as_str(),
                "readyForNativeDxgiSelectedOutput": native_route_readiness.ready_for_native_dxgi_selected_output,
                "blockers": native_route_readiness_blockers,
                "currentRouteRemains": "webview-rgba",
                "nativeRouteCandidate": "native-dxgi-selected-output",
                "changesDefaultAltA": false,
                "readiness": "diagnostic-only"
            }
        }
    }))
}

#[tauri::command]
pub fn run_native_dxgi_texture_smoke() -> Result<serde_json::Value, String> {
    let report = crate::screenshot_native::dxgi_smoke::run_dxgi_texture_acquisition_smoke(
        crate::screenshot_native::MonitorCaptureBounds::new(0, 0, 1, 1),
    );
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": report.stage.as_str(),
        "elapsedMs": report.elapsed_ms,
        "frameId": report.frame_id,
        "width": report.width,
        "height": report.height,
        "format": report.format.map(debug_value),
        "sessionState": debug_value(report.session_state),
        "releasedFrame": report.released_frame,
        "stopped": report.stopped,
        "error": report.error,
        "scope": "diagnostic-only; does not mark DXGI presenter, selected readback, native overlay, or C/E readiness complete"
    }))
}

fn dxgi_frame_info_probe_disabled_response(
    request: &NativeDxgiFrameInfoProbeRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "defaultOutput": serde_json::Value::Null,
        "selectedOutput": serde_json::Value::Null,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": error,
        "scope": "diagnostic-only; probes DXGI AcquireNextFrame attempts and frame-info metadata without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

fn dxgi_desktop_update_pulse_disabled_response(
    request: &NativeDxgiDesktopUpdatePulseDiagnosticRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "before": serde_json::Value::Null,
        "pulse": serde_json::Value::Null,
        "after": serde_json::Value::Null,
        "comparison": {
            "beforeBothTimedOut": false,
            "afterAnyFrameConfirmed": false,
            "pulseUnblockedDefault": false,
            "pulseUnblockedSelected": false,
        },
        "pulseSizePx": request.pulse_size_px.unwrap_or(2),
        "pulseAlpha": request.pulse_alpha.unwrap_or(1),
        "dwellMs": request.dwell_ms.unwrap_or(16),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealDesktopPulse": request.allow_real_desktop_pulse.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": error,
        "scope": "diagnostic-only; creates a tiny non-activating layered desktop update pulse between DXGI frame-info probes; no clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

fn dxgi_pulse_before_acquire_disabled_response(
    request: &NativeDxgiPulseBeforeAcquireProbeRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "defaultOutput": serde_json::Value::Null,
        "selectedOutput": serde_json::Value::Null,
        "comparison": {
            "defaultFrameConfirmed": false,
            "selectedFrameConfirmed": false,
            "anyFrameConfirmed": false,
            "bothTimedOut": false,
        },
        "pulseSizePx": request.pulse_size_px.unwrap_or(2),
        "pulseAlpha": request.pulse_alpha.unwrap_or(1),
        "dwellMs": request.dwell_ms.unwrap_or(16),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealDesktopPulse": request.allow_real_desktop_pulse.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": error,
        "scope": "diagnostic-only; opens DXGI duplication, creates a tiny non-activating desktop pulse, then immediately calls AcquireNextFrame in the same session without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

#[tauri::command]
pub fn run_native_dxgi_desktop_update_pulse_diagnostic_smoke(
    request: NativeDxgiDesktopUpdatePulseDiagnosticRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_desktop_update_pulse_disabled_response(
            &request,
            "DXGI desktop-update pulse diagnostic requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_desktop_update_pulse_disabled_response(
            &request,
            "DXGI desktop-update pulse diagnostic real DXGI API calls are not allowed",
        ));
    }
    if !request.allow_real_desktop_pulse.unwrap_or(false) {
        return Ok(dxgi_desktop_update_pulse_disabled_response(
            &request,
            "DXGI desktop-update pulse diagnostic real desktop pulse is not allowed",
        ));
    }
    let bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    if bounds.is_empty() {
        return Ok(dxgi_desktop_update_pulse_disabled_response(
            &request,
            "DXGI desktop-update pulse diagnostic requires non-empty bounds",
        ));
    }

    let probe_request = NativeDxgiFrameInfoProbeRequest {
        x: request.x,
        y: request.y,
        width: request.width,
        height: request.height,
        explicit_opt_in: Some(true),
        allow_real_dxgi_api: Some(true),
    };
    let before = run_native_dxgi_frame_info_probe(probe_request)?;
    let pulse_request =
        crate::screenshot_native::win32_desktop_update_pulse::DesktopUpdatePulseRequest::new(
            bounds,
            request.pulse_size_px.unwrap_or(2),
            request.pulse_alpha.unwrap_or(1),
            request.dwell_ms.unwrap_or(16),
        );
    let pulse = crate::screenshot_native::win32_desktop_update_pulse::run_desktop_update_pulse(
        pulse_request,
    );
    let pulse_ok = pulse.ok;
    let pulse_json = desktop_update_pulse_report_json(pulse);
    let after = if pulse_ok {
        run_native_dxgi_frame_info_probe(NativeDxgiFrameInfoProbeRequest {
            x: request.x,
            y: request.y,
            width: request.width,
            height: request.height,
            explicit_opt_in: Some(true),
            allow_real_dxgi_api: Some(true),
        })?
    } else {
        serde_json::Value::Null
    };
    let before_default = before["comparison"]["defaultFrameConfirmed"]
        .as_bool()
        .unwrap_or(false);
    let before_selected = before["comparison"]["selectedFrameConfirmed"]
        .as_bool()
        .unwrap_or(false);
    let after_default = after["comparison"]["defaultFrameConfirmed"]
        .as_bool()
        .unwrap_or(false);
    let after_selected = after["comparison"]["selectedFrameConfirmed"]
        .as_bool()
        .unwrap_or(false);
    let ok = after_default || after_selected;
    Ok(serde_json::json!({
        "attempted": true,
        "ok": ok,
        "stage": if ok { "pulse-unblocked-frame" } else { "pulse-did-not-unblock-frame" },
        "requestedBounds": screenshot_physical_bounds_json(bounds),
        "before": before,
        "pulse": pulse_json,
        "after": after,
        "comparison": {
            "beforeBothTimedOut": before["comparison"]["bothTimedOut"].as_bool().unwrap_or(false),
            "afterAnyFrameConfirmed": after_default || after_selected,
            "pulseUnblockedDefault": !before_default && after_default,
            "pulseUnblockedSelected": !before_selected && after_selected,
        },
        "pulseSizePx": request.pulse_size_px.unwrap_or(2),
        "pulseAlpha": request.pulse_alpha.unwrap_or(1),
        "dwellMs": request.dwell_ms.unwrap_or(16),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealDesktopPulse": request.allow_real_desktop_pulse.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": true,
        "frameCaptureAttempted": true,
        "frameCaptureConfirmed": after_default && after_selected,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "scope": "diagnostic-only; creates a tiny non-activating layered desktop update pulse between DXGI frame-info probes; no clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_dxgi_pulse_before_acquire_probe(
    request: NativeDxgiPulseBeforeAcquireProbeRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_pulse_before_acquire_disabled_response(
            &request,
            "DXGI pulse-before-acquire probe requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_pulse_before_acquire_disabled_response(
            &request,
            "DXGI pulse-before-acquire probe real DXGI API calls are not allowed",
        ));
    }
    if !request.allow_real_desktop_pulse.unwrap_or(false) {
        return Ok(dxgi_pulse_before_acquire_disabled_response(
            &request,
            "DXGI pulse-before-acquire probe real desktop pulse is not allowed",
        ));
    }
    let bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    if bounds.is_empty() {
        return Ok(dxgi_pulse_before_acquire_disabled_response(
            &request,
            "DXGI pulse-before-acquire probe requires non-empty bounds",
        ));
    }
    let report = crate::screenshot_native::dxgi_pulse_before_acquire_probe::run_dxgi_pulse_before_acquire_probe(
        bounds,
        request.pulse_size_px.unwrap_or(2),
        request.pulse_alpha.unwrap_or(1),
        request.dwell_ms.unwrap_or(16),
    );
    let comparison = dxgi_pulse_before_acquire_report_json(&report)["comparison"].clone();
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": if report.ok { "pulse-before-acquire-frame-confirmed" } else { "pulse-before-acquire-did-not-confirm-frame" },
        "requestedBounds": screenshot_physical_bounds_json(bounds),
        "defaultOutput": dxgi_pulse_before_acquire_path_json(&report.default_output),
        "selectedOutput": dxgi_pulse_before_acquire_path_json(&report.selected_output),
        "comparison": comparison,
        "pulseSizePx": report.pulse_size_px,
        "pulseAlpha": report.pulse_alpha,
        "dwellMs": report.dwell_ms,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealDesktopPulse": request.allow_real_desktop_pulse.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": report.attempted,
        "frameCaptureAttempted": report.attempted,
        "frameCaptureConfirmed": report.ok,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": report.error,
        "scope": "diagnostic-only; opens DXGI duplication, creates a tiny non-activating desktop pulse, then immediately calls AcquireNextFrame in the same session without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_dxgi_frame_info_probe(
    request: NativeDxgiFrameInfoProbeRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_frame_info_probe_disabled_response(
            &request,
            "DXGI frame-info probe requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_frame_info_probe_disabled_response(
            &request,
            "DXGI frame-info probe real API calls are not allowed",
        ));
    }
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    if requested_bounds.is_empty() {
        return Ok(dxgi_frame_info_probe_disabled_response(
            &request,
            "DXGI frame-info probe requires non-empty bounds",
        ));
    }

    let report = crate::screenshot_native::dxgi_frame_info_probe::run_dxgi_frame_info_probe(
        requested_bounds,
    );
    let default_confirmed = report
        .default_output
        .attempts
        .iter()
        .any(|attempt| attempt.ok);
    let selected_confirmed = report
        .selected_output
        .attempts
        .iter()
        .any(|attempt| attempt.ok);
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": if report.ok { "frame-info-acquired" } else { "frame-info-timeout-or-failure" },
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "defaultOutput": dxgi_frame_info_probe_path_json(&report.default_output),
        "selectedOutput": dxgi_frame_info_probe_path_json(&report.selected_output),
        "comparison": {
            "defaultFrameConfirmed": default_confirmed,
            "selectedFrameConfirmed": selected_confirmed,
            "bothTimedOut": report.default_output.attempts.iter().any(|attempt| attempt.timed_out)
                && report.selected_output.attempts.iter().any(|attempt| attempt.timed_out),
            "defaultAttemptCount": report.default_output.attempts.len(),
            "selectedAttemptCount": report.selected_output.attempts.len(),
        },
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": report.attempted,
        "frameCaptureAttempted": report.attempted,
        "frameCaptureConfirmed": default_confirmed && selected_confirmed,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": report.error,
        "scope": "diagnostic-only; probes DXGI AcquireNextFrame attempts and frame-info metadata without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_dxgi_default_vs_selected_acquire_comparison_smoke(
    request: NativeDxgiDefaultVsSelectedAcquireComparisonRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_acquire_comparison_disabled_response(
            &request,
            "DXGI default-vs-selected acquire comparison requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_acquire_comparison_disabled_response(
            &request,
            "DXGI default-vs-selected acquire comparison real API calls are not allowed",
        ));
    }

    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    if requested_bounds.is_empty() {
        return Ok(dxgi_acquire_comparison_disabled_response(
            &request,
            "DXGI default-vs-selected acquire comparison requires non-empty bounds",
        ));
    }

    let default_report =
        crate::screenshot_native::dxgi_smoke::run_dxgi_texture_acquisition_smoke(requested_bounds);
    let selected_report =
        crate::screenshot_native::dxgi_output_bridge_smoke::run_dxgi_selected_output_bridge_dry_run(
            requested_bounds,
        );
    let default_frame_confirmed = default_report.frame_id.is_some();
    let selected_frame_confirmed = selected_report.frame_id.is_some();
    let default_error = default_report.error.clone().unwrap_or_default();
    let selected_error = selected_report.error.clone().unwrap_or_default();
    let default_timed_out = default_error.contains("Frame timed out")
        || default_error.contains("frame timed out")
        || default_error.contains("WAIT_TIMEOUT")
        || default_error.contains("0x887A0027");
    let selected_timed_out = selected_error.contains("Frame timed out")
        || selected_error.contains("frame timed out")
        || selected_error.contains("WAIT_TIMEOUT")
        || selected_error.contains("0x887A0027");
    let ok = default_frame_confirmed && selected_frame_confirmed;

    Ok(serde_json::json!({
        "attempted": true,
        "ok": ok,
        "stage": if ok { "compared" } else { "compared-with-failure" },
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "defaultOutput": dxgi_acquire_path_json(
            "default-output",
            default_report.attempted,
            default_report.ok,
            default_report.stage.as_str(),
            default_report.elapsed_ms,
            default_report.frame_id,
            default_report.width,
            default_report.height,
            default_report.format.map(|format| format!("{format:?}")),
            None,
            None,
            None,
            default_report.released_frame,
            default_report.stopped,
            default_report.error,
        ),
        "selectedOutput": dxgi_acquire_path_json(
            "selected-output",
            selected_report.attempted,
            selected_report.ok,
            selected_report.stage.as_str(),
            selected_report.elapsed_ms,
            selected_report.frame_id,
            selected_report.width,
            selected_report.height,
            selected_report.format,
            selected_report.output_bounds,
            selected_report.adapter_index,
            selected_report.output_index,
            selected_report.released_frame,
            selected_report.stopped,
            selected_report.error,
        ),
        "comparison": {
            "defaultFrameConfirmed": default_frame_confirmed,
            "selectedFrameConfirmed": selected_frame_confirmed,
            "bothTimedOut": default_timed_out && selected_timed_out,
            "defaultOnlySucceeded": default_frame_confirmed && !selected_frame_confirmed,
            "selectedOnlySucceeded": selected_frame_confirmed && !default_frame_confirmed,
            "sameFailureClass": default_timed_out == selected_timed_out,
        },
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": default_report.attempted || selected_report.attempted,
        "frameCaptureAttempted": default_report.attempted || selected_report.attempted,
        "frameCaptureConfirmed": default_frame_confirmed && selected_frame_confirmed,
        "error": if ok { serde_json::Value::Null } else { serde_json::json!("one or more DXGI acquire comparison paths failed") },
        "scope": "diagnostic-only; compares default DXGI output and selected DXGI output acquire without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_dxgi_selected_readback_smoke(
    request: NativeDxgiSelectedReadbackSmokeRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_selected_readback_disabled_response(
            &request,
            "DXGI selected readback requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_selected_readback_disabled_response(
            &request,
            "DXGI selected readback real API calls are not allowed",
        ));
    }
    let report = crate::screenshot_native::dxgi_smoke::run_dxgi_selected_readback_smoke(
        crate::screenshot_native::MonitorCaptureBounds::new(
            request.x,
            request.y,
            request.width,
            request.height,
        ),
    );
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": report.stage.as_str(),
        "elapsedMs": report.elapsed_ms,
        "frameId": report.frame_id,
        "x": request.x,
        "y": request.y,
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "outputBounds": report.output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": report.adapter_index,
        "outputIndex": report.output_index,
        "crop": report.crop.map(crop_rect_json),
        "selectedReadbackPlan": report
            .selected_readback_plan
            .as_ref()
            .map(|plan| selected_readback_plan_json(plan, "ranked-dxgi-output-desktop-coordinates"))
            .unwrap_or_else(|| {
                selected_readback_plan_error_json(
                    crate::screenshot_native::SelectedReadbackPlanBackend::DxgiOutput,
                    report.requested_bounds,
                    "ranked-dxgi-output-desktop-coordinates",
                    "dxgi-selected-readback-plan-unavailable",
                    report
                        .error
                        .clone()
                        .unwrap_or_else(|| "DXGI selected readback plan unavailable".to_string()),
                )
            }),
        "selectedOutputReadyPlanningOnly": report.selected_output_ready_planning_only,
        "width": report.width,
        "height": report.height,
        "format": report.format.map(debug_value),
        "selectedOnly": report.selected_only,
        "boundedCropValid": report.bounded_crop_valid,
        "copySubresourceRegion": report.copy_subresource_region,
        "bgraToRgba": report.bgra_to_rgba,
        "pngSignatureValid": report.png_signature_valid,
        "releasedFrame": report.released_frame,
        "stopped": report.stopped,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": report.attempted,
        "frameCaptureAttempted": report.attempted,
        "frameCaptureConfirmed": report.frame_id.is_some(),
        "error": report.error,
        "scope": "diagnostic-only; validates DXGI selected-region readback without marking presenter, native overlay, Alt+A, or C/E readiness complete"
    }))
}

#[tauri::command]
pub fn run_native_dxgi_selected_output_bridge_dry_run(
    request: NativeDxgiSelectedOutputBridgeDryRunRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_selected_output_bridge_disabled_response(
            &request,
            "DXGI selected output bridge dry-run requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_selected_output_bridge_disabled_response(
            &request,
            "DXGI selected output bridge dry-run real API calls are not allowed",
        ));
    }
    let report =
        crate::screenshot_native::dxgi_output_bridge_smoke::run_dxgi_selected_output_bridge_dry_run(
            crate::screenshot_native::MonitorCaptureBounds::new(
                request.x,
                request.y,
                request.width,
                request.height,
            ),
        );
    let selected_png_evidence = selected_png_evidence_json(report.selected_image.as_ref());
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": report.stage.as_str(),
        "elapsedMs": report.elapsed_ms,
        "frameId": report.frame_id,
        "x": request.x,
        "y": request.y,
        "width": report.width,
        "height": report.height,
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "outputBounds": report.output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": report.adapter_index,
        "outputIndex": report.output_index,
        "outputRanking": dxgi_output_ranking_json(report.output_ranking.as_ref()),
        "desktopPulse": report.desktop_pulse.clone().map(desktop_update_pulse_report_json),
        "crop": report.crop.map(crop_rect_json),
        "selectedReadbackPlan": report
            .selected_readback_plan
            .as_ref()
            .map(|plan| selected_readback_plan_json(plan, "ranked-dxgi-output-bridge-desktop-coordinates"))
            .unwrap_or_else(|| {
                selected_readback_plan_error_json(
                    crate::screenshot_native::SelectedReadbackPlanBackend::DxgiOutput,
                    report.requested_bounds,
                    "ranked-dxgi-output-bridge-desktop-coordinates",
                    "dxgi-selected-output-bridge-plan-unavailable",
                    report
                        .error
                        .clone()
                        .unwrap_or_else(|| "DXGI selected output bridge plan unavailable".to_string()),
                )
            }),
        "selectedOutputReadyPlanningOnly": report.selected_output_ready_planning_only,
        "format": report.format,
        "selectedOnly": report.selected_only,
        "pngSignatureValid": report.png_signature_valid,
        "selectedPngEvidence": selected_png_evidence,
        "releasedFrame": report.released_frame,
        "stopped": report.stopped,
        "bridge": report.bridge,
        "actions": report.action_diagnostics,
        "bridgeValidated": report.bridge_validated,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": report.attempted,
        "frameCaptureAttempted": report.attempted,
        "frameCaptureConfirmed": report.frame_id.is_some(),
        "error": report.error,
        "scope": "diagnostic-only; validates DXGI selected PNG evidence against copy/save/OCR/translate bridge contracts without clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}
#[derive(Debug, Default)]
struct DiagnosticFakeClipboardSink {
    calls: usize,
    last_png_len: usize,
}

impl crate::screenshot_native::selected_output_effects::SelectedOutputEffectSink
    for DiagnosticFakeClipboardSink
{
    fn copy_png_to_clipboard(
        &mut self,
        png_bytes: &[u8],
    ) -> Result<(), crate::screenshot_native::selected_output_effects::SelectedOutputEffectError>
    {
        self.calls += 1;
        self.last_png_len = png_bytes.len();
        Ok(())
    }
}

fn dxgi_selected_output_acceptance_env_guard_present() -> bool {
    std::env::var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE")
        .ok()
        .as_deref()
        == Some("1")
}

fn dxgi_selected_output_acceptance_disabled_response(
    request: &NativeDxgiSelectedOutputClipboardAcceptanceRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    let command_guard_present = request.explicit_opt_in.unwrap_or(false)
        && request.allow_real_dxgi_api.unwrap_or(false)
        && (request.allow_fake_clipboard_sink.unwrap_or(false)
            ^ request.allow_real_clipboard.unwrap_or(false));
    let env_guard_present = dxgi_selected_output_acceptance_env_guard_present();
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowFakeClipboardSink": request.allow_fake_clipboard_sink.unwrap_or(false),
        "allowRealClipboard": request.allow_real_clipboard.unwrap_or(false),
        "guarded": true,
        "commandGuardPresent": command_guard_present,
        "envGuardPresent": env_guard_present,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "selectedOutputEffectConfirmed": false,
        "clipboardReadbackAttempted": false,
        "clipboardReadbackConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "sink": serde_json::Value::Null,
        "receipt": serde_json::Value::Null,
        "error": error,
        "scope": "diagnostic-only; DXGI selected-output clipboard acceptance is disabled unless explicitOptIn, allowRealDxgiApi, exactly one clipboard sink mode, and YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1 are present; real clipboard remains separately opt-in; no save, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

#[tauri::command]
pub fn run_native_dxgi_selected_output_clipboard_acceptance_smoke(
    request: NativeDxgiSelectedOutputClipboardAcceptanceRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_selected_output_acceptance_disabled_response(
            &request,
            "DXGI selected output clipboard acceptance requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_selected_output_acceptance_disabled_response(
            &request,
            "DXGI selected output clipboard acceptance real API calls are not allowed",
        ));
    }
    let allow_fake_clipboard_sink = request.allow_fake_clipboard_sink.unwrap_or(false);
    let allow_real_clipboard = request.allow_real_clipboard.unwrap_or(false);
    if !allow_fake_clipboard_sink && !allow_real_clipboard {
        return Ok(dxgi_selected_output_acceptance_disabled_response(
            &request,
            "DXGI selected output clipboard acceptance requires fake sink or real clipboard opt-in",
        ));
    }
    if allow_fake_clipboard_sink && allow_real_clipboard {
        return Ok(dxgi_selected_output_acceptance_disabled_response(
            &request,
            "DXGI selected output clipboard acceptance requires exactly one clipboard sink mode",
        ));
    }
    if !dxgi_selected_output_acceptance_env_guard_present() {
        return Ok(dxgi_selected_output_acceptance_disabled_response(
            &request,
            "DXGI selected output clipboard acceptance requires YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1",
        ));
    }

    let report =
        crate::screenshot_native::dxgi_output_bridge_smoke::run_dxgi_selected_output_bridge_dry_run(
            crate::screenshot_native::MonitorCaptureBounds::new(
                request.x,
                request.y,
                request.width,
                request.height,
            ),
        );
    let selected_png_evidence = selected_png_evidence_json(report.selected_image.as_ref());
    let selected_image_contract = report.selected_image.clone();
    let selected_image = selected_image_contract.ok_or_else(|| {
        report.error.clone().unwrap_or_else(|| {
            "DXGI selected output bridge did not produce selected PNG evidence".to_string()
        })
    });
    let sink_mode = if allow_real_clipboard { "real" } else { "fake" };
    let mut fake_sink = DiagnosticFakeClipboardSink::default();
    let mut real_sink =
        crate::screenshot_native::selected_output_clipboard::VerifyingArboardSelectedOutputEffectSink::new();
    let acceptance = selected_image.and_then(|image| {
        if allow_real_clipboard {
            crate::screenshot_native::dxgi_selected_output_acceptance::accept_dxgi_selected_output_clipboard_with_sink(
                image,
                true,
                &mut real_sink,
            )
            .map_err(|error| error.to_string())
        } else {
            crate::screenshot_native::dxgi_selected_output_acceptance::accept_dxgi_selected_output_clipboard_with_sink(
                image,
                true,
                &mut fake_sink,
            )
            .map_err(|error| error.to_string())
        }
    });
    let clipboard_verification = real_sink.verification().cloned();
    let clipboard_readback_attempted = real_sink.readback_attempted();
    let acceptance_error = acceptance.as_ref().err().cloned();
    let acceptance = acceptance.ok();
    let copy_effect_confirmed = acceptance
        .as_ref()
        .map(|receipt| receipt.effect.is_copy_only())
        .unwrap_or(false);
    let selected_output_effect_confirmed = report.ok
        && report.bridge_validated
        && report.selected_only
        && report.png_signature_valid
        && copy_effect_confirmed
        && (!allow_fake_clipboard_sink || fake_sink.calls == 1);
    let sink_json = serde_json::json!({
        "mode": sink_mode,
        "calls": if allow_fake_clipboard_sink { serde_json::json!(fake_sink.calls) } else { serde_json::Value::Null },
        "lastPngLen": if allow_fake_clipboard_sink { serde_json::json!(fake_sink.last_png_len) } else { serde_json::Value::Null },
        "clipboardVerification": clipboard_verification.as_ref().map(|verification| serde_json::json!({
            "expectedWidth": verification.expected_width,
            "expectedHeight": verification.expected_height,
            "actualWidth": verification.actual_width,
            "actualHeight": verification.actual_height,
            "expectedByteLen": verification.expected_byte_len,
            "actualByteLen": verification.actual_byte_len,
            "expectedRgbaFingerprint": verification.expected_rgba_fingerprint,
            "actualRgbaFingerprint": verification.actual_rgba_fingerprint,
            "dimensionsMatch": verification.dimensions_match,
            "bytesMatch": verification.bytes_match,
            "confirmed": verification.confirmed(),
        })),
    });
    let receipt_json = acceptance.as_ref().map(|receipt| {
        serde_json::json!({
            "action": output_action_label(receipt.effect.action),
            "target": output_bridge_target_label(receipt.effect.target),
            "format": output_image_format_label(receipt.effect.format),
            "selectedOnlyPng": receipt.effect.selected_only_png,
            "pngByteLen": receipt.effect.png_byte_len,
            "copiedToClipboard": receipt.effect.copied_to_clipboard,
            "saveInvoked": receipt.effect.save_invoked,
            "ocrInvoked": receipt.effect.ocr_invoked,
            "translationInvoked": receipt.effect.translation_invoked,
            "diagnosticOnly": receipt.diagnostic_only,
            "readinessChanged": receipt.readiness_changed,
            "persistentHandleExposed": receipt.persistent_handle_exposed,
            "sink": receipt.sink,
        })
    });

    Ok(serde_json::json!({
        "attempted": true,
        "ok": selected_output_effect_confirmed,
        "stage": report.stage.as_str(),
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "outputBounds": report.output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": report.adapter_index,
        "outputIndex": report.output_index,
        "outputRanking": dxgi_output_ranking_json(report.output_ranking.as_ref()),
        "desktopPulse": report.desktop_pulse.clone().map(desktop_update_pulse_report_json),
        "crop": report.crop.map(crop_rect_json),
        "bridgeValidated": report.bridge_validated,
        "selectedOnly": report.selected_only,
        "pngSignatureValid": report.png_signature_valid,
        "selectedPngEvidence": selected_png_evidence,
        "releasedFrame": report.released_frame,
        "stopped": report.stopped,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowFakeClipboardSink": request.allow_fake_clipboard_sink.unwrap_or(false),
        "allowRealClipboard": request.allow_real_clipboard.unwrap_or(false),
        "guarded": true,
        "commandGuardPresent": true,
        "envGuardPresent": true,
        "attemptedRealDxgiApi": report.attempted,
        "frameCaptureAttempted": report.attempted,
        "frameCaptureConfirmed": report.frame_id.is_some(),
        "selectedOutputEffectConfirmed": selected_output_effect_confirmed,
        "clipboardVerificationConfirmed": clipboard_verification
            .as_ref()
            .map(|verification| verification.confirmed())
            .unwrap_or(false),
        "clipboardReadbackAttempted": clipboard_readback_attempted,
        "clipboardReadbackConfirmed": clipboard_verification
            .as_ref()
            .map(|verification| verification.confirmed())
            .unwrap_or(false),
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "sink": sink_json,
        "receipt": receipt_json,
        "bridgeError": report.error,
        "acceptanceError": acceptance_error,
        "scope": "diagnostic-only; guarded DXGI selected-output clipboard acceptance; real clipboard requires allowRealClipboard and live selected PNG evidence; no file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

fn dxgi_cursor_nudge_disabled_response(
    request: &NativeDxgiCursorNudgeDiagnosticRequest,
    error: &str,
) -> serde_json::Value {
    let requested_bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": screenshot_physical_bounds_json(requested_bounds),
        "before": serde_json::Value::Null,
        "cursor": serde_json::Value::Null,
        "after": serde_json::Value::Null,
        "comparison": {
            "beforeBothTimedOut": false,
            "afterAnyFrameConfirmed": false,
            "nudgeUnblockedDefault": false,
            "nudgeUnblockedSelected": false,
        },
        "dx": request.dx.unwrap_or(1),
        "dy": request.dy.unwrap_or(0),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealCursorNudge": request.allow_real_cursor_nudge.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "error": error,
        "scope": "diagnostic-only; compares DXGI acquire before and after an explicitly allowed cursor nudge; restores cursor; no clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    })
}

#[tauri::command]
pub fn run_native_dxgi_cursor_nudge_diagnostic_smoke(
    request: NativeDxgiCursorNudgeDiagnosticRequest,
) -> Result<serde_json::Value, String> {
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(dxgi_cursor_nudge_disabled_response(
            &request,
            "DXGI cursor-nudge diagnostic requires explicit opt-in",
        ));
    }
    if !request.allow_real_dxgi_api.unwrap_or(false) {
        return Ok(dxgi_cursor_nudge_disabled_response(
            &request,
            "DXGI cursor-nudge diagnostic real DXGI API calls are not allowed",
        ));
    }
    if !request.allow_real_cursor_nudge.unwrap_or(false) {
        return Ok(dxgi_cursor_nudge_disabled_response(
            &request,
            "DXGI cursor-nudge diagnostic real cursor movement is not allowed",
        ));
    }
    let bounds = crate::screenshot_native::MonitorCaptureBounds::new(
        request.x,
        request.y,
        request.width,
        request.height,
    );
    if bounds.is_empty() {
        return Ok(dxgi_cursor_nudge_disabled_response(
            &request,
            "DXGI cursor-nudge diagnostic requires non-empty bounds",
        ));
    }

    let comparison_request = |explicit_opt_in, allow_real_dxgi_api| {
        NativeDxgiDefaultVsSelectedAcquireComparisonRequest {
            x: request.x,
            y: request.y,
            width: request.width,
            height: request.height,
            explicit_opt_in: Some(explicit_opt_in),
            allow_real_dxgi_api: Some(allow_real_dxgi_api),
        }
    };
    let before = run_native_dxgi_default_vs_selected_acquire_comparison_smoke(comparison_request(
        true, true,
    ))?;
    let cursor = crate::screenshot_native::win32_cursor::nudge_cursor_temporarily(
        request.dx.unwrap_or(1),
        request.dy.unwrap_or(0),
    );
    let cursor_ok = cursor.ok;
    let cursor_json = cursor_nudge_report_json(cursor);
    let after = if cursor_ok {
        run_native_dxgi_default_vs_selected_acquire_comparison_smoke(comparison_request(
            true, true,
        ))?
    } else {
        serde_json::Value::Null
    };

    let before_default = comparison_frame_confirmed(&before, "defaultOutput");
    let before_selected = comparison_frame_confirmed(&before, "selectedOutput");
    let after_default = comparison_frame_confirmed(&after, "defaultOutput");
    let after_selected = comparison_frame_confirmed(&after, "selectedOutput");
    let ok = after_default || after_selected;

    Ok(serde_json::json!({
        "attempted": true,
        "ok": ok,
        "stage": if ok { "nudge-unblocked-frame" } else { "nudge-did-not-unblock-frame" },
        "requestedBounds": screenshot_physical_bounds_json(bounds),
        "before": before,
        "cursor": cursor_json,
        "after": after,
        "comparison": {
            "beforeBothTimedOut": comparison_both_timed_out(&before),
            "afterAnyFrameConfirmed": after_default || after_selected,
            "nudgeUnblockedDefault": !before_default && after_default,
            "nudgeUnblockedSelected": !before_selected && after_selected,
        },
        "dx": request.dx.unwrap_or(1),
        "dy": request.dy.unwrap_or(0),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealDxgiApi": request.allow_real_dxgi_api.unwrap_or(false),
        "allowRealCursorNudge": request.allow_real_cursor_nudge.unwrap_or(false),
        "guarded": true,
        "attemptedRealDxgiApi": true,
        "frameCaptureAttempted": true,
        "frameCaptureConfirmed": after_default || after_selected,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "fileWritten": false,
        "scope": "diagnostic-only; compares DXGI acquire before and after an explicitly allowed cursor nudge; restores cursor; no clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_cursor_nudge_smoke(
    request: NativeCursorNudgeSmokeRequest,
) -> Result<serde_json::Value, String> {
    let dx = request.dx.unwrap_or(1);
    let dy = request.dy.unwrap_or(0);
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(serde_json::json!({
            "attempted": false,
            "ok": false,
            "stage": "disabled",
            "dx": dx,
            "dy": dy,
            "nudge": serde_json::Value::Null,
            "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
            "allowRealCursorNudge": request.allow_real_cursor_nudge.unwrap_or(false),
            "guarded": true,
            "diagnosticOnly": true,
            "persistentHandleExposed": false,
            "readinessChanged": false,
            "altAChanged": false,
            "clipboardChanged": false,
            "error": "native cursor nudge smoke requires explicit opt-in",
            "scope": "diagnostic-only; may move the cursor by at most two pixels and restores it; does not click, type, use clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
        }));
    }
    if !request.allow_real_cursor_nudge.unwrap_or(false) {
        return Ok(serde_json::json!({
            "attempted": false,
            "ok": false,
            "stage": "disabled",
            "dx": dx,
            "dy": dy,
            "nudge": serde_json::Value::Null,
            "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
            "allowRealCursorNudge": request.allow_real_cursor_nudge.unwrap_or(false),
            "guarded": true,
            "diagnosticOnly": true,
            "persistentHandleExposed": false,
            "readinessChanged": false,
            "altAChanged": false,
            "clipboardChanged": false,
            "error": "native cursor nudge smoke real cursor movement is not allowed",
            "scope": "diagnostic-only; may move the cursor by at most two pixels and restores it; does not click, type, use clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
        }));
    }

    let report = crate::screenshot_native::win32_cursor::nudge_cursor_temporarily(dx, dy);
    Ok(serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "stage": if report.ok { "restored" } else if report.attempted { "attempted" } else { "blocked" },
        "dx": dx,
        "dy": dy,
        "nudge": cursor_nudge_report_json(report),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealCursorNudge": request.allow_real_cursor_nudge.unwrap_or(false),
        "guarded": true,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "clipboardChanged": false,
        "scope": "diagnostic-only; may move the cursor by at most two pixels and restores it; does not click, type, use clipboard, file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
    }))
}

#[tauri::command]
pub fn run_native_input_synthetic_smoke() -> Result<serde_json::Value, String> {
    let drag = crate::screenshot_native::native_input_smoke::run_synthetic_native_drag_smoke();
    let cancel = crate::screenshot_native::native_input_smoke::run_synthetic_native_cancel_smoke();
    Ok(serde_json::json!({
        "drag": {
            "status": drag.status.as_str(),
            "decodedEvents": drag.decoded_events,
            "transitions": drag.transitions,
            "completedRect": drag.completed_rect.map(|rect| serde_json::json!({
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
                "right": rect.right(),
                "bottom": rect.bottom()
            })),
            "finalState": debug_value(drag.final_state),
            "error": drag.error
        },
        "cancel": {
            "status": cancel.status.as_str(),
            "decodedEvents": cancel.decoded_events,
            "transitions": cancel.transitions,
            "finalState": debug_value(cancel.final_state),
            "error": cancel.error
        },
        "scope": "synthetic diagnostic only; does not prove real Win32 message pump, Alt+A native input, or C/E readiness"
    }))
}

#[tauri::command]
pub fn run_native_overlay_planned_smoke() -> Result<serde_json::Value, String> {
    let report =
        crate::screenshot_native::native_overlay_smoke::planned_native_overlay_smoke_report();
    Ok(serde_json::json!({
        "status": debug_value(report.status),
        "ok": report.is_success(),
        "completedSteps": report.completed_steps,
        "failedStep": report.failed_step.map(debug_value),
        "recoveryAction": report.recovery_action.map(debug_value),
        "plan": {
            "steps": report.plan.steps.iter().map(debug_value).collect::<Vec<_>>(),
            "stepCount": report.plan.steps.len(),
            "hasRuntimeSideEffects": report.plan.has_runtime_side_effects,
            "lifecycleChecksRequired": report.plan.lifecycle_checks_required(),
            "requiresCandidateEdgeCaseCheck": report.plan.requires_candidate_edge_case_check,
            "requiresDpiBoundaryCheck": report.plan.requires_dpi_boundary_check
        },
        "scope": "planned diagnostic only; does not create a window, dispatch real input, run Alt+A, or mark C/E readiness complete"
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
pub fn get_latest_screenshot_shell_payload() -> Result<Option<serde_json::Value>, String> {
    get_latest_screenshot_shell_payload_store()
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
            let screens =
                Screen::all().map_err(|error| format!("Resolve toast display failed: {error}"))?;
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
    .title("Screenshot saved")
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

    if CAPTURING.swap(true, Ordering::SeqCst) {
        println!("[screenshot-trace] start_screenshot: CAPTURING was already true, canceling active screenshot");
        notify_screenshot_session_cancelled(&app, "repeat-hotkey-cancel");
        crate::screenshot_native::advance_run_generation();
        let _ = crate::screenshot_native::cancel_cpu_native_overlay_session("repeat-hotkey-cancel");
        unregister_capture_escape_shortcut(&app);
        close_screenshot_windows(&app, true);
        CAPTURING.store(false, Ordering::SeqCst);
        clear_latest_screenshot_payload();
        crate::window_lifecycle::restore_main_window_after_screenshot(&app, "repeat-hotkey-cancel");
        return Ok(());
    }
    let run_generation = crate::screenshot_native::begin_run_generation();
    println!("[screenshot-trace] start_screenshot: CAPTURING is now true");
    register_capture_escape_shortcut(&app);

    match start_screenshot_impl(app.clone(), mode, run_generation).await {
        Ok(()) => Ok(()),
        Err(e) => {
            CAPTURING.store(false, Ordering::SeqCst);
            unregister_capture_escape_shortcut(&app);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn force_close_screenshots(app: tauri::AppHandle) -> Result<(), String> {
    println!("[screenshot-trace] enter force_close_screenshots");
    crate::screenshot_native::advance_run_generation();
    let _ = crate::screenshot_native::cancel_cpu_native_overlay_session("force-close-screenshots");
    notify_screenshot_session_cancelled(&app, "force-close-screenshots");
    unregister_capture_escape_shortcut(&app);
    close_screenshot_windows(&app, true);
    CAPTURING.store(false, Ordering::SeqCst);
    clear_latest_screenshot_payload();
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
    let _ = crate::screenshot_native::cancel_cpu_native_overlay_session("cancel-screenshot");
    notify_screenshot_session_cancelled(&app, "cancel-screenshot");
    unregister_capture_escape_shortcut(&app);
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
    crate::screenshot_native::advance_run_generation();
    clear_latest_screenshot_payload();
    if should_restore_main {
        crate::window_lifecycle::restore_main_window_after_screenshot(&app, "cancel-screenshot");
    }
    Ok(())
}

#[tauri::command]
pub fn get_fullscreen_image() -> Result<String, String> {
    Ok(BASE64_STANDARD.encode(get_or_encode_screenshot_png()?))
}

#[tauri::command]
pub fn get_fullscreen_image_bytes() -> Result<tauri::ipc::Response, String> {
    Ok(tauri::ipc::Response::new(get_or_encode_screenshot_png()?))
}

#[tauri::command]
pub fn get_fullscreen_rgba_bytes(
    session_id: Option<String>,
) -> Result<tauri::ipc::Response, String> {
    let rgba = get_matching_screenshot_rgba(session_id.as_deref())?;
    Ok(tauri::ipc::Response::new(rgba.bytes.clone()))
}

#[tauri::command]
pub fn post_fullscreen_rgba_shared_buffer(
    webview: tauri::Webview,
    session_id: Option<String>,
) -> Result<crate::screenshot_shared_buffer::ScreenshotSharedBufferPostResult, String> {
    let started_at = Instant::now();
    let session_id = session_id.unwrap_or_else(|| "unknown".to_string());
    let rgba = get_matching_screenshot_rgba(Some(&session_id))?;
    let result = crate::screenshot_shared_buffer::post_rgba_frame_to_webview(
        webview,
        session_id.clone(),
        &rgba,
    );
    match &result {
        Ok(posted) if posted.posted => log_screenshot_baseline(
            &session_id,
            "shared_buffer_posted",
            &started_at,
            &format!(
                "bytes={} size={}x{} transfer_type={}",
                posted.bytes, posted.width, posted.height, posted.transfer_type
            ),
        ),
        Ok(posted) => log_screenshot_baseline(
            &session_id,
            "shared_buffer_unavailable",
            &started_at,
            posted.reason.as_deref().unwrap_or("unknown"),
        ),
        Err(error) => {
            log_screenshot_baseline(&session_id, "shared_buffer_failed", &started_at, error)
        }
    }
    result
}

#[tauri::command]
pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection region".to_string());
    }

    let screenshot_bytes = get_or_encode_screenshot_png()?;

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
        .set_file_name(default_image_save_file_name());
    if let Some(default_dir) = default_image_save_directory() {
        dialog = dialog.set_directory(default_dir);
    }
    let file_path = dialog.save_file().await;
    if let Some(file_handle) = file_path {
        let path = ensure_png_extension(file_handle.path().to_path_buf());
        fs::write(&path, &bytes).map_err(|e| format!("Write file failed: {}", e))?;
        if !path.exists() {
            return Err("No display detected".to_string());
        }
        remember_image_save_directory(&path);
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Save cancelled by user".to_string())
    }
}

#[tauri::command]
pub async fn choose_image_save_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let started_at = Instant::now();
    println!("[screenshot-baseline] session=save-as phase=dialog_open_start elapsed_ms=0");
    unregister_capture_escape_shortcut(&app);
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_file_name(default_image_save_file_name());
    if let Some(default_dir) = default_image_save_directory() {
        dialog = dialog.set_directory(default_dir);
    }
    let file_path = dialog.save_file().await;
    let result = file_path.map(|file_handle| {
        ensure_png_extension(file_handle.path().to_path_buf())
            .to_string_lossy()
            .to_string()
    });
    if result.is_some() {
        register_capture_escape_shortcut(&app);
    }
    println!(
        "[screenshot-baseline] session=save-as phase=dialog_open_end elapsed_ms={} cancelled={}",
        started_at.elapsed().as_millis(),
        result.is_none()
    );
    Ok(result)
}

#[tauri::command]
pub async fn choose_image_save_directory(
    initial_dir: Option<String>,
) -> Result<Option<String>, String> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("选择图片默认保存位置");
    if let Some(initial_dir) = usable_directory(initial_dir) {
        dialog = dialog.set_directory(initial_dir);
    } else if let Some(default_dir) = default_image_save_directory() {
        dialog = dialog.set_directory(default_dir);
    }
    Ok(dialog
        .pick_folder()
        .await
        .map(|folder| folder.path().to_string_lossy().to_string()))
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
    remember_image_save_directory(&path);
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

pub async fn run_screenshot_lifecycle_smoke(app: tauri::AppHandle) {
    println!("[screenshot-smoke] start lifecycle smoke");
    let first_app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = start_screenshot(first_app, None).await {
            eprintln!("[screenshot-smoke] first start failed: {error}");
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(90)).await;
    if let Err(error) = start_screenshot(app.clone(), None).await {
        eprintln!("[screenshot-smoke] repeat cancel failed: {error}");
    }
    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    let visible_after_cancel = app
        .get_webview_window("screenshot")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    println!(
        "[screenshot-smoke] after repeat cancel visible={} capturing={}",
        visible_after_cancel,
        CAPTURING.load(Ordering::SeqCst)
    );
    if let Err(error) = start_screenshot(app.clone(), None).await {
        eprintln!("[screenshot-smoke] second start failed: {error}");
    }
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    let visible_after_ready = app
        .get_webview_window("screenshot")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    println!(
        "[screenshot-smoke] after ready visible={} capturing={}",
        visible_after_ready,
        CAPTURING.load(Ordering::SeqCst)
    );
    let _ = cancel_screenshot(app.clone(), Some("screenshot".to_string()), Some(true)).await;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let visible_after_final_cancel = app
        .get_webview_window("screenshot")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    println!(
        "[screenshot-smoke] after final cancel visible={} capturing={}",
        visible_after_final_cancel,
        CAPTURING.load(Ordering::SeqCst)
    );
    app.exit(0);
}
#[tauri::command]
pub fn log_screenshot_perf(message: String) {
    println!("[screenshot-perf] {message}");
}

#[cfg(test)]
mod latest_screenshot_payload_wgc_monitor_diagnostic_tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn explicit_target_bounds_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> NativeWgcMonitorTargetDiagnosticRequest {
        NativeWgcMonitorTargetDiagnosticRequest {
            bounds: Some(NativeDxgiSelectedReadbackSmokeRequest {
                x,
                y,
                width,
                height,
                explicit_opt_in: None,
                allow_real_dxgi_api: None,
            }),
            validate: Some(false),
        }
    }

    fn explicit_session_bounds_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> NativeWgcMonitorSessionSmokeRequest {
        NativeWgcMonitorSessionSmokeRequest {
            bounds: Some(NativeDxgiSelectedReadbackSmokeRequest {
                x,
                y,
                width,
                height,
                explicit_opt_in: None,
                allow_real_dxgi_api: None,
            }),
            explicit_opt_in: Some(false),
            allow_real_wgc_api: Some(false),
            frame_timeout_ms: Some(0),
            include_cursor: Some(false),
            require_border: Some(false),
            buffer_count: Some(1),
            validate_target: Some(false),
        }
    }

    fn seed_latest_payload_bounds(session_id: &str, x: i32, y: i32, width: u32, height: u32) {
        set_latest_screenshot_payload(serde_json::json!({
            "kind": "rgba",
            "width": width,
            "height": height,
            "physicalBounds": {
                "x": x,
                "y": y,
                "width": width,
                "height": height
            },
            "sessionId": session_id
        }));
    }

    #[test]
    fn parse_latest_screenshot_physical_bounds_accepts_negative_origin() {
        let payload = serde_json::json!({
            "kind": "rgba",
            "width": 1920,
            "height": 1080,
            "physicalBounds": {
                "x": -1920,
                "y": 0,
                "width": 1920,
                "height": 1080
            },
            "sessionId": "negative-monitor"
        });

        let bounds = parse_latest_screenshot_physical_bounds(&payload).expect("physical bounds");

        assert_eq!(bounds.origin_x, -1920);
        assert_eq!(bounds.origin_y, 0);
        assert_eq!(bounds.width, 1920);
        assert_eq!(bounds.height, 1080);
    }

    #[test]
    fn parse_latest_screenshot_physical_bounds_rejects_missing_bounds() {
        let payload = serde_json::json!({
            "kind": "rgba",
            "width": 1920,
            "height": 1080,
            "sessionId": "missing-bounds"
        });

        let error = parse_latest_screenshot_physical_bounds(&payload)
            .expect_err("missing physical bounds rejected");

        assert!(error.contains("physicalBounds"));
    }

    #[test]
    fn native_wgc_monitor_diagnostic_uses_latest_payload_bounds() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        set_latest_screenshot_payload(serde_json::json!({
            "kind": "rgba",
            "width": 800,
            "height": 600,
            "physicalBounds": {
                "x": -800,
                "y": 12,
                "width": 800,
                "height": 600
            },
            "sessionId": "latest-bounds-test"
        }));

        let response = resolve_native_wgc_monitor_target_diagnostic(None).expect("diagnostic");

        assert_eq!(response["boundsSource"], "latestPayload");
        assert_eq!(response["latestPayload"]["latestPayloadPresent"], true);
        assert_eq!(response["latestPayload"]["sessionId"], "latest-bounds-test");
        assert_eq!(response["latestPayload"]["captureWidth"], 800);
        assert_eq!(response["latestPayload"]["captureHeight"], 600);
        assert_eq!(response["bounds"]["x"], -800);
        assert_eq!(response["bounds"]["y"], 12);
        assert_eq!(response["bounds"]["width"], 800);
        assert_eq!(response["bounds"]["height"], 600);

        clear_latest_screenshot_payload();
    }

    #[test]
    fn native_wgc_monitor_diagnostic_rejects_empty_request_bounds() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response = resolve_native_wgc_monitor_target_diagnostic(Some(
            explicit_target_bounds_request(0, 0, 0, 100),
        ))
        .expect("diagnostic");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], false);
        assert_eq!(response["boundsSource"], "request");
        assert!(response["error"].as_str().unwrap().contains("non-empty"));
    }

    #[test]
    fn native_wgc_monitor_diagnostic_sanitizes_native_handles() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response = resolve_native_wgc_monitor_target_diagnostic(Some(
            explicit_target_bounds_request(0, 0, 1, 1),
        ))
        .expect("diagnostic");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(serialized.contains("hasTargetHandle"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
    }

    #[test]
    fn native_wgc_monitor_session_smoke_defaults_to_no_real_api_attempt() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response =
            run_native_wgc_monitor_session_smoke(Some(explicit_session_bounds_request(0, 0, 1, 1)))
                .expect("session smoke");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["session"]["state"], "disabled");
        assert_eq!(response["session"]["acquiredFrame"], false);
        assert_eq!(response["session"]["diagnosticOnly"], true);
        assert_eq!(response["session"]["persistentHandleExposed"], false);
        assert_eq!(response["session"]["readinessChanged"], false);
        assert_eq!(response["session"]["altAChanged"], false);
        assert_eq!(response["session"]["frameDimensionsMatchSession"], false);
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["diagnosticOnly"],
            true
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["framepoolSizeSource"],
            "target-monitor-bounds"
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["frameMatchesTargetMonitorBounds"],
            false
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["selectedPngProduced"],
            false
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["persistentHandleExposed"],
            false
        );
        assert_eq!(
            response["session"]["selectedFrameEvidence"],
            serde_json::Value::Null
        );
        assert_eq!(
            response["session"]["selectedPngEvidence"],
            serde_json::Value::Null
        );
        assert_eq!(response["session"]["selectedPngProduced"], false);
        assert!(response["session"]["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(serialized.contains("hasTargetHandle"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
    }

    #[test]
    fn native_wgc_monitor_session_smoke_uses_resolved_monitor_bounds_without_validation() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response =
            run_native_wgc_monitor_session_smoke(Some(explicit_session_bounds_request(0, 0, 1, 1)))
                .expect("session smoke");
        let plan = &response["selectedReadbackPlan"];
        let serialized = serde_json::to_string(&plan).expect("plan json");

        assert_eq!(plan["diagnosticOnly"], true);
        assert_eq!(plan["readinessChanged"], false);
        assert_eq!(plan["backend"], "wgc-monitor");
        assert_eq!(plan["requestedBoundsPhysical"]["width"], 1);
        assert_eq!(plan["mapping"]["status"], "planned");
        assert_eq!(plan["targetBoundsPhysical"]["known"], true);
        assert_eq!(
            plan["targetBoundsPhysical"]["source"],
            "resolved-target-monitor-bounds"
        );
        assert!(
            plan["targetBoundsPhysical"]["bounds"]["width"]
                .as_u64()
                .expect("target width")
                >= 1
        );
        assert_eq!(
            response["sessionBounds"],
            plan["targetBoundsPhysical"]["bounds"]
        );
        assert_eq!(plan["framepool"]["source"], "target-monitor-bounds");
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
    }

    #[test]
    fn selected_readback_plan_json_reports_full_monitor_crop_mismatches() {
        let requested = crate::screenshot_native::MonitorCaptureBounds::new(300, 200, 400, 300);
        let target = crate::screenshot_native::MonitorCaptureBounds::new(0, 0, 1920, 1080);
        let plan = crate::screenshot_native::plan_selected_readback_from_desktop_bounds(
            crate::screenshot_native::SelectedReadbackPlanBackend::WgcMonitor,
            requested,
            target,
            image_bounds_from_monitor_bounds(target),
        )
        .expect("selected readback plan");

        let json = selected_readback_plan_json(&plan, "validated-target-monitor-bounds");

        assert_eq!(json["diagnosticOnly"], true);
        assert_eq!(json["readinessChanged"], false);
        assert_eq!(json["backend"], "wgc-monitor");
        assert_eq!(json["targetBoundsPhysical"]["known"], true);
        assert_eq!(
            json["targetBoundsPhysical"]["source"],
            "validated-target-monitor-bounds"
        );
        assert_eq!(json["mapping"]["status"], "planned");
        assert_eq!(json["mapping"]["monitorLocalSelection"]["x"], 300);
        assert_eq!(json["mapping"]["crop"]["width"], 400);
        assert_eq!(json["status"], "planned");
        assert_eq!(json["framepool"]["matchesRequestedBounds"], false);
        assert_eq!(json["framepool"]["matchesTargetBounds"], true);
        assert_eq!(json["mismatches"]["requestedDiffersFromTargetBounds"], true);
        assert_eq!(
            json["mismatches"]["framepoolDiffersFromTargetBounds"],
            false
        );
        assert_eq!(
            json["mismatches"]["selectedCropRequiresFullMonitorCapture"],
            true
        );
        assert_eq!(json["selectedOutputReadyPlanningOnly"], true);
    }

    #[test]
    fn selected_readback_plan_json_blocks_frame_size_mismatch() {
        let requested = crate::screenshot_native::MonitorCaptureBounds::new(300, 200, 400, 300);
        let target = crate::screenshot_native::MonitorCaptureBounds::new(0, 0, 1920, 1080);
        let plan = crate::screenshot_native::plan_selected_readback_from_desktop_bounds(
            crate::screenshot_native::SelectedReadbackPlanBackend::WgcMonitor,
            requested,
            target,
            crate::screenshot_native::ImageBounds::new(1280, 720),
        )
        .expect("selected readback plan");

        let json = selected_readback_plan_json(&plan, "validated-target-monitor-bounds");

        assert_eq!(json["framepool"]["matchesRequestedBounds"], false);
        assert_eq!(json["framepool"]["matchesTargetBounds"], true);
        assert_eq!(
            json["mismatches"]["framepoolDiffersFromTargetBounds"],
            false
        );
        assert_eq!(
            json["mismatches"]["selectedCropRequiresFullMonitorCapture"],
            true
        );
        assert_eq!(json["mismatches"]["frameDiffersFromTargetBounds"], true);
        assert_eq!(json["selectedOutputReadyPlanningOnly"], false);
    }

    #[test]
    fn native_wgc_monitor_session_smoke_rejects_missing_latest_payload() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response = run_native_wgc_monitor_session_smoke(None).expect("session smoke");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], false);
        assert_eq!(response["boundsSource"], "missing");
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["session"], serde_json::Value::Null);
        assert_eq!(response["selectedReadbackPlan"], serde_json::Value::Null);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("latest screenshot"));
    }

    #[test]
    fn native_wgc_monitor_session_smoke_rejects_invalid_request_bounds() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();

        let response = run_native_wgc_monitor_session_smoke(Some(explicit_session_bounds_request(
            i32::MAX,
            0,
            2,
            1,
        )))
        .expect("session smoke");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], false);
        assert_eq!(response["boundsSource"], "request");
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["session"], serde_json::Value::Null);
        assert_eq!(response["selectedReadbackPlan"], serde_json::Value::Null);
        assert!(response["error"].as_str().unwrap().contains("within i32"));
    }

    #[test]
    fn native_wgc_monitor_session_smoke_uses_latest_payload_bounds() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        seed_latest_payload_bounds("session-latest-bounds", -640, 20, 640, 480);

        let response =
            run_native_wgc_monitor_session_smoke(Some(NativeWgcMonitorSessionSmokeRequest {
                bounds: None,
                explicit_opt_in: Some(false),
                allow_real_wgc_api: Some(false),
                frame_timeout_ms: Some(0),
                include_cursor: Some(false),
                require_border: Some(false),
                buffer_count: Some(1),
                validate_target: Some(false),
            }))
            .expect("session smoke");

        assert_eq!(response["boundsSource"], "latestPayload");
        assert_eq!(
            response["latestPayload"]["sessionId"],
            "session-latest-bounds"
        );
        assert_eq!(response["bounds"]["x"], -640);
        assert_eq!(response["bounds"]["y"], 20);
        assert_eq!(response["bounds"]["width"], 640);
        assert_eq!(response["bounds"]["height"], 480);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);

        clear_latest_screenshot_payload();
    }

    #[test]
    fn native_wgc_monitor_session_smoke_prefers_request_bounds_over_latest_payload() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        seed_latest_payload_bounds("ignored-latest-bounds", -640, 20, 640, 480);

        let response = run_native_wgc_monitor_session_smoke(Some(explicit_session_bounds_request(
            10, 30, 320, 240,
        )))
        .expect("session smoke");

        assert_eq!(response["boundsSource"], "request");
        assert_eq!(
            response["latestPayload"]["sessionId"],
            "ignored-latest-bounds"
        );
        assert_eq!(response["bounds"]["x"], 10);
        assert_eq!(response["bounds"]["y"], 30);
        assert_eq!(response["bounds"]["width"], 320);
        assert_eq!(response["bounds"]["height"], 240);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);

        clear_latest_screenshot_payload();
    }

    #[test]
    fn native_wgc_monitor_session_smoke_blocks_real_api_without_allow_flag() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        clear_latest_screenshot_payload();
        let mut request = explicit_session_bounds_request(0, 0, 1, 1);
        request.explicit_opt_in = Some(true);
        request.allow_real_wgc_api = Some(false);
        request.frame_timeout_ms = Some(250);

        let response = run_native_wgc_monitor_session_smoke(Some(request)).expect("session smoke");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], true);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["session"]["state"], "disabled");
        assert!(response["session"]["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn native_wgc_monitor_session_smoke_rejects_invalid_latest_payload() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        set_latest_screenshot_payload(serde_json::json!({
            "kind": "rgba",
            "width": 640,
            "height": 480,
            "sessionId": "invalid-latest-bounds"
        }));

        let response = run_native_wgc_monitor_session_smoke(None).expect("session smoke");

        assert_eq!(response["ok"], false);
        assert_eq!(response["valid"], false);
        assert_eq!(response["boundsSource"], "latestPayload");
        assert_eq!(response["latestPayloadPresent"], true);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["session"], serde_json::Value::Null);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("physicalBounds"));

        clear_latest_screenshot_payload();
    }
}

#[cfg(test)]
mod native_dxgi_desktop_update_pulse_diagnostic_command_tests {
    use super::*;

    fn request(
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
        allow_real_desktop_pulse: Option<bool>,
    ) -> NativeDxgiDesktopUpdatePulseDiagnosticRequest {
        NativeDxgiDesktopUpdatePulseDiagnosticRequest {
            x: 0,
            y: 0,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
            allow_real_desktop_pulse,
            pulse_size_px: Some(2),
            pulse_alpha: Some(1),
            dwell_ms: Some(16),
        }
    }

    #[test]
    fn dxgi_desktop_update_pulse_default_denies_side_effects() {
        let response = run_native_dxgi_desktop_update_pulse_diagnostic_smoke(request(
            320, 180, None, None, None,
        ))
        .expect("desktop pulse response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["before"], serde_json::Value::Null);
        assert_eq!(response["pulse"], serde_json::Value::Null);
        assert_eq!(response["after"], serde_json::Value::Null);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_desktop_update_pulse_blocks_without_dxgi_allow() {
        let response = run_native_dxgi_desktop_update_pulse_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(false),
            Some(true),
        ))
        .expect("desktop pulse response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real DXGI API calls are not allowed"));
    }

    #[test]
    fn dxgi_desktop_update_pulse_blocks_without_pulse_allow() {
        let response = run_native_dxgi_desktop_update_pulse_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(false),
        ))
        .expect("desktop pulse response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real desktop pulse is not allowed"));
    }

    #[test]
    fn dxgi_desktop_update_pulse_rejects_invalid_bounds() {
        let response = run_native_dxgi_desktop_update_pulse_diagnostic_smoke(request(
            0,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("desktop pulse response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("non-empty bounds"));
    }

    #[test]
    #[ignore = "creates a tiny non-activating layered window and runs live DXGI probes"]
    fn dxgi_desktop_update_pulse_diagnostic_live_smoke() {
        let response = run_native_dxgi_desktop_update_pulse_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("desktop pulse response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert_eq!(response["pulse"]["destroyAttempted"], true);
        assert_eq!(response["pulse"]["destroyConfirmed"], true);
        assert_eq!(response["pulse"]["hiddenFromAltTab"], true);
        assert_eq!(response["pulse"]["noActivate"], true);
    }
}

#[cfg(test)]
mod native_dxgi_pulse_before_acquire_probe_command_tests {
    use super::*;

    fn request(
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
        allow_real_desktop_pulse: Option<bool>,
    ) -> NativeDxgiPulseBeforeAcquireProbeRequest {
        NativeDxgiPulseBeforeAcquireProbeRequest {
            x: 0,
            y: 0,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
            allow_real_desktop_pulse,
            pulse_size_px: Some(2),
            pulse_alpha: Some(1),
            dwell_ms: Some(16),
        }
    }

    #[test]
    fn dxgi_pulse_before_acquire_default_denies_side_effects() {
        let response =
            run_native_dxgi_pulse_before_acquire_probe(request(320, 180, None, None, None))
                .expect("pulse-before-acquire response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["defaultOutput"], serde_json::Value::Null);
        assert_eq!(response["selectedOutput"], serde_json::Value::Null);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_pulse_before_acquire_blocks_without_dxgi_allow() {
        let response = run_native_dxgi_pulse_before_acquire_probe(request(
            320,
            180,
            Some(true),
            Some(false),
            Some(true),
        ))
        .expect("pulse-before-acquire response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real DXGI API calls are not allowed"));
    }

    #[test]
    fn dxgi_pulse_before_acquire_blocks_without_pulse_allow() {
        let response = run_native_dxgi_pulse_before_acquire_probe(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(false),
        ))
        .expect("pulse-before-acquire response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real desktop pulse is not allowed"));
    }

    #[test]
    fn dxgi_pulse_before_acquire_rejects_invalid_bounds() {
        let response = run_native_dxgi_pulse_before_acquire_probe(request(
            0,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("pulse-before-acquire response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("non-empty bounds"));
    }

    #[test]
    #[ignore = "opens DXGI duplication, creates a tiny non-activating pulse, and acquires live"]
    fn dxgi_pulse_before_acquire_live_smoke() {
        let response = run_native_dxgi_pulse_before_acquire_probe(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("pulse-before-acquire response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert_eq!(response["defaultOutput"]["pulse"]["destroyAttempted"], true);
        assert_eq!(
            response["selectedOutput"]["pulse"]["destroyAttempted"],
            true
        );
    }
}

#[cfg(test)]
mod native_dxgi_frame_info_probe_command_tests {
    use super::*;

    fn request(
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
    ) -> NativeDxgiFrameInfoProbeRequest {
        NativeDxgiFrameInfoProbeRequest {
            x: 0,
            y: 0,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
        }
    }

    #[test]
    fn dxgi_frame_info_probe_default_denies_real_api() {
        let response = run_native_dxgi_frame_info_probe(request(320, 180, None, None))
            .expect("frame-info probe response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 180);
        assert_eq!(response["defaultOutput"], serde_json::Value::Null);
        assert_eq!(response["selectedOutput"], serde_json::Value::Null);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_output_ranking_json_is_handle_free() {
        let candidates = [
            crate::screenshot_native::dxgi_output::DxgiOutputCandidate::new(
                0,
                0,
                crate::screenshot_native::MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
            ),
            crate::screenshot_native::dxgi_output::DxgiOutputCandidate::new(
                0,
                1,
                crate::screenshot_native::MonitorCaptureBounds::new(0, 0, 1920, 1080),
            ),
        ];
        let evidence = crate::screenshot_native::dxgi_output::rank_dxgi_outputs_for_selection(
            crate::screenshot_native::MonitorCaptureBounds::new(-50, 100, 200, 120),
            &candidates,
        );
        let json = dxgi_output_ranking_json(Some(&evidence));
        let serialized = serde_json::to_string(&json).expect("ranking json");

        assert_eq!(
            json["rankingPolicy"],
            crate::screenshot_native::dxgi_output::DXGI_OUTPUT_RANKING_POLICY
        );
        assert_eq!(json["selectedRank"], 1);
        assert_eq!(json["rankedOutputs"].as_array().unwrap().len(), 2);
        assert_eq!(json["persistentHandleExposed"], false);
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_frame_info_probe_blocks_without_allow_flag() {
        let response = run_native_dxgi_frame_info_probe(request(320, 180, Some(true), Some(false)))
            .expect("frame-info probe response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn dxgi_frame_info_probe_rejects_invalid_bounds() {
        let response = run_native_dxgi_frame_info_probe(request(0, 180, Some(true), Some(true)))
            .expect("frame-info probe response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("non-empty bounds"));
    }

    #[test]
    #[ignore = "requires a real Windows desktop duplication session"]
    fn dxgi_frame_info_probe_live_smoke() {
        let response = run_native_dxgi_frame_info_probe(request(320, 180, Some(true), Some(true)))
            .expect("frame-info probe response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert!(
            response["defaultOutput"]["attempts"]
                .as_array()
                .unwrap()
                .len()
                <= 10
        );
        assert!(
            response["selectedOutput"]["attempts"]
                .as_array()
                .unwrap()
                .len()
                <= 10
        );
    }
}

#[cfg(test)]
mod native_dxgi_cursor_nudge_diagnostic_command_tests {
    use super::*;

    fn request(
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
        allow_real_cursor_nudge: Option<bool>,
    ) -> NativeDxgiCursorNudgeDiagnosticRequest {
        NativeDxgiCursorNudgeDiagnosticRequest {
            x: 0,
            y: 0,
            width,
            height,
            dx: Some(1),
            dy: Some(0),
            explicit_opt_in,
            allow_real_dxgi_api,
            allow_real_cursor_nudge,
        }
    }

    #[test]
    fn dxgi_cursor_nudge_diagnostic_default_denies_side_effects() {
        let response =
            run_native_dxgi_cursor_nudge_diagnostic_smoke(request(320, 180, None, None, None))
                .expect("dxgi cursor nudge response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 180);
        assert_eq!(response["before"], serde_json::Value::Null);
        assert_eq!(response["cursor"], serde_json::Value::Null);
        assert_eq!(response["after"], serde_json::Value::Null);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_cursor_nudge_diagnostic_blocks_without_dxgi_allow() {
        let response = run_native_dxgi_cursor_nudge_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(false),
            Some(true),
        ))
        .expect("dxgi cursor nudge response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real DXGI API calls are not allowed"));
    }

    #[test]
    fn dxgi_cursor_nudge_diagnostic_blocks_without_cursor_allow() {
        let response = run_native_dxgi_cursor_nudge_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(false),
        ))
        .expect("dxgi cursor nudge response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real cursor movement is not allowed"));
    }

    #[test]
    fn dxgi_cursor_nudge_diagnostic_rejects_invalid_bounds() {
        let response = run_native_dxgi_cursor_nudge_diagnostic_smoke(request(
            0,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("dxgi cursor nudge response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("non-empty bounds"));
    }

    #[test]
    #[ignore = "moves the real cursor and runs live DXGI comparison before and after"]
    fn dxgi_cursor_nudge_diagnostic_live_smoke() {
        let response = run_native_dxgi_cursor_nudge_diagnostic_smoke(request(
            320,
            180,
            Some(true),
            Some(true),
            Some(true),
        ))
        .expect("dxgi cursor nudge response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["fileWritten"], false);
        assert_eq!(response["cursor"]["restoreAttempted"], true);
        assert_eq!(response["cursor"]["restoreConfirmed"], true);
    }
}

#[cfg(test)]
mod native_cursor_nudge_command_tests {
    use super::*;

    fn nudge_request(
        dx: Option<i32>,
        dy: Option<i32>,
        explicit_opt_in: Option<bool>,
        allow_real_cursor_nudge: Option<bool>,
    ) -> NativeCursorNudgeSmokeRequest {
        NativeCursorNudgeSmokeRequest {
            dx,
            dy,
            explicit_opt_in,
            allow_real_cursor_nudge,
        }
    }

    #[test]
    fn native_cursor_nudge_default_denies_real_cursor_movement() {
        let response = run_native_cursor_nudge_smoke(nudge_request(None, None, None, None))
            .expect("cursor nudge response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["dx"], 1);
        assert_eq!(response["dy"], 0);
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealCursorNudge"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
    }

    #[test]
    fn native_cursor_nudge_blocks_without_allow_flag() {
        let response =
            run_native_cursor_nudge_smoke(nudge_request(Some(1), Some(0), Some(true), Some(false)))
                .expect("cursor nudge response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealCursorNudge"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real cursor movement is not allowed"));
    }

    #[test]
    fn native_cursor_nudge_rejects_large_movement_after_allow() {
        let response =
            run_native_cursor_nudge_smoke(nudge_request(Some(3), Some(0), Some(true), Some(true)))
                .expect("cursor nudge response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "blocked");
        assert_eq!(response["dx"], 3);
        assert_eq!(response["dy"], 0);
        assert_eq!(response["nudge"]["attempted"], false);
        assert!(response["nudge"]["error"]
            .as_str()
            .unwrap()
            .contains("two pixels"));
    }

    #[test]
    #[ignore = "moves the real cursor by one pixel and restores it"]
    fn native_cursor_nudge_live_smoke() {
        let response =
            run_native_cursor_nudge_smoke(nudge_request(Some(1), Some(0), Some(true), Some(true)))
                .expect("cursor nudge response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["clipboardChanged"], false);
        assert_eq!(response["nudge"]["restoreAttempted"], true);
        assert_eq!(response["nudge"]["restoreConfirmed"], true);
    }
}

#[cfg(test)]
mod native_dxgi_acquire_comparison_command_tests {
    use super::*;

    fn comparison_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
    ) -> NativeDxgiDefaultVsSelectedAcquireComparisonRequest {
        NativeDxgiDefaultVsSelectedAcquireComparisonRequest {
            x,
            y,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
        }
    }

    #[test]
    fn dxgi_acquire_comparison_default_denies_real_api() {
        let response = run_native_dxgi_default_vs_selected_acquire_comparison_smoke(
            comparison_request(-10, 20, 320, 100, None, None),
        )
        .expect("dxgi acquire comparison response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["defaultOutput"]["path"], "default-output");
        assert_eq!(response["selectedOutput"]["path"], "selected-output");
        assert_eq!(response["defaultOutput"]["attempted"], false);
        assert_eq!(response["selectedOutput"]["attempted"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["comparison"]["defaultFrameConfirmed"], false);
        assert_eq!(response["comparison"]["selectedFrameConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_acquire_comparison_blocks_without_real_api_allow_flag() {
        let response = run_native_dxgi_default_vs_selected_acquire_comparison_smoke(
            comparison_request(1, 2, 3, 4, Some(true), Some(false)),
        )
        .expect("dxgi acquire comparison response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn dxgi_acquire_comparison_rejects_invalid_bounds_after_explicit_allow() {
        let response = run_native_dxgi_default_vs_selected_acquire_comparison_smoke(
            comparison_request(-10, 20, 0, 100, Some(true), Some(true)),
        )
        .expect("dxgi acquire comparison response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 0);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], true);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("non-empty bounds"));
    }

    #[test]
    #[ignore = "requires a real Windows desktop duplication session"]
    fn dxgi_acquire_comparison_live_smoke() {
        let response = run_native_dxgi_default_vs_selected_acquire_comparison_smoke(
            comparison_request(0, 0, 320, 180, Some(true), Some(true)),
        )
        .expect("dxgi acquire comparison response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["defaultOutput"]["path"], "default-output");
        assert_eq!(response["selectedOutput"]["path"], "selected-output");
        assert_eq!(response["defaultOutput"]["frameCaptureAttempted"], true);
        assert_eq!(response["selectedOutput"]["frameCaptureAttempted"], true);
    }
}

#[cfg(test)]
mod native_dxgi_selected_readback_command_tests {
    use super::*;

    fn dxgi_selected_readback_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
    ) -> NativeDxgiSelectedReadbackSmokeRequest {
        NativeDxgiSelectedReadbackSmokeRequest {
            x,
            y,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
        }
    }

    #[test]
    fn dxgi_selected_readback_smoke_default_denies_real_api() {
        let response = run_native_dxgi_selected_readback_smoke(dxgi_selected_readback_request(
            -10, 20, 320, 100, None, None,
        ))
        .expect("dxgi selected readback response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["outputBounds"], serde_json::Value::Null);
        assert_eq!(response["crop"], serde_json::Value::Null);
        assert_eq!(response["selectedReadbackPlan"]["backend"], "dxgi-output");
        assert_eq!(response["selectedReadbackPlan"]["status"], "failed");
        assert_eq!(response["selectedOutputReadyPlanningOnly"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(response["scope"]
            .as_str()
            .unwrap()
            .contains("diagnostic-only"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_selected_readback_smoke_blocks_without_real_api_allow_flag() {
        let response = run_native_dxgi_selected_readback_smoke(dxgi_selected_readback_request(
            1,
            2,
            3,
            4,
            Some(true),
            Some(false),
        ))
        .expect("dxgi selected readback response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["selectedReadbackPlan"]["backend"], "dxgi-output");
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn dxgi_selected_readback_smoke_rejects_invalid_bounds_after_explicit_allow() {
        let response = run_native_dxgi_selected_readback_smoke(dxgi_selected_readback_request(
            -10,
            20,
            0,
            100,
            Some(true),
            Some(true),
        ))
        .expect("dxgi selected readback response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 0);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["selectedReadbackPlan"]["backend"], "dxgi-output");
        assert_eq!(response["selectedReadbackPlan"]["status"], "failed");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], true);
        assert_eq!(response["attemptedRealDxgiApi"], true);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("invalid selected readback bounds"));
    }
}

#[cfg(test)]
mod native_dxgi_selected_output_bridge_command_tests {
    use super::*;

    fn bridge_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
    ) -> NativeDxgiSelectedOutputBridgeDryRunRequest {
        NativeDxgiSelectedOutputBridgeDryRunRequest {
            x,
            y,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
        }
    }

    #[test]
    fn dxgi_selected_output_bridge_default_denies_real_api() {
        let response = run_native_dxgi_selected_output_bridge_dry_run(bridge_request(
            -10, 20, 320, 100, None, None,
        ))
        .expect("dxgi bridge dry-run response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["outputBounds"], serde_json::Value::Null);
        assert_eq!(response["crop"], serde_json::Value::Null);
        assert_eq!(response["selectedReadbackPlan"]["backend"], "dxgi-output");
        assert_eq!(response["selectedReadbackPlan"]["status"], "failed");
        assert_eq!(response["bridge"], serde_json::Value::Null);
        assert!(response["actions"].as_array().unwrap().is_empty());
        assert_eq!(response["bridgeValidated"], false);
        assert_eq!(response["selectedOnly"], false);
        assert_eq!(response["pngSignatureValid"], false);
        assert_eq!(response["releasedFrame"], false);
        assert_eq!(response["stopped"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(response["scope"]
            .as_str()
            .unwrap()
            .contains("diagnostic-only"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_selected_output_bridge_blocks_without_real_api_allow_flag() {
        let response = run_native_dxgi_selected_output_bridge_dry_run(bridge_request(
            1,
            2,
            3,
            4,
            Some(true),
            Some(false),
        ))
        .expect("dxgi bridge dry-run response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn dxgi_selected_output_bridge_rejects_invalid_bounds_after_explicit_allow() {
        let response = run_native_dxgi_selected_output_bridge_dry_run(bridge_request(
            -10,
            20,
            0,
            100,
            Some(true),
            Some(true),
        ))
        .expect("dxgi bridge dry-run response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 0);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["selectedReadbackPlan"]["backend"], "dxgi-output");
        assert_eq!(response["selectedReadbackPlan"]["status"], "failed");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], true);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["attemptedRealDxgiApi"], true);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("empty selected output bridge bounds"));
    }
}
#[cfg(test)]
mod native_dxgi_selected_output_acceptance_command_tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard(Option<String>);

    impl EnvGuard {
        fn clear() -> Self {
            let previous = std::env::var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE").ok();
            std::env::remove_var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE");
            Self(previous)
        }

        fn set() -> Self {
            Self::set_to("1")
        }

        fn set_to(value: &str) -> Self {
            let previous = std::env::var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE").ok();
            std::env::set_var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE", value);
            Self(previous)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.0.as_ref() {
                std::env::set_var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE", previous);
            } else {
                std::env::remove_var("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE");
            }
        }
    }

    fn acceptance_request(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        explicit_opt_in: Option<bool>,
        allow_real_dxgi_api: Option<bool>,
        allow_fake_clipboard_sink: Option<bool>,
        allow_real_clipboard: Option<bool>,
    ) -> NativeDxgiSelectedOutputClipboardAcceptanceRequest {
        NativeDxgiSelectedOutputClipboardAcceptanceRequest {
            x,
            y,
            width,
            height,
            explicit_opt_in,
            allow_real_dxgi_api,
            allow_fake_clipboard_sink,
            allow_real_clipboard,
        }
    }

    #[test]
    fn dxgi_selected_output_acceptance_default_denies_real_api() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::clear();
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(-10, 20, 320, 100, None, None, None, None),
        )
        .expect("dxgi acceptance response");
        let serialized = serde_json::to_string(&response).expect("json response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 320);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["allowFakeClipboardSink"], false);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["commandGuardPresent"], false);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["frameCaptureConfirmed"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert_eq!(response["clipboardReadbackAttempted"], false);
        assert_eq!(response["clipboardReadbackConfirmed"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["persistentHandleExposed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["sink"], serde_json::Value::Null);
        assert_eq!(response["receipt"], serde_json::Value::Null);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit opt-in"));
        assert!(!serialized.contains("hmonitor"));
        assert!(!serialized.contains("hwnd"));
        assert!(!serialized.contains("diagnosticHandle"));
        assert!(!serialized.contains("IDXGI"));
        assert!(!serialized.contains("ID3D11"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_blocks_without_real_api_allow_flag() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::clear();
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(1, 2, 3, 4, Some(true), Some(false), Some(true), None),
        )
        .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], false);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["commandGuardPresent"], false);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real API calls are not allowed"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_blocks_without_fake_sink_allow_flag() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::clear();
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(1, 2, 3, 4, Some(true), Some(true), Some(false), Some(false)),
        )
        .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], true);
        assert_eq!(response["allowFakeClipboardSink"], false);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["commandGuardPresent"], false);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("fake sink or real clipboard opt-in"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_blocks_conflicting_sink_modes() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::clear();
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(1, 2, 3, 4, Some(true), Some(true), Some(true), Some(true)),
        )
        .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], true);
        assert_eq!(response["commandGuardPresent"], false);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("exactly one clipboard sink mode"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_blocks_without_env_guard_after_command_allows() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::clear();
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(1, 2, 3, 4, Some(true), Some(true), Some(true), Some(false)),
        )
        .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert_eq!(response["clipboardReadbackAttempted"], false);
        assert_eq!(response["clipboardReadbackConfirmed"], false);
        assert_eq!(response["altAChanged"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_blocks_non_one_env_guard_value() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::set_to("true");
        let response = run_native_dxgi_selected_output_clipboard_acceptance_smoke(
            acceptance_request(1, 2, 3, 4, Some(true), Some(true), Some(true), Some(false)),
        )
        .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealDxgiApi"], false);
        assert_eq!(response["altAChanged"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1"));
    }

    #[test]
    fn dxgi_selected_output_acceptance_rejects_invalid_bounds_after_allows() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::set();
        let response =
            run_native_dxgi_selected_output_clipboard_acceptance_smoke(acceptance_request(
                -10,
                20,
                0,
                100,
                Some(true),
                Some(true),
                Some(true),
                Some(false),
            ))
            .expect("dxgi acceptance response");

        assert_eq!(response["ok"], false);
        assert_eq!(response["requestedBounds"]["x"], -10);
        assert_eq!(response["requestedBounds"]["y"], 20);
        assert_eq!(response["requestedBounds"]["width"], 0);
        assert_eq!(response["requestedBounds"]["height"], 100);
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealDxgiApi"], true);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], true);
        assert_eq!(response["attemptedRealDxgiApi"], true);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert_eq!(response["clipboardVerificationConfirmed"], false);
        assert_eq!(response["clipboardReadbackAttempted"], false);
        assert_eq!(response["clipboardReadbackConfirmed"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["sink"]["mode"], "fake");
        assert_eq!(
            response["sink"]["clipboardVerification"],
            serde_json::Value::Null
        );
        assert_eq!(response["sink"]["calls"], 0);
        assert!(response["acceptanceError"]
            .as_str()
            .unwrap()
            .contains("empty selected output bridge bounds"));
    }

    #[test]
    #[ignore = "requires a real Windows desktop duplication session and explicit acceptance env"]
    fn dxgi_selected_output_acceptance_fake_sink_live_smoke() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let _env_guard = EnvGuard::set();
        let response =
            run_native_dxgi_selected_output_clipboard_acceptance_smoke(acceptance_request(
                0,
                0,
                320,
                180,
                Some(true),
                Some(true),
                Some(true),
                Some(false),
            ))
            .expect("dxgi acceptance response");
        let require_success = std::env::var("YSN_REQUIRE_DXGI_SELECTED_OUTPUT_ACCEPTANCE_SMOKE")
            .ok()
            .as_deref()
            == Some("1");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("json response")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], true);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["clipboardReadbackAttempted"], false);
        assert_eq!(response["clipboardReadbackConfirmed"], false);
        assert_eq!(response["altAChanged"], false);

        if response["ok"].as_bool().unwrap_or(false) {
            let serialized = serde_json::to_string(&response).expect("json response");
            let evidence = &response["selectedPngEvidence"];
            assert_eq!(response["attemptedRealDxgiApi"], true);
            assert_eq!(response["frameCaptureAttempted"], true);
            assert_eq!(response["frameCaptureConfirmed"], true);
            assert_eq!(response["stage"], "stopped");
            assert_eq!(response["releasedFrame"], true);
            assert_eq!(response["stopped"], true);
            assert_eq!(response["bridgeError"], serde_json::Value::Null);
            assert_eq!(response["acceptanceError"], serde_json::Value::Null);
            assert_eq!(response["bridgeValidated"], true);
            assert_eq!(response["selectedOnly"], true);
            assert_eq!(response["pngSignatureValid"], true);
            assert_eq!(response["selectedOutputEffectConfirmed"], true);
            assert_ne!(evidence, &serde_json::Value::Null);
            assert_eq!(evidence["selectedOnlyPng"], true);
            assert!(evidence["pngWidth"].as_u64().unwrap_or(0) > 0);
            assert!(evidence["pngHeight"].as_u64().unwrap_or(0) > 0);
            assert!(evidence["pngByteLen"].as_u64().unwrap_or(0) > 8);
            assert_eq!(evidence["dimensionsMatchCrop"], true);
            assert_eq!(
                evidence["decodedRgbaByteLenExpected"].as_u64().unwrap(),
                evidence["pngWidth"].as_u64().unwrap()
                    * evidence["pngHeight"].as_u64().unwrap()
                    * 4
            );
            assert_eq!(response["crop"]["width"], evidence["pngWidth"]);
            assert_eq!(response["crop"]["height"], evidence["pngHeight"]);
            assert!(
                evidence["sourceWidth"].as_u64().unwrap() >= evidence["pngWidth"].as_u64().unwrap()
            );
            assert!(
                evidence["sourceHeight"].as_u64().unwrap()
                    >= evidence["pngHeight"].as_u64().unwrap()
            );
            assert_eq!(response["sink"]["mode"], "fake");
            assert_eq!(response["sink"]["calls"], 1);
            assert_eq!(
                response["sink"]["lastPngLen"],
                response["receipt"]["pngByteLen"]
            );
            assert_eq!(response["sink"]["lastPngLen"], evidence["pngByteLen"]);
            assert_eq!(
                response["sink"]["clipboardVerification"],
                serde_json::Value::Null
            );
            assert_eq!(response["clipboardVerificationConfirmed"], false);
            assert_eq!(response["receipt"]["action"], "copy");
            assert_eq!(response["receipt"]["target"], "clipboard");
            assert_eq!(response["receipt"]["format"], "png");
            assert_eq!(response["receipt"]["selectedOnlyPng"], true);
            assert_eq!(response["receipt"]["copiedToClipboard"], true);
            assert_eq!(response["receipt"]["saveInvoked"], false);
            assert_eq!(response["receipt"]["ocrInvoked"], false);
            assert_eq!(response["receipt"]["translationInvoked"], false);
            assert_eq!(response["receipt"]["diagnosticOnly"], true);
            assert_eq!(response["receipt"]["readinessChanged"], false);
            assert_eq!(response["receipt"]["persistentHandleExposed"], false);
            assert_eq!(response["receipt"]["sink"], "provided-sink");
            assert_eq!(response["diagnosticOnly"], true);
            assert_eq!(response["persistentHandleExposed"], false);
            assert_eq!(response["readinessChanged"], false);
            assert!(!serialized.contains("hmonitor"));
            assert!(!serialized.contains("hwnd"));
            assert!(!serialized.contains("diagnosticHandle"));
            assert!(!serialized.contains("IDXGI"));
            assert!(!serialized.contains("ID3D11"));
        } else if require_success {
            panic!("required live DXGI selected-output acceptance smoke failed: {response}");
        }
    }
}
#[cfg(test)]
mod native_selected_output_copy_command_tests {
    use super::*;
    use crate::screenshot_native::selected_output_effects::{
        SelectedOutputEffectError, SelectedOutputEffectSink,
    };
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[derive(Default)]
    struct FakeClipboardSink {
        calls: usize,
        last_png_len: usize,
    }

    impl SelectedOutputEffectSink for FakeClipboardSink {
        fn copy_png_to_clipboard(
            &mut self,
            png_bytes: &[u8],
        ) -> Result<(), SelectedOutputEffectError> {
            self.calls += 1;
            self.last_png_len = png_bytes.len();
            Ok(())
        }
    }

    fn seed_rgba_cache(session_id: &str) {
        let frame = crate::screenshot_native::RgbaFrame::new(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .expect("test rgba frame");
        let mut guard = get_screenshot_rgba().lock().expect("rgba cache lock");
        *guard = Some(SessionScreenshotRgba {
            session_id: session_id.to_string(),
            frame: Arc::new(frame),
        });
    }

    fn copy_request(session_id: &str, explicit_opt_in: bool) -> NativeSelectedOutputCopyRequest {
        NativeSelectedOutputCopyRequest {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            session_id: Some(session_id.to_string()),
            explicit_opt_in,
        }
    }

    #[test]
    fn native_copy_command_requires_explicit_opt_in_without_sink_call() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        seed_rgba_cache("copy-opt-in-test");
        let mut sink = FakeClipboardSink::default();
        let error = copy_native_selected_output_to_clipboard_with_sink(
            copy_request("copy-opt-in-test", false),
            &mut sink,
        )
        .expect_err("explicit opt-in required");

        assert!(error.contains("selected output effects require explicit opt-in"));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn native_copy_command_rejects_stale_session_before_sink_call() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        seed_rgba_cache("current-session");
        let mut sink = FakeClipboardSink::default();
        let error = copy_native_selected_output_to_clipboard_with_sink(
            copy_request("old-session", true),
            &mut sink,
        )
        .expect_err("stale session rejected");

        assert!(error.contains("stale screenshot RGBA cache rejected"));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn native_copy_command_returns_copy_only_receipt_shape() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        seed_rgba_cache("copy-success-test");
        let mut sink = FakeClipboardSink::default();
        let response = copy_native_selected_output_to_clipboard_with_sink(
            copy_request("copy-success-test", true),
            &mut sink,
        )
        .expect("copy response");

        assert_eq!(sink.calls, 1);
        assert!(sink.last_png_len > 0);
        assert_eq!(response["action"], "copy");
        assert_eq!(response["target"], "clipboard");
        assert_eq!(response["format"], "png");
        assert_eq!(response["selectedOnlyPng"], true);
        assert_eq!(response["copiedToClipboard"], true);
        assert_eq!(response["saveInvoked"], false);
        assert_eq!(response["ocrInvoked"], false);
        assert_eq!(response["translationInvoked"], false);
        assert_eq!(response["diagnostics"]["isValidBridge"], true);
    }
}
