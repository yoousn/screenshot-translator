use serde::Serialize;

use crate::ysn_ocr_decode::OcrDetectionBox;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrLineCropPlan {
    pub index: usize,
    pub source_box: Vec<Vec<i32>>,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub rotation_degrees: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrCroppedLineImage {
    pub index: usize,
    pub source_plan: OcrLineCropPlan,
    pub width: u32,
    pub height: u32,
    pub color_format: String,
    #[serde(skip_serializing)]
    pub rgb_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OcrCropPlanConfig {
    pub image_width: u32,
    pub image_height: u32,
    pub padding: u32,
    pub minimum_width: u32,
    pub minimum_height: u32,
}

impl Default for OcrCropPlanConfig {
    fn default() -> Self {
        Self {
            image_width: 0,
            image_height: 0,
            padding: 2,
            minimum_width: 2,
            minimum_height: 2,
        }
    }
}

pub fn build_line_crop_plan(
    detections: &[OcrDetectionBox],
    config: &OcrCropPlanConfig,
) -> Result<Vec<OcrLineCropPlan>, String> {
    if config.image_width == 0 || config.image_height == 0 {
        return Err("OCR crop plan requires non-zero image dimensions.".to_string());
    }
    let mut plans = Vec::new();
    for (index, detection) in detections.iter().enumerate() {
        let Some((min_x, min_y, max_x, max_y)) = bounding_rect(&detection.box_coords) else {
            continue;
        };
        let padded_min_x = min_x.saturating_sub(config.padding as i32).max(0) as u32;
        let padded_min_y = min_y.saturating_sub(config.padding as i32).max(0) as u32;
        let padded_max_x =
            (max_x + config.padding as i32).clamp(0, config.image_width as i32) as u32;
        let padded_max_y =
            (max_y + config.padding as i32).clamp(0, config.image_height as i32) as u32;
        let width = padded_max_x.saturating_sub(padded_min_x);
        let height = padded_max_y.saturating_sub(padded_min_y);
        if width < config.minimum_width || height < config.minimum_height {
            continue;
        }
        plans.push(OcrLineCropPlan {
            index,
            source_box: detection.box_coords.clone(),
            x: padded_min_x,
            y: padded_min_y,
            width,
            height,
            padding: config.padding,
            rotation_degrees: estimate_rotation_degrees(&detection.box_coords),
        });
    }
    plans.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
    Ok(plans)
}

pub fn crop_line_images_from_bytes(
    image_bytes: &[u8],
    crop_plans: &[OcrLineCropPlan],
) -> Result<Vec<OcrCroppedLineImage>, String> {
    let image = image::load_from_memory(image_bytes)
        .map_err(|error| format!("failed to decode OCR crop source image: {error}"))?;
    let source_width = image.width();
    let source_height = image.height();
    if source_width == 0 || source_height == 0 {
        return Err("OCR crop source image has invalid dimensions.".to_string());
    }

    let rgb_image = image.to_rgb8();
    let mut cropped_lines = Vec::new();
    for (index, plan) in crop_plans.iter().enumerate() {
        if plan.width == 0 || plan.height == 0 {
            continue;
        }
        let right = plan
            .x
            .checked_add(plan.width)
            .ok_or_else(|| "OCR crop plan width overflows image bounds.".to_string())?;
        let bottom = plan
            .y
            .checked_add(plan.height)
            .ok_or_else(|| "OCR crop plan height overflows image bounds.".to_string())?;
        if right > source_width || bottom > source_height {
            return Err(format!(
                "OCR crop plan {} is outside source image bounds {}x{}.",
                plan.index, source_width, source_height
            ));
        }
        let crop = image::imageops::crop_imm(&rgb_image, plan.x, plan.y, plan.width, plan.height)
            .to_image();
        cropped_lines.push(OcrCroppedLineImage {
            index,
            source_plan: plan.clone(),
            width: plan.width,
            height: plan.height,
            color_format: "RGB8".to_string(),
            rgb_bytes: crop.into_raw(),
        });
    }

    Ok(cropped_lines)
}

fn bounding_rect(points: &[Vec<i32>]) -> Option<(i32, i32, i32, i32)> {
    let xs: Vec<i32> = points
        .iter()
        .filter_map(|point| point.first().copied())
        .collect();
    let ys: Vec<i32> = points
        .iter()
        .filter_map(|point| point.get(1).copied())
        .collect();
    if xs.is_empty() || ys.is_empty() {
        return None;
    }
    Some((
        *xs.iter().min()?,
        *ys.iter().min()?,
        *xs.iter().max()?,
        *ys.iter().max()?,
    ))
}

fn estimate_rotation_degrees(points: &[Vec<i32>]) -> f32 {
    if points.len() < 2 || points[0].len() < 2 || points[1].len() < 2 {
        return 0.0;
    }
    let dx = (points[1][0] - points[0][0]) as f32;
    let dy = (points[1][1] - points[0][1]) as f32;
    if dx.abs() < f32::EPSILON {
        return 0.0;
    }
    dy.atan2(dx).to_degrees()
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, ImageFormat, Rgb};
    use std::io::Cursor;

    use crate::ysn_ocr_decode::OcrDetectionBox;

    #[test]
    fn build_line_crop_plan_clamps_padding_to_image_bounds() {
        let detections = vec![OcrDetectionBox {
            box_coords: vec![vec![0, 1], vec![20, 1], vec![20, 10], vec![0, 10]],
            confidence: 0.9,
        }];
        let plan = super::build_line_crop_plan(
            &detections,
            &super::OcrCropPlanConfig {
                image_width: 30,
                image_height: 20,
                padding: 4,
                minimum_width: 2,
                minimum_height: 2,
            },
        )
        .unwrap();
        assert_eq!(plan[0].x, 0);
        assert_eq!(plan[0].y, 0);
        assert_eq!(plan[0].width, 24);
        assert_eq!(plan[0].height, 14);
    }

    #[test]
    fn build_line_crop_plan_keeps_reading_order_after_padding() {
        let detections = vec![
            OcrDetectionBox {
                box_coords: vec![vec![40, 30], vec![60, 30], vec![60, 45], vec![40, 45]],
                confidence: 0.9,
            },
            OcrDetectionBox {
                box_coords: vec![vec![5, 5], vec![35, 5], vec![35, 20], vec![5, 20]],
                confidence: 0.9,
            },
        ];
        let plan = super::build_line_crop_plan(
            &detections,
            &super::OcrCropPlanConfig {
                image_width: 80,
                image_height: 60,
                padding: 2,
                minimum_width: 2,
                minimum_height: 2,
            },
        )
        .unwrap();
        assert_eq!(plan[0].index, 1);
        assert_eq!(plan[1].index, 0);
    }

    #[test]
    fn build_line_crop_plan_estimates_small_rotation() {
        let detections = vec![OcrDetectionBox {
            box_coords: vec![vec![10, 10], vec![30, 12], vec![30, 22], vec![10, 20]],
            confidence: 0.9,
        }];
        let plan = super::build_line_crop_plan(
            &detections,
            &super::OcrCropPlanConfig {
                image_width: 50,
                image_height: 50,
                padding: 0,
                minimum_width: 2,
                minimum_height: 2,
            },
        )
        .unwrap();
        assert!(plan[0].rotation_degrees > 5.0);
        assert!(plan[0].rotation_degrees < 6.0);
    }

    #[test]
    fn crop_line_images_from_bytes_extracts_rgb_crop() {
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(4, 3, |x, y| {
            Rgb([(x * 20) as u8, (y * 30) as u8, ((x + y) * 10) as u8])
        });
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        let plans = vec![super::OcrLineCropPlan {
            index: 7,
            source_box: vec![vec![1, 1], vec![3, 1], vec![3, 2], vec![1, 2]],
            x: 1,
            y: 1,
            width: 2,
            height: 1,
            padding: 0,
            rotation_degrees: 0.0,
        }];

        let crops = super::crop_line_images_from_bytes(&bytes, &plans).unwrap();

        assert_eq!(crops.len(), 1);
        assert_eq!(crops[0].width, 2);
        assert_eq!(crops[0].height, 1);
        assert_eq!(crops[0].color_format, "RGB8");
        assert_eq!(crops[0].rgb_bytes, vec![20, 30, 20, 40, 30, 30]);
    }

    #[test]
    fn crop_line_images_from_bytes_rejects_out_of_bounds_plan() {
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(2, 2, Rgb([0, 0, 0]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        let plans = vec![super::OcrLineCropPlan {
            index: 0,
            source_box: vec![vec![1, 1], vec![4, 1], vec![4, 4], vec![1, 4]],
            x: 1,
            y: 1,
            width: 4,
            height: 4,
            padding: 0,
            rotation_degrees: 0.0,
        }];

        let error = super::crop_line_images_from_bytes(&bytes, &plans).unwrap_err();

        assert!(error.contains("outside source image bounds"));
    }
}
