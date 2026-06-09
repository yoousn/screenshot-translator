use crate::screenshot_diagnostics_json::screenshot_physical_bounds_json;

pub(crate) fn desktop_update_pulse_report_json(
    report: crate::screenshot_native::win32_desktop_update_pulse::DesktopUpdatePulseReport,
) -> serde_json::Value {
    serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "pulseBounds": report.pulse_bounds.map(screenshot_physical_bounds_json),
        "pulseSizePx": report.pulse_size_px,
        "pulseAlpha": report.pulse_alpha,
        "dwellMs": report.dwell_ms,
        "classRegistered": report.class_registered,
        "windowCreated": report.window_created,
        "layeredAttributesSet": report.layered_attributes_set,
        "shownNoActivate": report.shown_no_activate,
        "updateWindowCalled": report.update_window_called,
        "invalidateCalled": report.invalidate_called,
        "dwmFlushCalled": report.dwm_flush_called,
        "destroyAttempted": report.destroy_attempted,
        "destroyConfirmed": report.destroy_confirmed,
        "hiddenFromAltTab": report.hidden_from_alt_tab,
        "noActivate": report.no_activate,
        "appWindowExcluded": report.appwindow_excluded,
        "error": report.error,
    })
}

pub(crate) fn cursor_point_json(
    point: Option<crate::screenshot_native::win32_cursor::CursorPoint>,
) -> serde_json::Value {
    point
        .map(|point| {
            serde_json::json!({
                "x": point.x,
                "y": point.y,
            })
        })
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn cursor_nudge_report_json(
    report: crate::screenshot_native::win32_cursor::CursorNudgeReport,
) -> serde_json::Value {
    serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "dx": report.dx,
        "dy": report.dy,
        "original": cursor_point_json(report.original),
        "nudged": cursor_point_json(report.nudged),
        "afterNudge": cursor_point_json(report.after_nudge),
        "restored": cursor_point_json(report.restored),
        "restoreAttempted": report.restore_attempted,
        "restoreConfirmed": report.restore_confirmed,
        "error": report.error,
    })
}
