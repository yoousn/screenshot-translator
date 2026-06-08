#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayCapabilityFlag {
    Disabled,
    EnabledMvp,
}

impl NativeOverlayCapabilityFlag {
    pub const fn from_enabled(enabled: bool) -> Self {
        if enabled {
            Self::EnabledMvp
        } else {
            Self::Disabled
        }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::EnabledMvp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayFallbackReason {
    CapabilityDisabled,
    MvpNotWired,
}

impl NativeOverlayFallbackReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityDisabled => "capability-disabled",
            Self::MvpNotWired => "mvp-not-wired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotOverlayRuntime {
    WebviewRgba,
    NativeOverlayMvp,
}

impl ScreenshotOverlayRuntime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WebviewRgba => "webview-rgba",
            Self::NativeOverlayMvp => "native-overlay-mvp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayLaunchPlan {
    pub capability: NativeOverlayCapabilityFlag,
    pub runtime: ScreenshotOverlayRuntime,
    pub fallback_reason: Option<NativeOverlayFallbackReason>,
}

impl NativeOverlayLaunchPlan {
    pub const fn fallback(reason: NativeOverlayFallbackReason) -> Self {
        Self {
            capability: NativeOverlayCapabilityFlag::Disabled,
            runtime: ScreenshotOverlayRuntime::WebviewRgba,
            fallback_reason: Some(reason),
        }
    }

    pub const fn native_mvp(capability: NativeOverlayCapabilityFlag) -> Self {
        Self {
            capability,
            runtime: ScreenshotOverlayRuntime::NativeOverlayMvp,
            fallback_reason: None,
        }
    }

    pub const fn uses_native_overlay(self) -> bool {
        matches!(self.runtime, ScreenshotOverlayRuntime::NativeOverlayMvp)
    }
}

pub const fn resolve_native_overlay_launch_plan(
    capability: NativeOverlayCapabilityFlag,
) -> NativeOverlayLaunchPlan {
    if capability.is_enabled() {
        NativeOverlayLaunchPlan::native_mvp(capability)
    } else {
        NativeOverlayLaunchPlan::fallback(NativeOverlayFallbackReason::CapabilityDisabled)
    }
}

pub const fn default_native_overlay_capability() -> NativeOverlayCapabilityFlag {
    NativeOverlayCapabilityFlag::Disabled
}

pub const fn default_native_overlay_launch_plan() -> NativeOverlayLaunchPlan {
    resolve_native_overlay_launch_plan(default_native_overlay_capability())
}
