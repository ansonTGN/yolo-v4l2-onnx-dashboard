use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    application::ports::{CameraCatalogPort, CameraControlPort, ModelCatalogPort, StreamPort},
    domain::{
        camera::{CameraControl, CameraId, CameraInfo, CameraMode, FrameSize, PixelFormat, SetControl},
        errors::DomainResult,
        model::InferenceConfig,
        stream::FrameMeta,
    },
};

/// Servicio encargado de la gestión de dispositivos físicos de captura.
/// Permite listar cámaras, consultar sus capacidades y ajustar parámetros de hardware.
#[derive(Clone)]
pub struct CameraService {
    catalog: Arc<dyn CameraCatalogPort>,
    control: Arc<dyn CameraControlPort>,
}

impl CameraService {
    pub fn new(catalog: Arc<dyn CameraCatalogPort>, control: Arc<dyn CameraControlPort>) -> Self {
        Self { catalog, control }
    }

    pub async fn list_cameras(&self) -> DomainResult<Vec<CameraInfo>> {
        self.catalog.list_cameras().await
    }

    pub async fn list_formats(&self, camera: CameraId) -> DomainResult<Vec<PixelFormat>> {
        self.catalog.list_formats(&camera).await
    }

    pub async fn list_frame_sizes(
        &self,
        camera: CameraId,
        fourcc: String,
    ) -> DomainResult<Vec<FrameSize>> {
        self.catalog.list_frame_sizes(&camera, &fourcc).await
    }

    pub async fn list_controls(&self, camera: CameraId) -> DomainResult<Vec<CameraControl>> {
        self.catalog.list_controls(&camera).await
    }

    pub async fn set_controls(&self, camera: CameraId, values: Vec<SetControl>) -> DomainResult<()> {
        self.control.set_controls(&camera, values).await
    }
}

/// Orquestador del pipeline (captura + inferencia).
#[derive(Clone)]
pub struct PipelineService {
    stream: Arc<dyn StreamPort>,
    model_catalog: Arc<dyn ModelCatalogPort>,
}

impl PipelineService {
    pub fn new(stream: Arc<dyn StreamPort>, model_catalog: Arc<dyn ModelCatalogPort>) -> Self {
        Self {
            stream,
            model_catalog,
        }
    }

    /// Configura el pipeline completo.
    /// Antes de aplicar la configuración, valida que el modelo seleccionado sea válido.
    pub async fn configure(
        &self,
        camera: CameraId,
        mode: CameraMode,
        infer: InferenceConfig,
    ) -> DomainResult<()> {
        // Validación preventiva antes de arrancar el hardware
        self.model_catalog.validate_model(&infer.model).await?;

        // Delegar la configuración al adaptador de stream (PipelineAdapter)
        self.stream.configure(camera, mode, infer).await
    }

    /// Proporciona un receptor para el canal de difusión (broadcast)
    /// donde se publican los frames procesados y los metadatos.
    pub async fn subscribe(&self) -> DomainResult<broadcast::Receiver<(FrameMeta, Vec<u8>)>> {
        self.stream.subscribe().await
    }
}
