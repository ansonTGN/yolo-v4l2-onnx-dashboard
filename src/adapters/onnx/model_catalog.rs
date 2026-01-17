use async_trait::async_trait;
use std::path::Path;

use crate::application::ports::ModelCatalogPort;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::model::ModelId;

pub struct OnnxModelCatalog;

impl OnnxModelCatalog {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl ModelCatalogPort for OnnxModelCatalog {
    async fn validate_model(&self, model: &ModelId) -> DomainResult<()> {
        if model.onnx_path.trim().is_empty() {
            return Err(DomainError::InvalidInput("onnx_path empty".into()));
        }
        if !Path::new(&model.onnx_path).exists() {
            return Err(DomainError::NotFound(format!("model file not found: {}", model.onnx_path)));
        }
        Ok(())
    }
}
