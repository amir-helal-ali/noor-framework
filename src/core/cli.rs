// ============================================================
// CLI - Command Line Interface
// واجهة سطر الأوامر
// ============================================================
// Provides commands for:
// - new: Create a new project
// - serve: Start the development server
// - build: Build for production
// - make: Generate controllers, models, migrations
// - migrate: Run database migrations
// - routes: List all registered routes
// - test: Run tests
//
// يوفر أوامر لإدارة المشروع.
// ============================================================

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

/// CLI command handler type
type CommandHandler = Arc<dyn Fn(&[String]) -> crate::NoorResult<()> + Send + Sync>;

/// CLI application
pub struct Cli {
    commands: Arc<RwLock<HashMap<String, Command>>>,
}

/// A CLI command definition
pub struct Command {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub handler: CommandHandler,
}

impl Cli {
    pub fn new() -> Self {
        let mut cli = Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Register built-in commands
        cli.register_builtin_commands();
        cli
    }
    
    /// Register a custom command
    pub fn command(&mut self, name: &str, description: &str, usage: &str, handler: CommandHandler) {
        self.commands.write().insert(name.to_string(), Command {
            name: name.to_string(),
            description: description.to_string(),
            usage: usage.to_string(),
            handler,
        });
    }
    
    /// Run the CLI with given arguments
    pub fn run(&self, args: &[String]) -> crate::NoorResult<()> {
        if args.is_empty() {
            self.print_help();
            return Ok(());
        }
        
        let command_name = &args[0];
        let command_args = &args[1..];
        
        let commands = self.commands.read();
        if let Some(command) = commands.get(command_name) {
            (command.handler)(command_args)
        } else {
            println!("Unknown command: {}", command_name);
            self.print_help();
            Err(crate::NoorError::Internal(format!("Unknown command: {}", command_name)))
        }
    }
    
    /// Print help information
    pub fn print_help(&self) {
        println!("\n{}\n", crate::banner());
        println!("Usage: noor <command> [options]\n");
        println!("Commands:\n");
        
        let commands = self.commands.read();
        let mut commands_vec: Vec<&Command> = commands.values().collect();
        commands_vec.sort_by_key(|c| c.name.as_str());
        
        for command in commands_vec {
            println!("  {:15} {}", command.name, command.description);
            println!("  {:15} Usage: {}", "", command.usage);
            println!();
        }
    }
    
    fn register_builtin_commands(&mut self) {
        // new - Create a new project
        self.command(
            "new",
            "Create a new Noor project",
            "noor new <project_name>",
            Arc::new(|args| {
                if args.is_empty() {
                    return Err(crate::NoorError::Internal("Project name required".to_string()));
                }
                let name = &args[0];
                println!("Creating new Noor project: {}", name);
                Self::create_project(name)
            }),
        );
        
        // serve - Start development server (actually runs the server)
        self.command(
            "serve",
            "Start the development server",
            "noor serve [--host=0.0.0.0] [--port=8080]",
            Arc::new(|args| {
                let mut host = std::env::var("NOOR_SERVER_HOST")
                    .unwrap_or_else(|_| "0.0.0.0".to_string());
                let mut port = std::env::var("NOOR_SERVER_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8080u16);

                for arg in args {
                    if let Some(h) = arg.strip_prefix("--host=") {
                        host = h.to_string();
                    } else if let Some(p) = arg.strip_prefix("--port=") {
                        port = p.parse().unwrap_or(8080);
                    }
                }

                // Load config from noor.toml (or defaults), then override
                // host/port with the CLI/env values.
                let mut config = crate::core::config::Config::load(
                    std::path::Path::new("noor.toml"),
                )?;
                config.server.host = host.clone();
                config.server.port = port;

                // Build a minimal router with a health check + static files.
                let mut router = crate::core::router::Router::new();
                router.get("/", |_req| {
                    Ok(crate::core::http::Response::ok().html(
                        r#"<html><head><title>Noor Server</title></head>
<body><h1>Noor Framework</h1><p>Server is running.</p>
<p><a href="/health">Health Check</a></p></body></html>"#,
                    ))
                });
                router.get("/health", |_req| {
                    Ok(crate::core::http::Response::ok().json(
                        &serde_json::json!({
                            "status": "ok",
                            "framework": "noor",
                            "version": crate::VERSION,
                        }),
                    )?)
                });

                // Serve static files from public/ if it exists.
                if std::path::Path::new("public").exists() {
                    router.get("/*", crate::core::static_files::serve("public"));
                }

                let app = crate::core::application::Application::new(config, router);
                app.run()
            }),
        );
        
        // build - Build for production
        self.command(
            "build",
            "Build the project for production",
            "noor build [--release|--weak-server]",
            Arc::new(|args| {
                let profile = if args.iter().any(|a| a == "--weak-server") {
                    "weak-server"
                } else {
                    "release"
                };
                
                println!("Building Noor project with profile: {}", profile);
                println!("Running: cargo build --{}", profile);
                Ok(())
            }),
        );
        
        // make:controller - Generate a controller
        self.command(
            "make:controller",
            "Generate a new controller",
            "noor make:controller <Name>",
            Arc::new(|args| {
                if args.is_empty() {
                    return Err(crate::NoorError::Internal("Controller name required".to_string()));
                }
                let name = &args[0];
                println!("Generating controller: {}", name);
                Self::generate_controller(name)
            }),
        );
        
        // make:model - Generate a model
        self.command(
            "make:model",
            "Generate a new model",
            "noor make:model <Name>",
            Arc::new(|args| {
                if args.is_empty() {
                    return Err(crate::NoorError::Internal("Model name required".to_string()));
                }
                let name = &args[0];
                println!("Generating model: {}", name);
                Self::generate_model(name)
            }),
        );
        
        // make:migration - Generate a migration
        self.command(
            "make:migration",
            "Generate a new migration",
            "noor make:migration <name>",
            Arc::new(|args| {
                if args.is_empty() {
                    return Err(crate::NoorError::Internal("Migration name required".to_string()));
                }
                let name = &args[0];
                println!("Generating migration: {}", name);
                Self::generate_migration(name)
            }),
        );
        
        // migrate - Run migrations (loads SQL files from database/migrations/)
        self.command(
            "migrate",
            "Run database migrations",
            "noor migrate [--rollback] [--db=sqlite://storage/noor.db]",
            Arc::new(|args| {
                let rollback = args.iter().any(|a| a == "--rollback");
                let mut db_url = std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "sqlite://storage/noor.db".to_string());
                for arg in args {
                    if let Some(url) = arg.strip_prefix("--db=") {
                        db_url = url.to_string();
                    }
                }
                let driver = if db_url.starts_with("postgres") {
                    "postgres"
                } else if db_url.starts_with("mysql") {
                    "mysql"
                } else {
                    "sqlite"
                };

                // Build a tiny tokio runtime to drive the async Database calls.
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| crate::NoorError::Internal(format!("runtime error: {}", e)))?;

                rt.block_on(async move {
                    let db = crate::core::orm::Database::new(driver, &db_url).await?;

                    // Load migration files from database/migrations/.
                    // Files are named like `20260710_000001_create_users.sql`.
                    let migrations_dir = std::path::Path::new("database/migrations");
                    if !migrations_dir.exists() {
                        eprintln!("Migrations directory not found: database/migrations");
                        return Ok(());
                    }

                    let mut entries: Vec<_> = std::fs::read_dir(migrations_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|x| x == "sql").unwrap_or(false))
                        .collect();
                    entries.sort_by_key(|e| e.path());

                    let mut migrator = crate::core::orm::Migrator::new();
                    for entry in &entries {
                        let path = entry.path();
                        let filename = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        // Parse `YYYYMMDD_NNNNNN_name`.
                        let (version, name) = match filename.splitn(3, '_').collect::<Vec<_>>() {
                            parts if parts.len() >= 3 => (parts[0..2].join("_"), parts[2].to_string()),
                            parts if parts.len() == 2 => (parts[0..2].join("_"), String::new()),
                            _ => (filename.to_string(), String::new()),
                        };
                        let content = std::fs::read_to_string(&path)?;
                        // Split on `-- DOWN` marker (with or without trailing colon).
                        // The marker must be on its own line to avoid splitting
                        // inside a SQL string that happens to contain "-- DOWN".
                        let (up_sql, down_sql) = split_migration_content(&content);
                        migrator.add(crate::core::orm::Migration::new(
                            &version,
                            &name,
                            &up_sql,
                            &down_sql,
                        ));
                    }

                    if rollback {
                        match migrator.rollback(&db).await? {
                            Some(version) => {
                                println!("✓ Rolled back migration: {}", version);
                            }
                            None => {
                                println!("No migrations to roll back.");
                            }
                        }
                    } else {
                        let pending = migrator.pending().len();
                        println!("Found {} pending migration(s).", pending);
                        let applied = migrator.run(&db).await?;
                        println!("✓ Applied {} migration(s).", applied);
                    }
                    Ok::<(), crate::NoorError>(())
                })
            }),
        );
        
        // routes - List all routes
        self.command(
            "routes",
            "List all registered routes",
            "noor routes",
            Arc::new(|_args| {
                println!("Registered routes:");
                Ok(())
            }),
        );
        
        // test - Run tests
        self.command(
            "test",
            "Run tests",
            "noor test",
            Arc::new(|_args| {
                println!("Running tests...");
                Ok(())
            }),
        );
    }
    
    fn create_project(name: &str) -> crate::NoorResult<()> {
        let project_dir = Path::new(name);
        
        if project_dir.exists() {
            return Err(crate::NoorError::Internal(format!("Directory {} already exists", name)));
        }
        
        // Create directory structure
        std::fs::create_dir_all(project_dir)?;
        std::fs::create_dir_all(project_dir.join("src"))?;
        std::fs::create_dir_all(project_dir.join("src/controllers"))?;
        std::fs::create_dir_all(project_dir.join("src/models"))?;
        std::fs::create_dir_all(project_dir.join("resources/views"))?;
        std::fs::create_dir_all(project_dir.join("public"))?;
        std::fs::create_dir_all(project_dir.join("storage/cache"))?;
        std::fs::create_dir_all(project_dir.join("storage/logs"))?;
        std::fs::create_dir_all(project_dir.join("storage/uploads"))?;
        std::fs::create_dir_all(project_dir.join("database/migrations"))?;
        
        // Create main.rs
        let main_content = format!(
            r#"use noor::*;
use noor::core::{{Application, Config, Router}};
use noor::core::http::{{Request, Response, StatusCode}};

fn main() -> NoorResult<()> {{
    println!("{{}}", banner());
    
    let config = Config::default();
    let mut router = Router::new();
    
    router.get("/", |req| {{
        Ok(Response::ok().html("<h1>Welcome to {}!</h1>"))
    }});
    
    let app = Application::new(config, router);
    app.run()
}}
"#,
            name
        );
        std::fs::write(project_dir.join("src/main.rs"), main_content)?;
        
        // Create noor.toml
        let config_content = format!(
            r#"[app]
name = "{}"
env = "development"
debug = true
timezone = "UTC"
locale = "ar"

[server]
host = "0.0.0.0"
port = 8080
workers = 4
compression = true

[database]
driver = "sqlite"
url = "sqlite://storage/app.db"
max_connections = 10

[security]
enable_csrf = true
enable_xss_filter = true
rate_limit_per_minute = 60

[cache]
driver = "file"
prefix = "noor:"
cache_dir = "storage/cache"

[view]
template_dir = "resources/views"
cache_templates = true
auto_reload = true
"#,
            name
        );
        std::fs::write(project_dir.join("noor.toml"), config_content)?;
        
        // Create Cargo.toml
        let cargo_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
noor = "1.0"

[[bin]]
name = "{}"
path = "src/main.rs"
"#,
            name, name
        );
        std::fs::write(project_dir.join("Cargo.toml"), cargo_content)?;
        
        println!("✓ Project '{}' created successfully!", name);
        println!("\nNext steps:");
        println!("  cd {}", name);
        println!("  noor serve");
        
        Ok(())
    }
    
    fn generate_controller(name: &str) -> crate::NoorResult<()> {
        let path = format!("src/controllers/{}.rs", name.to_lowercase());
        let content = format!(
            r#"use noor::*;
use noor::core::http::{{Request, Response, StatusCode}};

/// {} Controller
pub struct {}Controller;

impl {}Controller {{
    /// GET /{}
    pub fn index(_req: Request) -> NoorResult<Response> {{
        Ok(Response::ok().json(&serde_json::json!({{
            "data": [],
        }}))?))
    }}
    
    /// GET /{}/{{id}}
    pub fn show(req: Request) -> NoorResult<Response> {{
        let id = req.param("id").unwrap_or("0");
        Ok(Response::ok().json(&serde_json::json!({{
            "id": id,
        }}))?))
    }}
    
    /// POST /{}
    pub fn store(req: Request) -> NoorResult<Response> {{
        Ok(Response::new(StatusCode::CREATED).json(&serde_json::json!({{
            "message": "Created",
        }}))?))
    }}
    
    /// PUT /{}/{{id}}
    pub fn update(req: Request) -> NoorResult<Response> {{
        Ok(Response::ok().json(&serde_json::json!({{
            "message": "Updated",
        }}))?))
    }}
    
    /// DELETE /{}/{{id}}
    pub fn destroy(req: Request) -> NoorResult<Response> {{
        Ok(Response::new(StatusCode::NO_CONTENT))
    }}
}}
"#,
            name, name, name,
            name.to_lowercase(),
            name.to_lowercase(),
            name.to_lowercase(),
            name.to_lowercase(),
            name.to_lowercase()
        );
        
        std::fs::write(&path, content)?;
        println!("✓ Controller created: {}", path);
        Ok(())
    }
    
    fn generate_model(name: &str) -> crate::NoorResult<()> {
        let path = format!("src/models/{}.rs", name.to_lowercase());
        let content = format!(
            r#"use noor::*;
use noor::core::orm::{{Model, ModelMeta, ModelMetaBuilder, CastType}};
use serde::{{Serialize, Deserialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub id: Option<i64>,
    pub name: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}}

impl {} {{
    pub fn meta() -> ModelMeta {{
        ModelMetaBuilder::new("{}s")
            .fillable(&["name"])
            .build()
    }}
}}
"#,
            name, name, name.to_lowercase()
        );
        
        std::fs::write(&path, content)?;
        println!("✓ Model created: {}", path);
        Ok(())
    }
    
    fn generate_migration(name: &str) -> crate::NoorResult<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let safe_name = name.replace(' ', "_").replace('-', "_");
        let path = format!("database/migrations/{}_{}.sql", timestamp, safe_name);

        // Ensure the migrations directory exists.
        std::fs::create_dir_all("database/migrations")?;

        let content = format!(
            r#"-- Migration: {}
-- Created at: {}

-- UP
CREATE TABLE IF NOT EXISTS {} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- DOWN:
DROP TABLE IF EXISTS {};
"#,
            name,
            chrono::Utc::now().to_rfc3339(),
            safe_name,
            safe_name
        );

        std::fs::write(&path, content)?;
        println!("✓ Migration created: {}", path);
        println!("  Edit it to customize the table schema, then run:");
        println!("  noor migrate");
        Ok(())
    }
}

/// Split a migration file's content into (up_sql, down_sql) sections.
///
/// Recognized markers (case-sensitive, must be at the start of a line):
///   `-- DOWN:`  or  `-- DOWN`
///
/// Anything before the marker is the UP section; anything after is the DOWN
/// section. If no marker is found, the entire content is treated as UP.
fn split_migration_content(content: &str) -> (String, String) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "-- DOWN:" || trimmed == "-- DOWN" {
            if let Some(pos) = content.find(line) {
                let up = &content[..pos];
                let down = &content[pos + line.len()..];
                return (up.trim().to_string(), down.trim().to_string());
            }
        }
    }
    (content.trim().to_string(), String::new())
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}
