// ============================================================
// Configuration - إعدادات الإطار
// ============================================================
// Zero-config by default, but fully configurable via
// noor.toml or environment variables.
//
// افتراضياً بدون إعدادات، لكن قابل للتخصيص الكامل عبر
// noor.toml أو متغيرات البيئة.
// ============================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

/// Main configuration structure
/// هيكل الإعدادات الرئيسية
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Application name
    pub app: AppConfig,
    /// Server settings
    pub server: ServerConfig,
    /// Database settings
    pub database: DatabaseConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Cache settings
    pub cache: CacheConfig,
    /// View/template settings
    pub view: ViewConfig,
    /// Logging settings
    pub log: LogConfig,
    /// Custom key-value settings
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub env: Environment,
    pub debug: bool,
    pub timezone: String,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "testing")]
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: usize,
    pub request_timeout: u64,
    pub body_limit: usize,
    pub static_dir: String,
    pub upload_dir: String,
    /// Enable gzip compression
    pub compression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub enable_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseDriver {
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "postgres")]
    Postgres,
    #[serde(rename = "mysql")]
    Mysql,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiry: u64,
    pub session_lifetime: u64,
    pub bcrypt_cost: u32,
    pub enable_csrf: bool,
    pub enable_xss_filter: bool,
    pub rate_limit_per_minute: u32,
    pub cors_origins: Vec<String>,
    pub secure_headers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub driver: CacheDriver,
    pub prefix: String,
    pub default_ttl: u64,
    pub cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheDriver {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "memory")]
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    pub template_dir: String,
    pub cache_templates: bool,
    pub auto_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<String>,
    pub json: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig {
                name: "Noor App".to_string(),
                env: Environment::Development,
                debug: true,
                timezone: "UTC".to_string(),
                locale: "ar".to_string(),
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 4,
                max_connections: 1024,
                request_timeout: 30,
                body_limit: 10 * 1024 * 1024, // 10MB
                static_dir: "public".to_string(),
                upload_dir: "storage/uploads".to_string(),
                compression: true,
            },
            database: DatabaseConfig {
                driver: DatabaseDriver::Sqlite,
                url: "sqlite://storage/noor.db".to_string(),
                max_connections: 10,
                min_connections: 1,
                enable_logging: false,
            },
            security: SecurityConfig {
                jwt_secret: Self::generate_secret(),
                jwt_expiry: 3600,
                session_lifetime: 86400,
                bcrypt_cost: 12,
                enable_csrf: true,
                enable_xss_filter: true,
                rate_limit_per_minute: 60,
                cors_origins: vec!["*".to_string()],
                secure_headers: true,
            },
            cache: CacheConfig {
                driver: CacheDriver::File,
                prefix: "noor:".to_string(),
                default_ttl: 3600,
                cache_dir: "storage/cache".to_string(),
            },
            view: ViewConfig {
                template_dir: "resources/views".to_string(),
                cache_templates: true,
                auto_reload: true,
            },
            log: LogConfig {
                level: "info".to_string(),
                file: Some("storage/logs/app.log".to_string()),
                json: false,
            },
            custom: HashMap::new(),
        }
    }
}

impl Config {
    /// Generate a random secret for JWT
    /// توليد سر عشوائي لـ JWT
    fn generate_secret() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| {
                let n = rng.gen_range(0..16);
                char::from_digit(n, 16).unwrap_or('0')
            })
            .collect()
    }
    
    /// Load configuration from a TOML file
    /// تحميل الإعدادات من ملف TOML
    ///
    /// Falls back to built-in defaults when the file is missing, but still
    /// applies environment-variable overrides either way.
    pub fn load(path: &Path) -> crate::NoorResult<Self> {
        let mut config = if !path.exists() {
            tracing::warn!("Config file {:?} not found, using defaults", path);
            Self::default()
        } else {
            let content = std::fs::read_to_string(path)?;
            toml::from_str::<Config>(&content)?
        };

        // Always apply env overrides, even when we fell back to defaults.
        config.apply_env_overrides();

        Ok(config)
    }
    
    /// Apply environment variable overrides
    /// تطبيق تجاوزات متغيرات البيئة
    pub fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("NOOR_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("NOOR_SERVER_PORT") {
            if let Ok(p) = port.parse() {
                self.server.port = p;
            }
        }
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }
        if let Ok(secret) = std::env::var("JWT_SECRET") {
            self.security.jwt_secret = secret;
        }
        if let Ok(env) = std::env::var("APP_ENV") {
            self.app.env = match env.as_str() {
                "production" => Environment::Production,
                "testing" => Environment::Testing,
                _ => Environment::Development,
            };
            self.app.debug = self.app.env == Environment::Development;
        }
    }
    
    /// Check if running in production
    /// فحص إذا كان يعمل في بيئة الإنتاج
    pub fn is_production(&self) -> bool {
        self.app.env == Environment::Production
    }
    
    /// Check if running in development
    /// فحص إذا كان يعمل في بيئة التطوير
    pub fn is_development(&self) -> bool {
        self.app.env == Environment::Development
    }
}
