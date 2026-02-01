use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::{prelude::BASE64_STANDARD, Engine};
use image::ImageFormat;
use serde_json::json;
use tracing::{error, info, warn};

use crate::domain::detection::Detection;
use crate::domain::stream::summarize_detections;

pub struct SpeechService {
    last_spoken: Arc<Mutex<Instant>>,
    min_interval: Duration,
    client: reqwest::Client,
    ollama_url: String,
    model_name: String,
    tokio_handle: tokio::runtime::Handle,
    is_ready: Arc<Mutex<bool>>,
}

impl SpeechService {
    pub fn new(interval_secs: u64, handle: tokio::runtime::Handle) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let svc = Self {
            last_spoken: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(interval_secs))),
            min_interval: Duration::from_secs(interval_secs),
            client,
            ollama_url: "http://localhost:11434".to_string(),
            model_name: "moondream:latest".to_string(),
            tokio_handle: handle,
            is_ready: Arc::new(Mutex::new(false)),
        };

        let svc_clone = svc.clone_internal();
        svc.tokio_handle.spawn(async move {
            svc_clone.check_ollama_status().await;
        });

        svc
    }

    fn clone_internal(&self) -> Self {
        Self {
            last_spoken: self.last_spoken.clone(),
            min_interval: self.min_interval,
            client: self.client.clone(),
            ollama_url: self.ollama_url.clone(),
            model_name: self.model_name.clone(),
            tokio_handle: self.tokio_handle.clone(),
            is_ready: self.is_ready.clone(),
        }
    }

    async fn check_ollama_status(&self) {
        let url = format!("{}/api/tags", self.ollama_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                info!("✅ Ollama Moondream ready.");
                let mut ready = self.is_ready.lock().unwrap();
                *ready = true;
            }
        }
    }

    pub fn process_frame(&self, detections: Vec<Detection>, image_data: Vec<u8>) {
        if detections.is_empty() || !*self.is_ready.lock().unwrap() {
            return;
        }

        let mut last_spoken = self.last_spoken.lock().unwrap();
        if Instant::now().duration_since(*last_spoken) < self.min_interval {
            return;
        }
        *last_spoken = Instant::now();

        let client = self.client.clone();
        let url = format!("{}/api/generate", self.ollama_url);
        let model = self.model_name.clone();

        self.tokio_handle.spawn(async move {
            let optimized_base64 = tokio::task::spawn_blocking(move || {
                if let Ok(img) = image::load_from_memory_with_format(&image_data, ImageFormat::Jpeg) {
                    let scaled = img.thumbnail(640, 640);
                    let mut buf = Vec::new();
                    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
                    if encoder.encode_image(&scaled).is_ok() {
                        return Some(BASE64_STANDARD.encode(buf));
                    }
                }
                None
            })
            .await
            .ok()
            .flatten();

            let Some(base64_image) = optimized_base64 else {
                return;
            };

            let det_summary = summarize_detections(&detections);
            let prompt = if det_summary.is_empty() {
                "Describe this image in one short sentence.".to_string()
            } else {
                format!(
                    "Describe this image in one short sentence. Focus on: {}",
                    det_summary
                )
            };

            info!("🔍 Analyzing scene...");

            let body = json!({
                "model": model,
                "prompt": prompt,
                "images": [base64_image],
                "stream": false,
                "options": {
                    "temperature": 0.0,
                    "num_predict": 30
                }
            });

            match client.post(&url).json(&body).send().await {
                Ok(res) => {
                    let status = res.status();
                    if let Ok(json_resp) = res.json::<serde_json::Value>().await {
                        if let Some(text) = json_resp["response"].as_str() {
                            let clean_text = text.trim().replace('"', "");
                            if !clean_text.is_empty() && clean_text.len() > 5 {
                                speak_with_piper(&clean_text);
                            } else {
                                warn!("⚠️ Ollama returned empty/short response: {:?}", json_resp);
                            }
                        } else {
                            error!("❌ Unexpected JSON format from Ollama: {:?}", json_resp);
                        }
                    } else {
                        error!("❌ Failed to parse Ollama JSON response. Status: {}", status);
                    }
                }
                Err(e) => error!("❌ Ollama Error: {}", e),
            }
        });
    }
}

fn speak_with_piper(text: &str) {
    let text = text.to_string();
    info!("🎙️ Narrating: {}", text);

    std::thread::spawn(move || {
        let piper_path = "./piper_voice/piper/piper";
        let model_path = "./piper_voice/en_US-lessac-medium.onnx";

        if !std::path::Path::new(model_path).exists() {
            error!("❌ VOICE MODEL NOT FOUND: {}", model_path);
            return;
        }

        let mut piper = match Command::new(piper_path)
            .args(["--model", model_path, "--output_raw"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(p) => p,
            Err(e) => {
                error!("❌ Failed to start Piper: {}", e);
                return;
            }
        };

        if let Some(mut stdin) = piper.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            drop(stdin);
        }

        if let Some(stdout) = piper.stdout.take() {
            let _ = Command::new("aplay")
                .args(["-r", "22050", "-f", "S16_LE", "-t", "raw"])
                .stdin(stdout)
                .spawn();
        }
    });
}