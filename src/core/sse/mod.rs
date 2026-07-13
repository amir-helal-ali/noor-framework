// ============================================================
// Server-Sent Events (SSE) - الأحداث المرسلة من الخادم
// ============================================================
// Push real-time updates to clients over HTTP.
// One-way server-to-client streaming with automatic reconnection.
//
// دفع تحديثات فورية للعملاء عبر HTTP.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

/// SSE event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
    pub retry: Option<u32>,
}

impl SseEvent {
    /// Create a new SSE event with data
    pub fn data(data: &str) -> Self {
        Self {
            id: None,
            event: None,
            data: data.to_string(),
            retry: None,
        }
    }
    
    /// Set the event type
    pub fn event_type(mut self, event: &str) -> Self {
        self.event = Some(event.to_string());
        self
    }
    
    /// Set the event ID
    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }
    
    /// Set retry interval (milliseconds)
    pub fn retry(mut self, ms: u32) -> Self {
        self.retry = Some(ms);
        self
    }
    
    /// Create a JSON event
    pub fn json<T: Serialize>(data: &T) -> crate::NoorResult<Self> {
        let json = serde_json::to_string(data)?;
        Ok(Self::data(&json).event_type("message"))
    }
    
    /// Convert to SSE text format
    pub fn to_sse(&self) -> String {
        let mut output = String::new();
        
        if let Some(ref id) = self.id {
            output.push_str(&format!("id: {}\n", id));
        }
        
        if let Some(ref event) = self.event {
            output.push_str(&format!("event: {}\n", event));
        }
        
        // Data can be multiline
        for line in self.data.lines() {
            output.push_str(&format!("data: {}\n", line));
        }
        
        if let Some(retry) = self.retry {
            output.push_str(&format!("retry: {}\n", retry));
        }
        
        output.push_str("\n");
        output
    }
}

/// SSE client connection
pub struct SseClient {
    pub id: String,
    pub sender: mpsc::UnboundedSender<SseEvent>,
    pub subscribed_events: Vec<String>,
    pub last_event_id: Option<String>,
    pub connected_at: i64,
}

impl SseClient {
    pub fn new(id: String, sender: mpsc::UnboundedSender<SseEvent>) -> Self {
        Self {
            id,
            sender,
            subscribed_events: Vec::new(),
            last_event_id: None,
            connected_at: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Send an event to this client
    pub fn send(&self, event: &SseEvent) -> crate::NoorResult<()> {
        self.sender
            .send(event.clone())
            .map_err(|_| crate::NoorError::Internal("SSE client disconnected".to_string()))
    }
    
    /// Subscribe to specific event types
    pub fn subscribe(&mut self, event_type: &str) {
        if !self.subscribed_events.contains(&event_type.to_string()) {
            self.subscribed_events.push(event_type.to_string());
        }
    }
    
    /// Check if client is subscribed to an event type
    pub fn is_subscribed(&self, event_type: &str) -> bool {
        self.subscribed_events.is_empty() || self.subscribed_events.contains(&event_type.to_string())
    }
}

/// SSE channel for grouping clients
pub struct SseChannel {
    pub name: String,
    pub clients: HashMap<String, SseClient>,
}

impl SseChannel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            clients: HashMap::new(),
        }
    }
    
    /// Add a client to this channel
    pub fn add_client(&mut self, client: SseClient) {
        self.clients.insert(client.id.clone(), client);
    }
    
    /// Remove a client
    pub fn remove_client(&mut self, client_id: &str) -> bool {
        self.clients.remove(client_id).is_some()
    }
    
    /// Broadcast an event to all clients in this channel
    pub fn broadcast(&self, event: &SseEvent) -> usize {
        let mut sent = 0;
        
        for client in self.clients.values() {
            if client.is_subscribed(event.event.as_deref().unwrap_or("message")) {
                if client.send(event).is_ok() {
                    sent += 1;
                }
            }
        }
        
        sent
    }
    
    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// SSE server
pub struct SseServer {
    channels: Arc<RwLock<HashMap<String, SseChannel>>>,
    /// Global clients (not in any channel)
    global_clients: Arc<RwLock<HashMap<String, SseClient>>>,
    /// Event counter for generating IDs
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for SseServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SseServer {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            global_clients: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Create a new client connection
    pub fn connect(&self) -> (String, mpsc::UnboundedReceiver<SseEvent>) {
        let id = self.generate_id();
        let (tx, rx) = mpsc::unbounded_channel();
        
        let client = SseClient::new(id.clone(), tx);
        
        self.global_clients.write().insert(id.clone(), client);
        
        (id, rx)
    }
    
    /// Create a client and subscribe to a channel
    pub fn connect_to_channel(&self, channel_name: &str) -> crate::NoorResult<(String, mpsc::UnboundedReceiver<SseEvent>)> {
        let id = self.generate_id();
        let (tx, rx) = mpsc::unbounded_channel();
        
        let client = SseClient::new(id.clone(), tx);
        
        // Add to channel
        let mut channels = self.channels.write();
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| SseChannel::new(channel_name));
        
        channel.add_client(client);
        
        Ok((id, rx))
    }
    
    /// Disconnect a client
    pub fn disconnect(&self, client_id: &str) {
        // Remove from global clients
        self.global_clients.write().remove(client_id);
        
        // Remove from all channels
        let mut channels = self.channels.write();
        for channel in channels.values_mut() {
            channel.remove_client(client_id);
        }
    }
    
    /// Subscribe a client to a channel
    pub fn subscribe(&self, client_id: &str, channel_name: &str) -> crate::NoorResult<()> {
        // Remove from global clients
        let client = self.global_clients.write().remove(client_id)
            .ok_or_else(|| crate::NoorError::Internal("Client not found".to_string()))?;
        
        // Add to channel
        let mut channels = self.channels.write();
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| SseChannel::new(channel_name));
        
        channel.add_client(client);
        
        Ok(())
    }
    
    /// Broadcast to all connected clients
    pub fn broadcast(&self, event: &SseEvent) -> usize {
        let mut sent = 0;
        
        // Send to global clients
        for client in self.global_clients.read().values() {
            if client.send(event).is_ok() {
                sent += 1;
            }
        }
        
        // Send to all channels
        for channel in self.channels.read().values() {
            sent += channel.broadcast(event);
        }
        
        sent
    }
    
    /// Broadcast to a specific channel
    pub fn broadcast_to_channel(&self, channel_name: &str, event: &SseEvent) -> usize {
        let channels = self.channels.read();
        
        channels
            .get(channel_name)
            .map(|ch| ch.broadcast(event))
            .unwrap_or(0)
    }
    
    /// Send to a specific client
    pub fn send_to(&self, client_id: &str, event: &SseEvent) -> crate::NoorResult<()> {
        // Check global clients
        if let Some(client) = self.global_clients.read().get(client_id) {
            return client.send(event);
        }
        
        // Check channels
        for channel in self.channels.read().values() {
            if let Some(client) = channel.clients.get(client_id) {
                return client.send(event);
            }
        }
        
        Err(crate::NoorError::Internal(format!("Client {} not found", client_id)))
    }
    
    /// Get the total number of connected clients
    pub fn client_count(&self) -> usize {
        let global = self.global_clients.read().len();
        let channels: usize = self.channels.read().values().map(|c| c.client_count()).sum();
        global + channels
    }
    
    /// Get the number of channels
    pub fn channel_count(&self) -> usize {
        self.channels.read().len()
    }
    
    /// List all channels
    pub fn list_channels(&self) -> Vec<String> {
        self.channels.read().keys().cloned().collect()
    }
    
    /// Generate a unique client ID
    fn generate_id(&self) -> String {
        let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("sse_{}", count)
    }
    
    /// Create a heartbeat event
    pub fn heartbeat() -> SseEvent {
        SseEvent::data("heartbeat").event_type("ping")
    }
    
    /// Clean up disconnected clients
    pub fn cleanup(&self) -> usize {
        let mut cleaned = 0;
        
        // Clean global clients
        let global_ids: Vec<String> = self.global_clients
            .read()
            .iter()
            .filter(|(_, c)| c.sender.is_closed())
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in global_ids {
            self.global_clients.write().remove(&id);
            cleaned += 1;
        }
        
        // Clean channel clients
        let mut channels = self.channels.write();
        for channel in channels.values_mut() {
            let disconnected: Vec<String> = channel
                .clients
                .iter()
                .filter(|(_, c)| c.sender.is_closed())
                .map(|(id, _)| id.clone())
                .collect();
            
            for id in disconnected {
                channel.remove_client(&id);
                cleaned += 1;
            }
        }
        
        cleaned
    }
}

/// SSE response headers
pub fn sse_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "text/event-stream".to_string());
    headers.insert("Cache-Control".to_string(), "no-cache".to_string());
    headers.insert("Connection".to_string(), "keep-alive".to_string());
    headers.insert("X-Accel-Buffering".to_string(), "no".to_string());
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sse_event_creation() {
        let event = SseEvent::data("Hello, World!")
            .event_type("greeting")
            .id("123");
        
        let sse = event.to_sse();
        
        assert!(sse.contains("id: 123"));
        assert!(sse.contains("event: greeting"));
        assert!(sse.contains("data: Hello, World!"));
    }
    
    #[test]
    fn test_sse_json_event() {
        let data = serde_json::json!({"message": "Hello"});
        let event = SseEvent::json(&data).unwrap();
        
        let sse = event.to_sse();
        assert!(sse.contains("data: {\"message\":\"Hello\"}"));
    }
    
    #[test]
    fn test_sse_multiline_data() {
        let event = SseEvent::data("line1\nline2\nline3");
        let sse = event.to_sse();
        
        assert!(sse.contains("data: line1"));
        assert!(sse.contains("data: line2"));
        assert!(sse.contains("data: line3"));
    }
    
    #[test]
    fn test_sse_server_connect() {
        let server = SseServer::new();
        
        let (id, _rx) = server.connect();
        
        assert!(!id.is_empty());
        assert_eq!(server.client_count(), 1);
    }
    
    #[test]
    fn test_sse_channel() {
        let server = SseServer::new();
        
        let (id1, _rx1) = server.connect_to_channel("updates").unwrap();
        let (id2, _rx2) = server.connect_to_channel("updates").unwrap();
        
        assert_eq!(server.client_count(), 2);
        assert_eq!(server.channel_count(), 1);
        
        let event = SseEvent::data("Update!");
        let sent = server.broadcast_to_channel("updates", &event);
        assert_eq!(sent, 2);
    }
    
    #[test]
    fn test_sse_subscribe_unsubscribe() {
        let server = SseServer::new();
        
        let (id, _rx) = server.connect();
        assert_eq!(server.client_count(), 1);
        
        server.subscribe(&id, "news").unwrap();
        
        server.disconnect(&id);
    }
    
    #[test]
    fn test_sse_heartbeat() {
        let event = SseServer::heartbeat();
        let sse = event.to_sse();
        
        assert!(sse.contains("event: ping"));
        assert!(sse.contains("data: heartbeat"));
    }
}
