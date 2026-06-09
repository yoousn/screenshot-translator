use crate::screenshot_diagnostics_json::{
    desktop_update_pulse_report_json, screenshot_physical_bounds_json,
};

pub(crate) fn dxgi_acquire_path_json(
    path: &str,
    attempted: bool,
    ok: bool,
    stage: &str,
    elapsed_ms: u128,
    frame_id: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    format: Option<String>,
    output_bounds: Option<crate::screenshot_native::MonitorCaptureBounds>,
    adapter_index: Option<u32>,
    output_index: Option<u32>,
    released_frame: bool,
    stopped: bool,
    error: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "path": path,
        "attempted": attempted,
        "ok": ok,
        "stage": stage,
        "elapsedMs": elapsed_ms,
        "frameId": frame_id,
        "width": width,
        "height": height,
        "format": format,
        "outputBounds": output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": adapter_index,
        "outputIndex": output_index,
        "frameCaptureAttempted": attempted,
        "frameCaptureConfirmed": frame_id.is_some(),
        "releasedFrame": released_frame,
        "stopped": stopped,
        "error": error,
    })
}

pub(crate) fn dxgi_frame_info_json(
    frame_info: Option<crate::screenshot_native::dxgi_session::DxgiOutduplFrameInfoDiagnostics>,
) -> serde_json::Value {
    frame_info
        .map(|info| {
            serde_json::json!({
                "lastPresentTimeQpc": info.last_present_time_qpc,
                "lastMouseUpdateTimeQpc": info.last_mouse_update_time_qpc,
                "accumulatedFrames": info.accumulated_frames,
                "rectsCoalesced": info.rects_coalesced,
                "protectedContentMaskedOut": info.protected_content_masked_out,
                "pointerPosition": {
                    "visible": info.pointer_position.visible,
                    "x": info.pointer_position.position_x,
                    "y": info.pointer_position.position_y,
                },
                "totalMetadataBufferSize": info.total_metadata_buffer_size,
                "pointerShapeBufferSize": info.pointer_shape_buffer_size,
            })
        })
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn dxgi_frame_info_probe_attempt_json(
    attempt: &crate::screenshot_native::dxgi_frame_info_probe::DxgiFrameInfoProbeAttempt,
) -> serde_json::Value {
    serde_json::json!({
        "attempt": attempt.attempt,
        "timeoutMs": attempt.timeout_ms,
        "elapsedBudgetMs": attempt.elapsed_budget_ms,
        "ok": attempt.ok,
        "timedOut": attempt.timed_out,
        "accessLost": attempt.access_lost,
        "hresultHex": attempt.hresult_hex,
        "error": attempt.error,
        "frameInfo": dxgi_frame_info_json(attempt.frame_info),
        "releasedFrame": attempt.released_frame,
    })
}

pub(crate) fn dxgi_output_ranking_json(
    evidence: Option<&crate::screenshot_native::dxgi_output::DxgiOutputRankingEvidence>,
) -> serde_json::Value {
    let Some(evidence) = evidence else {
        return serde_json::Value::Null;
    };
    serde_json::json!({
        "policyVersion": evidence.policy_version,
        "rankingPolicy": evidence.policy,
        "requestedBounds": screenshot_physical_bounds_json(evidence.requested_bounds),
        "selectionCenter": evidence.selection_center.map(|(x, y)| serde_json::json!({
            "x": x,
            "y": y,
        })),
        "candidateCount": evidence.candidate_count,
        "selectedRank": evidence.selected_rank,
        "selectedOutput": evidence.selected_output.map(|candidate| serde_json::json!({
            "adapterIndex": candidate.adapter_index,
            "outputIndex": candidate.output_index,
            "desktopBounds": screenshot_physical_bounds_json(candidate.desktop_bounds),
        })),
        "rankedOutputs": evidence.ranked_outputs.iter().map(|output| serde_json::json!({
            "rank": output.rank,
            "adapterIndex": output.candidate.adapter_index,
            "outputIndex": output.candidate.output_index,
            "desktopBounds": screenshot_physical_bounds_json(output.candidate.desktop_bounds),
            "intersectionBounds": output.intersection_bounds.map(screenshot_physical_bounds_json),
            "intersectionArea": output.intersection_area,
            "intersectionRatio": output.intersection_ratio,
            "containsSelectionCenter": output.contains_selection_center,
            "selectable": output.selectable,
            "selected": output.selected,
            "rejectionReason": output.rejection_reason,
        })).collect::<Vec<_>>(),
        "persistentHandleExposed": evidence.persistent_handle_exposed,
    })
}

pub(crate) fn dxgi_frame_info_probe_path_json(
    path: &crate::screenshot_native::dxgi_frame_info_probe::DxgiFrameInfoProbePathReport,
) -> serde_json::Value {
    serde_json::json!({
        "path": path.path.as_str(),
        "attempted": path.attempted,
        "ok": path.ok,
        "outputBounds": path.output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": path.adapter_index,
        "outputIndex": path.output_index,
        "outputRanking": dxgi_output_ranking_json(path.output_ranking.as_ref()),
        "attempts": path.attempts.iter().map(dxgi_frame_info_probe_attempt_json).collect::<Vec<_>>(),
        "stopped": path.stopped,
        "error": path.error,
    })
}

pub(crate) fn dxgi_pulse_before_acquire_path_json(
    path: &crate::screenshot_native::dxgi_pulse_before_acquire_probe::DxgiPulseBeforeAcquirePathReport,
) -> serde_json::Value {
    serde_json::json!({
        "path": path.path.as_str(),
        "attempted": path.attempted,
        "ok": path.ok,
        "outputBounds": path.output_bounds.map(screenshot_physical_bounds_json),
        "adapterIndex": path.adapter_index,
        "outputIndex": path.output_index,
        "outputRanking": dxgi_output_ranking_json(path.output_ranking.as_ref()),
        "pulse": path.pulse.clone().map(desktop_update_pulse_report_json),
        "attempts": path.attempts.iter().map(dxgi_frame_info_probe_attempt_json).collect::<Vec<_>>(),
        "acquire": path.acquire.as_ref().map(dxgi_frame_info_probe_attempt_json),
        "stopped": path.stopped,
        "error": path.error,
    })
}

pub(crate) fn dxgi_pulse_before_acquire_report_json(
    report: &crate::screenshot_native::dxgi_pulse_before_acquire_probe::DxgiPulseBeforeAcquireProbeReport,
) -> serde_json::Value {
    let default_frame_confirmed = report
        .default_output
        .acquire
        .as_ref()
        .map(|attempt| attempt.ok)
        .unwrap_or(false);
    let selected_frame_confirmed = report
        .selected_output
        .acquire
        .as_ref()
        .map(|attempt| attempt.ok)
        .unwrap_or(false);
    let default_timed_out = report
        .default_output
        .attempts
        .iter()
        .any(|attempt| attempt.timed_out);
    let selected_timed_out = report
        .selected_output
        .attempts
        .iter()
        .any(|attempt| attempt.timed_out);
    let default_attempts = report.default_output.attempts.len();
    let selected_attempts = report.selected_output.attempts.len();
    let default_success_attempt = report
        .default_output
        .acquire
        .as_ref()
        .map(|attempt| attempt.attempt);
    let selected_success_attempt = report
        .selected_output
        .acquire
        .as_ref()
        .map(|attempt| attempt.attempt);
    serde_json::json!({
        "attempted": report.attempted,
        "ok": report.ok,
        "requestedBounds": screenshot_physical_bounds_json(report.requested_bounds),
        "pulseSizePx": report.pulse_size_px,
        "pulseAlpha": report.pulse_alpha,
        "dwellMs": report.dwell_ms,
        "defaultOutput": dxgi_pulse_before_acquire_path_json(&report.default_output),
        "selectedOutput": dxgi_pulse_before_acquire_path_json(&report.selected_output),
        "comparison": {
            "defaultFrameConfirmed": default_frame_confirmed,
            "selectedFrameConfirmed": selected_frame_confirmed,
            "anyFrameConfirmed": default_frame_confirmed || selected_frame_confirmed,
            "bothTimedOut": default_timed_out && selected_timed_out,
            "defaultTimedOut": default_timed_out,
            "selectedTimedOut": selected_timed_out,
            "defaultAttemptCount": default_attempts,
            "selectedAttemptCount": selected_attempts,
            "defaultSuccessAttempt": default_success_attempt,
            "selectedSuccessAttempt": selected_success_attempt,
        },
        "error": report.error,
    })
}
