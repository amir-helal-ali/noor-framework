// ============================================================
// Example: Real-time Chat with WebSocket
// مثال: دردشة فورية باستخدام WebSocket
// ============================================================

use noor::*;
use noor::core::websocket::{WebSocketServer, WsMessage};
use std::sync::Arc;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    
    let ws_server = Arc::new(WebSocketServer::new());
    
    // Broadcast welcome message
    let welcome = WsMessage::new("welcome", serde_json::json!({
        "message": "Welcome to Noor Chat!"
    }));
    ws_server.broadcast(&welcome);
    
    println!("WebSocket chat server started");
    println!("Connect to ws://localhost:8080/ws");
    
    Ok(())
}
