use std::sync::Arc;
use crate::application::services::{CameraService, PipelineService};

/// Estado compartido para los manejadores HTTP de Axum.
/// Siguiendo la Arquitectura Hexagonal, el estado contiene los servicios (Casos de Uso).
#[derive(Clone)]
pub struct HttpState {
    /// Servicio para gestionar inventario y controles de hardware de cámaras.
    pub camera: Arc<CameraService>,
    /// Servicio para orquestar el flujo de captura e inferencia.
    pub pipeline: Arc<PipelineService>,
}
