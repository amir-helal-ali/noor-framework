// ============================================================
// WebAssembly Module Support - دعم وحدات WebAssembly
// ============================================================
// Load and execute WebAssembly modules for extensibility.
// Allows plugins written in any language that compiles to WASM.
//
// تحميل وتنفيذ وحدات WebAssembly للتوسعة.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// WASM module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub name: String,
    pub version: String,
    pub path: String,
    pub size: u64,
    pub hash: String,
    pub loaded_at: i64,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
}

/// WASM module manager
pub struct WasmManager {
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
    /// Module cache directory
    cache_dir: PathBuf,
    /// Whether WASM execution is enabled
    enabled: Arc<RwLock<bool>>,
}

impl Default for WasmManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmManager {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            cache_dir: PathBuf::from("storage/wasm"),
            enabled: Arc::new(RwLock::new(true)),
        }
    }
    
    /// Set the cache directory
    pub fn cache_dir(mut self, dir: &str) -> Self {
        self.cache_dir = PathBuf::from(dir);
        self
    }
    
    /// Enable or disable WASM execution
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }
    
    /// Check if WASM is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }
    
    /// Load a WASM module from file
    pub fn load(&self, name: &str, path: &str) -> crate::NoorResult<WasmModule> {
        let path_obj = Path::new(path);
        
        if !path_obj.exists() {
            return Err(crate::NoorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("WASM module not found: {}", path),
            )));
        }
        
        let content = std::fs::read(path_obj)?;
        let metadata = std::fs::metadata(path_obj)?;
        
        // Verify WASM magic number
        if content.len() < 4 || &content[..4] != b"\x00asm" {
            return Err(crate::NoorError::Internal("Invalid WASM file".to_string()));
        }
        
        // Generate hash
        let hash = crate::core::security::Encryption::sha256_hex(&content);
        
        // In a real implementation, we'd use wasmtime or wasmer to:
        // 1. Parse the module
        // 2. Extract exports and imports
        // 3. Compile the module
        
        let module = WasmModule {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            path: path.to_string(),
            size: metadata.len(),
            hash,
            loaded_at: chrono::Utc::now().timestamp(),
            exports: vec!["handle_request".to_string(), "process_data".to_string()],
            imports: vec![],
        };
        
        self.modules.write().insert(name.to_string(), module.clone());
        
        tracing::info!("Loaded WASM module: {} ({} bytes)", name, module.size);
        
        Ok(module)
    }
    
    /// Unload a module
    pub fn unload(&self, name: &str) -> bool {
        self.modules.write().remove(name).is_some()
    }
    
    /// Get a loaded module
    pub fn get(&self, name: &str) -> Option<WasmModule> {
        self.modules.read().get(name).cloned()
    }
    
    /// List all loaded modules
    pub fn list(&self) -> Vec<WasmModule> {
        self.modules.read().values().cloned().collect()
    }
    
    /// Get the number of loaded modules
    pub fn count(&self) -> usize {
        self.modules.read().len()
    }
    
    /// Execute a function in the module
    pub fn execute(&self, module_name: &str, function: &str, args: &[serde_json::Value]) -> crate::NoorResult<serde_json::Value> {
        if !self.is_enabled() {
            return Err(crate::NoorError::Internal("WASM execution is disabled".to_string()));
        }
        
        let module = self.get(module_name)
            .ok_or_else(|| crate::NoorError::Internal(format!("Module '{}' not loaded", module_name)))?;
        
        if !module.exports.contains(&function.to_string()) {
            return Err(crate::NoorError::Internal(
                format!("Function '{}' not exported by module '{}'", function, module_name)
            ));
        }
        
        // In a real implementation:
        // let engine = Engine::default();
        // let module = Module::from_file(&engine, &module.path)?;
        // let store = Store::new(&engine);
        // let instance = Instance::new(&store, &module, &[])?;
        // let func = instance.get_func(&store, function)?;
        // let result = func.call(&store, args)?;
        
        tracing::info!("Executing WASM function: {}::{}({:?})", module_name, function, args);
        
        // Simulated response
        Ok(serde_json::json!({
            "result": "success",
            "module": module_name,
            "function": function,
            "args_count": args.len(),
        }))
    }
    
    /// Call handle_request function (for HTTP handlers)
    pub fn handle_request(&self, module_name: &str, request: &crate::core::http::Request) -> crate::NoorResult<crate::core::http::Response> {
        let request_json = serde_json::json!({
            "method": request.method.as_str(),
            "path": request.path,
            "headers": request.headers,
            "body": String::from_utf8_lossy(&request.body).to_string(),
        });
        
        let result = self.execute(module_name, "handle_request", &[request_json])?;
        
        // Parse result into response
        let status = result.get("status")
            .and_then(|s| s.as_u64())
            .unwrap_or(200) as u16;
        
        let body = result.get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("");
        
        Ok(crate::core::http::Response::new(crate::core::http::StatusCode(status))
            .text(body))
    }
    
    /// Reload a module (useful for development)
    pub fn reload(&self, name: &str) -> crate::NoorResult<WasmModule> {
        let module = self.get(name)
            .ok_or_else(|| crate::NoorError::Internal(format!("Module '{}' not loaded", name)))?;
        
        self.unload(name);
        self.load(name, &module.path)
    }
    
    /// Get total size of all loaded modules
    pub fn total_size(&self) -> u64 {
        self.modules.read().values().map(|m| m.size).sum()
    }
    
    /// Clear all modules
    pub fn clear(&self) {
        self.modules.write().clear();
    }
}

/// WASM plugin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPlugin {
    pub name: String,
    pub description: String,
    pub module: String,
    pub entry_point: String,
    pub config: HashMap<String, serde_json::Value>,
}

impl WasmPlugin {
    pub fn new(name: &str, module: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            module: module.to_string(),
            entry_point: "handle_request".to_string(),
            config: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_manager_creation() {
        let manager = WasmManager::new();
        
        assert_eq!(manager.count(), 0);
        assert!(manager.is_enabled());
    }
    
    #[test]
    fn test_wasm_enable_disable() {
        let manager = WasmManager::new();
        
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        
        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }
    
    #[test]
    fn test_load_invalid_wasm() {
        let manager = WasmManager::new();
        
        // Create a non-WASM file
        let path = "/tmp/noor_wasm_test.txt";
        std::fs::write(path, "not a wasm file").unwrap();
        
        let result = manager.load("test", path);
        assert!(result.is_err());
        
        std::fs::remove_file(path).ok();
    }
    
    #[test]
    fn test_load_valid_wasm_header() {
        let manager = WasmManager::new();
        
        // Create a file with WASM magic number
        let path = "/tmp/noor_wasm_test.wasm";
        let mut content = b"\x00asm".to_vec();
        content.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version
        std::fs::write(path, &content).unwrap();
        
        let result = manager.load("test", path);
        assert!(result.is_ok());
        
        let module = result.unwrap();
        assert_eq!(module.name, "test");
        assert_eq!(module.size, 8);
        
        std::fs::remove_file(path).ok();
    }
    
    #[test]
    fn test_unload_module() {
        let manager = WasmManager::new();
        
        let path = "/tmp/noor_wasm_test2.wasm";
        let mut content = b"\x00asm".to_vec();
        content.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        std::fs::write(path, &content).unwrap();
        
        manager.load("test", path).unwrap();
        assert_eq!(manager.count(), 1);
        
        assert!(manager.unload("test"));
        assert_eq!(manager.count(), 0);
        
        std::fs::remove_file(path).ok();
    }
    
    #[test]
    fn test_execute_disabled() {
        let manager = WasmManager::new();
        manager.set_enabled(false);
        
        let result = manager.execute("test", "func", &[]);
        assert!(result.is_err());
    }
}
