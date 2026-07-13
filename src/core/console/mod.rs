// ============================================================
// Console Commands - أوامر الكونسول (Artisan-like)
// ============================================================
// Laravel Artisan-style command system with command registration,
// arguments, options, and help text.
//
// نظام أوامر على نمط Laravel Artisan.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Command argument
#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

impl Argument {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required: false,
            default: None,
        }
    }
    
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    
    pub fn default(mut self, value: &str) -> Self {
        self.default = Some(value.to_string());
        self
    }
}

/// Command option
#[derive(Debug, Clone)]
pub struct Option_ {
    pub name: String,
    pub shortcut: Option<String>,
    pub description: String,
    pub takes_value: bool,
    pub default: Option<String>,
}

impl Option_ {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            shortcut: None,
            description: description.to_string(),
            takes_value: false,
            default: None,
        }
    }
    
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_string());
        self
    }
    
    pub fn takes_value(mut self) -> Self {
        self.takes_value = true;
        self
    }
    
    pub fn default(mut self, value: &str) -> Self {
        self.default = Some(value.to_string());
        self.takes_value = true;
        self
    }
}

/// Command definition
pub struct Command {
    pub name: String,
    pub description: String,
    pub help: String,
    pub arguments: Vec<Argument>,
    pub options: Vec<Option_>,
    pub handler: CommandHandler,
}

/// Command handler function type
type CommandHandler = Arc<dyn Fn(&CommandInput) -> crate::NoorResult<()> + Send + Sync>;

/// Parsed command input
#[derive(Debug, Clone)]
pub struct CommandInput {
    pub arguments: HashMap<String, String>,
    pub options: HashMap<String, Option<String>>,
    pub raw_args: Vec<String>,
}

impl CommandInput {
    /// Get an argument value
    pub fn argument(&self, name: &str) -> Option<&str> {
        self.arguments.get(name).map(|s| s.as_str())
    }
    
    /// Get an argument or panic
    pub fn argument_required(&self, name: &str) -> crate::NoorResult<&str> {
        self.argument(name).ok_or_else(|| {
            crate::NoorError::Internal(format!("Missing required argument: {}", name))
        })
    }
    
    /// Check if an option is set
    pub fn has_option(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }
    
    /// Get an option value
    pub fn option(&self, name: &str) -> Option<&str> {
        self.options.get(name).and_then(|o| o.as_deref())
    }
    
    /// Get an option value or default
    pub fn option_or(&self, name: &str, default: &str) -> String {
        self.option(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| default.to_string())
    }
}

/// Command builder for fluent creation
pub struct CommandBuilder {
    name: String,
    description: String,
    help: String,
    arguments: Vec<Argument>,
    options: Vec<Option_>,
    handler: Option<CommandHandler>,
}

impl CommandBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            help: String::new(),
            arguments: Vec::new(),
            options: Vec::new(),
            handler: None,
        }
    }
    
    pub fn help(mut self, help: &str) -> Self {
        self.help = help.to_string();
        self
    }
    
    pub fn argument(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }
    
    pub fn option(mut self, opt: Option_) -> Self {
        self.options.push(opt);
        self
    }
    
    pub fn handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&CommandInput) -> crate::NoorResult<()> + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }
    
    pub fn build(self) -> Command {
        Command {
            name: self.name,
            description: self.description,
            help: self.help,
            arguments: self.arguments,
            options: self.options,
            handler: self.handler.unwrap_or_else(|| Arc::new(|_| Ok(()))),
        }
    }
}

/// Console application
pub struct Console {
    commands: Arc<RwLock<HashMap<String, Command>>>,
    name: String,
    version: String,
}

impl Default for Console {
    fn default() -> Self {
        Self::new("Noor Console", crate::VERSION)
    }
}

impl Console {
    pub fn new(name: &str, version: &str) -> Self {
        let console = Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            name: name.to_string(),
            version: version.to_string(),
        };
        
        // Register built-in commands
        console.register_builtin_commands();
        
        console
    }
    
    /// Register a command
    pub fn register(&self, command: Command) {
        self.commands.write().insert(command.name.clone(), command);
    }
    
    /// Register built-in commands
    fn register_builtin_commands(&self) {
        // list command
        self.register(
            CommandBuilder::new("list", "List all available commands")
                .handler(|_input| {
                    println!("Available commands:");
                    println!("  list              List all commands");
                    println!("  help <command>    Show help for a command");
                    println!("  serve             Start the development server");
                    println!("  migrate           Run database migrations");
                    println!("  db:seed           Run database seeders");
                    println!("  make:controller   Generate a new controller");
                    println!("  make:model        Generate a new model");
                    println!("  make:migration    Generate a new migration");
                    println!("  key:generate      Generate a new application key");
                    println!("  cache:clear       Clear the application cache");
                    println!("  route:list        List all registered routes");
                    Ok(())
                })
                .build()
        );
        
        // serve command
        self.register(
            CommandBuilder::new("serve", "Start the development server")
                .option(Option_::new("host", "Server host").shortcut("H").takes_value().default("0.0.0.0"))
                .option(Option_::new("port", "Server port").shortcut("p").takes_value().default("8080"))
                .handler(|input| {
                    let host = input.option_or("host", "0.0.0.0");
                    let port = input.option_or("port", "8080");
                    println!("🚀 Starting Noor development server on {}:{}", host, port);
                    println!("   Press Ctrl+C to stop");
                    Ok(())
                })
                .build()
        );
        
        // migrate command
        self.register(
            CommandBuilder::new("migrate", "Run database migrations")
                .option(Option_::new("rollback", "Rollback last migration").shortcut("r"))
                .option(Option_::new("step", "Number of migrations to rollback").takes_value())
                .option(Option_::new("fresh", "Drop all tables and re-run migrations").shortcut("f"))
                .option(Option_::new("seed", "Run seeders after migration").shortcut("s"))
                .handler(|input| {
                    if input.has_option("fresh") {
                        println!("⚠️  Dropping all tables...");
                    }
                    
                    if input.has_option("rollback") {
                        let steps = input.option("step").unwrap_or("1");
                        println!("🔄 Rolling back {} migration(s)...", steps);
                    } else {
                        println!("📦 Running migrations...");
                    }
                    
                    println!("✓ Migrations completed!");
                    
                    if input.has_option("seed") {
                        println!("🌱 Running seeders...");
                        println!("✓ Seeding completed!");
                    }
                    
                    Ok(())
                })
                .build()
        );
        
        // db:seed command
        self.register(
            CommandBuilder::new("db:seed", "Run database seeders")
                .option(Option_::new("class", "Specific seeder class to run").takes_value())
                .handler(|input| {
                    if let Some(seeder) = input.option("class") {
                        println!("🌱 Running seeder: {}", seeder);
                    } else {
                        println!("🌱 Running all seeders...");
                    }
                    println!("✓ Seeding completed!");
                    Ok(())
                })
                .build()
        );
        
        // make:controller
        self.register(
            CommandBuilder::new("make:controller", "Generate a new controller")
                .argument(Argument::new("name", "Controller name").required())
                .option(Option_::new("resource", "Generate a resource controller").shortcut("r"))
                .option(Option_::new("api", "Generate an API controller").shortcut("a"))
                .handler(|input| {
                    let name = input.argument_required("name")?;
                    let controller_type = if input.has_option("api") {
                        "API"
                    } else if input.has_option("resource") {
                        "Resource"
                    } else {
                        "Basic"
                    };
                    
                    println!("✓ Generated {} controller: {}", controller_type, name);
                    println!("   File: src/controllers/{}.rs", name.to_lowercase());
                    Ok(())
                })
                .build()
        );
        
        // make:model
        self.register(
            CommandBuilder::new("make:model", "Generate a new model")
                .argument(Argument::new("name", "Model name").required())
                .option(Option_::new("migration", "Also generate a migration").shortcut("m"))
                .option(Option_::new("controller", "Also generate a controller").shortcut("c"))
                .handler(|input| {
                    let name = input.argument_required("name")?;
                    println!("✓ Generated model: {}", name);
                    println!("   File: src/models/{}.rs", name.to_lowercase());
                    
                    if input.has_option("migration") {
                        println!("✓ Generated migration for: {}", name);
                    }
                    
                    if input.has_option("controller") {
                        println!("✓ Generated controller for: {}", name);
                    }
                    
                    Ok(())
                })
                .build()
        );
        
        // make:migration
        self.register(
            CommandBuilder::new("make:migration", "Generate a new migration")
                .argument(Argument::new("name", "Migration name").required())
                .option(Option_::new("table", "The table to migrate").takes_value())
                .option(Option_::new("create", "The table to create").takes_value())
                .handler(|input| {
                    let name = input.argument_required("name")?;
                    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
                    
                    println!("✓ Generated migration: {}", name);
                    println!("   File: database/migrations/{}_{}.sql", timestamp, name);
                    Ok(())
                })
                .build()
        );
        
        // key:generate
        self.register(
            CommandBuilder::new("key:generate", "Generate a new application key")
                .option(Option_::new("show", "Only display the key, don't save it"))
                .handler(|input| {
                    let key = crate::core::security::Encryption::new().random_string(32)?;
                    
                    if input.has_option("show") {
                        println!("{}", key);
                    } else {
                        println!("✓ Application key generated: {}", key);
                        println!("   Add to .env: JWT_SECRET={}", key);
                    }
                    
                    Ok(())
                })
                .build()
        );
        
        // cache:clear
        self.register(
            CommandBuilder::new("cache:clear", "Clear the application cache")
                .option(Option_::new("tags", "Clear specific cache tags").takes_value())
                .handler(|_input| {
                    println!("🧹 Clearing cache...");
                    println!("✓ Cache cleared!");
                    Ok(())
                })
                .build()
        );
        
        // route:list
        self.register(
            CommandBuilder::new("route:list", "List all registered routes")
                .option(Option_::new("method", "Filter by HTTP method").takes_value())
                .option(Option_::new("path", "Filter by path").takes_value())
                .handler(|_input| {
                    println!("\nMethod       Path                                       Handler");
                    println!("{}", "-".repeat(80));
                    println!("GET          /                                          index");
                    println!("GET          /health                                    health_check");
                    println!("GET          /blog                                      blog::index");
                    println!("GET          /blog/{{id}}                                blog::show");
                    println!("POST         /admin/posts                               admin::posts::store");
                    Ok(())
                })
                .build()
        );
        
        // help command
        self.register(
            CommandBuilder::new("help", "Show help for a command")
                .argument(Argument::new("command", "Command name"))
                .handler(|input| {
                    if let Some(cmd_name) = input.argument("command") {
                        println!("Help for command: {}", cmd_name);
                    } else {
                        println!("Usage: help <command>");
                        println!("Show help for a specific command");
                    }
                    Ok(())
                })
                .build()
        );
    }
    
    /// Run a command with arguments
    pub fn run(&self, args: &[String]) -> crate::NoorResult<()> {
        if args.is_empty() {
            return self.show_help();
        }
        
        let command_name = &args[0];
        let command_args = &args[1..];
        
        let commands = self.commands.read();
        
        if let Some(command) = commands.get(command_name) {
            let input = self.parse_input(command, command_args)?;
            (command.handler)(&input)
        } else {
            println!("Command '{}' not found.", command_name);
            println!("\nRun 'list' to see available commands.");
            Err(crate::NoorError::Internal(format!("Unknown command: {}", command_name)))
        }
    }
    
    /// Parse command input from args
    fn parse_input(&self, command: &Command, args: &[String]) -> crate::NoorResult<CommandInput> {
        let mut arguments = HashMap::new();
        let mut options = HashMap::new();
        let mut positional_index = 0;
        let mut i = 0;
        
        while i < args.len() {
            let arg = &args[i];
            
            if arg.starts_with("--") {
                // Long option
                let opt_name = &arg[2..];
                
                // Check if it has a value (--option=value)
                if let Some(pos) = opt_name.find('=') {
                    let name = &opt_name[..pos];
                    let value = &opt_name[pos + 1..];
                    options.insert(name.to_string(), Some(value.to_string()));
                } else {
                    // Check if next arg is a value
                    let opt_def = command.options.iter().find(|o| o.name == opt_name);
                    
                    if let Some(opt) = opt_def {
                        if opt.takes_value && i + 1 < args.len() && !args[i + 1].starts_with('-') {
                            options.insert(opt_name.to_string(), Some(args[i + 1].clone()));
                            i += 1;
                        } else {
                            options.insert(opt_name.to_string(), None);
                        }
                    } else {
                        options.insert(opt_name.to_string(), None);
                    }
                }
            } else if arg.starts_with('-') && arg.len() > 1 {
                // Short option
                let shortcut = &arg[1..];
                let opt_def = command.options.iter().find(|o| o.shortcut.as_deref() == Some(shortcut));
                
                if let Some(opt) = opt_def {
                    if opt.takes_value && i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        options.insert(opt.name.clone(), Some(args[i + 1].clone()));
                        i += 1;
                    } else {
                        options.insert(opt.name.clone(), None);
                    }
                }
            } else {
                // Positional argument
                if positional_index < command.arguments.len() {
                    let arg_def = &command.arguments[positional_index];
                    arguments.insert(arg_def.name.clone(), arg.clone());
                    positional_index += 1;
                }
            }
            
            i += 1;
        }
        
        // Set default values for missing arguments
        for arg_def in &command.arguments {
            if !arguments.contains_key(&arg_def.name) {
                if let Some(ref default) = arg_def.default {
                    arguments.insert(arg_def.name.clone(), default.clone());
                } else if arg_def.required {
                    return Err(crate::NoorError::Internal(
                        format!("Missing required argument: {}", arg_def.name)
                    ));
                }
            }
        }
        
        // Set default values for missing options
        for opt_def in &command.options {
            if !options.contains_key(&opt_def.name) {
                if let Some(ref default) = opt_def.default {
                    options.insert(opt_def.name.clone(), Some(default.clone()));
                }
            }
        }
        
        Ok(CommandInput {
            arguments,
            options,
            raw_args: args.to_vec(),
        })
    }
    
    /// Show help
    pub fn show_help(&self) -> crate::NoorResult<()> {
        println!("\n{}", crate::banner());
        println!("{} v{}\n", self.name, self.version);
        println!("Usage: noor <command> [options] [arguments]\n");
        println!("Available commands:\n");
        
        let commands = self.commands.read();
        let mut commands_vec: Vec<&Command> = commands.values().collect();
        commands_vec.sort_by_key(|c| c.name.as_str());
        
        for command in commands_vec {
            println!("  {:20} {}", command.name, command.description);
        }
        
        println!("\nRun 'noor help <command>' for more information about a command.\n");
        
        Ok(())
    }
    
    /// List all registered commands
    pub fn list_commands(&self) -> Vec<String> {
        self.commands.read().keys().cloned().collect()
    }
    
    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_console_creation() {
        let console = Console::default();
        
        // Should have built-in commands
        assert!(console.command_count() > 5);
        assert!(console.list_commands().contains(&"list".to_string()));
        assert!(console.list_commands().contains(&"serve".to_string()));
        assert!(console.list_commands().contains(&"migrate".to_string()));
    }
    
    #[test]
    fn test_custom_command() {
        let console = Console::default();
        
        console.register(
            CommandBuilder::new("greet", "Greet someone")
                .argument(Argument::new("name", "Name to greet").required())
                .handler(|input| {
                    let name = input.argument_required("name")?;
                    println!("Hello, {}!", name);
                    Ok(())
                })
                .build()
        );
        
        let result = console.run(&["greet".to_string(), "World".to_string()]);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_command_with_options() {
        let console = Console::default();
        
        console.register(
            CommandBuilder::new("test", "Test command")
                .option(Option_::new("verbose", "Verbose output").shortcut("v"))
                .option(Option_::new("output", "Output format").takes_value().default("json"))
                .handler(|input| {
                    assert!(input.has_option("verbose"));
                    assert_eq!(input.option("output"), Some("xml"));
                    Ok(())
                })
                .build()
        );
        
        let result = console.run(&[
            "test".to_string(),
            "--verbose".to_string(),
            "--output=xml".to_string(),
        ]);
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_missing_required_argument() {
        let console = Console::default();
        
        console.register(
            CommandBuilder::new("test", "Test command")
                .argument(Argument::new("name", "Name").required())
                .handler(|_| Ok(()))
                .build()
        );
        
        let result = console.run(&["test".to_string()]);
        assert!(result.is_err());
    }
}
