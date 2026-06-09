use super::MonitorCaptureBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiFrameInfoProbePath {
    DefaultOutput,
    SelectedOutput,
}

impl DxgiFrameInfoProbePath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DefaultOutput => "default-output",
            Self::SelectedOutput => "selected-output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiFrameInfoProbeAttempt {
    pub attempt: u32,
    pub timeout_ms: u32,
    pub elapsed_budget_ms: u32,
    pub ok: bool,
    pub timed_out: bool,
    pub access_lost: bool,
    pub hresult_hex: Option<String>,
    pub error: Option<String>,
    pub frame_info: Option<super::dxgi_session::DxgiOutduplFrameInfoDiagnostics>,
    pub released_frame: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiFrameInfoProbePathReport {
    pub path: DxgiFrameInfoProbePath,
    pub attempted: bool,
    pub ok: bool,
    pub output_bounds: Option<MonitorCaptureBounds>,
    pub adapter_index: Option<u32>,
    pub output_index: Option<u32>,
    pub output_ranking: Option<super::dxgi_output::DxgiOutputRankingEvidence>,
    pub attempts: Vec<DxgiFrameInfoProbeAttempt>,
    pub stopped: bool,
    pub error: Option<String>,
}

impl DxgiFrameInfoProbePathReport {
    fn failed_before_start(path: DxgiFrameInfoProbePath, error: impl ToString) -> Self {
        Self {
            path,
            attempted: false,
            ok: false,
            output_bounds: None,
            adapter_index: None,
            output_index: None,
            output_ranking: None,
            attempts: Vec::new(),
            stopped: false,
            error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiFrameInfoProbeReport {
    pub attempted: bool,
    pub ok: bool,
    pub requested_bounds: MonitorCaptureBounds,
    pub default_output: DxgiFrameInfoProbePathReport,
    pub selected_output: DxgiFrameInfoProbePathReport,
    pub error: Option<String>,
}

pub fn run_dxgi_frame_info_probe(bounds: MonitorCaptureBounds) -> DxgiFrameInfoProbeReport {
    if bounds.is_empty() {
        let error = format!("DXGI frame-info probe requires non-empty bounds: {bounds:?}");
        return DxgiFrameInfoProbeReport {
            attempted: false,
            ok: false,
            requested_bounds: bounds,
            default_output: DxgiFrameInfoProbePathReport::failed_before_start(
                DxgiFrameInfoProbePath::DefaultOutput,
                &error,
            ),
            selected_output: DxgiFrameInfoProbePathReport::failed_before_start(
                DxgiFrameInfoProbePath::SelectedOutput,
                &error,
            ),
            error: Some(error),
        };
    }

    #[cfg(not(windows))]
    {
        let error = "DXGI frame-info probe requires Windows";
        DxgiFrameInfoProbeReport {
            attempted: false,
            ok: false,
            requested_bounds: bounds,
            default_output: DxgiFrameInfoProbePathReport::failed_before_start(
                DxgiFrameInfoProbePath::DefaultOutput,
                error,
            ),
            selected_output: DxgiFrameInfoProbePathReport::failed_before_start(
                DxgiFrameInfoProbePath::SelectedOutput,
                error,
            ),
            error: Some(error.to_string()),
        }
    }

    #[cfg(windows)]
    {
        let default_output = run_path(DxgiFrameInfoProbePath::DefaultOutput, bounds);
        let selected_output = run_path(DxgiFrameInfoProbePath::SelectedOutput, bounds);
        let ok = default_output.ok && selected_output.ok;
        DxgiFrameInfoProbeReport {
            attempted: true,
            ok,
            requested_bounds: bounds,
            default_output,
            selected_output,
            error: if ok {
                None
            } else {
                Some("one or more DXGI frame-info probe paths failed".to_string())
            },
        }
    }
}

#[cfg(windows)]
fn run_path(
    path: DxgiFrameInfoProbePath,
    bounds: MonitorCaptureBounds,
) -> DxgiFrameInfoProbePathReport {
    use super::dxgi_capture::DxgiDesktopDuplicationBackend;
    const BUDGET_MS: u32 = 500;
    const ATTEMPT_TIMEOUT_MS: u32 = 50;

    let mut backend = DxgiDesktopDuplicationBackend::new();
    let start_result = match path {
        DxgiFrameInfoProbePath::DefaultOutput => backend.start(),
        DxgiFrameInfoProbePath::SelectedOutput => backend.start_for_bounds(bounds),
    };
    if let Err(error) = start_result {
        return DxgiFrameInfoProbePathReport {
            path,
            attempted: true,
            ok: false,
            output_bounds: backend.output_bounds(),
            adapter_index: backend.output_identity().map(|identity| identity.0),
            output_index: backend.output_identity().map(|identity| identity.1),
            output_ranking: backend.output_ranking().cloned(),
            attempts: Vec::new(),
            stopped: false,
            error: Some(error.to_string()),
        };
    }

    let output_bounds = backend.output_bounds();
    let output_identity = backend.output_identity();
    let output_ranking = backend.output_ranking().cloned();
    let mut elapsed_budget = 0;
    let mut attempts = Vec::new();
    while elapsed_budget < BUDGET_MS {
        let attempt_index = attempts.len() as u32 + 1;
        let remaining = BUDGET_MS.saturating_sub(elapsed_budget);
        let timeout_ms = remaining.min(ATTEMPT_TIMEOUT_MS).max(1);
        let attempt = run_attempt(
            backend.native_session_for_diagnostics(),
            attempt_index,
            timeout_ms,
            elapsed_budget,
        );
        let ok = attempt.ok;
        attempts.push(attempt);
        elapsed_budget = elapsed_budget.saturating_add(timeout_ms);
        if ok {
            break;
        }
    }
    let stopped = backend.stop().is_ok();
    let ok = attempts.iter().any(|attempt| attempt.ok) && stopped;
    DxgiFrameInfoProbePathReport {
        path,
        attempted: true,
        ok,
        output_bounds,
        adapter_index: output_identity.map(|identity| identity.0),
        output_index: output_identity.map(|identity| identity.1),
        output_ranking,
        attempts,
        stopped,
        error: if ok {
            None
        } else {
            Some("DXGI frame-info probe did not acquire a frame within 500 ms".to_string())
        },
    }
}

#[cfg(windows)]
fn run_attempt(
    session: Option<&super::dxgi_session::DxgiDuplicationSession>,
    attempt: u32,
    timeout_ms: u32,
    elapsed_budget_ms: u32,
) -> DxgiFrameInfoProbeAttempt {
    let Some(session) = session else {
        return DxgiFrameInfoProbeAttempt {
            attempt,
            timeout_ms,
            elapsed_budget_ms,
            ok: false,
            timed_out: false,
            access_lost: false,
            hresult_hex: None,
            error: Some("DXGI duplication session is unavailable".to_string()),
            frame_info: None,
            released_frame: false,
        };
    };
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
                        "DXGI frame-info probe acquired a frame but ReleaseFrame failed"
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

fn extract_hresult_hex(error: &str) -> Option<String> {
    error
        .split(|character: char| character == '(' || character == ')' || character.is_whitespace())
        .find(|part| part.starts_with("0x") && part.len() >= 10)
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_info_probe_rejects_empty_bounds() {
        let report = run_dxgi_frame_info_probe(MonitorCaptureBounds::new(0, 0, 0, 1));
        assert!(!report.attempted);
        assert!(!report.ok);
        assert_eq!(report.default_output.path.as_str(), "default-output");
        assert_eq!(report.selected_output.path.as_str(), "selected-output");
        assert!(report.default_output.attempts.is_empty());
        assert!(report.error.unwrap().contains("non-empty bounds"));
    }

    #[test]
    fn extracts_hresult_hex_from_error_text() {
        assert_eq!(
            extract_hresult_hex("failed with code (0x887A0027)"),
            Some("0x887A0027".to_string())
        );
        assert_eq!(extract_hresult_hex("no code"), None);
    }
}
