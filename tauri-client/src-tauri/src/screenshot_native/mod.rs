pub mod candidates;
pub mod capability;
pub mod capture;
pub mod capture_router;
pub mod d3d11_frame;
pub mod dxgi_capture;
#[cfg(test)]
mod dxgi_capture_tests;
pub mod dxgi_frame_info_probe;
pub mod dxgi_output;
pub mod dxgi_output_bridge_plan;
pub mod dxgi_output_bridge_smoke;
pub mod dxgi_presenter_smoke;
pub mod dxgi_probe;
pub mod dxgi_pulse_before_acquire_probe;
pub mod dxgi_readback;
pub mod dxgi_selected_output_acceptance;
pub mod dxgi_session;
pub mod dxgi_smoke;
pub mod dxgi_texture;
pub mod focus;
pub mod gpu;
pub mod gpu_device;
pub mod input;
pub mod monitor_output_mapping;
#[cfg(test)]
mod monitor_output_mapping_tests;
pub mod native_input_smoke;
pub mod native_overlay_session;
pub mod native_overlay_smoke;
pub mod native_route_readiness;
pub mod output;
pub mod overlay;
pub mod overlay_renderer;
pub mod presenter;
pub mod selected_image_bridge;
pub(crate) mod selected_output_clipboard;
pub mod selected_output_effects;
pub mod selected_readback_plan;
#[cfg(test)]
mod selected_readback_plan_tests;
pub mod selection_state;
pub mod session;
pub mod wgc_capture;
pub mod wgc_contract;
pub mod wgc_device;
#[cfg(test)]
mod wgc_monitor_session_live_smoke;
pub mod wgc_probe;
pub mod wgc_readback;
pub mod wgc_selected_output_acceptance;
pub mod wgc_session;
#[cfg(test)]
mod wgc_session_tests;
pub mod wgc_session_types;
pub mod wgc_target;
#[cfg(test)]
mod wgc_target_tests;
pub mod win32_cursor;
pub mod win32_desktop_update_pulse;
pub mod win32_input;
pub mod win32_overlay;
pub mod win32_overlay_dispatch;
pub mod win32_overlay_input;
pub mod win32_overlay_pump;

pub use candidates::{CandidateKind, SelectionCandidate};
pub use capability::{
    default_native_overlay_capability, default_native_overlay_launch_plan,
    resolve_native_overlay_launch_plan, resolve_native_overlay_launch_plan_with_route,
    NativeOverlayCapabilityFlag, NativeOverlayFallbackReason, NativeOverlayLaunchPlan,
    ScreenshotOverlayRuntime,
};
pub use capture::{
    CaptureBackendContract, CaptureBackendKind, CaptureError, CaptureFrameSource,
    CapturePixelFormat, CaptureReadbackMode, CaptureResult, MonitorCaptureBounds, RgbaFrame,
};
pub use capture_router::{
    default_capture_route_decision, resolve_capture_route, resolve_diagnostics_capture_route,
    CaptureDiagnosticsRouteOptions, CaptureRouteDecision, CaptureRouteFallbackReason,
    CaptureRouteOptions, CaptureRoutePreference, CaptureRouteStatus,
};
#[cfg(windows)]
pub use dxgi_readback::{
    build_selected_png_contract_from_dxgi_texture, readback_dxgi_d3d11_texture_2d_region_rgba,
    DxgiD3d11SelectedRegionReadbackSource,
};
pub use focus::{FocusRestorePolicy, FocusSnapshot};
pub use gpu::{
    GpuCapabilityRequirement, GpuCaptureBackend, GpuCaptureCapability, GpuCaptureFallback,
    GpuCapturePlan, GpuCaptureStatus, GpuTextureInterop,
};
pub use input::ScreenshotInputEvent;
pub use monitor_output_mapping::{
    map_desktop_selection_to_output_frame, MonitorOutputSelectionMapping,
    MonitorOutputSelectionMappingError,
};
pub use native_overlay_session::{
    begin_cpu_native_overlay_session, cancel_cpu_native_overlay_session,
    cancel_cpu_native_overlay_session_if_matches, cleanup_stale_cpu_native_overlay_session,
    cpu_native_overlay_selection_snapshot, cpu_native_overlay_session_diagnostics,
    raise_cpu_native_overlay_session, NativeOverlaySelectionSnapshot,
    NativeOverlaySessionDiagnostics, NativeOverlaySessionError, NativeOverlaySessionRuntime,
    NativeOverlaySessionState,
};
pub use native_route_readiness::{
    default_native_route_readiness, resolve_native_route_readiness, NativeRouteReadinessBlocker,
    NativeRouteReadinessDecision, NativeRouteReadinessInputs, NativeScreenshotMainRoute,
};
pub use output::{
    ClampedSelectionRect, CropRect, ImageBounds, OutputAction, OutputBridgeContract,
    OutputBridgeTarget, OutputImageFormat, SelectedImageContract, SelectionRect,
};
pub use overlay::{NativeOverlayOptions, NativeOverlayState};
pub use overlay_renderer::{
    OverlayRenderError, OverlayRenderReceipt, OverlayRenderTarget, OverlayRendererContract,
};
pub use presenter::{PresentationFrame, PresenterKind};
pub use selected_readback_plan::{
    plan_selected_readback_from_desktop_bounds, PhysicalOverflowPixels, SelectedReadbackPlan,
    SelectedReadbackPlanBackend, SelectedReadbackPlanError,
};
pub use selection_state::{SelectionState, SelectionStateStatus, SelectionTransition};
pub use session::{
    advance_run_generation, begin_run_generation, generation_state, is_stale_generation,
    next_screenshot_session_id, ScreenshotGenerationState, ScreenshotRunGeneration,
};
pub use win32_overlay::{
    create_win32_overlay, destroy_win32_overlay, hide_win32_overlay, set_win32_overlay_bitmap,
    set_win32_overlay_candidate, set_win32_overlay_selection, show_win32_overlay,
    Win32OverlayConfig, Win32OverlayError, Win32OverlayHandle, Win32OverlayLifecycleState,
    Win32OverlaySelectionRect, Win32OverlayWindow,
};
pub use win32_overlay_dispatch::{
    input_event_label, run_win32_overlay_message_tuple_diagnostic_pump, Win32OverlayWaitResult,
};
pub use win32_overlay_input::{
    win32_overlay_native_input_snapshot, win32_overlay_native_input_started,
    Win32OverlayNativeInputPhase, Win32OverlayNativeInputSnapshot,
};
pub use win32_overlay_pump::{
    classify_pump_event, run_win32_overlay_diagnostic_pump, win32_overlay_pump_contract,
    Win32OverlayPumpContract, Win32OverlayPumpDiagnostics, Win32OverlayPumpExitReason,
    Win32OverlayPumpOptions,
};
