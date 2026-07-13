// ============================================================
// مثال: Webhooks | Example: Webhook System
// ============================================================

use noor::*;
use noor::core::webhook::{WebhookManager, WebhookPayload};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🪝 Webhook System Demo\n");
    
    let manager = WebhookManager::new();
    
    // Register webhooks
    let webhook1 = manager.create(
        "https://api.stripe.com/webhook",
        vec!["payment.received".to_string(), "payment.failed".to_string()],
        Some("stripe_secret".to_string()),
    );
    
    let webhook2 = manager.create(
        "https://api.slack.com/webhook",
        vec!["notification.send".to_string()],
        Some("slack_secret".to_string()),
    );
    
    let webhook3 = manager.create(
        "https://api.zapier.com/webhook",
        vec!["user.created".to_string(), "user.updated".to_string(), "payment.received".to_string()],
        None,
    );
    
    println!("✓ Registered {} webhooks:", manager.count());
    println!("  1. Stripe (payment events)");
    println!("  2. Slack (notifications)");
    println!("  3. Zapier (user + payment events)");
    
    // Dispatch events
    println!("\n📤 Dispatching 'payment.received' event...");
    let results = manager.dispatch("payment.received", serde_json::json!({
        "payment_id": "pay_123456",
        "amount": 99.99,
        "currency": "USD",
        "customer": "cust_789"
    }));
    
    println!("  Results: {} webhooks triggered", results.len());
    for result in &results {
        let status = if result.success { "✓" } else { "✗" };
        println!("    {} Webhook {}: {}", status, result.webhook_id, 
            result.status_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()));
    }
    
    println!("\n📤 Dispatching 'user.created' event...");
    let results = manager.dispatch("user.created", serde_json::json!({
        "user_id": 123,
        "name": "John Doe",
        "email": "john@example.com"
    }));
    
    println!("  Results: {} webhooks triggered", results.len());
    
    // Test payload signing
    println!("\n🔐 Testing payload signing...");
    let secret = "my_webhook_secret";
    let mut payload = WebhookPayload::new("test.event", serde_json::json!({"data": "test"}));
    
    payload.sign(secret)?;
    println!("  Signature: {}...", &payload.signature.as_ref().unwrap()[..20]);
    println!("  Verify (correct secret): {}", payload.verify(secret));
    println!("  Verify (wrong secret): {}", payload.verify("wrong_secret"));
    
    // List all webhooks
    println!("\n📋 All webhooks:");
    for (i, webhook) in manager.list().iter().enumerate() {
        println!("  {}. {} - Events: {:?}", i + 1, webhook.url, webhook.events);
    }
    
    // Deactivate one webhook
    println!("\n🚫 Deactivating Zapier webhook...");
    manager.set_active(&webhook3, false);
    
    println!("\n📤 Dispatching 'user.created' event again...");
    let results = manager.dispatch("user.created", serde_json::json!({"user_id": 456}));
    println!("  Results: {} webhooks triggered (Zapier was deactivated)", results.len());
    
    println!("\n✅ Webhook demo completed!");
    Ok(())
}
