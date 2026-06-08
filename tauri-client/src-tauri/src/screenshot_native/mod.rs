pub mod candidates;
pub mod capability;
pub mod capture;
pub mod capture_router;
pub mod d3d11_frame;
pub mod dxgi_capture;
pub mod focus;
pub mod gpu;
pub mod gpu_device;
pub mod input;
pub mod native_overlay_smoke;
pub mod output;
pub mod overlay;
pub mod overlay_renderer;
pub mod presenter;
pub mod selected_image_bridge;
pub mod selection_state;
pub mod session;
pub mod wgc_probe;
pub mod win32_input;
pub mod win32_overlay;

pub use candidates::{CandidateKind, SelectionCandidate};
pub use capability::{
    default_native_overlay_capability, default_native_overlay_launch_plan,
    resolve_native_overlay_launch_plan, NativeOverlayCapabilityFlag, NativeOverlayFallbackReason,
    NativeOverlayLaunchPlan, ScreenshotOverlayRuntime,
};
pub use capture::{
    CaptureBackendContract, CaptureBackendKind, CaptureError, CaptureFrameSource,
    CapturePixelFormat, CaptureReadbackMode, CaptureResult, MonitorCaptureBounds, RgbaFrame,
};
pub use focus::{FocusRestorePolicy, FocusSnapshot};
pub use gpu::{
    GpuCapabilityRequirement, GpuCaptureBackend, GpuCaptureCapability, GpuCaptureFallback,
    GpuCapturePlan, GpuCaptureStatus, GpuTextureInterop,
};
pub use input::ScreenshotInputEvent;
pub use output::{
    ClampedSelectionRect, CropRect, ImageBounds, OutputAction, OutputBridgeContract,
    OutputBridgeTarget, OutputImageFormat, SelectedImageContract, SelectionRect,
};
pub use overlay::{NativeOverlayOptions, NativeOverlayState};
pub use overlay_renderer::{
    OverlayRenderError, OverlayRenderReceipt, OverlayRenderTarget, OverlayRendererContract,
};
pub use presenter::{PresentationFrame, PresenterKind};
pub use selection_state::{SelectionState, SelectionStateStatus, SelectionTransition};
pub use session::{
    advance_run_generation, begin_run_generation, generation_state, is_stale_generation,
    next_screenshot_session_id, ScreenshotGenerationState, ScreenshotRunGeneration,
};
pub use win32_overlay::{
    create_win32_overlay, destroy_win32_overlay, hide_win32_overlay, show_win32_overlay,
    Win32OverlayConfig, Win32OverlayError, Win32OverlayHandle, Win32OverlayLifecycleState,
    Win32OverlayWindow,
};
