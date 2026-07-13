// ============================================================
// CQRS Pattern - فصل المسؤولية بين الأوامر والاستعلامات
// ============================================================
// Command Query Responsibility Segregation
// Separates write operations (commands) from read operations (queries)
// for better scalability and performance.
//
// فصل عمليات الكتابة عن عمليات القراءة.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

// ============= Command Side =============

/// Command trait for write operations
pub trait Command: Send + Sync + Clone {
    type Result: Send + Sync;
    fn command_type() -> &'static str;
}

/// Command handler trait
pub trait CommandHandler<C: Command>: Send + Sync {
    fn handle(&self, command: C) -> crate::NoorResult<C::Result>;
}

/// Command bus for dispatching commands
pub struct CommandBus {
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Box<dyn std::any::Any>) -> crate::NoorResult<Box<dyn std::any::Any + Send + Sync>> + Send + Sync>>>>,
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a command handler
    pub fn register<C, H>(&self, handler: H)
    where
        C: Command + 'static,
        C::Result: 'static,
        H: CommandHandler<C> + 'static,
    {
        let handler = Arc::new(handler);
        
        self.handlers.write().insert(
            C::command_type().to_string(),
            Box::new(move |cmd_box| {
                let cmd = cmd_box.downcast_ref::<C>()
                    .ok_or_else(|| crate::NoorError::Internal("Invalid command type".to_string()))?
                    .clone();
                let result = handler.handle(cmd)?;
                Ok(Box::new(result) as Box<dyn std::any::Any + Send + Sync>)
            }),
        );
    }
    
    /// Dispatch a command
    pub fn dispatch<C: Command + 'static>(&self, command: C) -> crate::NoorResult<C::Result>
    where
        C::Result: 'static,
    {
        let cmd_type = C::command_type();
        
        let handlers = self.handlers.read();
        let handler = handlers.get(cmd_type)
            .ok_or_else(|| crate::NoorError::Internal(format!("No handler for command: {}", cmd_type)))?;
        
        let cmd_box: Box<dyn std::any::Any> = Box::new(command);
        let result_box = handler(cmd_box)?;
        
        let result = result_box.downcast::<C::Result>()
            .map_err(|_| crate::NoorError::Internal("Invalid result type".to_string()))?;
        
        Ok(*result)
    }
}

// ============= Query Side =============

/// Query trait for read operations
pub trait Query: Send + Sync + Clone {
    type Result: Send + Sync;
    fn query_type() -> &'static str;
}

/// Query handler trait
pub trait QueryHandler<Q: Query>: Send + Sync {
    fn handle(&self, query: Q) -> crate::NoorResult<Q::Result>;
}

/// Query bus for dispatching queries
pub struct QueryBus {
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Box<dyn std::any::Any>) -> crate::NoorResult<Box<dyn std::any::Any + Send + Sync>> + Send + Sync>>>>,
}

impl Default for QueryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a query handler
    pub fn register<Q, H>(&self, handler: H)
    where
        Q: Query + 'static,
        Q::Result: 'static,
        H: QueryHandler<Q> + 'static,
    {
        let handler = Arc::new(handler);
        
        self.handlers.write().insert(
            Q::query_type().to_string(),
            Box::new(move |query_box| {
                let query = query_box.downcast_ref::<Q>()
                    .ok_or_else(|| crate::NoorError::Internal("Invalid query type".to_string()))?
                    .clone();
                let result = handler.handle(query)?;
                Ok(Box::new(result) as Box<dyn std::any::Any + Send + Sync>)
            }),
        );
    }
    
    /// Dispatch a query
    pub fn dispatch<Q: Query + 'static>(&self, query: Q) -> crate::NoorResult<Q::Result>
    where
        Q::Result: 'static,
    {
        let query_type = Q::query_type();
        
        let handlers = self.handlers.read();
        let handler = handlers.get(query_type)
            .ok_or_else(|| crate::NoorError::Internal(format!("No handler for query: {}", query_type)))?;
        
        let query_box: Box<dyn std::any::Any> = Box::new(query);
        let result_box = handler(query_box)?;
        
        let result = result_box.downcast::<Q::Result>()
            .map_err(|_| crate::NoorError::Internal("Invalid result type".to_string()))?;
        
        Ok(*result)
    }
}

// ============= CQRS System =============

/// CQRS system combining command and query buses
pub struct CqrsSystem {
    pub command_bus: Arc<CommandBus>,
    pub query_bus: Arc<QueryBus>,
}

impl Default for CqrsSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CqrsSystem {
    pub fn new() -> Self {
        Self {
            command_bus: Arc::new(CommandBus::new()),
            query_bus: Arc::new(QueryBus::new()),
        }
    }
    
    /// Execute a command (write operation)
    pub fn execute<C: Command + 'static>(&self, command: C) -> crate::NoorResult<C::Result>
    where
        C::Result: 'static,
    {
        self.command_bus.dispatch(command)
    }
    
    /// Execute a query (read operation)
    pub fn query<Q: Query + 'static>(&self, query: Q) -> crate::NoorResult<Q::Result>
    where
        Q::Result: 'static,
    {
        self.query_bus.dispatch(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test commands
    #[derive(Debug, Clone)]
    struct CreateUserCommand {
        name: String,
        email: String,
    }
    
    impl Command for CreateUserCommand {
        type Result = i64;
        fn command_type() -> &'static str { "CreateUserCommand" }
    }
    
    struct CreateUserHandler;
    
    impl CommandHandler<CreateUserCommand> for CreateUserHandler {
        fn handle(&self, cmd: CreateUserCommand) -> crate::NoorResult<i64> {
            // Simulate user creation
            Ok(1)
        }
    }
    
    // Test queries
    #[derive(Debug, Clone)]
    struct GetUserByIdQuery {
        id: i64,
    }
    
    impl Query for GetUserByIdQuery {
        type Result = Option<String>;
        fn query_type() -> &'static str { "GetUserByIdQuery" }
    }
    
    struct GetUserByIdHandler;
    
    impl QueryHandler<GetUserByIdQuery> for GetUserByIdHandler {
        fn handle(&self, query: GetUserByIdQuery) -> crate::NoorResult<Option<String>> {
            if query.id == 1 {
                Ok(Some("John Doe".to_string()))
            } else {
                Ok(None)
            }
        }
    }
    
    #[test]
    fn test_command_dispatch() {
        let system = CqrsSystem::new();
        system.command_bus.register(CreateUserHandler);
        
        let cmd = CreateUserCommand {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let id = system.execute(cmd).unwrap();
        assert_eq!(id, 1);
    }
    
    #[test]
    fn test_query_dispatch() {
        let system = CqrsSystem::new();
        system.query_bus.register(GetUserByIdHandler);
        
        let query = GetUserByIdQuery { id: 1 };
        let result = system.query(query).unwrap();
        
        assert_eq!(result, Some("John Doe".to_string()));
    }
    
    #[test]
    fn test_cqrs_separation() {
        let system = CqrsSystem::new();
        
        system.command_bus.register(CreateUserHandler);
        system.query_bus.register(GetUserByIdHandler);
        
        // Write via command
        let id = system.execute(CreateUserCommand {
            name: "Jane".to_string(),
            email: "jane@example.com".to_string(),
        }).unwrap();
        
        // Read via query
        let user = system.query(GetUserByIdQuery { id }).unwrap();
        
        assert_eq!(id, 1);
        assert!(user.is_some());
    }
}
