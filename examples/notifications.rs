// ============================================================
// مثال: نظام الإشعارات | Example: Notifications
// ============================================================

use noor::*;
use noor::core::notification::{NotificationManager, Notification, Channel, Priority};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🔔 Notification System Demo\n");
    
    let manager = NotificationManager::new();
    
    // Send in-app notification
    let notif = Notification::new("user1", "Welcome!", 
        "Welcome to our platform!", Channel::InApp)
        .with_priority(Priority::High);
    
    manager.send(notif)?;
    
    // Send email notification
    let notif = Notification::new("user1", "Password Changed",
        "Your password was changed successfully.", Channel::Email);
    manager.send(notif)?;
    
    // Send to multiple channels
    let results = manager.send_multi_channel(
        "user1",
        "New Comment",
        "Someone commented on your post!",
        vec![Channel::InApp, Channel::Email],
        Some(serde_json::json!({"post_id": 123})),
    );
    
    println!("✓ Sent {} notifications", results.len());
    
    // Broadcast to multiple users
    let users = vec!["user1".to_string(), "user2".to_string(), "user3".to_string()];
    manager.broadcast(&users, "Announcement", "System maintenance at 3 AM", Channel::InApp);
    
    // Check unread count
    println!("✓ User1 has {} unread notifications", manager.unread_count("user1"));
    
    // List notifications
    let notifs = manager.get_notifications("user1");
    println!("✓ User1 notifications:");
    for n in &notifs {
        println!("  - {} ({}): {}", n.title, n.channel.as_str(), n.body);
    }
    
    // Mark all as read
    let read_count = manager.mark_all_read("user1");
    println!("\n✓ Marked {} notifications as read", read_count);
    
    println!("\n✅ Notification demo completed!");
    Ok(())
}
