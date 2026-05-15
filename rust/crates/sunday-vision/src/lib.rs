use ort::session::Session;
use image::{DynamicImage, GenericImageView};
use ndarray::Array4;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum VisionError {
    #[error("Inference engine error: {0}")]
    Ort(#[from] ort::Error),
    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Model file not found: {0}")]
    ModelNotFound(String),
}

pub struct VisionEngine {
    session: Session,
}

impl VisionEngine {
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self, VisionError> {
        if !model_path.as_ref().exists() {
            return Err(VisionError::ModelNotFound(model_path.as_ref().display().to_string()));
        }

        let session = Session::builder()?
            .commit_from_file(model_path)?;

        Ok(Self { session })
    }

    /// Run object detection (YOLO style) on a screenshot
    pub fn detect_objects(&mut self, image_data: &[u8]) -> Result<serde_json::Value, VisionError> {
        let img = image::load_from_memory(image_data)?;
        let (width, height) = img.dimensions();
        
        // Preprocess: Resize to 640x640 (standard for YOLOv8/v10)
        let resized = img.resize_exact(640, 640, image::imageops::FilterType::Lanczos3);
        let input_tensor = self.image_to_tensor(&resized)?;

        // Run inference
        let input_value = ort::value::Value::from_array(input_tensor)?;
        let _outputs = self.session.run(ort::inputs!["images" => input_value])?;
        
        // Post-process (Placeholder logic - in reality, we'd parse boxes, scores, class_ids)
        // For now, we return a success signal and image dimensions
        Ok(serde_json::json!({
            "status": "success",
            "original_size": { "width": width, "height": height },
            "message": "Vision inference complete (Result parsing in progress)"
        }))
    }

    fn image_to_tensor(&self, img: &DynamicImage) -> Result<Array4<f32>, VisionError> {
        let mut array = Array4::zeros((1, 3, 640, 640));
        for (x, y, pixel) in img.pixels() {
            let [r, g, b, _] = pixel.0;
            array[[0, 0, y as usize, x as usize]] = r as f32 / 255.0;
            array[[0, 1, y as usize, x as usize]] = g as f32 / 255.0;
            array[[0, 2, y as usize, x as usize]] = b as f32 / 255.0;
        }
        Ok(array)
    }
}

pub fn version() -> &'static str {
    "0.1.0-native-vision"
}
