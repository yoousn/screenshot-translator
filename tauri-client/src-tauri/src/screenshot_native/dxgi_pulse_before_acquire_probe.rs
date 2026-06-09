use super::dxgi_frame_info_probe::{DxgiFrameInfoProbeAttempt, DxgiFrameInfoProbePath};
use super::win32_desktop_update_pulse::{
    run_desktop_update_pulse, DesktopUpdatePulseReport, DesktopUpdatePulseRequest,
};
use super::MonitorCaptureBounds;

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiPulseBeforeAcquirePathReport {
    pub path: DxgiFrameInfoProbePath,
    pub attempted: bool,
    pub ok: bool,
    pub output_bounds: Option<MonitorCaptureBounds>,
    pub adapter_index: Option<u32>,
    pub output_index: Option<u32>,
    pub output_ranking: Option<super::dxgi_output::DxgiOutputRankingEvidence>,
    pub pulse: Option<DesktopUpdatePulseReport>,
    pub attempts: Vec<DxgiFrameInfoProbeAttempt>,
    pub acquire: Option<DxgiFrameInfoProbeAttempt>,
    pub stopped: bool,
    pub error: Option<String>,
}

impl DxgiPulseBeforeAcquirePathReport {
    fn failed_before_start(path: DxgiFrameInfoProbePath, error: impl ToString) -> Self {
        Self {
            path,
            attempted: false,
            ok: false,
            output_bounds: None,
            adapter_index: None,
            output_index: None,
            output_ranking: None,
            pulse: None,
            attempts: Vec::new(),
            acquire: None,
            stopped: false,
            error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiPulseBeforeAcquireProbeReport {
    pub attempted: bool,
    pub ok: bool,
    pub requested_bounds: MonitorCaptureBounds,
    pub pulse_size_px: u32,
    pub pulse_alpha: u8,
    pub dwell_ms: u64,
    pub default_output: DxgiPulseBeforeAcquirePathReport,
    pub selected_output: DxgiPulseBeforeAcquirePathReport,
    pub error: Option<String>,
}

pub fn run_dxgi_pulse_before_acquire_probe(
    bounds: MonitorCaptureBounds,
    pulse_size_px: u32,
    pulse_alpha: u8,
    dwell_ms: u64,
) -> DxgiPulseBeforeAcquireProbeReport {
    if bounds.is_empty() {
        let error =
            format!("DXGI pulse-before-acquire probe requires non-empty bounds: {bounds:?}");
        return DxgiPulseBeforeAcquireProbeReport {
            attempted: false,
            ok: false,
            requested_bounds: bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
            default_output: DxgiPulseBeforeAcquirePathReport::failed_before_start(
                DxgiFrameInfoProbePath::DefaultOutput,
                &error,
            ),
            selected_output: DxgiPulseBeforeAcquirePathReport::failed_before_start(
                DxgiFrameInfoProbePath::SelectedOutput,
                &error,
            ),
            error: Some(error),
        };
    }

    #[cfg(not(windows))]
    {
        let error = "DXGI pulse-before-acquire probe requires Windows";
        DxgiPulseBeforeAcquireProbeReport {
            attempted: false,
            ok: false,
            requested_bounds: bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
            default_output: DxgiPulseBeforeAcquirePathReport::failed_before_start(
                DxgiFrameInfoProbePath::DefaultOutput,
                error,
            ),
            selected_output: DxgiPulseBeforeAcquirePathReport::failed_before_start(
                DxgiFrameInfoProbePath::SelectedOutput,
                error,
            ),
            error: Some(error.to_string()),
        }
    }

    #[cfg(windows)]
    {
        let default_output = run_path(
            DxgiFrameInfoProbePath::DefaultOutput,
            bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
        );
        let selected_output = run_path(
            DxgiFrameInfoProbePath::SelectedOutput,
            bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
        );
        let ok = default_output.ok || selected_output.ok;
        DxgiPulseBeforeAcquireProbeReport {
            attempted: true,
            ok,
            requested_bounds: bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
            default_output,
            selected_output,
            error: if ok {
                None
            } else {
                Some("DXGI pulse-before-acquire probe did not acquire a frame".to_string())
            },
        }
    }
}

#[cfg(windows)]
fn run_path(
    path: DxgiFrameInfoProbePath,
    bounds: MonitorCaptureBounds,
    pulse_size_px: u32,
    pulse_alpha: u8,
    dwell_ms: u64,
) -> DxgiPulseBeforeAcquirePathReport {
    use super::dxgi_capture::DxgiDesktopDuplicationBackend;

    const ACQUIRE_BUDGET_MS: u32 = 1_000;
    const ATTEMPT_TIMEOUT_MS: u32 = 100;

    let mut backend = DxgiDesktopDuplicationBackend::new();
    let start_result = match path {
        DxgiFrameInfoProbePath::DefaultOutput => backend.start(),
        DxgiFrameInfoProbePath::SelectedOutput => backend.start_for_bounds(bounds),
    };
    if let Err(error) = start_result {
        return DxgiPulseBeforeAcquirePathReport {
            path,
            attempted: true,
            ok: false,
            output_bounds: backend.output_bounds(),
            adapter_index: backend.output_identity().map(|identity| identity.0),
            output_index: backend.output_identity().map(|identity| identity.1),
            output_ranking: backend.output_ranking().cloned(),
            pulse: None,
            attempts: Vec::new(),
            acquire: None,
            stopped: false,
            error: Some(error.to_string()),
        };
    }

    let output_bounds = backend.output_bounds();
    let output_identity = backend.output_identity();
    let output_ranking = backend.output_ranking().cloned();
    let pulse_bounds = output_bounds.unwrap_or(bounds);
    let pulse = run_desktop_update_pulse(DesktopUpdatePulseRequest::new(
        pulse_bounds,
        pulse_size_px,
        pulse_alpha,
        dwell_ms,
    ));
    let attempts = if pulse.ok {
        run_attempts(
            backend.native_session_for_diagnostics(),
            ACQUIRE_BUDGET_MS,
            ATTEMPT_TIMEOUT_MS,
        )
    } else {
        Vec::new()
    };
    let acquire = attempts.iter().find(|attempt| attempt.ok).cloned();
    let stopped = backend.stop().is_ok();
    let ok = pulse.ok && acquire.is_some() && stopped;
    let error = if ok {
        None
    } else if !pulse.ok {
        pulse.error.clone()
    } else if !stopped {
        Some("DXGI pulse-before-acquire probe failed to stop duplication session".to_string())
    } else if attempts.iter().any(|attempt| attempt.access_lost) {
        Some("DXGI pulse-before-acquire probe lost access after pulse".to_string())
    } else {
        Some(format!(
            "DXGI pulse-before-acquire probe timed out after pulse across {} attempts and {ACQUIRE_BUDGET_MS} ms",
            attempts.len()
        ))
    };

    DxgiPulseBeforeAcquirePathReport {
        path,
        attempted: true,
        ok,
        output_bounds,
        adapter_index: output_identity.map(|identity| identity.0),
        output_index: output_identity.map(|identity| identity.1),
        output_ranking,
        pulse: Some(pulse),
        attempts,
        acquire,
        stopped,
        error,
    }
}

#[cfg(windows)]
fn run_attempts(
    session: Option<&super::dxgi_session::DxgiDuplicationSession>,
    budget_ms: u32,
    attempt_timeout_ms: u32,
) -> Vec<DxgiFrameInfoProbeAttempt> {
    let Some(session) = session else {
        return vec![DxgiFrameInfoProbeAttempt {
            attempt: 1,
            timeout_ms: 0,
            elapsed_budget_ms: 0,
            ok: false,
            timed_out: false,
            access_lost: false,
            hresult_hex: None,
            error: Some("DXGI duplication session is unavailable".to_string()),
            frame_info: None,
            released_frame: false,
        }];
    };

    let mut attempts = Vec::new();
    let mut elapsed_budget_ms = 0;
    while elapsed_budget_ms < budget_ms {
        let attempt_index = attempts.len() as u32 + 1;
        let timeout_ms = (budget_ms - elapsed_budget_ms)
            .min(attempt_timeout_ms)
            .max(1);
        let attempt = run_attempt(session, attempt_index, timeout_ms, elapsed_budget_ms);
        let should_stop = attempt.ok || attempt.access_lost;
        elapsed_budget_ms = elapsed_budget_ms.saturating_add(timeout_ms);
        attempts.push(attempt);
        if should_stop {
            break;
        }
    }
    attempts
}

#[cfg(windows)]
fn run_attempt(
    session: &super::dxgi_session::DxgiDuplicationSession,
    attempt: u32,
    timeout_ms: u32,
    elapsed_budget_ms: u32,
) -> DxgiFrameInfoProbeAttempt {
    match session.acquire_next_frame_with_info(timeout_ms) {
        Ok(frame) => {
            let released_frame = session.release_frame().is_ok();
            DxgiFrameInfoProbeAttempt {
                attempt,
                timeout_ms,
                elapsed_budget_ms,
                ok: released_frame,
                timed_out: false,
                access_lost: false,
                hresult_hex: None,
                error: if released_frame {
                    None
                } else {
                    Some(
                        "DXGI pulse-before-acquire acquired a frame but ReleaseFrame failed"
                            .to_string(),
                    )
                },
                frame_info: Some(frame.frame_info),
                released_frame,
            }
        }
        Err(error) => {
            let error_text = error.to_string();
            DxgiFrameInfoProbeAttempt {
                attempt,
                timeout_ms,
                elapsed_budget_ms,
                ok: false,
                timed_out: error_text.contains("0x887A0027") || error_text.contains("timed out"),
                access_lost: error_text.contains("DXGI_ERROR_ACCESS_LOST")
                    || error_text.contains("access was lost"),
                hresult_hex: extract_hresult_hex(&error_text),
                error: Some(error_text),
                frame_info: None,
                released_frame: false,
            }
        }
    }
}

#[cfg(windows)]
fn extract_hresult_hex(error: &str) -> Option<String> {
    let start = error.find("0x")?;
    let hex = error[start..]
        .chars()
        .take_while(|character| character.is_ascii_hexdigit() || *character == 'x')
        .collect::<String>();
    (hex.len() >= 10).then_some(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_before_acquire_rejects_empty_bounds() {
        let report =
            run_dxgi_pulse_before_acquire_probe(MonitorCaptureBounds::new(0, 0, 0, 1), 2, 1, 16);

        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("non-empty"));
    }

    #[cfg(windows)]
    #[test]
    fn extracts_hresult_inside_parentheses() {
        assert_eq!(
            extract_hresult_hex("超时值已过，资源还不可用。 (0x887A0027)"),
            Some("0x887A0027".to_string())
        );
    }
}
