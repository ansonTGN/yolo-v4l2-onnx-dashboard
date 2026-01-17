use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::application::ports::StreamPort;
use crate::application::speech_service::SpeechService; 
use crate::domain::{
    camera::{CameraId, CameraMode},
    errors::{DomainError, DomainResult},
    model::InferenceConfig,
    stream::FrameMeta,
};

use crate::adapters::v4l2::capture::{CaptureConfig, V4l2Capture};
use crate::adapters::onnx::yolo_engine::OnnxYoloEngine;

pub struct PipelineAdapter {
    cfg: Arc<RwLock<Option<PipelineConfig>>>,
    tx: broadcast::Sender<(FrameMeta, Vec<u8>)>,
}

#[derive(Clone)]
struct PipelineConfig { 
    camera: CameraId, 
    mode: CameraMode, 
    infer: InferenceConfig 
}

impl PipelineAdapter {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        
        // Capturamos el handle de Tokio para que el SpeechService 
        // pueda realizar peticiones HTTP asíncronas a Ollama.
        let tokio_handle = tokio::runtime::Handle::current();

        let adapter = Self { 
            cfg: Arc::new(RwLock::new(None)), 
            tx 
        };
        
        adapter.spawn_worker(tokio_handle);
        adapter
    }

    fn spawn_worker(&self, tokio_handle: tokio::runtime::Handle) {
        let cfg_handle = self.cfg.clone();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            // Inicializamos el servicio de voz (intervalo de 12 segundos entre narraciones)
            let speech_service = SpeechService::new(12, tokio_handle);
            
            let mut capture: Option<V4l2Capture> = None;
            let mut engine: Option<OnnxYoloEngine> = None;
            let mut last_key: Option<String> = None;
            
            let mut fps_est: f32 = 0.0;
            let mut last_t = std::time::Instant::now();

            info!("Pipeline Worker: Hilo de procesamiento y visión iniciado.");

            loop {
                // 1. Obtener configuración actual
                let current = {
                    let lock = cfg_handle.read().unwrap();
                    lock.clone()
                };

                let Some(current) = current else {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    continue;
                };

                // 2. Comprobar si hay cambios en cámara o modelo
                let config_key = format!("{}-{}-{}", 
                    current.camera.path, 
                    current.mode.size.width, 
                    current.infer.model.onnx_path
                );

                if Some(config_key.clone()) != last_key {
                    info!("Pipeline: Recargando recursos para {}", config_key);
                    
                    capture = V4l2Capture::open(&CaptureConfig {
                        camera_path: current.camera.path.clone(),
                        fourcc: current.mode.format.clone(),
                        width: current.mode.size.width,
                        height: current.mode.size.height,
                        fps: current.mode.fps,
                    }).map_err(|e| error!("Error abriendo cámara: {:?}", e)).ok();

                    engine = OnnxYoloEngine::load(&current.infer.model.onnx_path)
                        .map_err(|e| error!("Error cargando modelo YOLO: {:?}", e)).ok();
                    
                    last_key = Some(config_key);
                }

                // 3. Captura e Inferencia
                if let (Some(cap), Some(eng)) = (capture.as_mut(), engine.as_mut()) {
                    match cap.next_rgb_and_jpeg() {
                        Ok((rgb, jpeg, w, h)) => {
                            let t_infer_start = std::time::Instant::now();
                            
                            // Inferencia YOLO para obtener cajas y etiquetas
                            let detections = eng.infer(&rgb, &current.infer.params)
                                .unwrap_or_default();
                            
                            let infer_ms = t_infer_start.elapsed().as_secs_f32() * 1000.0;

                            // --- MEJORA AVANZADA: PROCESAMIENTO VISUAL ---
                            // Enviamos las detecciones Y el frame JPEG al servicio de voz.
                            // Esto permite que el servicio use un VLM (Vision Language Model) 
                            // para ver qué está pasando realmente.
                            speech_service.process_frame(detections.clone(), jpeg.clone());

                            // Cálculo de FPS para la interfaz
                            let dt = last_t.elapsed().as_secs_f32().max(0.001);
                            last_t = std::time::Instant::now();
                            fps_est = 0.9 * fps_est + 0.1 * (1.0 / dt);

                            let meta = FrameMeta { 
                                width: w, 
                                height: h, 
                                infer_ms, 
                                fps_est, 
                                detections 
                            };
                            
                            // 4. Enviar resultado al Dashboard vía WebSocket
                            if tx.receiver_count() > 0 {
                                let _ = tx.send((meta, jpeg));
                            }
                        }
                        Err(e) => {
                            warn!("Error capturando frame: {}", e);
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }
        });
    }
}

#[async_trait]
impl StreamPort for PipelineAdapter {
    async fn configure(&self, camera: CameraId, mode: CameraMode, infer: InferenceConfig) -> DomainResult<()> {
        let mut lock = self.cfg.write()
            .map_err(|_| DomainError::OperationFailed("Lock de configuración fallido".into()))?;
        *lock = Some(PipelineConfig { camera, mode, infer });
        Ok(())
    }

    async fn subscribe(&self) -> DomainResult<broadcast::Receiver<(FrameMeta, Vec<u8>)>> {
        Ok(self.tx.subscribe())
    }
}
