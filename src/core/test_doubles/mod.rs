// ============================================================
// Test Doubles - بدائل الاختبار
// ============================================================
// Mock, Stub, Fake, and Spy implementations for testing.
// Stub: returns predefined responses
// Mock: verifies interactions
// Fake: simplified working implementation
// Spy: records interactions
//
// بدائل الاختبار: Mock, Stub, Fake, Spy
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Mock - records all interactions and verifies expectations
pub struct Mock<T> {
    expectations: Arc<RwLock<Vec<Expectation>>>,
    invocations: Arc<RwLock<Vec<Invocation>>>,
    _phantom: std::marker::PhantomData<T>,
}

/// Expected call
#[derive(Debug, Clone)]
struct Expectation {
    method: String,
    args: Vec<serde_json::Value>,
    return_value: Option<serde_json::Value>,
    times: Option<usize>,
}

/// Actual invocation
#[derive(Debug, Clone)]
struct Invocation {
    method: String,
    args: Vec<serde_json::Value>,
    timestamp: i64,
}

impl<T> Mock<T> {
    pub fn new() -> Self {
        Self {
            expectations: Arc::new(RwLock::new(Vec::new())),
            invocations: Arc::new(RwLock::new(Vec::new())),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Expect a method call with specific arguments
    pub fn expect(&self, method: &str, args: Vec<serde_json::Value>) -> &Self {
        self.expectations.write().push(Expectation {
            method: method.to_string(),
            args,
            return_value: None,
            times: None,
        });
        self
    }
    
    /// Expect a method call and return a value
    pub fn expect_return(&self, method: &str, args: Vec<serde_json::Value>, return_value: serde_json::Value) -> &Self {
        self.expectations.write().push(Expectation {
            method: method.to_string(),
            args,
            return_value: Some(return_value),
            times: None,
        });
        self
    }
    
    /// Expect a method to be called exactly N times
    pub fn expect_times(&self, method: &str, args: Vec<serde_json::Value>, times: usize) -> &Self {
        self.expectations.write().push(Expectation {
            method: method.to_string(),
            args,
            return_value: None,
            times: Some(times),
        });
        self
    }
    
    /// Record an invocation
    pub fn invoke(&self, method: &str, args: Vec<serde_json::Value>) -> Option<serde_json::Value> {
        self.invocations.write().push(Invocation {
            method: method.to_string(),
            args: args.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        });
        
        // Find matching expectation
        let expectations = self.expectations.read();
        for exp in expectations.iter() {
            if exp.method == method && args_match(&exp.args, &args) {
                return exp.return_value.clone();
            }
        }
        
        None
    }
    
    /// Verify all expectations were met
    pub fn verify(&self) -> Result<(), String> {
        let invocations = self.invocations.read();
        let expectations = self.expectations.read();
        
        for exp in expectations.iter() {
            let matching: Vec<&Invocation> = invocations
                .iter()
                .filter(|inv| inv.method == exp.method && args_match(&exp.args, &inv.args))
                .collect();
            
            if let Some(times) = exp.times {
                if matching.len() != times {
                    return Err(format!(
                        "Expected {} to be called {} times, but was called {} times",
                        exp.method, times, matching.len()
                    ));
                }
            } else if matching.is_empty() {
                return Err(format!(
                    "Expected {} to be called with {:?}, but was never called",
                    exp.method, exp.args
                ));
            }
        }
        
        Ok(())
    }
    
    /// Get all invocations
    pub fn invocations(&self) -> Vec<Invocation> {
        self.invocations.read().clone()
    }
    
    /// Get invocation count for a method
    pub fn invocation_count(&self, method: &str) -> usize {
        self.invocations.read().iter().filter(|i| i.method == method).count()
    }
    
    /// Check if a method was called
    pub fn was_called(&self, method: &str) -> bool {
        self.invocation_count(method) > 0
    }
    
    /// Reset all expectations and invocations
    pub fn reset(&self) {
        self.expectations.write().clear();
        self.invocations.write().clear();
    }
}

impl<T> Default for Mock<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if args match
fn args_match(expected: &[serde_json::Value], actual: &[serde_json::Value]) -> bool {
    if expected.is_empty() {
        return true; // Empty expected matches any args
    }
    expected == actual
}

/// Stub - returns predefined responses
pub struct Stub {
    responses: HashMap<String, serde_json::Value>,
}

impl Default for Stub {
    fn default() -> Self {
        Self::new()
    }
}

impl Stub {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }
    
    /// Set a response for a method
    pub fn when(&mut self, method: &str, response: serde_json::Value) -> &mut Self {
        self.responses.insert(method.to_string(), response);
        self
    }
    
    /// Get the response for a method
    pub fn call(&self, method: &str) -> Option<serde_json::Value> {
        self.responses.get(method).cloned()
    }
}

/// Fake - simplified working implementation
pub struct FakeDatabase {
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl Default for FakeDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeDatabase {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn insert(&self, key: &str, value: serde_json::Value) {
        self.data.write().insert(key.to_string(), value);
    }
    
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().get(key).cloned()
    }
    
    pub fn delete(&self, key: &str) -> bool {
        self.data.write().remove(key).is_some()
    }
    
    pub fn count(&self) -> usize {
        self.data.read().len()
    }
    
    pub fn clear(&self) {
        self.data.write().clear();
    }
}

/// Spy - wraps an object and records all interactions
pub struct Spy<T> {
    inner: T,
    interactions: Arc<RwLock<Vec<String>>>,
}

impl<T> Spy<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            interactions: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Record an interaction and access the inner object
    pub fn call<F, R>(&self, method: &str, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.interactions.write().push(method.to_string());
        f(&self.inner)
    }
    
    /// Get all interactions
    pub fn interactions(&self) -> Vec<String> {
        self.interactions.read().clone()
    }
    
    /// Check if a method was called
    pub fn was_called(&self, method: &str) -> bool {
        self.interactions.read().contains(&method.to_string())
    }
    
    /// Get the number of times a method was called
    pub fn call_count(&self, method: &str) -> usize {
        self.interactions.read().iter().filter(|m| *m == method).count()
    }
    
    /// Get a reference to the inner object
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

/// Test fixture - setup and teardown for tests
pub struct Fixture<T> {
    setup: Box<dyn Fn() -> T + Send + Sync>,
    teardown: Option<Box<dyn Fn(&T) + Send + Sync>>,
}

impl<T> Fixture<T> {
    pub fn new<F>(setup: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            setup: Box::new(setup),
            teardown: None,
        }
    }
    
    pub fn with_teardown<F>(mut self, teardown: F) -> Self
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.teardown = Some(Box::new(teardown));
        self
    }
    
    /// Run a test with the fixture
    pub fn run<F, R>(&self, test: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let data = (self.setup)();
        let result = test(&data);
        if let Some(ref teardown) = self.teardown {
            teardown(&data);
        }
        result
    }
}

/// Assertion helpers
pub mod assert {
    /// Assert that a condition is true
    pub fn is_true(condition: bool, message: &str) {
        assert!(condition, "Assertion failed: {}", message);
    }
    
    /// Assert that two values are equal
    pub fn equals<T: PartialEq + std::fmt::Debug>(actual: T, expected: T) {
        assert_eq!(actual, expected);
    }
    
    /// Assert that a value is Some
    pub fn is_some<T: std::fmt::Debug>(value: Option<T>) {
        assert!(value.is_some(), "Expected Some but got None");
    }
    
    /// Assert that a value is None
    pub fn is_none<T: std::fmt::Debug>(value: Option<T>) {
        assert!(value.is_none(), "Expected None but got Some({:?})", value);
    }
    
    /// Assert that a collection is empty
    pub fn is_empty<T>(collection: &[T]) {
        assert!(collection.is_empty(), "Expected empty collection but got {} items", collection.len());
    }
    
    /// Assert that a collection is not empty
    pub fn is_not_empty<T>(collection: &[T]) {
        assert!(!collection.is_empty(), "Expected non-empty collection");
    }
    
    /// Assert that a collection has a specific length
    pub fn has_length<T>(collection: &[T], expected: usize) {
        assert_eq!(collection.len(), expected, "Expected length {} but got {}", expected, collection.len());
    }
    
    /// Assert that a string contains a substring
    pub fn contains(haystack: &str, needle: &str) {
        assert!(haystack.contains(needle), "Expected '{}' to contain '{}'", haystack, needle);
    }
    
    /// Assert that a function panics
    pub fn panics<F>(f: F)
    where
        F: FnOnce() + std::panic::UnwindSafe,
    {
        let result = std::panic::catch_unwind(f);
        assert!(result.is_err(), "Expected function to panic but it didn't");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock() {
        let mock: Mock<()> = Mock::new();
        
        mock.expect_return("get_user", vec![serde_json::json!(1)], serde_json::json!({"name": "John"}));
        
        let result = mock.invoke("get_user", vec![serde_json::json!(1)]);
        
        assert_eq!(result, Some(serde_json::json!({"name": "John"})));
        
        mock.verify().unwrap();
    }
    
    #[test]
    fn test_mock_verification_fails() {
        let mock: Mock<()> = Mock::new();
        
        mock.expect("get_user", vec![]);
        
        // Don't call the method
        assert!(mock.verify().is_err());
    }
    
    #[test]
    fn test_stub() {
        let mut stub = Stub::new();
        
        stub.when("get_name", serde_json::json!("John"));
        
        let result = stub.call("get_name");
        assert_eq!(result, Some(serde_json::json!("John")));
    }
    
    #[test]
    fn test_fake_database() {
        let db = FakeDatabase::new();
        
        db.insert("user:1", serde_json::json!({"name": "John"}));
        
        assert_eq!(db.count(), 1);
        
        let user = db.get("user:1").unwrap();
        assert_eq!(user["name"], "John");
        
        assert!(db.delete("user:1"));
        assert_eq!(db.count(), 0);
    }
    
    #[test]
    fn test_spy() {
        let spy = Spy::new(vec![1, 2, 3]);
        
        spy.call("len", |v| v.len());
        spy.call("first", |v| v.first().copied());
        spy.call("len", |v| v.len());
        
        assert!(spy.was_called("len"));
        assert!(spy.was_called("first"));
        assert_eq!(spy.call_count("len"), 2);
        assert_eq!(spy.call_count("first"), 1);
    }
    
    #[test]
    fn test_fixture() {
        let fixture = Fixture::new(|| vec![1, 2, 3]);
        
        let result = fixture.run(|data| {
            assert_eq!(data.len(), 3);
            data.iter().sum::<i32>()
        });
        
        assert_eq!(result, 6);
    }
}
