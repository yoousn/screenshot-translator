use std::fmt;

#[cfg(windows)]
use windows::Win32::Foundation::{HMODULE, LUID};
#[cfg(not(windows))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LUID {
    pub LowPart: u32,
    pub HighPart: i32,
}
#[cfg(windows)]
use windows::core::Interface;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_0,
    D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_12_0,
    D3D_FEATURE_LEVEL_12_1,
};
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_DEBUG, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIFactory1, IDXGIFactory6,
    DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_GPU_PREFERENCE, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
    DXGI_GPU_PREFERENCE_MINIMUM_POWER, DXGI_GPU_PREFERENCE_UNSPECIFIED,
};

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

    #[cfg(windows)]
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

#[cfg(windows)]
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
    pub flags: u32,
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
            flags: 0,
            luid: LUID::default(),
        }
    }

    pub fn diagnostic_label(&self) -> String {
        if self.description.is_empty() {
            return "default hardware adapter".to_string();
        }
        format!(
            "{} (vendor=0x{:04X}, device=0x{:04X}, luid={:08X}:{:08X})",
            self.description,
            self.vendor_id,
            self.device_id,
            self.luid.HighPart as u32,
            self.luid.LowPart
        )
    }
}

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct D3d11AdapterHandle {
    pub adapter: IDXGIAdapter1,
    pub info: D3d11AdapterInfo,
}

#[cfg(not(windows))]
#[derive(Debug, Clone)]
pub struct D3d11AdapterHandle {
    pub info: D3d11AdapterInfo,
}

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct D3d11DeviceHandle {
    pub device: ID3D11Device,
    pub immediate_context: ID3D11DeviceContext,
    pub adapter: Option<D3d11AdapterHandle>,
    pub feature_level: D3d11FeatureLevel,
    pub diagnostics: D3d11DeviceDiagnostics,
}

#[cfg(not(windows))]
#[derive(Debug, Clone)]
pub struct D3d11DeviceHandle {
    pub adapter: Option<D3d11AdapterHandle>,
    pub feature_level: D3d11FeatureLevel,
    pub diagnostics: D3d11DeviceDiagnostics,
}

impl D3d11DeviceHandle {
    pub fn supports_capture(&self) -> bool {
        self.feature_level.supports_capture()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3d11DeviceDiagnostics {
    pub adapter_preference: D3d11AdapterPreference,
    pub adapter_label: String,
    pub feature_level: D3d11FeatureLevel,
    pub debug_layer_requested: bool,
    pub used_default_adapter: bool,
    pub fallback_reason: Option<String>,
}

impl D3d11DeviceDiagnostics {
    pub fn fallback(options: D3d11DeviceCreateOptions, reason: impl Into<String>) -> Self {
        Self {
            adapter_preference: options.adapter_preference,
            adapter_label: "unavailable".to_string(),
            feature_level: D3d11FeatureLevel::Unknown,
            debug_layer_requested: options.enable_debug_layer,
            used_default_adapter: matches!(
                options.adapter_preference,
                D3d11AdapterPreference::Default
            ),
            fallback_reason: Some(reason.into()),
        }
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
    UnsupportedPlatform(&'static str),
    AdapterUnavailable(String),
    UnsupportedFeatureLevel(D3d11FeatureLevel),
    DeviceUnavailable(String),
    ImmediateContextUnavailable,
    WindowsApi(String),
}

impl D3d11DeviceError {
    pub fn safe_fallback_reason(&self) -> String {
        let reason = match self {
            Self::UnsupportedPlatform(reason) => format!("D3D11 unavailable: {reason}"),
            Self::AdapterUnavailable(reason) => format!("D3D11 adapter unavailable: {reason}"),
            Self::UnsupportedFeatureLevel(level) => {
                format!("D3D11 feature level {level:?} is below capture minimum")
            }
            Self::DeviceUnavailable(reason) => format!("D3D11 device unavailable: {reason}"),
            Self::ImmediateContextUnavailable => "D3D11 immediate context unavailable".to_string(),
            Self::WindowsApi(reason) => format!("D3D11 API failed: {reason}"),
        };
        format!("{reason}; use existing CPU screenshot path.")
    }
}

impl fmt::Display for D3d11DeviceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform(reason) => {
                write!(formatter, "D3D11 is unsupported on this platform: {reason}")
            }
            Self::AdapterUnavailable(reason) => {
                write!(formatter, "D3D11 adapter is unavailable: {reason}")
            }
            Self::UnsupportedFeatureLevel(level) => write!(
                formatter,
                "D3D11 feature level {level:?} is unsupported for capture"
            ),
            Self::DeviceUnavailable(reason) => {
                write!(formatter, "D3D11 device is unavailable: {reason}")
            }
            Self::ImmediateContextUnavailable => {
                formatter.write_str("D3D11 immediate context was not returned")
            }
            Self::WindowsApi(reason) => {
                write!(formatter, "D3D11 Windows API call failed: {reason}")
            }
        }
    }
}

impl std::error::Error for D3d11DeviceError {}

pub type D3d11DeviceResult<T> = Result<T, D3d11DeviceError>;

#[cfg(not(windows))]
pub fn create_d3d11_capture_device(
    _options: D3d11DeviceCreateOptions,
) -> D3d11DeviceResult<D3d11DeviceHandle> {
    Err(D3d11DeviceError::UnsupportedPlatform(
        "D3D11 capture device creation is compile-gated to Windows builds",
    ))
}

#[cfg(windows)]
pub fn create_d3d11_capture_device(
    options: D3d11DeviceCreateOptions,
) -> D3d11DeviceResult<D3d11DeviceHandle> {
    let adapter = choose_adapter(options.adapter_preference)?;
    create_d3d11_capture_device_from_adapter(adapter, options)
}

#[cfg(windows)]
pub(crate) fn create_d3d11_capture_device_for_adapter(
    adapter: D3d11AdapterHandle,
    mut options: D3d11DeviceCreateOptions,
) -> D3d11DeviceResult<D3d11DeviceHandle> {
    options.adapter_preference = D3d11AdapterPreference::Default;
    create_d3d11_capture_device_from_adapter(Some(adapter), options)
}

#[cfg(windows)]
fn create_d3d11_capture_device_from_adapter(
    adapter: Option<D3d11AdapterHandle>,
    options: D3d11DeviceCreateOptions,
) -> D3d11DeviceResult<D3d11DeviceHandle> {
    let adapter_for_create = adapter
        .as_ref()
        .map(|handle| handle.adapter.cast::<IDXGIAdapter>())
        .transpose()
        .map_err(|error| D3d11DeviceError::WindowsApi(error.to_string()))?;
    let adapter_ref = adapter_for_create.as_ref();
    let adapter_label = adapter
        .as_ref()
        .map(|handle| handle.info.diagnostic_label())
        .unwrap_or_else(|| "default hardware adapter".to_string());
    let feature_levels = feature_level_candidates(options.minimum_feature_level)?;
    let flags = device_flags(options.enable_debug_layer);

    let mut device = None;
    let mut immediate_context = None;
    let mut created_feature_level = D3D_FEATURE_LEVEL_10_0;
    let driver_type = if adapter_ref.is_some() {
        D3D_DRIVER_TYPE_UNKNOWN
    } else {
        D3D_DRIVER_TYPE_HARDWARE
    };

    unsafe {
        D3D11CreateDevice(
            adapter_ref,
            driver_type,
            HMODULE::default(),
            flags,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut created_feature_level),
            Some(&mut immediate_context),
        )
        .map_err(|error| D3d11DeviceError::WindowsApi(error.to_string()))?;
    }

    let feature_level = D3d11FeatureLevel::from(created_feature_level);
    if !feature_level.supports_capture() || feature_level < options.minimum_feature_level {
        return Err(D3d11DeviceError::UnsupportedFeatureLevel(feature_level));
    }

    let device =
        device.ok_or_else(|| D3d11DeviceError::DeviceUnavailable(adapter_label.clone()))?;
    let immediate_context =
        immediate_context.ok_or(D3d11DeviceError::ImmediateContextUnavailable)?;
    let diagnostics = D3d11DeviceDiagnostics {
        adapter_preference: options.adapter_preference,
        adapter_label,
        feature_level,
        debug_layer_requested: options.enable_debug_layer,
        used_default_adapter: adapter.is_none(),
        fallback_reason: None,
    };

    Ok(D3d11DeviceHandle {
        device,
        immediate_context,
        adapter,
        feature_level,
        diagnostics,
    })
}

#[cfg(windows)]
fn device_flags(enable_debug_layer: bool) -> D3D11_CREATE_DEVICE_FLAG {
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    if enable_debug_layer {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }
    flags
}

#[cfg(windows)]
fn feature_level_candidates(
    minimum: D3d11FeatureLevel,
) -> D3d11DeviceResult<Vec<D3D_FEATURE_LEVEL>> {
    let candidates: Vec<_> = D3d11FeatureLevel::PREFERRED_ORDER
        .iter()
        .copied()
        .filter(|level| *level >= minimum)
        .filter_map(D3d11FeatureLevel::as_raw)
        .collect();
    if candidates.is_empty() {
        return Err(D3d11DeviceError::UnsupportedFeatureLevel(minimum));
    }
    Ok(candidates)
}

#[cfg(windows)]
fn choose_adapter(
    preference: D3d11AdapterPreference,
) -> D3d11DeviceResult<Option<D3d11AdapterHandle>> {
    if matches!(preference, D3d11AdapterPreference::Default) {
        return Ok(None);
    }

    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|error| D3d11DeviceError::WindowsApi(error.to_string()))?;
    if let Ok(factory6) = factory.cast::<IDXGIFactory6>() {
        if let Some(adapter) = enum_adapter_by_preference(&factory6, preference)? {
            return Ok(Some(adapter));
        }
    }
    enum_first_hardware_adapter(&factory)
}

#[cfg(windows)]
fn enum_adapter_by_preference(
    factory: &IDXGIFactory6,
    preference: D3d11AdapterPreference,
) -> D3d11DeviceResult<Option<D3d11AdapterHandle>> {
    let gpu_preference = dxgi_gpu_preference(preference);
    for index in 0..32 {
        match unsafe { factory.EnumAdapterByGpuPreference::<IDXGIAdapter1>(index, gpu_preference) }
        {
            Ok(adapter) if is_hardware_adapter(&adapter) => {
                return Ok(Some(adapter_handle(adapter)?));
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn enum_first_hardware_adapter(
    factory: &IDXGIFactory1,
) -> D3d11DeviceResult<Option<D3d11AdapterHandle>> {
    for index in 0..32 {
        match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) if is_hardware_adapter(&adapter) => {
                return Ok(Some(adapter_handle(adapter)?))
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    Err(D3d11DeviceError::AdapterUnavailable(
        "no hardware DXGI adapter was found".to_string(),
    ))
}

#[cfg(windows)]
fn dxgi_gpu_preference(preference: D3d11AdapterPreference) -> DXGI_GPU_PREFERENCE {
    match preference {
        D3d11AdapterPreference::Default => DXGI_GPU_PREFERENCE_UNSPECIFIED,
        D3d11AdapterPreference::HighPerformance => DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
        D3d11AdapterPreference::MinimumPower => DXGI_GPU_PREFERENCE_MINIMUM_POWER,
    }
}

#[cfg(windows)]
pub(crate) fn adapter_handle(adapter: IDXGIAdapter1) -> D3d11DeviceResult<D3d11AdapterHandle> {
    let info = adapter_info(&adapter)?;
    Ok(D3d11AdapterHandle { adapter, info })
}

#[cfg(windows)]
fn adapter_info(adapter: &IDXGIAdapter1) -> D3d11DeviceResult<D3d11AdapterInfo> {
    let desc = unsafe { adapter.GetDesc1() }
        .map_err(|error| D3d11DeviceError::WindowsApi(error.to_string()))?;
    let end = desc
        .Description
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(desc.Description.len());
    let description = String::from_utf16_lossy(&desc.Description[..end]);
    Ok(D3d11AdapterInfo {
        description,
        vendor_id: desc.VendorId,
        device_id: desc.DeviceId,
        subsystem_id: desc.SubSysId,
        revision: desc.Revision,
        flags: desc.Flags,
        luid: desc.AdapterLuid,
    })
}

#[cfg(windows)]
fn is_hardware_adapter(adapter: &IDXGIAdapter1) -> bool {
    adapter_info(adapter)
        .map(|info| {
            let software_flag = DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32;
            (info.flags & software_flag) == 0
        })
        .unwrap_or(false)
}
