use async_trait::async_trait;
use v4l::video::Capture;
use v4l::Device;
use crate::application::ports::CameraCatalogPort;
use crate::domain::camera::*;
use crate::domain::errors::{DomainError, DomainResult};

pub struct V4l2CameraCatalog;
impl V4l2CameraCatalog { pub fn new() -> Self { Self } }

#[async_trait]
impl CameraCatalogPort for V4l2CameraCatalog {
    async fn list_cameras(&self) -> DomainResult<Vec<CameraInfo>> {
        let nodes = v4l::context::enum_devices();
        let mut out = Vec::new();
        for node in nodes {
            let path = node.path().to_string_lossy().to_string();
            if let Ok(dev) = Device::with_path(&path) {
                if let Ok(caps) = dev.query_caps() {
                    out.push(CameraInfo {
                        id: CameraId { path },
                        name: node.name().unwrap_or_else(|| "Unknown".to_string()),
                        driver: caps.driver,
                        card: caps.card,
                        bus: caps.bus,
                    });
                }
            }
        }
        Ok(out)
    }

    async fn list_formats(&self, camera: &CameraId) -> DomainResult<Vec<PixelFormat>> {
        let dev = Device::with_path(&camera.path).map_err(|e| DomainError::NotFound(e.to_string()))?;
        let formats = dev.enum_formats().unwrap_or_default();
        Ok(formats.into_iter().map(|f| PixelFormat {
            fourcc: f.fourcc.str().unwrap_or("????").to_string(),
            description: f.description,
        }).collect())
    }

    async fn list_frame_sizes(&self, camera: &CameraId, fourcc: &str) -> DomainResult<Vec<FrameSize>> {
        let dev = Device::with_path(&camera.path).map_err(|e| DomainError::NotFound(e.to_string()))?;
        let fcc_bytes = fourcc.as_bytes();
        if fcc_bytes.len() != 4 { return Ok(vec![]); }
        let fcc = v4l::FourCC::new(&[fcc_bytes[0], fcc_bytes[1], fcc_bytes[2], fcc_bytes[3]]);
        
        let mut out = Vec::new();
        if let Ok(sizes) = dev.enum_framesizes(fcc) {
            for s in sizes {
                for d in s.size.to_discrete() {
                    out.push(FrameSize { width: d.width, height: d.height });
                }
            }
        }
        Ok(out)
    }

    async fn list_controls(&self, camera: &CameraId) -> DomainResult<Vec<CameraControl>> {
        let dev = Device::with_path(&camera.path).map_err(|e| DomainError::NotFound(e.to_string()))?;
        let descs = dev.query_controls().unwrap_or_default();
        let mut out = Vec::new();
        for d in descs {
            let current_value = dev.control(d.id).map(|c| match c.value {
                v4l::control::Value::Integer(v) => v,
                v4l::control::Value::Boolean(v) => if v { 1 } else { 0 },
                _ => 0,
            }).unwrap_or(0);

            out.push(CameraControl {
                id: d.id, name: d.name, kind: ControlKind::Integer,
                minimum: d.minimum, maximum: d.maximum, step: d.step as i64,
                current_value, default_value: d.default as i32,
                flags: d.flags.bits(), menu_items: None,
            });
        }
        Ok(out)
    }
}