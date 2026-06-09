pub(crate) fn debug_value<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}")
}

pub(crate) fn bytes_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

pub(crate) fn screenshot_physical_bounds_json(
    bounds: crate::screenshot_native::MonitorCaptureBounds,
) -> serde_json::Value {
    serde_json::json!({
        "x": bounds.origin_x,
        "y": bounds.origin_y,
        "width": bounds.width,
        "height": bounds.height,
    })
}

pub(crate) fn parse_physical_bounds_value(
    value: &serde_json::Value,
) -> Result<crate::screenshot_native::MonitorCaptureBounds, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "physical bounds must be an object".to_string())?;
    let x = object
        .get("x")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| "physical bounds x must be a valid i32".to_string())?;
    let y = object
        .get("y")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| "physical bounds y must be a valid i32".to_string())?;
    let width = object
        .get("width")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "physical bounds width must be a valid u32".to_string())?;
    let height = object
        .get("height")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "physical bounds height must be a valid u32".to_string())?;
    let bounds = crate::screenshot_native::MonitorCaptureBounds::new(x, y, width, height);
    if bounds.is_empty() || bounds.right().is_none() || bounds.bottom().is_none() {
        return Err(
            "physical bounds must be non-empty and within i32 desktop coordinates".to_string(),
        );
    }
    Ok(bounds)
}

pub(crate) fn parse_latest_screenshot_physical_bounds(
    payload: &serde_json::Value,
) -> Result<crate::screenshot_native::MonitorCaptureBounds, String> {
    let bounds = payload
        .get("physicalBounds")
        .ok_or_else(|| "latest screenshot payload has no physicalBounds".to_string())?;
    parse_physical_bounds_value(bounds)
}

pub(crate) fn latest_payload_summary(
    latest_payload: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "latestPayloadPresent": latest_payload.is_some(),
        "sessionId": latest_payload.and_then(|payload| payload.get("sessionId")).and_then(serde_json::Value::as_str),
        "captureWidth": latest_payload.and_then(|payload| payload.get("width")).and_then(serde_json::Value::as_u64),
        "captureHeight": latest_payload.and_then(|payload| payload.get("height")).and_then(serde_json::Value::as_u64),
    })
}

pub(crate) fn sanitized_wgc_target_report_json(
    report: &crate::screenshot_native::wgc_target::WgcTargetValidationReport,
) -> serde_json::Value {
    serde_json::json!({
        "kind": report.kind.as_str(),
        "valid": report.valid,
        "bounds": report.bounds.map(screenshot_physical_bounds_json),
        "hasTargetHandle": report.valid,
        "error": report.error_message(),
    })
}

pub(crate) fn sanitized_wgc_session_report_json(
    report: &crate::screenshot_native::wgc_session::WgcOneFrameSessionReport,
) -> serde_json::Value {
    let evidence = &report.selected_monitor_frame_evidence;
    let frame_dimensions_match_session =
        evidence.frame_width == Some(report.width) && evidence.frame_height == Some(report.height);
    let frame_format = report
        .frame
        .as_ref()
        .map(|frame| debug_value(frame.metadata.format));
    let selected_png_evidence = selected_png_evidence_json(report.selected_image.as_ref());
    let selected_frame_evidence = report.frame.as_ref().map(|frame| {
        serde_json::json!({
            "diagnosticOnly": evidence.diagnostic_only,
            "requestedBoundsPhysical": evidence.requested_bounds_physical.map(screenshot_physical_bounds_json),
            "targetMonitorBoundsPhysical": evidence.target_monitor_bounds_physical.map(screenshot_physical_bounds_json),
            "framepoolSizeSource": evidence.framepool_size_source,
            "frameAcquired": report.acquired_frame,
            "frameId": frame.metadata.frame_id,
            "frameWidth": evidence.frame_width,
            "frameHeight": evidence.frame_height,
            "requestedSessionWidth": report.width,
            "requestedSessionHeight": report.height,
            "dimensionsMatchSession": frame_dimensions_match_session,
            "frameMatchesTargetMonitorBounds": evidence.frame_matches_target_monitor_bounds,
            "selectedCropWithinFrame": evidence.selected_crop_within_frame,
            "format": debug_value(frame.metadata.format),
            "source": debug_value(frame.metadata.source),
            "textureMetadataPresent": frame.metadata.texture.is_some(),
            "stagingReadbackPresent": frame.metadata.staging_readback.is_some(),
            "readbackBytesPresent": evidence.readback_bytes_present,
            "readbackByteLen": frame.readback_bytes.as_ref().map(Vec::len),
            "selectedPngEvidence": selected_png_evidence,
            "selectedPngProduced": evidence.selected_png_produced,
            "persistentHandleExposed": evidence.persistent_handle_exposed,
            "readinessChanged": evidence.readiness_changed,
            "scope": "diagnostic-only WGC selected-monitor one-frame evidence; selected PNG/readback evidence does not prove output effects, Alt+A routing, or readiness",
        })
    });
    let frame = report.frame.as_ref().map(|frame| {
        serde_json::json!({
            "contractVersion": frame.metadata.contract_version,
            "source": debug_value(frame.metadata.source),
            "width": frame.metadata.width,
            "height": frame.metadata.height,
            "format": debug_value(frame.metadata.format),
            "frameId": frame.metadata.frame_id,
            "captureTimestamp100ns": frame.metadata.capture_timestamp_100ns,
            "hasTextureHandle": frame.metadata.texture.is_some(),
            "sharedHandleKind": debug_value(frame.metadata.shared_handle.kind),
            "hasSharedHandle": frame.metadata.shared_handle.handle.is_some(),
            "stagingReadbackPresent": frame.metadata.staging_readback.is_some(),
            "readbackBytesPresent": frame.readback_bytes.is_some(),
            "readbackByteLen": frame.readback_bytes.as_ref().map(Vec::len),
        })
    });
    serde_json::json!({
        "state": report.state.as_str(),
        "attemptedRealWgcApi": report.attempted_real_wgc_api,
        "createdDevice": report.created_device,
        "createdItem": report.created_item,
        "createdFramePool": report.created_frame_pool,
        "createdSession": report.created_session,
        "startedCapture": report.started_capture,
        "acquiredFrame": report.acquired_frame,
        "frameId": report.frame_id,
        "width": report.width,
        "height": report.height,
        "elapsedMs": report.elapsed_ms,
        "frameFormat": frame_format,
        "frameDimensionsMatchSession": frame_dimensions_match_session,
        "selectedMonitorFrameEvidence": {
            "diagnosticOnly": evidence.diagnostic_only,
            "requestedBoundsPhysical": evidence.requested_bounds_physical.map(screenshot_physical_bounds_json),
            "targetMonitorBoundsPhysical": evidence.target_monitor_bounds_physical.map(screenshot_physical_bounds_json),
            "framepoolSizeSource": evidence.framepool_size_source,
            "frameWidth": evidence.frame_width,
            "frameHeight": evidence.frame_height,
            "frameMatchesTargetMonitorBounds": evidence.frame_matches_target_monitor_bounds,
            "selectedCropWithinFrame": evidence.selected_crop_within_frame,
            "selectedPngProduced": evidence.selected_png_produced,
            "readbackBytesPresent": evidence.readback_bytes_present,
            "persistentHandleExposed": evidence.persistent_handle_exposed,
            "readinessChanged": evidence.readiness_changed,
        },
        "selectedFrameEvidence": selected_frame_evidence,
        "selectedPngEvidence": selected_png_evidence,
        "selectedPngProduced": evidence.selected_png_produced,
        "diagnosticOnly": true,
        "persistentHandleExposed": false,
        "readinessChanged": false,
        "altAChanged": false,
        "frame": frame,
        "error": report.error.as_ref().map(ToString::to_string),
    })
}

pub(crate) fn merge_wgc_session_fake_sink_acceptance(
    mut session: serde_json::Value,
    acceptance: serde_json::Value,
) -> serde_json::Value {
    if let Some(object) = session.as_object_mut() {
        object.insert("selectedOutputFakeSinkAcceptance".to_string(), acceptance);
    }
    session
}

pub(crate) fn dxgi_desktop_coordinates_json(
    coordinates: crate::screenshot_native::dxgi_output::DxgiDesktopCoordinates,
) -> serde_json::Value {
    serde_json::json!({
        "left": coordinates.left,
        "top": coordinates.top,
        "right": coordinates.right,
        "bottom": coordinates.bottom,
        "bounds": coordinates.bounds().map(screenshot_physical_bounds_json),
    })
}

pub(crate) fn image_bounds_json(
    bounds: crate::screenshot_native::ImageBounds,
) -> serde_json::Value {
    serde_json::json!({
        "width": bounds.width,
        "height": bounds.height,
    })
}

pub(crate) fn selection_rect_json(
    rect: crate::screenshot_native::SelectionRect,
) -> serde_json::Value {
    serde_json::json!({
        "x": rect.x,
        "y": rect.y,
        "width": rect.width,
        "height": rect.height,
    })
}

pub(crate) fn crop_rect_json(crop: crate::screenshot_native::CropRect) -> serde_json::Value {
    serde_json::json!({
        "x": crop.x,
        "y": crop.y,
        "width": crop.width,
        "height": crop.height,
    })
}

pub(crate) fn selected_png_evidence_json(
    image: Option<&crate::screenshot_native::SelectedImageContract>,
) -> Option<serde_json::Value> {
    image.map(|image| {
        serde_json::json!({
            "pngWidth": image.crop.width,
            "pngHeight": image.crop.height,
            "pngByteLen": image.byte_len(),
            "pngFingerprint": bytes_fingerprint(&image.png_bytes),
            "sourceWidth": image.source_width,
            "sourceHeight": image.source_height,
            "crop": crop_rect_json(image.crop),
            "selectedOnlyPng": image.is_selected_only_png(),
            "dimensionsMatchCrop": !image.crop.is_empty(),
            "decodedRgbaByteLenExpected": image.crop.rgba_byte_len(),
        })
    })
}

pub(crate) fn wgc_fake_sink_acceptance_json(
    receipt: Option<&crate::screenshot_native::wgc_selected_output_acceptance::WgcSelectedOutputFakeSinkAcceptanceReceipt>,
    error: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "ok": receipt.map(|receipt| receipt.proves_fake_sink_copy()).unwrap_or(false),
        "source": receipt.map(|receipt| receipt.source),
        "diagnosticOnly": true,
        "readinessChanged": false,
        "altAChanged": false,
        "persistentHandleExposed": false,
        "wgcSelectedPngEvidencePresent": receipt.map(|receipt| receipt.wgc_selected_png_evidence_present).unwrap_or(false),
        "fakeSinkCopyAccepted": receipt.map(|receipt| receipt.fake_sink_copy_accepted).unwrap_or(false),
        "sink": receipt.map(|receipt| receipt.sink),
        "sinkCalls": receipt.map(|receipt| receipt.sink_calls).unwrap_or(0),
        "selectedOnlyPng": receipt.map(|receipt| receipt.selected_only_png).unwrap_or(false),
        "pngByteLen": receipt.map(|receipt| receipt.png_byte_len).unwrap_or(0),
        "copiedPngByteLen": receipt.map(|receipt| receipt.copied_png_byte_len).unwrap_or(0),
        "effect": receipt.map(|receipt| serde_json::json!({
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
        })),
        "error": error,
        "scope": "diagnostic-only WGC selected-output fake-sink acceptance; proves selected PNG can flow through injected copy sink only and does not change Alt+A, readiness, presenter, OCR, translation, or real clipboard behavior",
    })
}

pub(crate) fn physical_overflow_json(
    overflow: crate::screenshot_native::PhysicalOverflowPixels,
) -> serde_json::Value {
    serde_json::json!({
        "left": overflow.left,
        "top": overflow.top,
        "right": overflow.right,
        "bottom": overflow.bottom,
    })
}

pub(crate) fn selected_readback_plan_json(
    plan: &crate::screenshot_native::SelectedReadbackPlan,
    target_bounds_source: &str,
) -> serde_json::Value {
    serde_json::json!({
        "diagnosticOnly": plan.diagnostic_only,
        "readinessChanged": plan.readiness_changed,
        "backend": plan.backend.as_str(),
        "status": "planned",
        "requestedBoundsPhysical": screenshot_physical_bounds_json(plan.requested_bounds_physical),
        "targetBoundsPhysical": {
            "known": true,
            "source": target_bounds_source,
            "bounds": screenshot_physical_bounds_json(plan.target_bounds_physical),
        },
        "outputFrameBounds": image_bounds_json(plan.output_frame_bounds),
        "mapping": {
            "status": "planned",
            "desktopSelection": selection_rect_json(plan.mapping.desktop_selection),
            "monitorLocalSelection": selection_rect_json(plan.mapping.monitor_local_selection),
            "crop": crop_rect_json(plan.mapping.crop),
            "wasClampedToMonitorOrFrame": plan.mapping.was_clamped_to_monitor,
            "frameMatchesMonitorBounds": plan.mapping.frame_matches_monitor_bounds,
        },
        "cropOverflowPhysical": physical_overflow_json(plan.crop_overflow_physical),
        "cropWithinTargetMonitor": plan.crop_within_target(),
        "requestedTargetIntersectionRatio": plan.requested_target_intersection_ratio,
        "framepool": {
            "requestedSize": image_bounds_json(plan.session_frame_pool_requested_size),
            "source": "target-monitor-bounds",
            "matchesRequestedBounds": plan.session_frame_pool_requested_size.width == plan.requested_bounds_physical.width
                && plan.session_frame_pool_requested_size.height == plan.requested_bounds_physical.height,
            "matchesTargetBounds": plan.frame_pool_matches_capture_item,
        },
        "captureItemExpectedSize": image_bounds_json(plan.capture_item_expected_size),
        "mismatches": {
            "requestedDiffersFromTargetBounds": plan.requested_bounds_physical != plan.target_bounds_physical,
            "framepoolDiffersFromTargetBounds": !plan.frame_pool_matches_capture_item,
            "selectedCropRequiresFullMonitorCapture": plan.requested_bounds_physical != plan.target_bounds_physical,
            "frameDiffersFromTargetBounds": !plan.mapping.frame_matches_monitor_bounds,
        },
        "selectedOutputReadyPlanningOnly": plan.selected_output_ready(),
        "scope": "diagnostic-only selected readback planning; does not prove WGC/DXGI readback, presenter, Alt+A readiness, or output side effects"
    })
}

pub(crate) fn selected_readback_plan_error_json(
    backend: crate::screenshot_native::SelectedReadbackPlanBackend,
    requested_bounds: crate::screenshot_native::MonitorCaptureBounds,
    target_bounds_source: &str,
    error_code: &str,
    error_message: String,
) -> serde_json::Value {
    serde_json::json!({
        "diagnosticOnly": true,
        "readinessChanged": false,
        "backend": backend.as_str(),
        "status": "failed",
        "requestedBoundsPhysical": screenshot_physical_bounds_json(requested_bounds),
        "targetBoundsPhysical": {
            "known": false,
            "source": target_bounds_source,
            "bounds": serde_json::Value::Null,
        },
        "errorCode": error_code,
        "error": error_message,
        "selectedOutputReadyPlanningOnly": false,
        "scope": "diagnostic-only selected readback planning failure; does not prove WGC/DXGI readback, presenter, Alt+A readiness, or output side effects"
    })
}

pub(crate) fn build_wgc_selected_readback_plan_json(
    requested_bounds: crate::screenshot_native::MonitorCaptureBounds,
    _target_report: &crate::screenshot_native::wgc_target::WgcTargetValidationReport,
    validation_report: Option<&crate::screenshot_native::wgc_target::WgcTargetValidationReport>,
) -> serde_json::Value {
    let Some((target_bounds_source, target_bounds)) =
        resolved_wgc_target_bounds(_target_report, validation_report)
    else {
        return selected_readback_plan_error_json(
            crate::screenshot_native::SelectedReadbackPlanBackend::WgcMonitor,
            requested_bounds,
            "unavailable-current-target-resolution",
            "target-bounds-unavailable",
            "selected readback planning requires validated target monitor bounds".to_string(),
        );
    };
    match crate::screenshot_native::plan_selected_readback_from_desktop_bounds(
        crate::screenshot_native::SelectedReadbackPlanBackend::WgcMonitor,
        requested_bounds,
        target_bounds,
        image_bounds_from_monitor_bounds(target_bounds),
    ) {
        Ok(plan) => selected_readback_plan_json(&plan, target_bounds_source),
        Err(error) => selected_readback_plan_error_json(
            crate::screenshot_native::SelectedReadbackPlanBackend::WgcMonitor,
            requested_bounds,
            target_bounds_source,
            "selected-readback-plan-failed",
            error.to_string(),
        ),
    }
}

pub(crate) fn resolved_wgc_target_bounds(
    target_report: &crate::screenshot_native::wgc_target::WgcTargetValidationReport,
    validation_report: Option<&crate::screenshot_native::wgc_target::WgcTargetValidationReport>,
) -> Option<(&'static str, crate::screenshot_native::MonitorCaptureBounds)> {
    validation_report
        .and_then(|report| {
            report
                .bounds
                .map(|bounds| ("validated-target-monitor-bounds", bounds))
        })
        .or_else(|| {
            target_report
                .bounds
                .map(|bounds| ("resolved-target-monitor-bounds", bounds))
        })
}

pub(crate) fn image_bounds_from_monitor_bounds(
    bounds: crate::screenshot_native::MonitorCaptureBounds,
) -> crate::screenshot_native::ImageBounds {
    crate::screenshot_native::ImageBounds::new(bounds.width, bounds.height)
}

pub(crate) use crate::screenshot_dxgi_diagnostics_json::{
    dxgi_acquire_path_json, dxgi_frame_info_probe_path_json, dxgi_output_ranking_json,
    dxgi_pulse_before_acquire_path_json, dxgi_pulse_before_acquire_report_json,
};

pub(crate) use crate::screenshot_win32_diagnostics_json::{
    cursor_nudge_report_json, desktop_update_pulse_report_json,
};

pub(crate) fn comparison_frame_confirmed(value: &serde_json::Value, path: &str) -> bool {
    value[path]["frameCaptureConfirmed"]
        .as_bool()
        .unwrap_or(false)
}

pub(crate) fn comparison_both_timed_out(value: &serde_json::Value) -> bool {
    value["comparison"]["bothTimedOut"]
        .as_bool()
        .unwrap_or(false)
}

pub(crate) fn add_wgc_smoke_safety_fields(
    mut response: serde_json::Value,
    scope: &str,
) -> serde_json::Value {
    if let Some(object) = response.as_object_mut() {
        object.insert(
            "scope".to_string(),
            serde_json::Value::String(scope.to_string()),
        );
        object.insert("diagnosticOnly".to_string(), serde_json::Value::Bool(true));
        object.insert(
            "persistentHandleExposed".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "readinessChanged".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "attemptedRealWgcApi".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "frameCaptureAttempted".to_string(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "frameCaptureConfirmed".to_string(),
            serde_json::Value::Bool(false),
        );
        object
            .entry("session".to_string())
            .or_insert(serde_json::Value::Null);
        object
            .entry("selectedReadbackPlan".to_string())
            .or_insert(serde_json::Value::Null);
    }
    response
}
