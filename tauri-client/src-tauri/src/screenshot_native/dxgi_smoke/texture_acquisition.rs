use super::super::capture::MonitorCaptureBounds;
use super::super::dxgi_capture::DxgiDesktopDuplicationBackend;
use super::types::{DxgiTextureSmokeReport, DxgiTextureSmokeStage};
use std::time::Instant;
#[cfg(windows)]
pub fn run_dxgi_texture_acquisition_smoke(bounds: MonitorCaptureBounds) -> DxgiTextureSmokeReport {
    let started_at = Instant::now();
    let mut backend = DxgiDesktopDuplicationBackend::new();
    let mut stage = DxgiTextureSmokeStage::NotStarted;

    if let Err(error) = backend.start() {
        return DxgiTextureSmokeReport::failed(
            stage,
            started_at.elapsed().as_millis(),
            backend.session_contract().state,
            error,
        );
    }
    stage = DxgiTextureSmokeStage::Started;

    let frame = match backend.capture_texture_frame(bounds) {
        Ok(frame) => frame,
        Err(error) => {
            let stop_error = backend.stop().err().map(|error| error.to_string());
            let mut report = DxgiTextureSmokeReport::failed(
                stage,
                started_at.elapsed().as_millis(),
                backend.session_contract().state,
                error,
            );
            report.stopped = stop_error.is_none();
            if let Some(stop_error) = stop_error {
                report.error = report
                    .error
                    .map(|error| format!("{error}; stop failed: {stop_error}"));
            }
            return report;
        }
    };
    stage = DxgiTextureSmokeStage::TextureAcquired;
    let metadata = frame.metadata();

    let release_error = backend.release_acquired_frame().err();
    let released_frame = release_error.is_none();
    if released_frame {
        stage = DxgiTextureSmokeStage::FrameReleased;
    }

    let stop_error = backend.stop().err();
    let stopped = stop_error.is_none();
    if stopped {
        stage = DxgiTextureSmokeStage::Stopped;
    }

    let error = release_error
        .map(|error| format!("release failed: {error}"))
        .or_else(|| stop_error.map(|error| format!("stop failed: {error}")));

    DxgiTextureSmokeReport {
        attempted: true,
        ok: error.is_none(),
        stage,
        elapsed_ms: started_at.elapsed().as_millis(),
        frame_id: Some(metadata.frame_id),
        width: Some(metadata.width),
        height: Some(metadata.height),
        format: Some(metadata.format),
        session_state: backend.session_contract().state,
        released_frame,
        stopped,
        error,
    }
}

#[cfg(not(windows))]
pub fn run_dxgi_texture_acquisition_smoke(_bounds: MonitorCaptureBounds) -> DxgiTextureSmokeReport {
    DxgiTextureSmokeReport {
        attempted: false,
        ok: false,
        stage: DxgiTextureSmokeStage::Failed,
        elapsed_ms: 0,
        frame_id: None,
        width: None,
        height: None,
        format: None,
        session_state: DxgiDuplicationSessionState::Failed,
        released_frame: false,
        stopped: false,
        error: Some("DXGI texture smoke requires Windows".to_string()),
    }
}
