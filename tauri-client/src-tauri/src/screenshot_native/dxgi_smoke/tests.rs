use super::super::capture::MonitorCaptureBounds;
use super::*;

#[test]
fn smoke_stage_labels_are_stable() {
    assert_eq!(
        DxgiTextureSmokeStage::TextureAcquired.as_str(),
        "texture-acquired"
    );
    assert_eq!(DxgiTextureSmokeStage::Stopped.as_str(), "stopped");
    assert_eq!(
        DxgiSelectedReadbackSmokeStage::SelectedReadback.as_str(),
        "selected-readback"
    );
}

#[test]
fn selected_readback_smoke_rejects_empty_bounds_without_attempting_runtime() {
    let report = run_dxgi_selected_readback_smoke(MonitorCaptureBounds::new(0, 0, 0, 1));
    if cfg!(windows) {
        assert!(report.attempted);
    } else {
        assert!(!report.attempted);
    }
    assert!(!report.ok);
    assert!(!report.selected_only);
    assert!(!report.png_signature_valid);
}

#[test]
#[ignore = "requires a real Windows desktop duplication session"]
fn dxgi_acquire_one_texture_frame_smoke() {
    let report = run_dxgi_texture_acquisition_smoke(MonitorCaptureBounds::new(0, 0, 1, 1));
    println!("{report:#?}");
    assert!(report.attempted);
    if std::env::var("YSN_REQUIRE_DXGI_TEXTURE_SMOKE")
        .ok()
        .as_deref()
        == Some("1")
    {
        assert!(report.ok, "DXGI texture smoke failed: {report:#?}");
    }
    if report.ok {
        assert!(report.released_frame);
        assert!(report.stopped);
        assert!(report.width.unwrap_or_default() > 0);
        assert!(report.height.unwrap_or_default() > 0);
    }
}

#[test]
#[ignore = "requires a real Windows desktop duplication session"]
fn dxgi_selected_readback_smoke() {
    let report = run_dxgi_selected_readback_smoke(MonitorCaptureBounds::new(0, 0, 1, 1));
    println!("{report:#?}");
    assert!(report.attempted);
    if std::env::var("YSN_REQUIRE_DXGI_SELECTED_READBACK_SMOKE")
        .ok()
        .as_deref()
        == Some("1")
    {
        assert!(
            report.ok,
            "DXGI selected readback smoke failed: {report:#?}"
        );
    }
    if report.ok {
        assert!(report.selected_only);
        assert!(report.bounded_crop_valid);
        assert!(report.copy_subresource_region);
        assert!(report.bgra_to_rgba);
        assert!(report.png_signature_valid);
        assert!(report.released_frame);
        assert!(report.stopped);
    }
}
