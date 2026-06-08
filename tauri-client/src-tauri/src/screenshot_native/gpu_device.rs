use std::fmt;

use windows::Win32::Foundation::LUID;
use windows::Win32::Graphics::Direct3D::{
    D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0,
    D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_12_1,
};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};
use windows::Win32::Graphics::Dxgi::IDXGIAdapter1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D3d11FeatureLevel {
    V10_0,
    V10_1,
    V11_0,
    V11_1,
    V12_0,
    V12_1,
    Unknown,
}

impl D3d11FeatureLevel {
    pub const PREFERRED_ORDER: [Self; 6] = [
        Self::V12_1,
        Self::V12_0,
        Self::V11_1,
        Self::V11_0,
        Self::V10_1,
        Self::V10_0,
    ];

    pub const MIN_CAPTURE_LEVEL: Self = Self::V11_0;

    pub const fn as_raw(self) -> Option<D3D_FEATURE_LEVEL> {
        match self {
            Self::V10_0 => Some(D3D_FEATURE_LEVEL_10_0),
            Self::V10_1 => Some(D3D_FEATURE_LEVEL_10_1),
            Self::V11_0 => Some(D3D_FEATURE_LEVEL_11_0),
            Self::V11_1 => Some(D3D_FEATURE_LEVEL_11_1),
            Self::V12_0 => Some(D3D_FEATURE_LEVEL_12_0),
            Self::V12_1 => Some(D3D_FEATURE_LEVEL_12_1),
            Self::Unknown => None,
        }
    }

    pub const fn supports_capture(self) -> bool {
        matches!(self, Self::V11_0 | Self::V11_1 | Self::V12_0 | Self::V12_1)
    }
}

impl From<D3D_FEATURE_LEVEL> for D3d11FeatureLevel {
    fn from(value: D3D_FEATURE_LEVEL) -> Self {
        match value {
            D3D_FEATURE_LEVEL_10_0 => Self::V10_0,
            D3D_FEATURE_LEVEL_10_1 => Self::V10_1,
            D3D_FEATURE_LEVEL_11_0 => Self::V11_0,
            D3D_FEATURE_LEVEL_11_1 => Self::V11_1,
            D3D_FEATURE_LEVEL_12_0 => Self::V12_0,
            D3D_FEATURE_LEVEL_12_1 => Self::V12_1,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11AdapterPreference {
    Default,
    HighPerformance,
    MinimumPower,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3d11AdapterInfo {
    pub description: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub subsystem_id: u32,
    pub revision: u32,
    pub luid: LUID,
}

impl D3d11AdapterInfo {
    pub fn unknown() -> Self {
        Self {
            description: String::new(),
            vendor_id: 0,
            device_id: 0,
            subsystem_id: 0,
            revision: 0,
            luid: LUID::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct D3d11AdapterHandle {
    pub adapter: IDXGIAdapter1,
    pub info: D3d11AdapterInfo,
}

#[derive(Debug, Clone)]
pub struct D3d11DeviceHandle {
    pub device: ID3D11Device,
    pub immediate_context: ID3D11DeviceContext,
    pub adapter: Option<D3d11AdapterHandle>,
    pub feature_level: D3d11FeatureLevel,
}

impl D3d11DeviceHandle {
    pub fn supports_capture(&self) -> bool {
        self.feature_level.supports_capture()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d11DeviceCreateOptions {
    pub adapter_preference: D3d11AdapterPreference,
    pub minimum_feature_level: D3d11FeatureLevel,
    pub enable_debug_layer: bool,
}

impl Default for D3d11DeviceCreateOptions {
    fn default() -> Self {
        Self {
            adapter_preference: D3d11AdapterPreference::Default,
            minimum_feature_level: D3d11FeatureLevel::MIN_CAPTURE_LEVEL,
            enable_debug_layer: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D3d11DeviceError {
    NotImplemented(&'static str),
    AdapterUnavailable(String),
    UnsupportedFeatureLevel(D3d11FeatureLevel),
    WindowsApi(String),
}

impl fmt::Display for D3d11DeviceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented(reason) => {
                write!(formatter, "D3D11 device creation is pending: {reason}")
            }
            Self::AdapterUnavailable(reason) => {
                write!(formatter, "D3D11 adapter is unavailable: {reason}")
            }
            Self::UnsupportedFeatureLevel(level) => write!(
                formatter,
                "D3D11 feature level {level:?} is unsupported for capture"
            ),
            Self::WindowsApi(reason) => {
                write!(formatter, "D3D11 Windows API call failed: {reason}")
            }
        }
    }
}

impl std::error::Error for D3d11DeviceError {}

pub type D3d11DeviceResult<T> = Result<T, D3d11DeviceError>;

pub fn create_d3d11_capture_device(
    _options: D3d11DeviceCreateOptions,
) -> D3d11DeviceResult<D3d11DeviceHandle> {
    Err(D3d11DeviceError::NotImplemented(
        "Phase E only defines the owned D3D11 device contract; native API wiring comes later.",
    ))
}
