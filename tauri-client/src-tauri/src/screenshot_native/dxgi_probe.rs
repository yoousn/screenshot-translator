use super::dxgi_output::DxgiDesktopCoordinates;

#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiNativeApiProbe {
    pub is_windows: bool,
    pub has_factory: bool,
    pub has_adapter: bool,
    pub has_output: bool,
    pub desktop_coordinates: Option<DxgiDesktopCoordinates>,
    pub reason: Option<String>,
}

impl DxgiNativeApiProbe {
    pub fn available() -> Self {
        Self {
            is_windows: true,
            has_factory: true,
            has_adapter: true,
            has_output: true,
            desktop_coordinates: None,
            reason: None,
        }
    }

    pub fn available_with_desktop_coordinates(
        desktop_coordinates: Option<DxgiDesktopCoordinates>,
        reason: Option<String>,
    ) -> Self {
        Self {
            is_windows: true,
            has_factory: true,
            has_adapter: true,
            has_output: true,
            desktop_coordinates,
            reason,
        }
    }

    pub fn unavailable(
        is_windows: bool,
        has_factory: bool,
        has_adapter: bool,
        has_output: bool,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            is_windows,
            has_factory,
            has_adapter,
            has_output,
            desktop_coordinates: None,
            reason: Some(reason.into()),
        }
    }

    pub fn supports_duplication_probe(&self) -> bool {
        self.is_windows && self.has_factory && self.has_adapter && self.has_output
    }
}

#[cfg(target_os = "windows")]
pub fn probe_dxgi_native_api_support() -> DxgiNativeApiProbe {
    let factory = match unsafe { CreateDXGIFactory1::<IDXGIFactory1>() } {
        Ok(factory) => factory,
        Err(error) => {
            return DxgiNativeApiProbe::unavailable(
                true,
                false,
                false,
                false,
                format!("CreateDXGIFactory1 failed: {error}"),
            );
        }
    };

    let adapter = match unsafe { factory.EnumAdapters1(0) } {
        Ok(adapter) => adapter,
        Err(error) => {
            return DxgiNativeApiProbe::unavailable(
                true,
                true,
                false,
                false,
                format!("IDXGIFactory1::EnumAdapters1 failed: {error}"),
            );
        }
    };

    match unsafe { adapter.EnumOutputs(0) } {
        Ok(output) => match unsafe { output.GetDesc() } {
            Ok(desc) => DxgiNativeApiProbe::available_with_desktop_coordinates(
                Some(DxgiDesktopCoordinates::new(
                    desc.DesktopCoordinates.left,
                    desc.DesktopCoordinates.top,
                    desc.DesktopCoordinates.right,
                    desc.DesktopCoordinates.bottom,
                )),
                None,
            ),
            Err(error) => DxgiNativeApiProbe::available_with_desktop_coordinates(
                None,
                Some(format!("IDXGIOutput::GetDesc failed: {error}")),
            ),
        },
        Err(error) => DxgiNativeApiProbe::unavailable(
            true,
            true,
            true,
            false,
            format!("IDXGIAdapter1::EnumOutputs failed: {error}"),
        ),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn probe_dxgi_native_api_support() -> DxgiNativeApiProbe {
    DxgiNativeApiProbe::unavailable(
        false,
        false,
        false,
        false,
        "DXGI Desktop Duplication requires Windows",
    )
}
