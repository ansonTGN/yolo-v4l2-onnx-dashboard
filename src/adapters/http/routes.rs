use axum::{extract::{Path, State, Query}, http::StatusCode, response::IntoResponse, Json};
use crate::adapters::http::state::HttpState;
use crate::application::dto::ConfigurePipelineRequest;
use serde::Deserialize;
use serde_json::json;
use std::fs;

#[derive(Deserialize)]
pub struct FileQuery { path: Option<String> }

fn get_video_path(idx: u32) -> String { format!("/dev/video{}", idx) }

pub async fn list_files(Query(query): Query<FileQuery>) -> impl IntoResponse {
    let current_path = query.path.unwrap_or_else(|| ".".into());
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(&current_path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            if is_dir || name.ends_with(".onnx") {
                entries.push(json!({
                    "name": name,
                    "path": path.to_string_lossy().to_string(),
                    "is_dir": is_dir
                }));
            }
        }
    }
    Json(json!({
        "current_path": fs::canonicalize(&current_path).unwrap_or(current_path.into()).to_string_lossy(),
        "entries": entries
    }))
}

pub async fn get_config() -> impl IntoResponse {
    Json(json!({
        "camera_index": 0,
        "fourcc": "MJPG",
        "width": 640,
        "height": 480,
        "fps": 30,
        "model_path": "models/yolo11n.onnx",
        "imgsz": 640,
        "conf_thres": 0.25,
        "iou_thres": 0.45,
        "max_det": 100
    }))
}

pub async fn list_cameras(State(st): State<HttpState>) -> impl IntoResponse {
    match st.camera.list_cameras().await {
        Ok(cameras) => {
            let res: Vec<_> = cameras.into_iter().map(|c| {
                let idx = c.id.path.chars().filter(|ch| ch.is_digit(10)).collect::<String>().parse::<u32>().unwrap_or(0);
                json!({ "index": idx, "card": c.card, "path": c.id.path })
            }).collect();
            Json(res).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_modes_by_index(State(st): State<HttpState>, Path(idx): Path<u32>) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId { path: get_video_path(idx) };
    let formats = st.camera.list_formats(cam.clone()).await.unwrap_or_default();
    let sizes = if let Some(f) = formats.iter().find(|f| f.fourcc == "MJPG").or(formats.get(0)) {
        st.camera.list_frame_sizes(cam, f.fourcc.clone()).await.unwrap_or_default()
    } else { vec![] };

    Json(json!({ "formats": formats, "frame_sizes": sizes, "fps_options": [15, 30, 60] })).into_response()
}

pub async fn list_controls_by_index(State(st): State<HttpState>, Path(idx): Path<u32>) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId { path: get_video_path(idx) };
    match st.camera.list_controls(cam).await {
        Ok(ctrls) => Json(ctrls).into_response(),
        Err(_) => Json(json!([])).into_response(),
    }
}

pub async fn set_controls_by_index(State(st): State<HttpState>, Path(idx): Path<u32>, Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId { path: get_video_path(idx) };
    if let Some(values) = req.get("values").and_then(|v| v.as_array()) {
        let mut sets = Vec::new();
        for item in values {
            if let Some(arr) = item.as_array() {
                sets.push(crate::domain::camera::SetControl { 
                    id: arr[0].as_u64().unwrap_or(0) as u32, 
                    value: arr[1].as_i64().unwrap_or(0) 
                });
            }
        }
        let _ = st.camera.set_controls(cam, sets).await;
    }
    Json(json!({ "ok": true }))
}

pub async fn apply_config(State(st): State<HttpState>, Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let idx = req["camera_index"].as_u64().unwrap_or(0) as u32;
    let (cam, mode, infer) = ConfigurePipelineRequest {
        camera_path: get_video_path(idx),
        fourcc: req["fourcc"].as_str().unwrap_or("MJPG").to_string(),
        width: req["width"].as_u64().unwrap_or(640) as u32,
        height: req["height"].as_u64().unwrap_or(480) as u32,
        fps: req["fps"].as_u64().unwrap_or(30) as u32,
        model_name: "yolo".to_string(),
        onnx_path: req["model_path"].as_str().unwrap_or("").to_string(),
        yolo: crate::domain::model::YoloParams {
            input_size: req["imgsz"].as_u64().unwrap_or(640) as u32,
            conf_threshold: req["conf_thres"].as_f64().unwrap_or(0.25) as f32,
            iou_threshold: req["iou_thres"].as_f64().unwrap_or(0.45) as f32,
            max_detections: req["max_det"].as_u64().unwrap_or(100) as usize,
        }
    }.into();

    match st.pipeline.configure(cam, mode, infer).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}