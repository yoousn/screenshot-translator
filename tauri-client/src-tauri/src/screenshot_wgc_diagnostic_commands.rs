use crate::screenshot_commands::latest_or_request_physical_bounds;
use crate::screenshot_diagnostics_json::*;
use crate::screenshot_diagnostics_requests::{
    NativeWgcMonitorSessionSmokeRequest, NativeWgcMonitorTargetDiagnosticRequest,
    NativeWgcOneFrameProbeSmokeRequest,
};

pub use crate::screenshot_wgc_selected_output_diagnostic_commands::{
    run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke,
    run_native_wgc_selected_output_clipboard_acceptance_smoke,
};
#[derive(Default)]
struct WgcDiagnosticFakeClipboardSink {
    calls: usize,
    last_png_len: usize,
}

impl crate::screenshot_native::selected_output_effects::SelectedOutputEffectSink
    for WgcDiagnosticFakeClipboardSink
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

fn wgc_monitor_session_smoke_scope() -> &'static str {
    "diagnostic-only; guarded WGC monitor session smoke requires explicit opt-in for real API and does not change Alt+A, readiness, presenter, OCR, translation, or capture routing"
}

fn wgc_monitor_target_diagnostic_scope() -> &'static str {
    "diagnostic-only; resolves WGC monitor target from explicit or latest screenshot physical bounds without changing Alt+A, readiness, presenter, OCR, translation, or capture routing"
}

#[tauri::command]
pub fn run_native_wgc_one_frame_probe_smoke(
    request: Option<NativeWgcOneFrameProbeSmokeRequest>,
) -> Result<serde_json::Value, String> {
    let request = request
        .map(
            |request| crate::screenshot_native::wgc_probe::WgcOneFrameProbeRequest {
                explicit_opt_in: request.explicit_opt_in.unwrap_or(false),
                allow_real_wgc_api: request.allow_real_wgc_api.unwrap_or(false),
                frame_timeout_ms: request.frame_timeout_ms.unwrap_or(0),
            },
        )
        .unwrap_or_else(crate::screenshot_native::wgc_probe::WgcOneFrameProbeRequest::disabled);
    let report = crate::screenshot_native::wgc_probe::resolve_wgc_one_frame_smoke_report(request);
    Ok(serde_json::json!({
        "status": report.status.as_str(),
        "ok": matches!(report.status, crate::screenshot_native::wgc_probe::WgcOneFrameSmokeStatus::ReadyToAttempt),
        "planStatus": debug_value(report.plan_status),
        "attemptedRealWgcApi": report.attempted_real_wgc_api,
        "frameCaptureAttempted": report.frame_capture_attempted,
        "frameCaptureConfirmed": report.frame_capture_confirmed,
        "shouldAttemptProbe": report.should_attempt_probe,
        "fallback": debug_value(report.fallback),
        "error": report.error.as_ref().map(debug_value),
        "reason": report.reason,
        "scope": "diagnostic-only; WGC smoke report distinguishes API readiness from frame capture and does not mark WGC, Alt+A, presenter, or C/E readiness complete"
    }))
}

#[tauri::command]
pub fn resolve_native_wgc_monitor_target_diagnostic(
    request: Option<NativeWgcMonitorTargetDiagnosticRequest>,
) -> Result<serde_json::Value, String> {
    let request = request.unwrap_or(NativeWgcMonitorTargetDiagnosticRequest {
        bounds: None,
        validate: Some(false),
    });
    let (bounds_source, bounds, latest_payload) =
        match latest_or_request_physical_bounds(request.bounds) {
            Ok(resolved) => resolved,
            Err(error) => {
                return Ok(add_wgc_smoke_safety_fields(
                    error,
                    wgc_monitor_target_diagnostic_scope(),
                ));
            }
        };
    let target_report =
        crate::screenshot_native::wgc_target::resolve_wgc_monitor_target_from_bounds(bounds);
    let validate = request.validate.unwrap_or(false);
    let validation_report = if validate && target_report.valid {
        Some(
            crate::screenshot_native::wgc_target::validate_wgc_capture_target(target_report.target),
        )
    } else {
        None
    };
    let ok = target_report.valid
        && validation_report
            .as_ref()
            .map(|report| report.valid)
            .unwrap_or(true);
    let latest_payload_ref = latest_payload.as_ref();
    Ok(serde_json::json!({
        "ok": ok,
        "valid": target_report.valid,
        "boundsSource": bounds_source,
        "latestPayload": latest_payload_summary(latest_payload_ref),
        "bounds": screenshot_physical_bounds_json(bounds),
        "resolution": sanitized_wgc_target_report_json(&target_report),
        "validation": validation_report.as_ref().map(sanitized_wgc_target_report_json),
        "selectedReadbackPlan": build_wgc_selected_readback_plan_json(bounds, &target_report, validation_report.as_ref()),
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "attemptedRealWgcApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "error": target_report.error_message(),
        "scope": wgc_monitor_target_diagnostic_scope()
    }))
}

#[tauri::command]
pub fn run_native_wgc_monitor_session_smoke(
    request: Option<NativeWgcMonitorSessionSmokeRequest>,
) -> Result<serde_json::Value, String> {
    let request = request.unwrap_or(NativeWgcMonitorSessionSmokeRequest {
        bounds: None,
        explicit_opt_in: Some(false),
        allow_real_wgc_api: Some(false),
        frame_timeout_ms: Some(0),
        include_cursor: Some(false),
        require_border: Some(false),
        buffer_count: Some(1),
        validate_target: Some(true),
    });
    let (bounds_source, bounds, latest_payload) =
        match latest_or_request_physical_bounds(request.bounds) {
            Ok(resolved) => resolved,
            Err(error) => {
                return Ok(add_wgc_smoke_safety_fields(
                    error,
                    wgc_monitor_session_smoke_scope(),
                ));
            }
        };
    let target_report =
        crate::screenshot_native::wgc_target::resolve_wgc_monitor_target_from_bounds(bounds);
    let validate_target = request.validate_target.unwrap_or(true);
    let validation_report = if validate_target && target_report.valid {
        Some(
            crate::screenshot_native::wgc_target::validate_wgc_capture_target(target_report.target),
        )
    } else {
        None
    };
    let target_valid = target_report.valid
        && validation_report
            .as_ref()
            .map(|report| report.valid)
            .unwrap_or(true);
    if !target_valid {
        return Ok(serde_json::json!({
            "ok": false,
            "valid": false,
            "boundsSource": bounds_source,
            "latestPayload": latest_payload_summary(latest_payload.as_ref()),
            "bounds": screenshot_physical_bounds_json(bounds),
            "resolution": sanitized_wgc_target_report_json(&target_report),
            "validation": validation_report.as_ref().map(sanitized_wgc_target_report_json),
            "selectedReadbackPlan": build_wgc_selected_readback_plan_json(bounds, &target_report, validation_report.as_ref()),
            "session": serde_json::Value::Null,
            "diagnosticOnly": true,
            "persistentHandleExposed": false,
            "readinessChanged": false,
            "attemptedRealWgcApi": false,
            "frameCaptureAttempted": false,
            "frameCaptureConfirmed": false,
            "error": target_report.error_message().or_else(|| validation_report.as_ref().and_then(|report| report.error_message())),
            "scope": wgc_monitor_session_smoke_scope()
        }));
    }
    let (_, session_bounds) =
        resolved_wgc_target_bounds(&target_report, validation_report.as_ref()).ok_or_else(
            || "validated WGC monitor bounds are required for session sizing".to_string(),
        )?;
    let mut options = crate::screenshot_native::wgc_session::default_wgc_one_frame_session_options(
        target_report.target,
        session_bounds.width,
        session_bounds.height,
    );
    options.requested_bounds = Some(bounds);
    options.target_bounds = Some(session_bounds);
    options.request = crate::screenshot_native::wgc_contract::WgcOneFrameProbeRequest {
        explicit_opt_in: request.explicit_opt_in.unwrap_or(false),
        allow_real_wgc_api: request.allow_real_wgc_api.unwrap_or(false),
        frame_timeout_ms: request.frame_timeout_ms.unwrap_or(0),
    };
    options.include_cursor = request.include_cursor.unwrap_or(false);
    options.require_border = request.require_border.unwrap_or(false);
    options.buffer_count = request.buffer_count.unwrap_or(1);
    let report = crate::screenshot_native::wgc_session::guarded_wgc_one_frame_session(options);
    let frame_capture_confirmed = report.acquired_frame && report.frame.is_some();
    let selected_frame_evidence = &report.selected_monitor_frame_evidence;
    let selected_monitor_frame_confirmed = matches!(
        report.state,
        crate::screenshot_native::wgc_session::WgcSessionState::FrameAcquired
    ) && frame_capture_confirmed
        && selected_frame_evidence.frame_matches_target_monitor_bounds
        && selected_frame_evidence.selected_crop_within_frame
        && selected_frame_evidence.selected_png_produced
        && selected_frame_evidence.readback_bytes_present;
    let mut fake_sink = WgcDiagnosticFakeClipboardSink::default();
    let fake_sink_acceptance = report.selected_image.clone().map(|image| {
        crate::screenshot_native::wgc_selected_output_acceptance::accept_wgc_selected_output_fake_sink_copy(
            image,
            true,
            &mut fake_sink,
        )
    });
    let fake_sink_receipt = fake_sink_acceptance
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let fake_sink_error = fake_sink_acceptance
        .as_ref()
        .and_then(|result| result.as_ref().err())
        .map(ToString::to_string)
        .or_else(|| {
            if selected_monitor_frame_confirmed {
                None
            } else {
                Some(
                    "WGC selected PNG evidence is required before fake-sink acceptance".to_string(),
                )
            }
        });
    let selected_output_fake_sink_acceptance =
        wgc_fake_sink_acceptance_json(fake_sink_receipt, fake_sink_error.as_deref());
    Ok(serde_json::json!({
        "ok": selected_monitor_frame_confirmed && fake_sink_receipt.map(|receipt| receipt.proves_fake_sink_copy()).unwrap_or(false),
        "valid": true,
        "boundsSource": bounds_source,
        "latestPayload": latest_payload_summary(latest_payload.as_ref()),
        "bounds": screenshot_physical_bounds_json(bounds),
        "sessionBounds": screenshot_physical_bounds_json(session_bounds),
        "resolution": sanitized_wgc_target_report_json(&target_report),
        "validation": validation_report.as_ref().map(sanitized_wgc_target_report_json),
        "selectedReadbackPlan": build_wgc_selected_readback_plan_json(bounds, &target_report, validation_report.as_ref()),
        "session": merge_wgc_session_fake_sink_acceptance(sanitized_wgc_session_report_json(&report), selected_output_fake_sink_acceptance.clone()),
        "selectedOutputFakeSinkAcceptance": selected_output_fake_sink_acceptance,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "attemptedRealWgcApi": report.attempted_real_wgc_api,
        "frameCaptureAttempted": report.started_capture,
        "frameCaptureConfirmed": frame_capture_confirmed,
        "error": report.error.as_ref().map(ToString::to_string),
        "scope": wgc_monitor_session_smoke_scope()
    }))
}
