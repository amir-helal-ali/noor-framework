// ============================================================
// Notification System - نظام الإشعارات
// ============================================================
// Multi-channel notification delivery (email, SMS, push, in-app)
// with user preferences and templating.
//
// إشعارات متعددة القنوات مع تفضيلات المستخدم.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Notification channel
/// قناة الإشعار
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Email,
    Sms,
    Push,
    InApp,
    Webhook,
    Slack,
    Discord,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Sms => "sms",
            Self::Push => "push",
            Self::InApp => "in_app",
            Self::Webhook => "webhook",
            Self::Slack => "slack",
            Self::Discord => "discord",
        }
    }
}

/// Notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub channel: Channel,
    pub priority: Priority,
    pub read: bool,
    pub created_at: i64,
    pub read_at: Option<i64>,
}

impl Notification {
    pub fn new(user_id: &str, title: &str, body: &str, channel: Channel) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            data: None,
            channel,
            priority: Priority::Normal,
            read: false,
            created_at: chrono::Utc::now().timestamp(),
            read_at: None,
        }
    }
    
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn mark_as_read(&mut self) {
        self.read = true;
        self.read_at = Some(chrono::Utc::now().timestamp());
    }
}

/// Channel handler trait
pub trait ChannelHandler: Send + Sync {
    fn send(&self, notification: &Notification) -> crate::NoorResult<()>;
    fn channel(&self) -> Channel;
}

/// In-app channel handler (stores notifications in memory)
pub struct InAppChannel {
    notifications: Arc<RwLock<Vec<Notification>>>,
}

impl InAppChannel {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn get_user_notifications(&self, user_id: &str) -> Vec<Notification> {
        self.notifications
            .read()
            .iter()
            .filter(|n| n.user_id == user_id)
            .cloned()
            .collect()
    }
    
    pub fn get_unread_count(&self, user_id: &str) -> usize {
        self.notifications
            .read()
            .iter()
            .filter(|n| n.user_id == user_id && !n.read)
            .count()
    }
    
    pub fn mark_as_read(&self, notification_id: &str) -> bool {
        let mut notifications = self.notifications.write();
        if let Some(n) = notifications.iter_mut().find(|n| n.id == notification_id) {
            n.mark_as_read();
            return true;
        }
        false
    }
    
    pub fn mark_all_as_read(&self, user_id: &str) -> usize {
        let mut count = 0;
        let mut notifications = self.notifications.write();
        for n in notifications.iter_mut() {
            if n.user_id == user_id && !n.read {
                n.mark_as_read();
                count += 1;
            }
        }
        count
    }
    
    pub fn delete(&self, notification_id: &str) -> bool {
        let mut notifications = self.notifications.write();
        let initial = notifications.len();
        notifications.retain(|n| n.id != notification_id);
        notifications.len() < initial
    }
}

impl Default for InAppChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelHandler for InAppChannel {
    fn send(&self, notification: &Notification) -> crate::NoorResult<()> {
        self.notifications.write().push(notification.clone());
        Ok(())
    }
    
    fn channel(&self) -> Channel {
        Channel::InApp
    }
}

/// Email channel handler (integrates with Mail module)
pub struct EmailChannel {
    mailer: Arc<crate::core::mail::Mailer>,
}

impl EmailChannel {
    pub fn new(mailer: Arc<crate::core::mail::Mailer>) -> Self {
        Self { mailer }
    }
}

impl ChannelHandler for EmailChannel {
    fn send(&self, notification: &Notification) -> crate::NoorResult<()> {
        let email = crate::core::mail::Email::new()
            .to(&notification.user_id)  // In real app, look up user's email
            .subject(&notification.title)
            .html(&notification.body);
        
        self.mailer.send(&email)
    }
    
    fn channel(&self) -> Channel {
        Channel::Email
    }
}

/// Log channel handler (for development/testing)
pub struct LogChannel;

impl ChannelHandler for LogChannel {
    fn send(&self, notification: &Notification) -> crate::NoorResult<()> {
        tracing::info!(
            channel = notification.channel.as_str(),
            user = %notification.user_id,
            title = %notification.title,
            "Notification: {}",
            notification.body
        );
        Ok(())
    }
    
    fn channel(&self) -> Channel {
        Channel::InApp
    }
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: String,
    pub enabled_channels: Vec<Channel>,
    pub disabled_channels: Vec<Channel>,
    pub quiet_hours_start: Option<u32>,  // Hour 0-23
    pub quiet_hours_end: Option<u32>,
    pub email_digest: bool,
    pub push_enabled: bool,
}

impl NotificationPreferences {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            enabled_channels: vec![Channel::Email, Channel::InApp, Channel::Push],
            disabled_channels: vec![],
            // Quiet hours should be opt-in; default to "no quiet hours" so
            // notifications are always delivered unless a user explicitly
            // configures a window. (Previously this defaulted to 22:00-08:00
            // UTC, which silently swallowed notifications sent at night.)
            quiet_hours_start: None,
            quiet_hours_end: None,
            email_digest: false,
            push_enabled: true,
        }
    }
    
    pub fn is_channel_enabled(&self, channel: Channel) -> bool {
        self.enabled_channels.contains(&channel) && !self.disabled_channels.contains(&channel)
    }
    
    pub fn is_quiet_hours(&self) -> bool {
        let now = chrono::Utc::now().hour() as u32;
        
        match (self.quiet_hours_start, self.quiet_hours_end) {
            (Some(start), Some(end)) => {
                if start < end {
                    now >= start && now < end
                } else {
                    // Crosses midnight
                    now >= start || now < end
                }
            }
            _ => false,
        }
    }
}

use chrono::Timelike;

/// Notification manager
pub struct NotificationManager {
    channels: Arc<RwLock<HashMap<Channel, Arc<dyn ChannelHandler>>>>,
    preferences: Arc<RwLock<HashMap<String, NotificationPreferences>>>,
    in_app: Arc<InAppChannel>,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    pub fn new() -> Self {
        let in_app = Arc::new(InAppChannel::new());
        
        let mut channels: HashMap<Channel, Arc<dyn ChannelHandler>> = HashMap::new();
        channels.insert(Channel::InApp, in_app.clone() as Arc<dyn ChannelHandler>);
        channels.insert(Channel::Email, Arc::new(LogChannel) as Arc<dyn ChannelHandler>);
        
        Self {
            channels: Arc::new(RwLock::new(channels)),
            preferences: Arc::new(RwLock::new(HashMap::new())),
            in_app,
        }
    }
    
    /// Register a channel handler
    pub fn register_channel(&self, handler: Arc<dyn ChannelHandler>) {
        let channel = handler.channel();
        self.channels.write().insert(channel, handler);
    }
    
    /// Set user preferences
    pub fn set_preferences(&self, prefs: NotificationPreferences) {
        self.preferences.write().insert(prefs.user_id.clone(), prefs);
    }
    
    /// Get user preferences
    pub fn get_preferences(&self, user_id: &str) -> NotificationPreferences {
        self.preferences
            .read()
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| NotificationPreferences::new(user_id))
    }
    
    /// Send a notification
    pub fn send(&self, mut notification: Notification) -> crate::NoorResult<()> {
        let prefs = self.get_preferences(&notification.user_id);
        
        // Check if channel is enabled
        if !prefs.is_channel_enabled(notification.channel) {
            tracing::debug!(
                user = %notification.user_id,
                channel = notification.channel.as_str(),
                "Notification skipped - channel disabled"
            );
            return Ok(());
        }
        
        // Check quiet hours (except for urgent)
        if notification.priority != Priority::Urgent && prefs.is_quiet_hours() {
            tracing::debug!(
                user = %notification.user_id,
                "Notification delayed - quiet hours"
            );
            // In a real app, we'd queue this for later
            return Ok(());
        }
        
        // Send via the appropriate channel
        let channels = self.channels.read();
        if let Some(handler) = channels.get(&notification.channel) {
            handler.send(&notification)?;
        }
        
        Ok(())
    }
    
    /// Send to multiple channels
    pub fn send_multi_channel(
        &self,
        user_id: &str,
        title: &str,
        body: &str,
        channels: Vec<Channel>,
        data: Option<serde_json::Value>,
    ) -> Vec<crate::NoorResult<()>> {
        channels
            .iter()
            .map(|&channel| {
                let mut notif = Notification::new(user_id, title, body, channel);
                if let Some(ref d) = data {
                    notif = notif.with_data(d.clone());
                }
                self.send(notif)
            })
            .collect()
    }
    
    /// Send to multiple users
    pub fn broadcast(
        &self,
        user_ids: &[String],
        title: &str,
        body: &str,
        channel: Channel,
    ) -> Vec<crate::NoorResult<()>> {
        user_ids
            .iter()
            .map(|user_id| {
                let notif = Notification::new(user_id, title, body, channel);
                self.send(notif)
            })
            .collect()
    }
    
    /// Get user's in-app notifications
    pub fn get_notifications(&self, user_id: &str) -> Vec<Notification> {
        self.in_app.get_user_notifications(user_id)
    }
    
    /// Get unread count
    pub fn unread_count(&self, user_id: &str) -> usize {
        self.in_app.get_unread_count(user_id)
    }
    
    /// Mark as read
    pub fn mark_as_read(&self, notification_id: &str) -> bool {
        self.in_app.mark_as_read(notification_id)
    }
    
    /// Mark all as read for a user
    pub fn mark_all_read(&self, user_id: &str) -> usize {
        self.in_app.mark_all_as_read(user_id)
    }
    
    /// Delete a notification
    pub fn delete(&self, notification_id: &str) -> bool {
        self.in_app.delete(notification_id)
    }
}

/// Predefined notification templates
pub mod templates {
    use super::*;
    
    pub fn welcome(name: &str) -> (String, String) {
        (
            "Welcome!".to_string(),
            format!("Welcome to our platform, {}! We're glad to have you.", name),
        )
    }
    
    pub fn password_changed() -> (String, String) {
        (
            "Password Changed".to_string(),
            "Your password has been successfully changed. If this wasn't you, please contact support immediately.".to_string(),
        )
    }
    
    pub fn new_comment(post_title: &str, commenter: &str) -> (String, String) {
        (
            format!("New comment on '{}'", post_title),
            format!("{} commented on your post '{}'", commenter, post_title),
        )
    }
    
    pub fn mention(user: &str, url: &str) -> (String, String) {
        (
            "You were mentioned".to_string(),
            format!("{} mentioned you. Click here: {}", user, url),
        )
    }
    
    pub fn security_alert(ip: &str, device: &str) -> (String, String) {
        (
            "Security Alert".to_string(),
            format!("New login from {} on {}. If this wasn't you, please secure your account.", ip, device),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_creation() {
        let notif = Notification::new("user1", "Hello", "World", Channel::InApp);
        
        assert_eq!(notif.user_id, "user1");
        assert_eq!(notif.title, "Hello");
        assert!(!notif.read);
    }
    
    #[test]
    fn test_in_app_channel() {
        let channel = InAppChannel::new();
        
        let notif = Notification::new("user1", "Test", "Body", Channel::InApp);
        channel.send(&notif).unwrap();
        
        let user_notifs = channel.get_user_notifications("user1");
        assert_eq!(user_notifs.len(), 1);
        assert_eq!(user_notifs[0].title, "Test");
        
        assert_eq!(channel.get_unread_count("user1"), 1);
        
        channel.mark_as_read(&notif.id);
        assert_eq!(channel.get_unread_count("user1"), 0);
    }
    
    #[test]
    fn test_notification_preferences() {
        let mut prefs = NotificationPreferences::new("user1");
        
        assert!(prefs.is_channel_enabled(Channel::Email));
        
        prefs.disabled_channels.push(Channel::Sms);
        assert!(!prefs.is_channel_enabled(Channel::Sms));
    }
    
    #[test]
    fn test_notification_manager() {
        let manager = NotificationManager::new();
        
        let notif = Notification::new("user1", "Hello", "World", Channel::InApp);
        manager.send(notif).unwrap();
        
        let notifs = manager.get_notifications("user1");
        assert_eq!(notifs.len(), 1);
        
        assert_eq!(manager.unread_count("user1"), 1);
        
        manager.mark_all_read("user1");
        assert_eq!(manager.unread_count("user1"), 0);
    }
    
    #[test]
    fn test_broadcast() {
        let manager = NotificationManager::new();
        
        let users = vec![
            "user1".to_string(),
            "user2".to_string(),
            "user3".to_string(),
        ];
        
        let results = manager.broadcast(&users, "Announcement", "Important update", Channel::InApp);
        
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
        
        assert_eq!(manager.get_notifications("user1").len(), 1);
        assert_eq!(manager.get_notifications("user2").len(), 1);
    }
}
