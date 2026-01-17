use async_trait::async_trait;
use crate::application::ports::CameraControlPort;
use crate::domain::camera::*;
use crate::domain::errors::{DomainError, DomainResult};

pub struct V4l2CameraControl;

impl V4l2CameraControl {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl CameraControlPort for V4l2CameraControl {
    async fn set_controls(&self, camera: &CameraId, values: Vec<SetControl>) -> DomainResult<()> {
        let dev = v4l::Device::with_path(&camera.path)
            .map_err(|e| DomainError::OperationFailed(format!("Error al abrir {}: {e}", camera.path)))?;

        let mut ctrls = Vec::new();
        for v in values {
            ctrls.push(v4l::control::Control {
                id: v.id,
                value: v4l::control::Value::Integer(v.value),
            });
        }

        dev.set_controls(ctrls)
            .map_err(|e| DomainError::OperationFailed(format!("Error en set_controls: {e}")))?;

        Ok(())
    }
    
    // El método set_mode ha sido eliminado de aquí porque ya no forma parte del trait CameraControlPort
}
