use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl SelectionRect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    pub fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    pub fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    pub fn normalized(self) -> Self {
        let mut x = self.x;
        let mut y = self.y;
        let mut width = self.width;
        let mut height = self.height;

        if width < 0 {
            x = x.saturating_add(width);
            width = width.saturating_abs();
        }

        if height < 0 {
            y = y.saturating_add(height);
            height = height.saturating_abs();
        }

        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn clamp_to(self, bounds: ImageBounds) -> Option<ClampedSelectionRect> {
        if bounds.is_empty() {
            return None;
        }

        let requested = self.normalized();
        if !requested.is_valid() {
            return None;
        }

        let left = i64::from(requested.x).clamp(0, i64::from(bounds.width));
        let top = i64::from(requested.y).clamp(0, i64::from(bounds.height));
        let right = requested.right().clamp(0, i64::from(bounds.width));
        let bottom = requested.bottom().clamp(0, i64::from(bounds.height));

        if right <= left || bottom <= top {
            return None;
        }

        let crop = CropRect {
            x: left as u32,
            y: top as u32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        };

        Some(ClampedSelectionRect {
            requested,
            crop,
            was_clamped: crop.as_selection_rect() != requested,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBounds {
    pub width: u32,
    pub height: u32,
}

impl ImageBounds {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CropRect {
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn right(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn as_selection_rect(self) -> SelectionRect {
        SelectionRect {
            x: self.x.min(i32::MAX as u32) as i32,
            y: self.y.min(i32::MAX as u32) as i32,
            width: self.width.min(i32::MAX as u32) as i32,
            height: self.height.min(i32::MAX as u32) as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClampedSelectionRect {
    pub requested: SelectionRect,
    pub crop: CropRect,
    pub was_clamped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputAction {
    Copy,
    SaveAs,
    Ocr,
    Translate,
    Record,
}

impl OutputAction {
    pub fn bridge_target(self) -> Option<OutputBridgeTarget> {
        match self {
            Self::Copy => Some(OutputBridgeTarget::Clipboard),
            Self::SaveAs => Some(OutputBridgeTarget::File),
            Self::Ocr => Some(OutputBridgeTarget::Ocr),
            Self::Translate => Some(OutputBridgeTarget::Translation),
            Self::Record => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputBridgeTarget {
    Clipboard,
    File,
    Ocr,
    Translation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputImageFormat {
    Png,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedImageContract {
    pub rect: SelectionRect,
    pub crop: CropRect,
    pub png_bytes: Vec<u8>,
    pub source_width: u32,
    pub source_height: u32,
    pub was_clamped: bool,
}

impl SelectedImageContract {
    pub fn new(clamped: ClampedSelectionRect, png_bytes: Vec<u8>, bounds: ImageBounds) -> Self {
        Self {
            rect: clamped.requested,
            crop: clamped.crop,
            png_bytes,
            source_width: bounds.width,
            source_height: bounds.height,
            was_clamped: clamped.was_clamped,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.png_bytes.len()
    }

    pub fn bounds(&self) -> ImageBounds {
        ImageBounds {
            width: self.source_width,
            height: self.source_height,
        }
    }

    pub fn image_format(&self) -> OutputImageFormat {
        OutputImageFormat::Png
    }

    pub fn is_empty(&self) -> bool {
        self.png_bytes.is_empty() || self.crop.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputBridgeContract {
    pub action: OutputAction,
    pub target: OutputBridgeTarget,
    pub image: SelectedImageContract,
}

impl OutputBridgeContract {
    pub fn new(action: OutputAction, image: SelectedImageContract) -> Option<Self> {
        action.bridge_target().map(|target| Self {
            action,
            target,
            image,
        })
    }
}
