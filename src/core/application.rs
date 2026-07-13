// ============================================================
// Application - التطبيق الرئيسي
// ============================================================
// The main application class that ties everything together.
// الصنف الرئيسي الذي يربط كل شيء.
// ============================================================

use std::sync::Arc;

use crate::core::cache::CacheManager;
use crate::core::config::Config;
use crate::core::router::Router;
use crate::core::server::Server;
use crate::core::view::ViewEngine;
use crate::NoorResult;

/// The main Noor application
/// تطبيق نور الرئيسي
pub struct Application {
    pub config: Arc<Config>,
    pub router: Arc<Router>,
    pub cache: Arc<CacheManager>,
    pub view: Option<Arc<ViewEngine>>,
    /// Optional database connection pool. Populated by `with_database()`
    /// or `ApplicationBuilder`. When `Some`, handlers can access the DB
    /// via `app.database()`.
    pub database: Option<Arc<crate::core::orm::Database>>,
}

impl Application {
    /// Create a new application with the given config and router
    pub fn new(config: Config, router: Router) -> Self {
        let config = Arc::new(config);

        // Initialize cache manager
        let cache = match config.cache.driver {
            crate::core::config::CacheDriver::File => {
                CacheManager::for_weak_server(&config.cache.cache_dir)
                    .expect("Failed to initialize cache")
            }
            crate::core::config::CacheDriver::Memory => CacheManager::memory_only(1000),
        };

        // Initialize view engine if template dir exists
        let view = if std::path::Path::new(&config.view.template_dir).exists() {
            ViewEngine::new(
                &config.view.template_dir,
                config.view.cache_templates,
                config.view.auto_reload,
            )
            .ok()
            .map(Arc::new)
        } else {
            None
        };

        Self {
            config,
            router: Arc::new(router),
            cache: Arc::new(cache),
            view,
            database: None,
        }
    }

    /// Attach a database connection pool to the application.
    ///
    /// This is separated from `new()` because database connection is async
    /// (sqlx pools must be `await`-ed into existence), while `new()` is
    /// sync. Call this from an async context (e.g. `#[tokio::main]`) before
    /// `run()`.
    pub fn with_database(mut self, db: crate::core::orm::Database) -> Self {
        self.database = Some(Arc::new(db));
        self
    }

    /// Connect to the database using `self.config.database` and attach the
    /// resulting pool. Convenience wrapper around `with_database`.
    pub async fn connect_database(mut self) -> NoorResult<Self> {
        let db = crate::core::orm::Database::from_config(&self.config.database).await?;
        self.database = Some(Arc::new(db));
        Ok(self)
    }

    /// Run the application
    pub fn run(self) -> NoorResult<()> {
        let server = Server::new(self.config.clone(), self.router.clone());
        server.run()
    }

    /// Get shared configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get router reference
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Get the database pool (if connected).
    pub fn database(&self) -> Option<&Arc<crate::core::orm::Database>> {
        self.database.as_ref()
    }
}

/// Builder for constructing an `Application` with optional components.
///
/// Usage:
/// ```ignore
/// let app = ApplicationBuilder::new(config, router)
///     .connect_database()
///     .await?
///     .build();
/// app.run()?;
/// ```
pub struct ApplicationBuilder {
    config: Config,
    router: Router,
    database: Option<crate::core::orm::Database>,
}

impl ApplicationBuilder {
    pub fn new(config: Config, router: Router) -> Self {
        Self {
            config,
            router,
            database: None,
        }
    }

    /// Connect to the database using the config's `database` section.
    pub async fn connect_database(mut self) -> NoorResult<Self> {
        let db = crate::core::orm::Database::from_config(&self.config.database).await?;
        self.database = Some(db);
        Ok(self)
    }

    /// Attach a pre-built database pool.
    pub fn with_database(mut self, db: crate::core::orm::Database) -> Self {
        self.database = Some(db);
        self
    }

    /// Build the final `Application`.
    pub fn build(self) -> Application {
        let mut app = Application::new(self.config, self.router);
        if let Some(db) = self.database {
            app.database = Some(Arc::new(db));
        }
        app
    }
}
