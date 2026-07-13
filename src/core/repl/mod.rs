// ============================================================
// REPL (Interactive Console) - وحدة التفاعل
// ============================================================
// Interactive Read-Eval-Print Loop for development.
// Execute commands and inspect application state interactively.
//
// وحدة تفاعلية للتطوير.
// ============================================================

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use parking_lot::RwLock;

/// REPL command
pub struct ReplCommand {
    pub name: String,
    pub description: String,
    pub handler: ReplHandler,
}

/// REPL command handler
type ReplHandler = Arc<dyn Fn(&ReplContext, &[String]) -> crate::NoorResult<String> + Send + Sync>;

/// REPL context (available to commands)
pub struct ReplContext {
    pub variables: Arc<RwLock<HashMap<String, String>>>,
    pub history: Arc<RwLock<Vec<String>>>,
}

impl Default for ReplContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplContext {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Set a variable
    pub fn set_var(&self, name: &str, value: &str) {
        self.variables.write().insert(name.to_string(), value.to_string());
    }
    
    /// Get a variable
    pub fn get_var(&self, name: &str) -> Option<String> {
        self.variables.read().get(name).cloned()
    }
    
    /// Get all variables
    pub fn vars(&self) -> HashMap<String, String> {
        self.variables.read().clone()
    }
    
    /// Add to history
    pub fn add_history(&self, input: &str) {
        self.history.write().push(input.to_string());
    }
    
    /// Get history
    pub fn history(&self) -> Vec<String> {
        self.history.read().clone()
    }
}

/// Interactive REPL
pub struct Repl {
    commands: Arc<RwLock<HashMap<String, ReplCommand>>>,
    context: ReplContext,
    prompt: String,
    banner: String,
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl Repl {
    pub fn new() -> Self {
        let repl = Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            context: ReplContext::new(),
            prompt: "noor> ".to_string(),
            banner: "Noor Framework REPL v1.0".to_string(),
        };
        
        repl.register_builtin_commands();
        repl
    }
    
    /// Set the prompt
    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
    }
    
    /// Set the banner
    pub fn set_banner(&mut self, banner: &str) {
        self.banner = banner.to_string();
    }
    
    /// Register a command
    pub fn register<F>(&self, name: &str, description: &str, handler: F)
    where
        F: Fn(&ReplContext, &[String]) -> crate::NoorResult<String> + Send + Sync + 'static,
    {
        self.commands.write().insert(
            name.to_string(),
            ReplCommand {
                name: name.to_string(),
                description: description.to_string(),
                handler: Arc::new(handler),
            },
        );
    }
    
    /// Execute a single command
    pub fn execute(&self, input: &str) -> crate::NoorResult<String> {
        self.context.add_history(input);
        
        let input = input.trim();
        
        if input.is_empty() {
            return Ok(String::new());
        }
        
        // Parse command and arguments
        let parts: Vec<String> = shell_words::parse(input).unwrap_or_else(|_| {
            input.split_whitespace().map(|s| s.to_string()).collect()
        });
        
        if parts.is_empty() {
            return Ok(String::new());
        }
        
        let command_name = &parts[0];
        let args = &parts[1..];
        
        // Handle variable assignment (var=value)
        if command_name.contains('=') {
            let eq_pos = command_name.find('=').unwrap();
            let var_name = &command_name[..eq_pos];
            let var_value = &command_name[eq_pos + 1..];
            self.context.set_var(var_name, var_value);
            return Ok(format!("{} = {}", var_name, var_value));
        }
        
        // Look up command
        let commands = self.commands.read();
        
        if let Some(command) = commands.get(command_name) {
            (command.handler)(&self.context, args)
        } else {
            Err(crate::NoorError::Internal(format!(
                "Unknown command: {}. Type 'help' for available commands.",
                command_name
            )))
        }
    }
    
    /// Run the REPL interactively
    pub fn run(&self) {
        println!("{}", self.banner);
        println!("Type 'help' for available commands, 'exit' to quit.\n");
        
        let stdin = io::stdin();
        
        loop {
            // Print prompt
            print!("{}", self.prompt);
            io::stdout().flush().unwrap();
            
            // Read input
            let mut input = String::new();
            if stdin.lock().read_line(&mut input).unwrap() == 0 {
                break; // EOF
            }
            
            let input = input.trim();
            
            // Check for exit
            if input == "exit" || input == "quit" {
                println!("Goodbye!");
                break;
            }
            
            // Execute command
            match self.execute(input) {
                Ok(output) => {
                    if !output.is_empty() {
                        println!("{}", output);
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }
    
    /// Register built-in commands
    fn register_builtin_commands(&self) {
        // help
        self.register("help", "Show available commands", |ctx, _args| {
            let mut output = String::new();
            output.push_str("Available commands:\n");
            output.push_str("  help              Show this help\n");
            output.push_str("  exit, quit        Exit the REPL\n");
            output.push_str("  vars              Show all variables\n");
            output.push_str("  history           Show command history\n");
            output.push_str("  echo <text>       Print text\n");
            output.push_str("  set <name> <val>  Set a variable\n");
            output.push_str("  get <name>        Get a variable\n");
            output.push_str("  clear             Clear screen\n");
            output.push_str("  eval <expr>       Evaluate expression\n");
            output.push_str("  info              Show application info\n");
            Ok(output)
        });
        
        // vars
        self.register("vars", "Show all variables", |ctx, _args| {
            let vars = ctx.vars();
            if vars.is_empty() {
                Ok("No variables set".to_string())
            } else {
                let mut output = String::new();
                for (k, v) in vars {
                    output.push_str(&format!("  {} = {}\n", k, v));
                }
                Ok(output.trim().to_string())
            }
        });
        
        // history
        self.register("history", "Show command history", |ctx, _args| {
            let history = ctx.history();
            if history.is_empty() {
                Ok("No history".to_string())
            } else {
                let mut output = String::new();
                for (i, cmd) in history.iter().enumerate() {
                    output.push_str(&format!("  {}: {}\n", i + 1, cmd));
                }
                Ok(output.trim().to_string())
            }
        });
        
        // echo
        self.register("echo", "Print text", |_ctx, args| {
            Ok(args.join(" "))
        });
        
        // set
        self.register("set", "Set a variable", |ctx, args| {
            if args.len() < 2 {
                return Err(crate::NoorError::Internal("Usage: set <name> <value>".to_string()));
            }
            let name = &args[0];
            let value = args[1..].join(" ");
            ctx.set_var(name, &value);
            Ok(format!("{} = {}", name, value))
        });
        
        // get
        self.register("get", "Get a variable", |ctx, args| {
            if args.is_empty() {
                return Err(crate::NoorError::Internal("Usage: get <name>".to_string()));
            }
            
            ctx.get_var(&args[0])
                .ok_or_else(|| crate::NoorError::Internal(format!("Variable '{}' not set", args[0])))
        });
        
        // clear
        self.register("clear", "Clear screen", |_ctx, _args| {
            print!("\x1b[2J\x1b[H");
            Ok(String::new())
        });
        
        // eval
        self.register("eval", "Evaluate expression", |_ctx, args| {
            if args.is_empty() {
                return Err(crate::NoorError::Internal("Usage: eval <expression>".to_string()));
            }
            
            let expr = args.join(" ");
            // In a real implementation, we'd use a proper expression evaluator
            Ok(format!("Result: {} (not actually evaluated)", expr))
        });
        
        // info
        self.register("info", "Show application info", |_ctx, _args| {
            Ok(format!(
                "Noor Framework v{}\n  Runtime: Rust\n  Uptime: 0s\n  Variables: see 'vars'",
                crate::VERSION
            ))
        });
    }
    
    /// Get all registered commands
    pub fn commands(&self) -> Vec<String> {
        self.commands.read().keys().cloned().collect()
    }
    
    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.read().len()
    }
}

/// Simple shell-like word splitting
mod shell_words {
    pub fn parse(input: &str) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        
        for ch in input.chars() {
            if in_quotes {
                if ch == quote_char {
                    in_quotes = false;
                } else {
                    current.push(ch);
                }
            } else if ch == '"' || ch == '\'' {
                in_quotes = true;
                quote_char = ch;
            } else if ch.is_whitespace() {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(ch);
            }
        }
        
        if !current.is_empty() {
            result.push(current);
        }
        
        if in_quotes {
            return Err("Unclosed quote".to_string());
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_repl_echo() {
        let repl = Repl::new();
        
        let result = repl.execute("echo Hello World").unwrap();
        assert_eq!(result, "Hello World");
    }
    
    #[test]
    fn test_repl_set_get() {
        let repl = Repl::new();
        
        repl.execute("set name John").unwrap();
        let result = repl.execute("get name").unwrap();
        assert_eq!(result, "John");
    }
    
    #[test]
    fn test_repl_variable_assignment() {
        let repl = Repl::new();
        
        repl.execute("x=42").unwrap();
        let result = repl.execute("get x").unwrap();
        assert_eq!(result, "42");
    }
    
    #[test]
    fn test_repl_vars() {
        let repl = Repl::new();
        
        repl.execute("set a 1").unwrap();
        repl.execute("set b 2").unwrap();
        
        let result = repl.execute("vars").unwrap();
        assert!(result.contains("a = 1"));
        assert!(result.contains("b = 2"));
    }
    
    #[test]
    fn test_repl_history() {
        let repl = Repl::new();
        
        repl.execute("echo first").unwrap();
        repl.execute("echo second").unwrap();
        
        let result = repl.execute("history").unwrap();
        assert!(result.contains("echo first"));
        assert!(result.contains("echo second"));
    }
    
    #[test]
    fn test_repl_unknown_command() {
        let repl = Repl::new();
        
        let result = repl.execute("unknown_command");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_repl_help() {
        let repl = Repl::new();
        
        let result = repl.execute("help").unwrap();
        assert!(result.contains("Available commands"));
        assert!(result.contains("help"));
        assert!(result.contains("exit"));
    }
    
    #[test]
    fn test_repl_info() {
        let repl = Repl::new();
        
        let result = repl.execute("info").unwrap();
        assert!(result.contains("Noor Framework"));
    }
    
    #[test]
    fn test_shell_words_parse() {
        let result = shell_words::parse("hello world").unwrap();
        assert_eq!(result, vec!["hello", "world"]);
        
        let result = shell_words::parse("hello \"world foo\"").unwrap();
        assert_eq!(result, vec!["hello", "world foo"]);
        
        let result = shell_words::parse("set name 'John Doe'").unwrap();
        assert_eq!(result, vec!["set", "name", "John Doe"]);
    }
}
