mod domain;
mod application;
mod adapters;

use std::sync::Arc;
use tower_http::services::ServeDir;
use crate::application::services::{CameraService, PipelineService};
use crate::adapters::{
    v4l2::{camera_repo::V4l2CameraCatalog, control_repo::V4l2CameraControl},
    onnx::{model_catalog::OnnxModelCatalog, pipeline::PipelineAdapter},
    http::{state::HttpState, router},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Inicializar logs (RUST_LOG=info por defecto)
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    tracing::info!("🔧 Inicializando adaptadores de infraestructura...");

    // 2. Instanciar Adaptadores (Capa de Infraestructura)
    // Usamos Arc porque serán compartidos entre servicios y el servidor HTTP.
    let camera_cat = Arc::new(V4l2CameraCatalog::new());
    let camera_ctrl = Arc::new(V4l2CameraControl::new());
    let model_cat = Arc::new(OnnxModelCatalog::new());
    let pipeline_adapter = Arc::new(PipelineAdapter::new());

    // 3. Instanciar Servicios (Capa de Aplicación - Casos de Uso)
    let camera_service = Arc::new(CameraService::new(camera_cat, camera_ctrl));
    let pipeline_service = Arc::new(PipelineService::new(pipeline_adapter, model_cat));

    // 4. Configurar el Estado de la API
    let state = HttpState {
        camera: camera_service,
        pipeline: pipeline_service,
    };

    // 5. Configurar el Router de Axum y Archivos Estáticos
    // 'router(state)' viene de src/adapters/http/mod.rs
    let app = router(state)
        .fallback_service(ServeDir::new("static"));

    // 6. Lanzar el Servidor
    let port = 8090;
    let addr = format!("0.0.0.0:{}", port);
    
    tracing::info!("🚀 Servidor YOLO iniciado en http://{}", addr);
    tracing::info!("📂 Archivos estáticos servidos desde la carpeta './static'");
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

