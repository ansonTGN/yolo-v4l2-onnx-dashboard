use async_trait::async_trait;
use crate::domain::{camera::*, model::*, stream::FrameMeta, errors::DomainResult};
use tokio::sync::broadcast;

#[async_trait]
pub trait CameraCatalogPort: Send + Sync {
    async fn list_cameras(&self) -> DomainResult<Vec<CameraInfo>>;
    async fn list_formats(&self, camera: &CameraId) -> DomainResult<Vec<PixelFormat>>;
    async fn list_frame_sizes(&self, camera: &CameraId, fourcc: &str) -> DomainResult<Vec<FrameSize>>;
    async fn list_controls(&self, camera: &CameraId) -> DomainResult<Vec<CameraControl>>;
}

#[async_trait]
pub trait CameraControlPort: Send + Sync {
    async fn set_controls(&self, camera: &CameraId, values: Vec<SetControl>) -> DomainResult<()>;
    // Se elimina set_mode de aquí porque el Pipeline usa V4l2Capture directamente
}

#[async_trait]
pub trait ModelCatalogPort: Send + Sync {
    async fn validate_model(&self, model: &ModelId) -> DomainResult<()>;
}

#[async_trait]
pub trait StreamPort: Send + Sync {
    async fn configure(&self, camera: CameraId, mode: CameraMode, infer: InferenceConfig) -> DomainResult<()>;
    async fn subscribe(&self) -> DomainResult<broadcast::Receiver<(FrameMeta, Vec<u8>)>>;
}