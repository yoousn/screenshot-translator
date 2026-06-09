use crate::screenshot_commands::latest_or_request_physical_bounds;
use crate::screenshot_diagnostics_json::*;
use crate::screenshot_diagnostics_requests::NativeWgcSelectedOutputClipboardAcceptanceRequest;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::path::PathBuf;
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

fn wgc_selected_output_clipboard_acceptance_scope() -> &'static str {
    "diagnostic-only; guarded WGC selected-output clipboard acceptance; real clipboard requires allowRealClipboard, live selected PNG evidence, YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1, and YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1; no file, OCR, translation, presenter, overlay, Alt+A, or readiness side effects"
}

fn wgc_explicit_selection_selected_output_scope() -> &'static str {
    "diagnostic-only; requires explicit desktop physical request bounds before WGC selected-output acceptance; rejects latest/fullscreen fallback and does not change Alt+A, readiness, presenter, OCR, translation, or capture routing"
}

fn wgc_selected_output_acceptance_env_guard_present() -> bool {
    std::env::var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE")
        .ok()
        .as_deref()
        == Some("1")
}

fn wgc_selected_output_real_clipboard_env_guard_present() -> bool {
    std::env::var("YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD")
        .ok()
        .as_deref()
        == Some("1")
}

fn wgc_selected_output_env_guards_present(allow_real_clipboard: bool) -> bool {
    wgc_selected_output_acceptance_env_guard_present()
        && (!allow_real_clipboard || wgc_selected_output_real_clipboard_env_guard_present())
}

fn wgc_selected_output_acceptance_default_request(
) -> NativeWgcSelectedOutputClipboardAcceptanceRequest {
    NativeWgcSelectedOutputClipboardAcceptanceRequest {
        bounds: None,
        explicit_opt_in: Some(false),
        allow_real_wgc_api: Some(false),
        allow_fake_clipboard_sink: Some(false),
        allow_real_clipboard: Some(false),
        frame_timeout_ms: Some(0),
        include_cursor: Some(false),
        require_border: Some(false),
        buffer_count: Some(1),
        validate_target: Some(true),
        include_selected_png_base64: Some(false),
        allow_file_write: Some(false),
        save_path: None,
    }
}

fn explicit_selection_missing_bounds_response(
    request: &NativeWgcSelectedOutputClipboardAcceptanceRequest,
) -> serde_json::Value {
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "missing-explicit-request-bounds",
        "valid": false,
        "boundsSource": "missingRequest",
        "requestedBounds": serde_json::Value::Null,
        "latestPayload": serde_json::Value::Null,
        "explicitSelectionDiagnostic": true,
        "latestFallbackRejected": true,
        "requiresExplicitRequestBounds": true,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealWgcApi": request.allow_real_wgc_api.unwrap_or(false),
        "allowFakeClipboardSink": request.allow_fake_clipboard_sink.unwrap_or(false),
        "allowRealClipboard": request.allow_real_clipboard.unwrap_or(false),
        "guarded": true,
        "commandGuardPresent": false,
        "envGuardPresent": false,
        "realClipboardEnvGuardPresent": wgc_selected_output_real_clipboard_env_guard_present(),
        "attemptedRealWgcApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "selectedMonitorFrameConfirmed": false,
        "selectedOutputEffectConfirmed": false,
        "realClipboardAttempted": false,
        "realClipboardVerified": false,
        "clipboardReadbackAttempted": false,
        "clipboardReadbackConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "sink": serde_json::Value::Null,
        "receipt": serde_json::Value::Null,
        "session": serde_json::Value::Null,
        "error": "explicit desktop physical request bounds are required; latest screenshot/fullscreen fallback is intentionally rejected",
        "scope": wgc_explicit_selection_selected_output_scope(),
    })
}

fn request_bounds_json(
    bounds: Option<&crate::screenshot_diagnostics_requests::NativeDxgiSelectedReadbackSmokeRequest>,
) -> serde_json::Value {
    bounds
        .map(|bounds| {
            screenshot_physical_bounds_json(crate::screenshot_native::MonitorCaptureBounds::new(
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
            ))
        })
        .unwrap_or(serde_json::Value::Null)
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
        path.with_extension("png")
    }
}

fn write_selected_png_file_json(
    allow_file_write: bool,
    save_path: Option<&str>,
    image: Option<&crate::screenshot_native::output::SelectedImageContract>,
) -> serde_json::Value {
    let Some(save_path) = save_path else {
        return serde_json::json!({
            "attempted": false,
            "ok": false,
            "allowFileWrite": allow_file_write,
            "path": serde_json::Value::Null,
            "byteLen": serde_json::Value::Null,
            "error": serde_json::Value::Null,
        });
    };
    if !allow_file_write {
        return serde_json::json!({
            "attempted": false,
            "ok": false,
            "allowFileWrite": false,
            "path": save_path,
            "byteLen": serde_json::Value::Null,
            "error": "selected-output file write requires allowFileWrite=true",
        });
    }
    let Some(image) = image else {
        return serde_json::json!({
            "attempted": true,
            "ok": false,
            "allowFileWrite": true,
            "path": save_path,
            "byteLen": serde_json::Value::Null,
            "error": "selected PNG evidence is required before file write",
        });
    };
    let path = ensure_png_extension(PathBuf::from(save_path));
    match std::fs::write(&path, &image.png_bytes) {
        Ok(()) => serde_json::json!({
            "attempted": true,
            "ok": true,
            "allowFileWrite": true,
            "path": path.to_string_lossy().to_string(),
            "byteLen": image.png_bytes.len(),
            "pngWidth": image.crop.width,
            "pngHeight": image.crop.height,
            "selectedOnlyPng": image.is_selected_only_png(),
            "error": serde_json::Value::Null,
        }),
        Err(error) => serde_json::json!({
            "attempted": true,
            "ok": false,
            "allowFileWrite": true,
            "path": path.to_string_lossy().to_string(),
            "byteLen": serde_json::Value::Null,
            "error": error.to_string(),
        }),
    }
}

fn clipboard_verification_json(
    verification: Option<
        &crate::screenshot_native::selected_output_clipboard::ClipboardImageVerification,
    >,
) -> Option<serde_json::Value> {
    verification.map(|verification| {
        serde_json::json!({
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
        })
    })
}

fn wgc_selected_output_acceptance_disabled_response(
    request: &NativeWgcSelectedOutputClipboardAcceptanceRequest,
    error: &str,
) -> serde_json::Value {
    let allow_fake_clipboard_sink = request.allow_fake_clipboard_sink.unwrap_or(false);
    let allow_real_clipboard = request.allow_real_clipboard.unwrap_or(false);
    let command_guard_present = request.explicit_opt_in.unwrap_or(false)
        && request.allow_real_wgc_api.unwrap_or(false)
        && (allow_fake_clipboard_sink ^ allow_real_clipboard);
    let env_guard_present = wgc_selected_output_env_guards_present(allow_real_clipboard);
    let real_clipboard_env_guard_present = wgc_selected_output_real_clipboard_env_guard_present();
    serde_json::json!({
        "attempted": false,
        "ok": false,
        "stage": "disabled",
        "requestedBounds": request_bounds_json(request.bounds.as_ref()),
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealWgcApi": request.allow_real_wgc_api.unwrap_or(false),
        "allowFakeClipboardSink": allow_fake_clipboard_sink,
        "allowRealClipboard": allow_real_clipboard,
        "guarded": true,
        "commandGuardPresent": command_guard_present,
        "envGuardPresent": env_guard_present,
        "realClipboardEnvGuardPresent": real_clipboard_env_guard_present,
        "attemptedRealWgcApi": false,
        "frameCaptureAttempted": false,
        "frameCaptureConfirmed": false,
        "selectedMonitorFrameConfirmed": false,
        "selectedOutputEffectConfirmed": false,
        "realClipboardAttempted": false,
        "realClipboardVerified": false,
        "clipboardReadbackAttempted": false,
        "clipboardReadbackConfirmed": false,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "sink": serde_json::Value::Null,
        "receipt": serde_json::Value::Null,
        "session": serde_json::Value::Null,
        "error": error,
        "scope": wgc_selected_output_clipboard_acceptance_scope(),
    })
}

#[tauri::command]
pub fn run_native_wgc_selected_output_clipboard_acceptance_smoke(
    request: Option<NativeWgcSelectedOutputClipboardAcceptanceRequest>,
) -> Result<serde_json::Value, String> {
    let request = request.unwrap_or_else(wgc_selected_output_acceptance_default_request);
    if !request.explicit_opt_in.unwrap_or(false) {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output clipboard acceptance requires explicit opt-in",
        ));
    }
    if !request.allow_real_wgc_api.unwrap_or(false) {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output clipboard acceptance real WGC API calls are not allowed",
        ));
    }
    let allow_fake_clipboard_sink = request.allow_fake_clipboard_sink.unwrap_or(false);
    let allow_real_clipboard = request.allow_real_clipboard.unwrap_or(false);
    if !allow_fake_clipboard_sink && !allow_real_clipboard {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output clipboard acceptance requires fake sink or real clipboard opt-in",
        ));
    }
    if allow_fake_clipboard_sink && allow_real_clipboard {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output clipboard acceptance requires exactly one clipboard sink mode",
        ));
    }
    if !wgc_selected_output_acceptance_env_guard_present() {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output clipboard acceptance requires YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1",
        ));
    }
    if allow_real_clipboard && !wgc_selected_output_real_clipboard_env_guard_present() {
        return Ok(wgc_selected_output_acceptance_disabled_response(
            &request,
            "WGC selected-output real clipboard acceptance requires YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1",
        ));
    }

    let (bounds_source, bounds, latest_payload) =
        match latest_or_request_physical_bounds(request.bounds) {
            Ok(resolved) => resolved,
            Err(error) => {
                return Ok(add_wgc_smoke_safety_fields(
                    error,
                    wgc_selected_output_clipboard_acceptance_scope(),
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
            "attempted": true,
            "ok": false,
            "stage": "invalid-target",
            "valid": false,
            "boundsSource": bounds_source,
            "latestPayload": latest_payload_summary(latest_payload.as_ref()),
            "requestedBounds": screenshot_physical_bounds_json(bounds),
            "resolution": sanitized_wgc_target_report_json(&target_report),
            "validation": validation_report.as_ref().map(sanitized_wgc_target_report_json),
            "selectedReadbackPlan": build_wgc_selected_readback_plan_json(bounds, &target_report, validation_report.as_ref()),
            "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
            "allowRealWgcApi": request.allow_real_wgc_api.unwrap_or(false),
            "allowFakeClipboardSink": allow_fake_clipboard_sink,
            "allowRealClipboard": allow_real_clipboard,
            "guarded": true,
            "commandGuardPresent": true,
            "envGuardPresent": true,
            "realClipboardEnvGuardPresent": wgc_selected_output_real_clipboard_env_guard_present(),
            "diagnosticOnly": true,
            "persistentHandleExposed": false,
            "readinessChanged": false,
            "altAChanged": false,
            "attemptedRealWgcApi": false,
            "frameCaptureAttempted": false,
            "frameCaptureConfirmed": false,
            "selectedMonitorFrameConfirmed": false,
            "selectedOutputEffectConfirmed": false,
            "realClipboardAttempted": false,
            "realClipboardVerified": false,
            "clipboardReadbackAttempted": false,
            "clipboardReadbackConfirmed": false,
            "sink": serde_json::Value::Null,
            "receipt": serde_json::Value::Null,
            "session": serde_json::Value::Null,
            "error": target_report.error_message().or_else(|| validation_report.as_ref().and_then(|report| report.error_message())),
            "scope": wgc_selected_output_clipboard_acceptance_scope(),
        }));
    }
    let (_, session_bounds) = resolved_wgc_target_bounds(
        &target_report,
        validation_report.as_ref(),
    )
    .ok_or_else(|| {
        "validated WGC monitor bounds are required for selected-output acceptance".to_string()
    })?;
    let mut options = crate::screenshot_native::wgc_session::default_wgc_one_frame_session_options(
        target_report.target,
        session_bounds.width,
        session_bounds.height,
    );
    options.requested_bounds = Some(bounds);
    options.target_bounds = Some(session_bounds);
    options.request = crate::screenshot_native::wgc_contract::WgcOneFrameProbeRequest {
        explicit_opt_in: true,
        allow_real_wgc_api: true,
        frame_timeout_ms: request.frame_timeout_ms.unwrap_or(500),
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
    let selected_png_evidence = selected_png_evidence_json(report.selected_image.as_ref());
    let selected_png_base64 = if request.include_selected_png_base64.unwrap_or(false) {
        report
            .selected_image
            .as_ref()
            .map(|image| BASE64_STANDARD.encode(&image.png_bytes))
    } else {
        None
    };
    let selected_file = write_selected_png_file_json(
        request.allow_file_write.unwrap_or(false),
        request.save_path.as_deref(),
        report.selected_image.as_ref(),
    );
    let selected_image =
        if selected_monitor_frame_confirmed {
            report.selected_image.clone().ok_or_else(|| {
                "WGC selected-output acceptance did not produce selected PNG evidence".to_string()
            })
        } else {
            Err(report.error.as_ref().map(ToString::to_string).unwrap_or_else(|| {
            "WGC selected monitor frame evidence is required before selected-output acceptance"
                .to_string()
        }))
        };
    let sink_mode = if allow_real_clipboard { "real" } else { "fake" };
    let mut fake_sink = WgcDiagnosticFakeClipboardSink::default();
    let mut real_sink =
        crate::screenshot_native::selected_output_clipboard::VerifyingArboardSelectedOutputEffectSink::new();
    let acceptance = selected_image.and_then(|image| {
        if allow_real_clipboard {
            crate::screenshot_native::wgc_selected_output_acceptance::accept_wgc_selected_output_clipboard_with_sink(
                image,
                true,
                "real-clipboard",
                &mut real_sink,
            )
            .map_err(|error| error.to_string())
        } else {
            crate::screenshot_native::wgc_selected_output_acceptance::accept_wgc_selected_output_clipboard_with_sink(
                image,
                true,
                "provided-fake-sink",
                &mut fake_sink,
            )
            .map_err(|error| error.to_string())
        }
    });
    let clipboard_verification = real_sink.verification().cloned();
    let clipboard_readback_attempted = real_sink.readback_attempted();
    let acceptance_error = acceptance.as_ref().err().cloned();
    let acceptance = acceptance.ok();
    let clipboard_confirmed = clipboard_verification
        .as_ref()
        .map(|verification| verification.confirmed())
        .unwrap_or(false);
    let selected_output_effect_confirmed = selected_monitor_frame_confirmed
        && acceptance
            .as_ref()
            .map(|receipt| receipt.proves_clipboard_copy())
            .unwrap_or(false)
        && (!allow_fake_clipboard_sink || fake_sink.calls == 1)
        && (!allow_real_clipboard || clipboard_confirmed);
    let sink_json = serde_json::json!({
        "mode": sink_mode,
        "calls": if allow_fake_clipboard_sink { serde_json::json!(fake_sink.calls) } else { serde_json::Value::Null },
        "lastPngLen": if allow_fake_clipboard_sink { serde_json::json!(fake_sink.last_png_len) } else { serde_json::Value::Null },
        "clipboardVerification": clipboard_verification_json(clipboard_verification.as_ref()),
    });
    let receipt_json = acceptance.as_ref().map(|receipt| {
        serde_json::json!({
            "source": receipt.source,
            "action": debug_value(receipt.effect.action),
            "target": debug_value(receipt.effect.target),
            "format": debug_value(receipt.effect.format),
            "selectedOnlyPng": receipt.effect.selected_only_png,
            "pngByteLen": receipt.effect.png_byte_len,
            "copiedToClipboard": receipt.effect.copied_to_clipboard,
            "saveInvoked": receipt.effect.save_invoked,
            "ocrInvoked": receipt.effect.ocr_invoked,
            "translationInvoked": receipt.effect.translation_invoked,
            "copyOnly": receipt.effect.is_copy_only(),
            "diagnosticOnly": receipt.diagnostic_only,
            "readinessChanged": receipt.readiness_changed,
            "altAChanged": receipt.alt_a_changed,
            "persistentHandleExposed": receipt.persistent_handle_exposed,
            "sink": receipt.sink,
            "selectedOutputEffectAccepted": receipt.selected_output_effect_accepted,
        })
    });

    Ok(serde_json::json!({
        "attempted": true,
        "ok": selected_output_effect_confirmed,
        "stage": report.state.as_str(),
        "valid": true,
        "boundsSource": bounds_source,
        "latestPayload": latest_payload_summary(latest_payload.as_ref()),
        "requestedBounds": screenshot_physical_bounds_json(bounds),
        "sessionBounds": screenshot_physical_bounds_json(session_bounds),
        "resolution": sanitized_wgc_target_report_json(&target_report),
        "validation": validation_report.as_ref().map(sanitized_wgc_target_report_json),
        "selectedReadbackPlan": build_wgc_selected_readback_plan_json(bounds, &target_report, validation_report.as_ref()),
        "session": sanitized_wgc_session_report_json(&report),
        "selectedPngEvidence": selected_png_evidence,
        "selectedPngBase64": selected_png_base64,
        "selectedFile": selected_file,
        "explicitOptIn": request.explicit_opt_in.unwrap_or(false),
        "allowRealWgcApi": request.allow_real_wgc_api.unwrap_or(false),
        "allowFakeClipboardSink": allow_fake_clipboard_sink,
        "allowRealClipboard": allow_real_clipboard,
        "guarded": true,
        "commandGuardPresent": true,
        "envGuardPresent": true,
        "realClipboardEnvGuardPresent": wgc_selected_output_real_clipboard_env_guard_present(),
        "attemptedRealWgcApi": report.attempted_real_wgc_api,
        "frameCaptureAttempted": report.started_capture,
        "frameCaptureConfirmed": frame_capture_confirmed,
        "selectedMonitorFrameConfirmed": selected_monitor_frame_confirmed,
        "selectedOutputEffectConfirmed": selected_output_effect_confirmed,
        "realClipboardAttempted": allow_real_clipboard,
        "realClipboardVerified": clipboard_confirmed,
        "clipboardReadbackAttempted": clipboard_readback_attempted,
        "clipboardReadbackConfirmed": clipboard_confirmed,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "sink": sink_json,
        "receipt": receipt_json,
        "sessionError": report.error.as_ref().map(ToString::to_string),
        "acceptanceError": acceptance_error,
        "scope": wgc_selected_output_clipboard_acceptance_scope(),
    }))
}

#[tauri::command]
pub fn run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke(
    request: Option<NativeWgcSelectedOutputClipboardAcceptanceRequest>,
) -> Result<serde_json::Value, String> {
    let Some(request) = request else {
        return Ok(explicit_selection_missing_bounds_response(
            &wgc_selected_output_acceptance_default_request(),
        ));
    };
    if request.bounds.is_none() {
        return Ok(explicit_selection_missing_bounds_response(&request));
    }

    let mut response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(request))?;
    if let Some(object) = response.as_object_mut() {
        object.insert(
            "explicitSelectionDiagnostic".to_string(),
            serde_json::Value::Bool(true),
        );
        object.insert(
            "latestFallbackRejected".to_string(),
            serde_json::Value::Bool(true),
        );
        object.insert(
            "requiresExplicitRequestBounds".to_string(),
            serde_json::Value::Bool(true),
        );
        object.insert(
            "scope".to_string(),
            serde_json::Value::String(wgc_explicit_selection_selected_output_scope().to_string()),
        );
    }
    Ok(response)
}

#[cfg(test)]
mod native_wgc_selected_output_clipboard_acceptance_command_tests {
    use super::*;
    use crate::screenshot_diagnostics_requests::{
        NativeDxgiSelectedReadbackSmokeRequest, NativeWgcSelectedOutputClipboardAcceptanceRequest,
    };
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn acceptance_request(
        explicit_opt_in: Option<bool>,
        allow_real_wgc_api: Option<bool>,
        allow_fake_clipboard_sink: Option<bool>,
        allow_real_clipboard: Option<bool>,
    ) -> NativeWgcSelectedOutputClipboardAcceptanceRequest {
        NativeWgcSelectedOutputClipboardAcceptanceRequest {
            bounds: Some(NativeDxgiSelectedReadbackSmokeRequest {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                explicit_opt_in: Some(true),
                allow_real_dxgi_api: Some(false),
            }),
            explicit_opt_in,
            allow_real_wgc_api,
            allow_fake_clipboard_sink,
            allow_real_clipboard,
            frame_timeout_ms: Some(500),
            include_cursor: Some(false),
            require_border: Some(false),
            buffer_count: Some(1),
            validate_target: Some(true),
            include_selected_png_base64: Some(false),
            allow_file_write: Some(false),
            save_path: None,
        }
    }

    #[test]
    fn wgc_selected_output_acceptance_default_denies_side_effects() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(None)
            .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["stage"], "disabled");
        assert_eq!(response["explicitOptIn"], false);
        assert_eq!(response["allowRealWgcApi"], false);
        assert_eq!(response["allowFakeClipboardSink"], false);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["guarded"], true);
        assert_eq!(response["commandGuardPresent"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["frameCaptureAttempted"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert_eq!(response["realClipboardAttempted"], false);
        assert_eq!(response["realClipboardVerified"], false);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["sink"], serde_json::Value::Null);
        assert_eq!(response["receipt"], serde_json::Value::Null);
    }

    #[test]
    fn wgc_selected_output_acceptance_blocks_without_real_api_allow() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(false), Some(true), Some(false)),
        ))
        .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["allowRealWgcApi"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("real WGC API calls are not allowed"));
    }

    #[test]
    fn wgc_selected_output_acceptance_blocks_without_sink_mode() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(true), Some(false), Some(false)),
        ))
        .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["allowFakeClipboardSink"], false);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["commandGuardPresent"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("requires fake sink or real clipboard opt-in"));
    }

    #[test]
    fn wgc_selected_output_acceptance_blocks_conflicting_sink_modes() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(true), Some(true), Some(true)),
        ))
        .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], true);
        assert_eq!(response["commandGuardPresent"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("requires exactly one clipboard sink mode"));
    }

    #[test]
    fn wgc_explicit_selection_acceptance_rejects_missing_request_object() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let response =
            run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke(None)
                .expect("strict explicit selection response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["boundsSource"], "missingRequest");
        assert_eq!(response["latestFallbackRejected"], true);
        assert_eq!(response["requiresExplicitRequestBounds"], true);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("explicit desktop physical request bounds"));
    }

    #[test]
    fn wgc_explicit_selection_acceptance_rejects_latest_fallback() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        let mut request = acceptance_request(Some(true), Some(true), Some(true), Some(false));
        request.bounds = None;
        let response =
            run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke(Some(
                request,
            ))
            .expect("strict explicit selection response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["ok"], false);
        assert_eq!(response["boundsSource"], "missingRequest");
        assert_eq!(response["latestFallbackRejected"], true);
        assert_eq!(response["requiresExplicitRequestBounds"], true);
        assert_eq!(response["explicitOptIn"], true);
        assert_eq!(response["allowRealWgcApi"], true);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["attemptedRealWgcApi"], false);
    }

    #[test]
    fn wgc_explicit_selection_acceptance_preserves_guarded_underlying_response() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        std::env::remove_var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE");
        let response =
            run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke(Some(
                acceptance_request(Some(true), Some(true), Some(true), Some(false)),
            ))
            .expect("strict explicit selection response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["explicitSelectionDiagnostic"], true);
        assert_eq!(response["latestFallbackRejected"], true);
        assert_eq!(response["requiresExplicitRequestBounds"], true);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1"));
    }

    #[test]
    fn wgc_selected_output_acceptance_blocks_without_env_guard() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        std::env::remove_var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(true), Some(true), Some(false)),
        ))
        .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["selectedOutputEffectConfirmed"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1"));
    }

    #[test]
    fn wgc_selected_output_acceptance_blocks_real_clipboard_without_real_env_guard() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        std::env::set_var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE", "1");
        std::env::remove_var("YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(true), Some(false), Some(true)),
        ))
        .expect("wgc acceptance response");

        assert_eq!(response["attempted"], false);
        assert_eq!(response["commandGuardPresent"], true);
        assert_eq!(response["envGuardPresent"], false);
        assert_eq!(response["realClipboardEnvGuardPresent"], false);
        assert_eq!(response["attemptedRealWgcApi"], false);
        assert_eq!(response["realClipboardAttempted"], false);
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1"));
    }

    #[test]
    #[ignore = "requires real WGC capture and YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE=1"]
    fn wgc_explicit_selection_fake_sink_non_1x1_live_smoke() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        if std::env::var("YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE")
            .ok()
            .as_deref()
            != Some("1")
        {
            eprintln!(
                "skipping WGC explicit-selection fake-sink live smoke; set YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE=1 to run"
            );
            return;
        }
        std::env::set_var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE", "1");
        let mut request = acceptance_request(Some(true), Some(true), Some(true), Some(false));
        let save_path = std::env::temp_dir().join("ysn-wgc-selected-output-live-smoke.png");
        let _ = std::fs::remove_file(&save_path);
        request.include_selected_png_base64 = Some(true);
        request.allow_file_write = Some(true);
        request.save_path = Some(save_path.to_string_lossy().to_string());
        request.bounds = Some(NativeDxgiSelectedReadbackSmokeRequest {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
            explicit_opt_in: Some(true),
            allow_real_dxgi_api: Some(false),
        });
        let response =
            run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke(Some(
                request,
            ))
            .expect("wgc explicit-selection response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("response json")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["ok"], true);
        assert_eq!(response["boundsSource"], "request");
        assert_eq!(response["explicitSelectionDiagnostic"], true);
        assert_eq!(response["latestFallbackRejected"], true);
        assert_eq!(response["requiresExplicitRequestBounds"], true);
        assert_eq!(response["allowFakeClipboardSink"], true);
        assert_eq!(response["allowRealClipboard"], false);
        assert_eq!(response["attemptedRealWgcApi"], true);
        assert_eq!(response["frameCaptureConfirmed"], true);
        assert_eq!(response["selectedMonitorFrameConfirmed"], true);
        assert_eq!(response["selectedOutputEffectConfirmed"], true);
        assert_eq!(response["realClipboardAttempted"], false);
        assert_eq!(response["selectedPngEvidence"]["pngWidth"], 64);
        assert_eq!(response["selectedPngEvidence"]["pngHeight"], 48);
        assert_eq!(response["selectedPngEvidence"]["selectedOnlyPng"], true);
        assert_eq!(response["selectedFile"]["attempted"], true);
        assert_eq!(response["selectedFile"]["ok"], true);
        assert_eq!(response["selectedFile"]["pngWidth"], 64);
        assert_eq!(response["selectedFile"]["pngHeight"], 48);
        assert_eq!(response["selectedFile"]["selectedOnlyPng"], true);
        assert_eq!(
            std::fs::metadata(&save_path)
                .expect("saved png metadata")
                .len(),
            12_404
        );
        let _ = std::fs::remove_file(&save_path);
        assert!(
            response["selectedPngBase64"]
                .as_str()
                .unwrap_or_default()
                .len()
                > 0
        );
        assert_eq!(response["sink"]["mode"], "fake");
        assert_eq!(response["sink"]["calls"], 1);
        assert_eq!(response["receipt"]["sink"], "provided-fake-sink");
        assert_eq!(response["receipt"]["copyOnly"], true);
        assert_eq!(response["receipt"]["selectedOnlyPng"], true);
    }

    #[test]
    #[ignore = "requires real WGC capture and writes the real OS clipboard"]
    fn wgc_selected_output_acceptance_real_clipboard_live_smoke() {
        let _guard = TEST_LOCK.lock().expect("test lock");
        if std::env::var("YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE")
            .ok()
            .as_deref()
            != Some("1")
        {
            eprintln!(
                "skipping WGC selected-output real clipboard smoke; set YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE=1 to run"
            );
            return;
        }
        std::env::set_var("YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE", "1");
        std::env::set_var("YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD", "1");
        let response = run_native_wgc_selected_output_clipboard_acceptance_smoke(Some(
            acceptance_request(Some(true), Some(true), Some(false), Some(true)),
        ))
        .expect("wgc acceptance response");

        println!(
            "{}",
            serde_json::to_string_pretty(&response).expect("response json")
        );
        assert_eq!(response["attempted"], true);
        assert_eq!(response["ok"], true);
        assert_eq!(response["allowRealClipboard"], true);
        assert_eq!(response["allowFakeClipboardSink"], false);
        assert_eq!(response["attemptedRealWgcApi"], true);
        assert_eq!(response["frameCaptureConfirmed"], true);
        assert_eq!(response["selectedMonitorFrameConfirmed"], true);
        assert_eq!(response["selectedOutputEffectConfirmed"], true);
        assert_eq!(response["realClipboardAttempted"], true);
        assert_eq!(response["realClipboardVerified"], true);
        assert_eq!(response["clipboardReadbackAttempted"], true);
        assert_eq!(response["clipboardReadbackConfirmed"], true);
        assert_eq!(response["diagnosticOnly"], true);
        assert_eq!(response["readinessChanged"], false);
        assert_eq!(response["altAChanged"], false);
        assert_eq!(response["sink"]["mode"], "real");
        assert_eq!(response["sink"]["clipboardVerification"]["confirmed"], true);
        assert_eq!(response["receipt"]["sink"], "real-clipboard");
        assert_eq!(response["receipt"]["copyOnly"], true);
        assert_eq!(response["receipt"]["selectedOnlyPng"], true);
    }
}
