use axum::{extract::Query, extract::Path, extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path as FsPath, PathBuf};

use crate::adapters::http::state::HttpState;
use crate::application::dto::ConfigurePipelineRequest;

#[derive(Deserialize)]
pub struct FileQuery {
    path: Option<String>,
}

fn get_video_path(idx: u32) -> String {
    format!("/dev/video{idx}")
}

fn model_root() -> PathBuf {
    std::env::var("MODEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn safe_join(root: &PathBuf, user_path: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root).map_err(|e| format!("MODEL_ROOT invalid: {e}"))?;
    let candidate = if user_path.trim().is_empty() {
        root.clone()
    } else {
        root.join(user_path)
    };
    let candidate = fs::canonicalize(&candidate).map_err(|e| format!("path invalid: {e}"))?;
    if !candidate.starts_with(&root) {
        return Err("path outside MODEL_ROOT".into());
    }
    Ok(candidate)
}

pub async fn list_files(Query(query): Query<FileQuery>) -> impl IntoResponse {
    let root = model_root();
    let user_path = query.path.unwrap_or_default();

    let current_path = match safe_join(&root, &user_path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e, "model_root": root.to_string_lossy() })),
            )
                .into_response();
        }
    };

    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&current_path) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_dir = path.is_dir();
            if is_dir || name.ends_with(".onnx") {
                // devolvemos path relativo al root para que el frontend no tenga que manejar rutas absolutas
                let rel = path.strip_prefix(&fs::canonicalize(&root).unwrap_or(root.clone())).ok();
                let rel_str = rel
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.clone());

                entries.push(json!({
                    "name": name,
                    "path": rel_str,
                    "is_dir": is_dir
                }));
            }
        }
    }

    Json(json!({
        "model_root": fs::canonicalize(&root).unwrap_or(root).to_string_lossy(),
        "current_path": current_path.to_string_lossy(),
        "entries": entries
    }))
    .into_response()
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
            let res: Vec<_> = cameras
                .into_iter()
                .map(|c| {
                    let idx = c
                        .id
                        .path
                        .chars()
                        .filter(|ch| ch.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u32>()
                        .unwrap_or(0);
                    json!({ "index": idx, "card": c.card, "path": c.id.path })
                })
                .collect();
            Json(res).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_modes_by_index(State(st): State<HttpState>, Path(idx): Path<u32>) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId {
        path: get_video_path(idx),
    };
    let formats = st.camera.list_formats(cam.clone()).await.unwrap_or_default();
    let sizes = if let Some(f) = formats.iter().find(|f| f.fourcc == "MJPG").or(formats.first()) {
        st.camera
            .list_frame_sizes(cam, f.fourcc.clone())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    Json(json!({
        "formats": formats,
        "frame_sizes": sizes,
        "fps_options": [15, 30, 60]
    }))
    .into_response()
}

pub async fn list_controls_by_index(State(st): State<HttpState>, Path(idx): Path<u32>) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId {
        path: get_video_path(idx),
    };
    match st.camera.list_controls(cam).await {
        Ok(ctrls) => Json(ctrls).into_response(),
        Err(_) => Json(json!([])).into_response(),
    }
}

pub async fn set_controls_by_index(
    State(st): State<HttpState>,
    Path(idx): Path<u32>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let cam = crate::domain::camera::CameraId {
        path: get_video_path(idx),
    };
    if let Some(values) = req.get("values").and_then(|v| v.as_array()) {
        let mut sets = Vec::new();
        for item in values {
            if let Some(arr) = item.as_array() {
                sets.push(crate::domain::camera::SetControl {
                    id: arr[0].as_u64().unwrap_or(0) as u32,
                    value: arr[1].as_i64().unwrap_or(0),
                });
            }
        }
        let _ = st.camera.set_controls(cam, sets).await;
    }
    Json(json!({ "ok": true })).into_response()
}

pub async fn apply_config(State(st): State<HttpState>, Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let idx = req["camera_index"].as_u64().unwrap_or(0) as u32;

    // Si el frontend usa /api/files ahora se devuelven rutas relativas a MODEL_ROOT.
    // Para compatibilidad: si el path es relativo lo resolvemos contra MODEL_ROOT.
    let model_path_raw = req["model_path"].as_str().unwrap_or("").to_string();
    let model_path = if FsPath::new(&model_path_raw).is_absolute() {
        model_path_raw
    } else {
        model_root().join(&model_path_raw).to_string_lossy().to_string()
    };

    let (cam, mode, infer) = ConfigurePipelineRequest {
        camera_path: get_video_path(idx),
        fourcc: req["fourcc"].as_str().unwrap_or("MJPG").to_string(),
        width: req["width"].as_u64().unwrap_or(640) as u32,
        height: req["height"].as_u64().unwrap_or(480) as u32,
        fps: req["fps"].as_u64().unwrap_or(30) as u32,
        model_name: "yolo".to_string(),
        onnx_path: model_path,
        yolo: crate::domain::model::YoloParams {
            input_size: req["imgsz"].as_u64().unwrap_or(640) as u32,
            conf_threshold: req["conf_thres"].as_f64().unwrap_or(0.25) as f32,
            iou_threshold: req["iou_thres"].as_f64().unwrap_or(0.45) as f32,
            max_detections: req["max_det"].as_u64().unwrap_or(100) as usize,
        },
    }
    .into();

    match st.pipeline.configure(cam, mode, infer).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}