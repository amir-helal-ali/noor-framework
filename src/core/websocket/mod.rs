// ============================================================
// WebSocket Module - وحدة WebSocket
// ============================================================
// Real-time bidirectional communication support.
// Supports channels, rooms, and broadcast messaging.
//
// دعم الاتصال ثنائي الاتجاه الفوري.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

/// WebSocket connection ID
pub type ConnectionId = String;

/// A WebSocket message
/// رسالة WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub event: String,
    pub data: serde_json::Value,
    pub channel: Option<String>,
    pub timestamp: i64,
}

impl WsMessage {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self {
            event: event.to_string(),
            data,
            channel: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
    
    pub fn to_json(&self) -> crate::NoorResult<String> {
        Ok(serde_json::to_string(self)?)
    }
    
    pub fn from_json(json: &str) -> crate::NoorResult<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// A WebSocket connection
/// اتصال WebSocket
pub struct Connection {
    pub id: ConnectionId,
    pub user_id: Option<String>,
    pub channels: Vec<String>,
    pub sender: mpsc::UnboundedSender<String>,
    pub connected_at: i64,
    pub last_ping: i64,
}

impl Connection {
    pub fn new(id: ConnectionId, sender: mpsc::UnboundedSender<String>) -> Self {
        Self {
            id,
            user_id: None,
            channels: Vec::new(),
            sender,
            connected_at: chrono::Utc::now().timestamp(),
            last_ping: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Send a message to this connection
    /// إرسال رسالة لهذا الاتصال
    pub fn send(&self, message: &WsMessage) -> crate::NoorResult<()> {
        let json = message.to_json()?;
        self.sender.send(json).map_err(|_| {
            crate::NoorError::Internal("Failed to send WebSocket message".to_string())
        })
    }
    
    /// Join a channel
    /// الانضمام لقناة
    pub fn join(&mut self, channel: &str) {
        if !self.channels.contains(&channel.to_string()) {
            self.channels.push(channel.to_string());
        }
    }
    
    /// Leave a channel
    /// مغادرة قناة
    pub fn leave(&mut self, channel: &str) {
        self.channels.retain(|c| c != channel);
    }
    
    /// Update ping timestamp
    pub fn ping(&mut self) {
        self.last_ping = chrono::Utc::now().timestamp();
    }
}

/// WebSocket server manager
/// مدير خادم WebSocket
pub struct WebSocketServer {
    /// All active connections
    connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
    /// Channel -> Connection IDs mapping
    channels: Arc<RwLock<HashMap<String, Vec<ConnectionId>>>>,
    /// User -> Connection IDs mapping
    user_connections: Arc<RwLock<HashMap<String, Vec<ConnectionId>>>>,
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a new connection
    /// تسجيل اتصال جديد
    pub fn add_connection(&self, connection: Connection) {
        let id = connection.id.clone();
        let user_id = connection.user_id.clone();
        
        if let Some(ref uid) = user_id {
            self.user_connections
                .write()
                .entry(uid.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }
        
        self.connections.write().insert(id, connection);
    }
    
    /// Remove a connection
    /// إزالة اتصال
    pub fn remove_connection(&self, id: &str) {
        if let Some(conn) = self.connections.write().remove(id) {
            // Remove from channels
            let mut channels = self.channels.write();
            for channel in &conn.channels {
                if let Some(conns) = channels.get_mut(channel) {
                    conns.retain(|c| c != id);
                }
            }
            
            // Remove from user connections
            if let Some(ref uid) = conn.user_id {
                if let Some(conns) = self.user_connections.write().get_mut(uid) {
                    conns.retain(|c| c != id);
                }
            }
        }
    }
    
    /// Associate a user with a connection
    /// ربط مستخدم باتصال
    pub fn set_user(&self, connection_id: &str, user_id: &str) {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.user_id = Some(user_id.to_string());
        }
        drop(connections);
        
        self.user_connections
            .write()
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(connection_id.to_string());
    }
    
    /// Broadcast a message to all connections
    /// بث رسالة لجميع الاتصالات
    pub fn broadcast(&self, message: &WsMessage) {
        let connections = self.connections.read();
        for conn in connections.values() {
            let _ = conn.send(message);
        }
    }
    
    /// Send a message to a specific connection
    /// إرسال رسالة لاتصال محدد
    pub fn send_to(&self, connection_id: &str, message: &WsMessage) -> crate::NoorResult<()> {
        let connections = self.connections.read();
        match connections.get(connection_id) {
            Some(conn) => conn.send(message),
            None => Err(crate::NoorError::Internal(
                format!("Connection {} not found", connection_id)
            )),
        }
    }
    
    /// Send a message to a specific user (all their connections)
    /// إرسال رسالة لمستخدم محدد
    pub fn send_to_user(&self, user_id: &str, message: &WsMessage) {
        let user_conns = self.user_connections.read().get(user_id).cloned();
        if let Some(conn_ids) = user_conns {
            let connections = self.connections.read();
            for id in conn_ids {
                if let Some(conn) = connections.get(&id) {
                    let _ = conn.send(message);
                }
            }
        }
    }
    
    /// Broadcast a message to a channel
    /// بث رسالة لقناة
    pub fn broadcast_to_channel(&self, channel: &str, message: &WsMessage) {
        let mut msg = message.clone();
        msg.channel = Some(channel.to_string());
        
        let channel_conns = self.channels.read().get(channel).cloned();
        if let Some(conn_ids) = channel_conns {
            let connections = self.connections.read();
            for id in conn_ids {
                if let Some(conn) = connections.get(&id) {
                    let _ = conn.send(&msg);
                }
            }
        }
    }
    
    /// Join a channel
    /// الانضمام لقناة
    pub fn join_channel(&self, connection_id: &str, channel: &str) {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.join(channel);
        }
        drop(connections);
        
        self.channels
            .write()
            .entry(channel.to_string())
            .or_insert_with(Vec::new)
            .push(connection_id.to_string());
    }
    
    /// Leave a channel
    /// مغادرة قناة
    pub fn leave_channel(&self, connection_id: &str, channel: &str) {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.leave(channel);
        }
        drop(connections);
        
        if let Some(conns) = self.channels.write().get_mut(channel) {
            conns.retain(|c| c != connection_id);
        }
    }
    
    /// Get the number of active connections
    /// عدد الاتصالات النشطة
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }
    
    /// Get the number of connections in a channel
    /// عدد الاتصالات في قناة
    pub fn channel_count(&self, channel: &str) -> usize {
        self.channels.read().get(channel).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Clean up stale connections (call periodically)
    /// تنظيف الاتصالات القديمة
    pub fn cleanup_stale(&self, timeout_secs: i64) {
        let now = chrono::Utc::now().timestamp();
        let stale_ids: Vec<String> = self.connections
            .read()
            .iter()
            .filter(|(_, conn)| now - conn.last_ping > timeout_secs)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in stale_ids {
            self.remove_connection(&id);
        }
    }
    
    /// Get all active channels
    /// الحصول على جميع القنوات النشطة
    pub fn get_channels(&self) -> Vec<String> {
        self.channels.read().keys().cloned().collect()
    }
    
    /// Get all connection IDs
    /// الحصول على جميع معرفات الاتصال
    pub fn get_connection_ids(&self) -> Vec<String> {
        self.connections.read().keys().cloned().collect()
    }
}

/// WebSocket event handler
/// معالج أحداث WebSocket
pub type WsEventHandler = Arc<dyn Fn(&WsMessage, &Connection) -> crate::NoorResult<()> + Send + Sync>;

/// Event dispatcher for WebSocket messages
/// موزع الأحداث لرسائل WebSocket
pub struct WsEventDispatcher {
    handlers: Arc<RwLock<HashMap<String, Vec<WsEventHandler>>>>,
}

impl Default for WsEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl WsEventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an event handler
    /// تسجيل معالج حدث
    pub fn on(&self, event: &str, handler: WsEventHandler) {
        self.handlers
            .write()
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
    }
    
    /// Dispatch a message to handlers
    /// توزيع رسالة للمعالجات
    pub fn dispatch(&self, message: &WsMessage, connection: &Connection) -> crate::NoorResult<()> {
        let handlers = self.handlers.read();
        
        if let Some(event_handlers) = handlers.get(&message.event) {
            for handler in event_handlers {
                handler(message, connection)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ws_message() {
        let msg = WsMessage::new("test", serde_json::json!({"hello": "world"}));
        assert_eq!(msg.event, "test");
        
        let json = msg.to_json().unwrap();
        let parsed = WsMessage::from_json(&json).unwrap();
        assert_eq!(parsed.event, "test");
    }
    
    #[test]
    fn test_websocket_server() {
        let server = WebSocketServer::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        
        let conn = Connection::new("conn1".to_string(), tx);
        server.add_connection(conn);
        
        assert_eq!(server.connection_count(), 1);
        
        server.remove_connection("conn1");
        assert_eq!(server.connection_count(), 0);
    }
}
