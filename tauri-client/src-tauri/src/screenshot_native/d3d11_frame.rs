use std::fmt;
use std::num::NonZeroU64;

pub const D3D11_TEXTURE_FRAME_CONTRACT_VERSION: u16 = 1;
pub const D3D11_RGBA_BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11TextureFrameFormat {
    Bgra8Unorm,
    Rgba8Unorm,
}

impl D3d11TextureFrameFormat {
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Bgra8Unorm | Self::Rgba8Unorm => D3D11_RGBA_BYTES_PER_PIXEL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11FrameSource {
    WindowsGraphicsCapture,
    DxgiDesktopDuplication,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11TextureUsage {
    CaptureTexture,
    SharedTexture,
    StagingReadback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11CpuAccessMode {
    None,
    Read,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d11SharedHandleKind {
    None,
    SharedHandle,
    SharedNtHandle,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct D3d11RawHandle(NonZeroU64);

impl D3d11RawHandle {
    pub const fn new(raw: NonZeroU64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for D3d11RawHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("D3d11RawHandle")
            .field(&format_args!("0x{:X}", self.get()))
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d11TextureHandle {
    pub raw: D3d11RawHandle,
    pub usage: D3d11TextureUsage,
    pub cpu_access: D3d11CpuAccessMode,
}

impl D3d11TextureHandle {
    pub const fn new(
        raw: D3d11RawHandle,
        usage: D3d11TextureUsage,
        cpu_access: D3d11CpuAccessMode,
    ) -> Self {
        Self {
            raw,
            usage,
            cpu_access,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d11SharedHandleMetadata {
    pub kind: D3d11SharedHandleKind,
    pub handle: Option<D3d11RawHandle>,
    pub keyed_mutex_required: bool,
    pub cross_adapter: bool,
}

impl D3d11SharedHandleMetadata {
    pub const fn none() -> Self {
        Self {
            kind: D3d11SharedHandleKind::None,
            handle: None,
            keyed_mutex_required: false,
            cross_adapter: false,
        }
    }

    pub const fn shared(handle: D3d11RawHandle) -> Self {
        Self {
            kind: D3d11SharedHandleKind::SharedHandle,
            handle: Some(handle),
            keyed_mutex_required: false,
            cross_adapter: false,
        }
    }

    pub const fn shared_nt(handle: D3d11RawHandle, keyed_mutex_required: bool) -> Self {
        Self {
            kind: D3d11SharedHandleKind::SharedNtHandle,
            handle: Some(handle),
            keyed_mutex_required,
            cross_adapter: false,
        }
    }

    pub const fn is_shared(self) -> bool {
        !matches!(self.kind, D3d11SharedHandleKind::None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d11StagingReadbackMetadata {
    pub row_pitch: u32,
    pub depth_pitch: u32,
    pub byte_len: usize,
    pub is_mapped: bool,
}

impl D3d11StagingReadbackMetadata {
    pub fn new(width: u32, height: u32, row_pitch: u32, is_mapped: bool) -> D3d11FrameResult<Self> {
        if width == 0 || height == 0 {
            return Err(D3d11FrameContractError::EmptyDimensions { width, height });
        }
        let depth_pitch = row_pitch
            .checked_mul(height)
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        Ok(Self {
            row_pitch,
            depth_pitch,
            byte_len: usize::try_from(depth_pitch)
                .map_err(|_| D3d11FrameContractError::PitchOverflow)?,
            is_mapped,
        })
    }

    pub fn validate(
        self,
        width: u32,
        height: u32,
        format: D3d11TextureFrameFormat,
    ) -> D3d11FrameResult<()> {
        let minimum_row_pitch = width
            .checked_mul(format.bytes_per_pixel())
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        if self.row_pitch < minimum_row_pitch {
            return Err(D3d11FrameContractError::RowPitchTooSmall {
                row_pitch: self.row_pitch,
                minimum: minimum_row_pitch,
            });
        }

        let minimum_depth_pitch = self
            .row_pitch
            .checked_mul(height)
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        if self.depth_pitch < minimum_depth_pitch {
            return Err(D3d11FrameContractError::DepthPitchTooSmall {
                depth_pitch: self.depth_pitch,
                minimum: minimum_depth_pitch,
            });
        }

        let minimum_byte_len = usize::try_from(minimum_depth_pitch)
            .map_err(|_| D3d11FrameContractError::PitchOverflow)?;
        if self.byte_len < minimum_byte_len {
            return Err(D3d11FrameContractError::ByteLenTooSmall {
                byte_len: self.byte_len,
                minimum: minimum_byte_len,
            });
        }

        Ok(())
    }

    pub fn compact_byte_len(
        self,
        width: u32,
        height: u32,
        format: D3d11TextureFrameFormat,
    ) -> D3d11FrameResult<usize> {
        let row_len = width
            .checked_mul(format.bytes_per_pixel())
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        let byte_len = row_len
            .checked_mul(height)
            .ok_or(D3d11FrameContractError::PitchOverflow)?;
        usize::try_from(byte_len).map_err(|_| D3d11FrameContractError::PitchOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d11TextureFrameMetadata {
    pub contract_version: u16,
    pub source: D3d11FrameSource,
    pub width: u32,
    pub height: u32,
    pub format: D3d11TextureFrameFormat,
    pub texture: Option<D3d11TextureHandle>,
    pub shared_handle: D3d11SharedHandleMetadata,
    pub staging_readback: Option<D3d11StagingReadbackMetadata>,
    pub frame_id: u64,
    pub capture_timestamp_100ns: Option<u64>,
}

impl D3d11TextureFrameMetadata {
    pub const fn new(
        source: D3d11FrameSource,
        width: u32,
        height: u32,
        format: D3d11TextureFrameFormat,
    ) -> Self {
        Self {
            contract_version: D3D11_TEXTURE_FRAME_CONTRACT_VERSION,
            source,
            width,
            height,
            format,
            texture: None,
            shared_handle: D3d11SharedHandleMetadata::none(),
            staging_readback: None,
            frame_id: 0,
            capture_timestamp_100ns: None,
        }
    }

    pub const fn with_texture(mut self, texture: D3d11TextureHandle) -> Self {
        self.texture = Some(texture);
        self
    }

    pub const fn with_shared_handle(mut self, shared_handle: D3d11SharedHandleMetadata) -> Self {
        self.shared_handle = shared_handle;
        self
    }

    pub const fn with_staging_readback(
        mut self,
        staging_readback: D3d11StagingReadbackMetadata,
    ) -> Self {
        self.staging_readback = Some(staging_readback);
        self
    }

    pub const fn with_frame_id(mut self, frame_id: u64) -> Self {
        self.frame_id = frame_id;
        self
    }

    pub const fn with_capture_timestamp_100ns(mut self, timestamp: u64) -> Self {
        self.capture_timestamp_100ns = Some(timestamp);
        self
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn validate(self) -> D3d11FrameResult<()> {
        if self.contract_version != D3D11_TEXTURE_FRAME_CONTRACT_VERSION {
            return Err(D3d11FrameContractError::UnsupportedContractVersion(
                self.contract_version,
            ));
        }
        if self.is_empty() {
            return Err(D3d11FrameContractError::EmptyDimensions {
                width: self.width,
                height: self.height,
            });
        }
        if self.shared_handle.is_shared() && self.shared_handle.handle.is_none() {
            return Err(D3d11FrameContractError::MissingSharedHandle);
        }
        if let Some(staging_readback) = self.staging_readback {
            staging_readback.validate(self.width, self.height, self.format)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3d11TextureFrame {
    pub metadata: D3d11TextureFrameMetadata,
    pub readback_bytes: Option<Vec<u8>>,
}

impl D3d11TextureFrame {
    pub fn new(
        metadata: D3d11TextureFrameMetadata,
        readback_bytes: Option<Vec<u8>>,
    ) -> D3d11FrameResult<Self> {
        metadata.validate()?;
        if let (Some(staging), Some(bytes)) = (metadata.staging_readback, readback_bytes.as_ref()) {
            if bytes.len() < staging.byte_len {
                return Err(D3d11FrameContractError::ByteLenTooSmall {
                    byte_len: bytes.len(),
                    minimum: staging.byte_len,
                });
            }
        }
        Ok(Self {
            metadata,
            readback_bytes,
        })
    }

    pub fn requires_gpu_texture(self) -> bool {
        self.metadata.texture.is_some() || self.metadata.shared_handle.is_shared()
    }

    pub fn has_cpu_readback(self) -> bool {
        self.readback_bytes.is_some()
    }

    pub fn validate_cpu_readback(&self) -> D3d11FrameResult<()> {
        let staging = self
            .metadata
            .staging_readback
            .ok_or(D3d11FrameContractError::MissingStagingReadback)?;
        if !staging.is_mapped {
            return Err(D3d11FrameContractError::StagingReadbackNotMapped);
        }
        let bytes = self
            .readback_bytes
            .as_ref()
            .ok_or(D3d11FrameContractError::MissingReadbackBytes)?;
        if bytes.len() < staging.byte_len {
            return Err(D3d11FrameContractError::ByteLenTooSmall {
                byte_len: bytes.len(),
                minimum: staging.byte_len,
            });
        }
        Ok(())
    }

    pub fn compact_readback_bytes(&self) -> D3d11FrameResult<Vec<u8>> {
        self.validate_cpu_readback()?;
        let staging = self
            .metadata
            .staging_readback
            .ok_or(D3d11FrameContractError::MissingStagingReadback)?;
        let source = self
            .readback_bytes
            .as_ref()
            .ok_or(D3d11FrameContractError::MissingReadbackBytes)?;
        let row_len = usize::try_from(
            self.metadata
                .width
                .checked_mul(self.metadata.format.bytes_per_pixel())
                .ok_or(D3d11FrameContractError::PitchOverflow)?,
        )
        .map_err(|_| D3d11FrameContractError::PitchOverflow)?;
        let height = usize::try_from(self.metadata.height)
            .map_err(|_| D3d11FrameContractError::PitchOverflow)?;
        let row_pitch = usize::try_from(staging.row_pitch)
            .map_err(|_| D3d11FrameContractError::PitchOverflow)?;
        let mut compact = Vec::with_capacity(staging.compact_byte_len(
            self.metadata.width,
            self.metadata.height,
            self.metadata.format,
        )?);
        for row in 0..height {
            let offset = row
                .checked_mul(row_pitch)
                .ok_or(D3d11FrameContractError::PitchOverflow)?;
            let end = offset
                .checked_add(row_len)
                .ok_or(D3d11FrameContractError::PitchOverflow)?;
            let slice =
                source
                    .get(offset..end)
                    .ok_or(D3d11FrameContractError::ByteLenTooSmall {
                        byte_len: source.len(),
                        minimum: end,
                    })?;
            compact.extend_from_slice(slice);
        }
        Ok(compact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D3d11FrameContractError {
    UnsupportedContractVersion(u16),
    EmptyDimensions { width: u32, height: u32 },
    MissingSharedHandle,
    MissingStagingTexture,
    MissingStagingReadback,
    MissingReadbackBytes,
    StagingReadbackNotMapped,
    UnsupportedTextureFormat(String),
    SelectedRegionOutsideFrame,
    WindowsApi(String),
    PitchOverflow,
    RowPitchTooSmall { row_pitch: u32, minimum: u32 },
    DepthPitchTooSmall { depth_pitch: u32, minimum: u32 },
    ByteLenTooSmall { byte_len: usize, minimum: usize },
}

impl fmt::Display for D3d11FrameContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedContractVersion(version) => write!(
                formatter,
                "unsupported D3D11 frame contract version: {version}"
            ),
            Self::EmptyDimensions { width, height } => write!(
                formatter,
                "empty D3D11 texture frame dimensions: {width}x{height}"
            ),
            Self::MissingSharedHandle => {
                formatter.write_str("D3D11 shared handle metadata has no handle")
            }
            Self::MissingStagingTexture => {
                formatter.write_str("D3D11 staging texture was not returned")
            }
            Self::MissingStagingReadback => {
                formatter.write_str("D3D11 CPU readback requires staging metadata")
            }
            Self::MissingReadbackBytes => {
                formatter.write_str("D3D11 CPU readback bytes are missing")
            }
            Self::StagingReadbackNotMapped => {
                formatter.write_str("D3D11 staging texture is not mapped for CPU readback")
            }
            Self::PitchOverflow => formatter.write_str("D3D11 staging readback pitch overflows"),
            Self::RowPitchTooSmall { row_pitch, minimum } => write!(
                formatter,
                "D3D11 staging row pitch {row_pitch} is smaller than minimum {minimum}"
            ),
            Self::DepthPitchTooSmall {
                depth_pitch,
                minimum,
            } => write!(
                formatter,
                "D3D11 staging depth pitch {depth_pitch} is smaller than minimum {minimum}"
            ),
            Self::ByteLenTooSmall { byte_len, minimum } => write!(
                formatter,
                "D3D11 staging byte length {byte_len} is smaller than minimum {minimum}"
            ),
            Self::UnsupportedTextureFormat(format) => write!(
                formatter,
                "unsupported D3D11 texture format for RGBA readback: {format}"
            ),
            Self::SelectedRegionOutsideFrame => {
                formatter.write_str("selected D3D11 readback region is outside the frame")
            }
            Self::WindowsApi(reason) => write!(formatter, "D3D11 readback API failed: {reason}"),
        }
    }
}

impl std::error::Error for D3d11FrameContractError {}

pub type D3d11FrameResult<T> = Result<T, D3d11FrameContractError>;
