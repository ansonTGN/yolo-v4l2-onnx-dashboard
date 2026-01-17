// src/domain/camera.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraId { pub path: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraInfo {
    pub id: CameraId,
    pub name: String,
    pub card: String,
    // Añadidos para que coincida con el repo:
    pub driver: String,
    pub bus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelFormat {
    pub fourcc: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraMode {
    pub format: String,
    pub size: FrameSize,
    pub fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlKind { Integer, Boolean, Menu, Button, Other(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMenuItem {
    pub index: u32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraControl {
    pub id: u32,
    pub name: String,
    pub kind: ControlKind,
    pub minimum: i64,
    pub maximum: i64,
    pub step: i64,
    pub current_value: i64,
    pub default_value: i32,
    pub flags: u32,
    pub menu_items: Option<Vec<ControlMenuItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetControl {
    pub id: u32,
    pub value: i64,
}