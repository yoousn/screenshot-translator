use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OcrTensorPreprocessConfig {
    pub width: u32,
    pub height: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrTensorInput {
    pub shape: Vec<usize>,
    pub width: u32,
    pub height: u32,
    pub original_width: u32,
    pub original_height: u32,
    pub channels: usize,
    pub layout: String,
    pub element_type: String,
    #[serde(skip_serializing)]
    pub data: Vec<f32>,
}

impl OcrTensorPreprocessConfig {
    pub fn ppocr_det_default() -> Self {
        Self {
            width: 960,
            height: 960,
            mean: [0.485, 0.456, 0.406],
            std: [0.229, 0.224, 0.225],
        }
    }

    pub fn ppocr_rec_default() -> Self {
        Self {
            width: 320,
            height: 48,
            mean: [0.5, 0.5, 0.5],
            std: [0.5, 0.5, 0.5],
        }
    }

    pub fn ppocr_cls_default() -> Self {
        Self {
            width: 192,
            height: 48,
            mean: [0.5, 0.5, 0.5],
            std: [0.5, 0.5, 0.5],
        }
    }

    pub fn for_model_descriptor(
        model: &serde_json::Value,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<Self, String> {
        let model_type = model["type"].as_str().unwrap_or("detection");
        let base = match model_type {
            "recognition" => Self::ppocr_rec_default(),
            "classification" => Self::ppocr_cls_default(),
            _ => Self::ppocr_det_default(),
        };
        let width = width.unwrap_or(base.width);
        let height = height.unwrap_or(base.height);
        Self::with_size_and_stats(width, height, base.mean, base.std)
    }

    pub fn with_size_and_stats(
        width: u32,
        height: u32,
        mean: [f32; 3],
        std: [f32; 3],
    ) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("OCR tensor width and height must be greater than zero.".to_string());
        }
        if width > 4096 || height > 4096 {
            return Err(
                "OCR tensor width and height are too large for the current runtime guard."
                    .to_string(),
            );
        }
        Ok(Self {
            width,
            height,
            mean,
            std,
        })
    }
}

pub fn image_bytes_to_nchw_rgb_tensor(
    image_bytes: &[u8],
    config: &OcrTensorPreprocessConfig,
) -> Result<OcrTensorInput, String> {
    let image = image::load_from_memory(image_bytes)
        .map_err(|error| format!("failed to decode OCR input image: {error}"))?;
    let original_width = image.width();
    let original_height = image.height();
    if original_width == 0 || original_height == 0 {
        return Err("OCR input image has invalid dimensions.".to_string());
    }
    rgb_bytes_to_nchw_rgb_tensor(
        &image.to_rgb8().into_raw(),
        original_width,
        original_height,
        config,
    )
}

pub fn rgb_bytes_to_nchw_rgb_tensor(
    rgb_bytes: &[u8],
    original_width: u32,
    original_height: u32,
    config: &OcrTensorPreprocessConfig,
) -> Result<OcrTensorInput, String> {
    if original_width == 0 || original_height == 0 {
        return Err("OCR RGB input has invalid dimensions.".to_string());
    }
    let expected_len = (original_width as usize)
        .checked_mul(original_height as usize)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| "OCR RGB input dimensions are too large.".to_string())?;
    if rgb_bytes.len() != expected_len {
        return Err(format!(
            "OCR RGB input length mismatch: expected {expected_len} bytes, got {}.",
            rgb_bytes.len()
        ));
    }
    let rgb_image = image::RgbImage::from_raw(original_width, original_height, rgb_bytes.to_vec())
        .ok_or_else(|| "failed to build OCR RGB image from raw bytes.".to_string())?;
    let resized = image::imageops::resize(
        &rgb_image,
        config.width,
        config.height,
        image::imageops::FilterType::Triangle,
    );
    let pixel_count = (config.width as usize) * (config.height as usize);
    let mut data = vec![0.0_f32; 3 * pixel_count];

    for (index, pixel) in resized.pixels().enumerate() {
        for channel in 0..3 {
            let raw = pixel[channel] as f32 / 255.0;
            data[channel * pixel_count + index] =
                (raw - config.mean[channel]) / config.std[channel];
        }
    }

    Ok(OcrTensorInput {
        shape: vec![1, 3, config.height as usize, config.width as usize],
        width: config.width,
        height: config.height,
        original_width,
        original_height,
        channels: 3,
        layout: "NCHW".to_string(),
        element_type: "f32".to_string(),
        data,
    })
}

pub fn cropped_line_to_nchw_rgb_tensor(
    crop: &crate::ysn_ocr_crop::OcrCroppedLineImage,
    config: &OcrTensorPreprocessConfig,
) -> Result<OcrTensorInput, String> {
    rgb_bytes_to_nchw_rgb_tensor(&crop.rgb_bytes, crop.width, crop.height, config)
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, ImageFormat, Rgb};
    use std::io::Cursor;

    #[test]
    fn test_preprocess_config_rejects_invalid_sizes() {
        assert!(
            super::OcrTensorPreprocessConfig::with_size_and_stats(0, 48, [0.5; 3], [0.5; 3])
                .is_err()
        );
        assert!(super::OcrTensorPreprocessConfig::with_size_and_stats(
            48, 4097, [0.5; 3], [0.5; 3]
        )
        .is_err());
    }

    #[test]
    fn test_for_model_descriptor_uses_recognition_defaults() {
        let model = serde_json::json!({ "type": "recognition" });
        let config =
            super::OcrTensorPreprocessConfig::for_model_descriptor(&model, None, None).unwrap();
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 48);
    }

    #[test]
    fn test_image_bytes_to_nchw_rgb_tensor_shape_and_values() {
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgb([255, 0, 0]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        let config = super::OcrTensorPreprocessConfig::with_size_and_stats(
            1,
            1,
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
        .unwrap();
        let tensor = super::image_bytes_to_nchw_rgb_tensor(&bytes, &config).unwrap();
        assert_eq!(tensor.shape, vec![1, 3, 1, 1]);
        assert_eq!(tensor.layout, "NCHW");
        assert_eq!(tensor.element_type, "f32");
        assert_eq!(tensor.data, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_rgb_bytes_to_nchw_rgb_tensor_validates_byte_length() {
        let config = super::OcrTensorPreprocessConfig::with_size_and_stats(
            1,
            1,
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
        .unwrap();
        let error = super::rgb_bytes_to_nchw_rgb_tensor(&[255, 0], 1, 1, &config).unwrap_err();
        assert!(error.contains("length mismatch"));
    }

    #[test]
    fn test_rgb_bytes_to_nchw_rgb_tensor_preserves_crop_source_dimensions() {
        let config = super::OcrTensorPreprocessConfig::with_size_and_stats(
            2,
            1,
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
        .unwrap();
        let tensor =
            super::rgb_bytes_to_nchw_rgb_tensor(&[255, 0, 0, 0, 255, 0], 2, 1, &config).unwrap();
        assert_eq!(tensor.shape, vec![1, 3, 1, 2]);
        assert_eq!(tensor.original_width, 2);
        assert_eq!(tensor.original_height, 1);
        assert_eq!(tensor.data, vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_cropped_line_to_nchw_rgb_tensor_uses_crop_metadata() {
        let crop = crate::ysn_ocr_crop::OcrCroppedLineImage {
            index: 0,
            source_plan: crate::ysn_ocr_crop::OcrLineCropPlan {
                index: 0,
                source_box: vec![vec![0, 0], vec![1, 0], vec![1, 1], vec![0, 1]],
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                padding: 0,
                rotation_degrees: 0.0,
            },
            width: 1,
            height: 1,
            color_format: "RGB8".to_string(),
            rgb_bytes: vec![0, 0, 255],
        };
        let config = super::OcrTensorPreprocessConfig::with_size_and_stats(
            1,
            1,
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        )
        .unwrap();

        let tensor = super::cropped_line_to_nchw_rgb_tensor(&crop, &config).unwrap();

        assert_eq!(tensor.original_width, 1);
        assert_eq!(tensor.original_height, 1);
        assert_eq!(tensor.data, vec![0.0, 0.0, 1.0]);
    }
}
