use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelId {
    pub name: String,       // logical name, e.g. "yolo11m"
    pub onnx_path: String,  // filesystem path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoloParams {
    pub input_size: u32,        // 640 typical
    pub conf_threshold: f32,    // 0..1
    pub iou_threshold: f32,     // 0..1
    pub max_detections: usize,  // e.g. 300
}

impl Default for YoloParams {
    fn default() -> Self {
        Self {
            input_size: 640,
            conf_threshold: 0.25,
            iou_threshold: 0.45,
            max_detections: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model: ModelId,
    pub params: YoloParams,
}
