// ============================================================
// نظام الويب هوك (Webhook System) - وحدة الويب هوك
// ============================================================
// إرسال واستقبال الويب هوك لتكامل الأنظمة الخارجية
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// تمثيل الويب هوك
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub headers: HashMap<String, String>,
    pub is_active: bool,
    pub created_at: i64,
    pub last_triggered_at: Option<i64>,
    pub last_response_code: Option<u16>,
    pub failure_count: u32,
}

/// حمولة الويب هوك
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
    pub signature: Option<String>,
}

impl WebhookPayload {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self {
            event: event.to_string(),
            data,
            timestamp: chrono::Utc::now().timestamp(),
            signature: None,
        }
    }
    
    /// التوقيع باستخدام HMAC-SHA256
    pub fn sign(&mut self, secret: &str) -> crate::NoorResult<()> {
        let payload = serde_json::to_vec(self)?;
        let signature = crate::core::security::Encryption::hmac_sha256(secret.as_bytes(), &payload)?;
        let signature_hex = hex::encode(&signature);
        self.signature = Some(signature_hex);
        Ok(())
    }
    
    /// التحقق من التوقيع
    pub fn verify(&self, secret: &str) -> bool {
        let signature = match &self.signature {
            Some(s) => s,
            None => return false,
        };
        
        let mut payload = self.clone();
        payload.signature = None;
        
        let payload_bytes = match serde_json::to_vec(&payload) {
            Ok(b) => b,
            Err(_) => return false,
        };
        
        let expected = match crate::core::security::Encryption::hmac_sha256(secret.as_bytes(), &payload_bytes) {
            Ok(sig) => hex::encode(&sig),
            Err(_) => return false,
        };
        
        crate::core::security::Encryption::constant_time_compare(
            signature.as_bytes(),
            expected.as_bytes(),
        )
    }
}

/// مدير الويب هوك
pub struct WebhookManager {
    webhooks: Arc<RwLock<HashMap<String, Webhook>>>,
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookManager {
    pub fn new() -> Self {
        Self {
            webhooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// تسجيل ويب هوك جديد
    pub fn register(&self, webhook: Webhook) -> String {
        let id = webhook.id.clone();
        self.webhooks.write().insert(id.clone(), webhook);
        id
    }
    
    /// إنشاء ويب هوك جديد بسهولة
    pub fn create(&self, url: &str, events: Vec<String>, secret: Option<String>) -> String {
        let webhook = Webhook {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            events,
            secret,
            headers: HashMap::new(),
            is_active: true,
            created_at: chrono::Utc::now().timestamp(),
            last_triggered_at: None,
            last_response_code: None,
            failure_count: 0,
        };
        self.register(webhook)
    }
    
    /// إلغاء تسجيل ويب هوك
    pub fn unregister(&self, id: &str) -> bool {
        self.webhooks.write().remove(id).is_some()
    }
    
    /// تفعيل/تعطيل ويب هوك
    pub fn set_active(&self, id: &str, active: bool) {
        if let Some(webhook) = self.webhooks.write().get_mut(id) {
            webhook.is_active = active;
        }
    }
    
    /// تشغيل الويب هوك لحدث معين
    pub fn dispatch(&self, event: &str, data: serde_json::Value) -> Vec<WebhookResult> {
        let mut results = Vec::new();
        
        let webhooks: Vec<Webhook> = self.webhooks
            .read()
            .values()
            .filter(|w| w.is_active && w.events.contains(&event.to_string()))
            .cloned()
            .collect();
        
        for webhook in webhooks {
            let result = self.send_webhook(&webhook, event, &data);
            results.push(result);
        }
        
        results
    }
    
    /// إرسال ويب هوك
    fn send_webhook(&self, webhook: &Webhook, event: &str, data: &serde_json::Value) -> WebhookResult {
        let mut payload = WebhookPayload::new(event, data.clone());
        
        // التوقيع إذا كان هناك سر
        if let Some(ref secret) = webhook.secret {
            if let Err(e) = payload.sign(secret) {
                return WebhookResult {
                    webhook_id: webhook.id.clone(),
                    success: false,
                    status_code: None,
                    error: Some(e.to_string()),
                };
            }
        }
        
        // في تطبيق حقيقي، سنرسل HTTP POST هنا
        // في الوقت الحالي، نسجل فقط
        tracing::info!(
            webhook_id = %webhook.id,
            event = %event,
            url = %webhook.url,
            "Webhook dispatched"
        );
        
        // محاكاة استجابة ناجحة
        let status_code = 200u16;
        
        // تحديث حالة الويب هوك
        if let Some(wh) = self.webhooks.write().get_mut(&webhook.id) {
            wh.last_triggered_at = Some(chrono::Utc::now().timestamp());
            wh.last_response_code = Some(status_code);
            if status_code >= 400 {
                wh.failure_count += 1;
            }
        }
        
        WebhookResult {
            webhook_id: webhook.id.clone(),
            success: status_code < 400,
            status_code: Some(status_code),
            error: None,
        }
    }
    
    /// الحصول على جميع الويب هوك
    pub fn list(&self) -> Vec<Webhook> {
        self.webhooks.read().values().cloned().collect()
    }
    
    /// الحصول على ويب هوك محدد
    pub fn get(&self, id: &str) -> Option<Webhook> {
        self.webhooks.read().get(id).cloned()
    }
    
    /// الحصول على عدد الويب هوك
    pub fn count(&self) -> usize {
        self.webhooks.read().len()
    }
}

/// نتيجة إرسال الويب هوك
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResult {
    pub webhook_id: String,
    pub success: bool,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

/// التحقق من الويب هوك الوارد
pub fn verify_incoming_webhook(
    payload: &[u8],
    signature: &str,
    secret: &str,
) -> bool {
    let expected = match crate::core::security::Encryption::hmac_sha256(secret.as_bytes(), payload) {
        Ok(sig) => hex::encode(&sig),
        Err(_) => return false,
    };
    
    crate::core::security::Encryption::constant_time_compare(
        signature.as_bytes(),
        expected.as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_webhook_payload_signing() {
        let secret = "my_secret";
        let mut payload = WebhookPayload::new(
            "user.created",
            serde_json::json!({"id": 123, "name": "John"}),
        );
        
        payload.sign(secret).unwrap();
        assert!(payload.signature.is_some());
        assert!(payload.verify(secret));
        assert!(!payload.verify("wrong_secret"));
    }
    
    #[test]
    fn test_webhook_manager() {
        let manager = WebhookManager::new();
        
        let id = manager.create(
            "https://example.com/webhook",
            vec!["user.created".to_string()],
            Some("secret".to_string()),
        );
        
        assert_eq!(manager.count(), 1);
        
        let results = manager.dispatch("user.created", serde_json::json!({"id": 1}));
        
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        
        // التأكد من أن الأحداث غير المسجلة لا تطلق الويب هوك
        let results = manager.dispatch("user.deleted", serde_json::json!({}));
        assert_eq!(results.len(), 0);
    }
    
    #[test]
    fn test_webhook_activation() {
        let manager = WebhookManager::new();
        
        let id = manager.create(
            "https://example.com/webhook",
            vec!["test.event".to_string()],
            None,
        );
        
        manager.set_active(&id, false);
        
        let results = manager.dispatch("test.event", serde_json::json!({}));
        assert_eq!(results.len(), 0); // معطل
        
        manager.set_active(&id, true);
        
        let results = manager.dispatch("test.event", serde_json::json!({}));
        assert_eq!(results.len(), 1);
    }
    
    #[test]
    fn test_incoming_webhook_verification() {
        let secret = "shared_secret";
        let payload = br#"{"event":"test","data":{}}"#;
        
        let signature = crate::core::security::Encryption::hmac_sha256(secret.as_bytes(), payload).unwrap();
        let signature_hex = hex::encode(&signature);
        
        assert!(verify_incoming_webhook(payload, &signature_hex, secret));
        assert!(!verify_incoming_webhook(payload, &signature_hex, "wrong_secret"));
        assert!(!verify_incoming_webhook(payload, "invalid", secret));
    }
}
