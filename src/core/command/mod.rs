// ============================================================
// Command Bus Pattern - نمط ناقل الأوامر
// ============================================================
// Decouples requesting an action from performing it.
// Supports middleware (pipelines) for commands.
//
// يفصل طلب الإجراء عن تنفيذه.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// أمر
pub trait Command: Send + Sync + Clone {
    type Result: Send + Sync;
    
    fn command_name(&self) -> &str;
}

/// معالج الأوامر
pub trait CommandHandler<C: Command>: Send + Sync {
    fn handle(&self, command: C) -> crate::NoorResult<C::Result>;
}

/// وسيط الأوامر (pipeline)
pub trait CommandMiddleware<C: Command>: Send + Sync {
    fn handle(&self, command: &mut C, next: &dyn Fn(&mut C) -> crate::NoorResult<()>) -> crate::NoorResult<()>;
}

/// ناقل الأوامر
pub struct CommandBus {
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Box<dyn std::any::Any>, &CommandBus) -> crate::NoorResult<Box<dyn std::any::Any + Send + Sync>> + Send + Sync>>>>,
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
    
    /// تسجيل معالج أوامر
    pub fn register<C, H>(&self, handler: H)
    where
        C: Command + 'static,
        C::Result: 'static,
        H: CommandHandler<C> + 'static,
    {
        let handler = Arc::new(handler);
        
        self.handlers.write().insert(
            std::any::type_name::<C>().to_string(),
            Box::new(move |cmd_any, _bus| {
                let cmd = cmd_any.downcast_ref::<C>()
                    .ok_or_else(|| crate::NoorError::Internal("Invalid command type".to_string()))?;
                let result = handler.handle(cmd.clone())?;
                Ok(Box::new(result) as Box<dyn std::any::Any + Send + Sync>)
            }),
        );
    }
    
    /// تنفيذ أمر
    pub fn dispatch<C>(&self, command: C) -> crate::NoorResult<C::Result>
    where
        C: Command + 'static,
        C::Result: 'static,
    {
        let command_name = std::any::type_name::<C>();
        
        let handlers = self.handlers.read();
        let handler = handlers.get(command_name)
            .ok_or_else(|| crate::NoorError::Internal(
                format!("No handler registered for command: {}", command_name)
            ))?;
        
        let cmd_box: Box<dyn std::any::Any> = Box::new(command);
        let result_box = handler(cmd_box, self)?;
        
        let result = result_box.downcast::<C::Result>()
            .map_err(|_| crate::NoorError::Internal("Invalid result type".to_string()))?;
        
        Ok(*result)
    }
}

/// أمر بسيط مع بيانات
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCommand {
    pub name: String,
    pub payload: serde_json::Value,
}

impl Command for SimpleCommand {
    type Result = serde_json::Value;
    
    fn command_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone)]
    struct CreateUser {
        name: String,
        email: String,
    }
    
    impl Command for CreateUser {
        type Result = i64;
        
        fn command_name(&self) -> &str {
            "CreateUser"
        }
    }
    
    struct CreateUserHandler;
    
    impl CommandHandler<CreateUser> for CreateUserHandler {
        fn handle(&self, cmd: CreateUser) -> crate::NoorResult<i64> {
            // محاكاة إنشاء مستخدم
            Ok(1)
        }
    }
    
    #[test]
    fn test_command_bus() {
        let bus = CommandBus::new();
        bus.register(CreateUserHandler);
        
        let cmd = CreateUser {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let result = bus.dispatch(cmd).unwrap();
        assert_eq!(result, 1);
    }
}
