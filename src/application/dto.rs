use serde::{Deserialize, Serialize};

use crate::domain::{
    camera::{CameraId, CameraMode},
    model::{InferenceConfig, ModelId, YoloParams},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetModeRequest {
    pub camera_path: String,
    pub fourcc: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl From<SetModeRequest> for (CameraId, CameraMode) {
    fn from(r: SetModeRequest) -> Self {
        (
            CameraId { path: r.camera_path },
            CameraMode {
                format: r.fourcc,
                size: crate::domain::camera::FrameSize {
                    width: r.width,
                    height: r.height,
                },
                fps: r.fps,
            },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurePipelineRequest {
    pub camera_path: String,
    pub fourcc: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,

    pub model_name: String,
    pub onnx_path: String,
    pub yolo: YoloParams,
}

impl From<ConfigurePipelineRequest> for (CameraId, CameraMode, InferenceConfig) {
    fn from(r: ConfigurePipelineRequest) -> Self {
        let cam = CameraId { path: r.camera_path };
        let mode = CameraMode {
            format: r.fourcc,
            size: crate::domain::camera::FrameSize {
                width: r.width,
                height: r.height,
            },
            fps: r.fps,
        };
        let infer = InferenceConfig {
            model: ModelId {
                name: r.model_name,
                onnx_path: r.onnx_path,
            },
            params: r.yolo,
        };
        (cam, mode, infer)
    }
}
