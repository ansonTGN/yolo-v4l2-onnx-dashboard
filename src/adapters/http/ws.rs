use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::extract::State;
use crate::adapters::http::state::HttpState;
use crate::domain::stream::WsFrameMetaMessage;

pub async fn ws_handler(ws: WebSocketUpgrade, State(st): State<HttpState>) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, st))
}

async fn handle_socket(mut socket: WebSocket, st: HttpState) {
    let mut rx = match st.pipeline.subscribe().await {
        Ok(r) => r,
        Err(_) => return,
    };

    while let Ok((meta, jpeg)) = rx.recv().await {
        let json = serde_json::to_string(&WsFrameMetaMessage { r#type: "frame".into(), meta }).unwrap_or_default();
        
        if socket.send(Message::Text(json.into())).await.is_err() { break; }
        if socket.send(Message::Binary(jpeg.into())).await.is_err() { break; }
    }
}

