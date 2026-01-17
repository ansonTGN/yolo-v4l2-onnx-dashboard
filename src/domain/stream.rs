use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::detection::Detection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMeta {
    pub width: u32,
    pub height: u32,
    pub infer_ms: f32,
    pub fps_est: f32,
    pub detections: Vec<Detection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrameMetaMessage {
    pub r#type: String,
    pub meta: FrameMeta,
}

pub fn summarize_detections(detections: &[Detection]) -> String {
    let mut counts = HashMap::new();
    for det in detections {
        *counts.entry(&det.label).or_insert(0) += 1;
    }
    counts.iter()
        .map(|(label, count)| format!("{} {}", count, label))
        .collect::<Vec<_>>()
        .join(", ")
}
