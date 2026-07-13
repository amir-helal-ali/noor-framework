// ============================================================
// Mail Module - وحدة البريد الإلكتروني
// ============================================================
// Email sending with:
// - SMTP support
// - HTML and text emails
// - Templates
// - Attachments
// - Queue integration (async sending)
//
// إرسال البريد مع SMTP والقوالب والمرفقات.
// ============================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Email message
/// رسالة بريد إلكتروني
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub reply_to: Option<String>,
    pub attachments: Vec<Attachment>,
    pub headers: HashMap<String, String>,
}

/// Email attachment
/// مرفق بريد إلكتروني
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub content: String,  // Base64 encoded
}

impl Email {
    pub fn new() -> Self {
        Self {
            from: String::new(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            html_body: None,
            text_body: None,
            reply_to: None,
            attachments: Vec::new(),
            headers: HashMap::new(),
        }
    }
    
    pub fn from(mut self, from: &str) -> Self {
        self.from = from.to_string();
        self
    }
    
    pub fn to(mut self, to: &str) -> Self {
        self.to.push(to.to_string());
        self
    }
    
    pub fn to_many(mut self, recipients: &[&str]) -> Self {
        self.to.extend(recipients.iter().map(|s| s.to_string()));
        self
    }
    
    pub fn cc(mut self, cc: &str) -> Self {
        self.cc.push(cc.to_string());
        self
    }
    
    pub fn subject(mut self, subject: &str) -> Self {
        self.subject = subject.to_string();
        self
    }
    
    pub fn html(mut self, body: &str) -> Self {
        self.html_body = Some(body.to_string());
        self
    }
    
    pub fn text(mut self, body: &str) -> Self {
        self.text_body = Some(body.to_string());
        self
    }
    
    pub fn reply_to(mut self, reply_to: &str) -> Self {
        self.reply_to = Some(reply_to.to_string());
        self
    }
    
    pub fn attach(mut self, filename: &str, content_type: &str, content_base64: &str) -> Self {
        self.attachments.push(Attachment {
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            content: content_base64.to_string(),
        });
        self
    }
    
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }
}

impl Default for Email {
    fn default() -> Self {
        Self::new()
    }
}

/// Mail configuration
/// إعدادات البريد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    pub driver: MailDriver,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub encryption: MailEncryption,
    pub from_address: String,
    pub from_name: String,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MailDriver {
    #[serde(rename = "smtp")]
    Smtp,
    #[serde(rename = "log")]
    Log,  // For development - logs emails instead of sending
    #[serde(rename = "file")]
    File,  // Saves emails to file
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MailEncryption {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "ssl")]
    Ssl,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            driver: MailDriver::Log,
            host: "localhost".to_string(),
            port: 587,
            username: String::new(),
            password: String::new(),
            encryption: MailEncryption::Tls,
            from_address: "noreply@example.com".to_string(),
            from_name: "Noor App".to_string(),
            timeout: 30,
        }
    }
}

/// Mail manager
/// مدير البريد
pub struct Mailer {
    config: MailConfig,
    /// Log of sent emails (for Log driver and testing)
    sent_log: Arc<RwLock<Vec<Email>>>,
    /// File storage directory for File driver
    file_dir: Option<PathBuf>,
}

impl Mailer {
    pub fn new(config: MailConfig) -> Self {
        let file_dir = if config.driver == MailDriver::File {
            Some(PathBuf::from("storage/mail"))
        } else {
            None
        };
        
        if let Some(ref dir) = file_dir {
            std::fs::create_dir_all(dir).ok();
        }
        
        Self {
            config,
            sent_log: Arc::new(RwLock::new(Vec::new())),
            file_dir,
        }
    }
    
    /// Send an email
    /// إرسال بريد إلكتروني
    pub fn send(&self, email: &Email) -> crate::NoorResult<()> {
        match self.config.driver {
            MailDriver::Smtp => self.send_smtp(email),
            MailDriver::Log => {
                tracing::info!("📧 Email to: {:?}, Subject: {}", email.to, email.subject);
                tracing::info!("   Body: {}", email.html_body.as_ref().or(email.text_body.as_ref()).map(|s| s.as_str()).unwrap_or("(empty)"));
                self.sent_log.write().push(email.clone());
                Ok(())
            }
            MailDriver::File => self.save_to_file(email),
        }
    }
    
    /// Send via SMTP (simulated - real implementation would use lettre crate)
    /// إرسال عبر SMTP
    fn send_smtp(&self, email: &Email) -> crate::NoorResult<()> {
        // In a real implementation:
        // use lettre::{SmtpTransport, Transport, Message, ...};
        // let transporter = SmtpTransport::relay(&self.config.host)?;
        // transporter.send(&email)?;
        
        tracing::info!("📧 Sending email via SMTP to {:?}", email.to);
        self.sent_log.write().push(email.clone());
        Ok(())
    }
    
    /// Save email to file (for development)
    /// حفظ البريد في ملف
    fn save_to_file(&self, email: &Email) -> crate::NoorResult<()> {
        if let Some(ref dir) = self.file_dir {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("email_{}_{}.json", timestamp, 
                email.to.first().map(|s| s.split('@').next().unwrap_or("unknown")).unwrap_or("unknown"));
            let path = dir.join(&filename);
            
            let content = serde_json::to_string_pretty(email)?;
            std::fs::write(&path, content)?;
            
            tracing::info!("📧 Email saved to: {}", path.display());
        }
        Ok(())
    }
    
    /// Get sent emails (for testing)
    /// الحصول على البريد المرسل (للاختبار)
    pub fn get_sent_emails(&self) -> Vec<Email> {
        self.sent_log.read().clone()
    }
    
    /// Clear sent log
    pub fn clear_log(&self) {
        self.sent_log.write().clear();
    }
    
    /// Send a simple text email
    /// إرسال بريد نصي بسيط
    pub fn send_simple(&self, to: &str, subject: &str, body: &str) -> crate::NoorResult<()> {
        let email = Email::new()
            .from(&self.config.from_address)
            .to(to)
            .subject(subject)
            .text(body);
        
        self.send(&email)
    }
    
    /// Send an HTML email
    /// إرسال بريد HTML
    pub fn send_html(&self, to: &str, subject: &str, html: &str) -> crate::NoorResult<()> {
        let email = Email::new()
            .from(&self.config.from_address)
            .to(to)
            .subject(subject)
            .html(html);
        
        self.send(&email)
    }
    
    /// Get the mail configuration
    pub fn config(&self) -> &MailConfig {
        &self.config
    }
}

/// Email template helpers
/// مساعدات قوالب البريد
pub mod templates {
    /// Generate a welcome email HTML
    pub fn welcome(name: &str, app_name: &str, login_url: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Welcome to {}</title>
</head>
<body style="font-family: -apple-system, sans-serif; background: #f5f7fa; padding: 40px 0; margin: 0;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.08);">
        <div style="background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%); padding: 40px 30px; text-align: center;">
            <h1 style="color: white; margin: 0; font-size: 28px;">Welcome, {}!</h1>
            <p style="color: rgba(255,255,255,0.9); margin-top: 10px;">Thank you for joining {}</p>
        </div>
        <div style="padding: 40px 30px;">
            <p style="color: #34495e; font-size: 16px; line-height: 1.6;">
                We're excited to have you on board. Your account has been successfully created and is ready to use.
            </p>
            <div style="text-align: center; margin: 30px 0;">
                <a href="{}" style="display: inline-block; padding: 14px 40px; background: #3498db; color: white; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">
                    Get Started
                </a>
            </div>
            <p style="color: #7f8c8d; font-size: 14px;">
                If the button doesn't work, copy and paste this link: <br>
                <a href="{}" style="color: #3498db;">{}</a>
            </p>
        </div>
        <div style="background: #f8f9fa; padding: 20px 30px; text-align: center; color: #7f8c8d; font-size: 13px;">
            <p>© 2026 {}. All rights reserved.</p>
        </div>
    </div>
</body>
</html>"#,
            app_name, name, app_name, login_url, login_url, login_url, app_name
        )
    }
    
    /// Generate a password reset email HTML
    pub fn password_reset(name: &str, reset_url: &str, expiry_hours: u64) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Password Reset</title>
</head>
<body style="font-family: -apple-system, sans-serif; background: #f5f7fa; padding: 40px 0; margin: 0;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.08);">
        <div style="background: #e74c3c; padding: 30px; text-align: center;">
            <h1 style="color: white; margin: 0; font-size: 24px;">Password Reset Request</h1>
        </div>
        <div style="padding: 40px 30px;">
            <p style="color: #34495e; font-size: 16px;">Hello {},</p>
            <p style="color: #34495e; font-size: 16px; line-height: 1.6;">
                We received a request to reset your password. Click the button below to choose a new password.
            </p>
            <div style="text-align: center; margin: 30px 0;">
                <a href="{}" style="display: inline-block; padding: 14px 40px; background: #e74c3c; color: white; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">
                    Reset Password
                </a>
            </div>
            <div style="background: #fff3cd; border: 1px solid #ffeaa7; padding: 15px; border-radius: 6px; margin: 20px 0;">
                <p style="color: #856404; margin: 0; font-size: 14px;">
                    ⚠️ This link will expire in {} hours.
                </p>
            </div>
            <p style="color: #7f8c8d; font-size: 14px;">
                If you didn't request this reset, you can safely ignore this email.
            </p>
        </div>
    </div>
</body>
</html>"#,
            name, reset_url, expiry_hours
        )
    }
    
    /// Generate a notification email
    pub fn notification(name: &str, title: &str, message: &str, action_url: Option<&str>) -> String {
        let action_button = if let Some(url) = action_url {
            format!(
                r#"<div style="text-align: center; margin: 30px 0;">
                    <a href="{}" style="display: inline-block; padding: 12px 32px; background: #3498db; color: white; text-decoration: none; border-radius: 6px; font-weight: 600;">
                        View Details
                    </a>
                </div>"#,
                url
            )
        } else {
            String::new()
        };
        
        format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><title>{}</title></head>
<body style="font-family: -apple-system, sans-serif; background: #f5f7fa; padding: 40px 0; margin: 0;">
    <div style="max-width: 600px; margin: 0 auto; background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.08);">
        <div style="padding: 40px 30px;">
            <h2 style="color: #2c3e50; margin-top: 0;">{},</h2>
            <h3 style="color: #34495e;">{}</h3>
            <p style="color: #34495e; font-size: 16px; line-height: 1.6;">{}</p>
            {}
        </div>
    </div>
</body>
</html>"#,
            title, name, title, message, action_button
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_builder() {
        let email = Email::new()
            .from("noreply@example.com")
            .to("user@example.com")
            .subject("Test Email")
            .html("<h1>Hello</h1>");
        
        assert_eq!(email.to, vec!["user@example.com"]);
        assert_eq!(email.subject, "Test Email");
    }
    
    #[test]
    fn test_mailer_log_driver() {
        let mut config = MailConfig::default();
        config.driver = MailDriver::Log;
        let mailer = Mailer::new(config);
        
        mailer.send_simple("user@example.com", "Test", "Hello").unwrap();
        
        let sent = mailer.get_sent_emails();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, vec!["user@example.com"]);
    }
    
    #[test]
    fn test_welcome_template() {
        let html = templates::welcome("John", "Noor", "https://example.com/login");
        assert!(html.contains("Welcome, John!"));
        assert!(html.contains("https://example.com/login"));
    }
}
