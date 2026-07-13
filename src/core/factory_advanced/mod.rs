// ============================================================
// Advanced Model Factory - مصنع النماذج المتقدم
// ============================================================
// Generate fake data for testing and seeding.
// Supports states, sequences, and relationships.
//
// توليد بيانات وهمية للاختبار والتعبئة.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Factory state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FactoryState {
    Draft,
    Published,
    Archived,
    Pending,
    Active,
    Inactive,
}

/// Factory definition
pub struct FactoryDefinition<T> {
    pub model: String,
    pub generator: Arc<dyn Fn() -> T + Send + Sync>,
    pub states: HashMap<String, Arc<dyn Fn(&mut T) + Send + Sync>>,
    pub after_creating: Vec<Arc<dyn Fn(&mut T) + Send + Sync>>,
    pub count: Arc<RwLock<usize>>,
}

impl<T: Clone + Send + Sync + 'static> FactoryDefinition<T> {
    pub fn new<F>(model: &str, generator: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            model: model.to_string(),
            generator: Arc::new(generator),
            states: HashMap::new(),
            after_creating: Vec::new(),
            count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Register a state modifier
    pub fn state<F>(mut self, name: &str, modifier: F) -> Self
    where
        F: Fn(&mut T) + Send + Sync + 'static,
    {
        self.states.insert(name.to_string(), Arc::new(modifier));
        self
    }
    
    /// Add an after-creating hook
    pub fn after_creating<F>(mut self, hook: F) -> Self
    where
        F: Fn(&mut T) + Send + Sync + 'static,
    {
        self.after_creating.push(Arc::new(hook));
        self
    }
    
    /// Create a single instance
    pub fn create(&self) -> T {
        let mut instance = (self.generator)();
        
        // Apply after-creating hooks
        for hook in &self.after_creating {
            hook(&mut instance);
        }
        
        // Increment counter
        *self.count.write() += 1;
        
        instance
    }
    
    /// Create an instance with a specific state
    pub fn create_with_state(&self, state: &str) -> Option<T> {
        let mut instance = (self.generator)();
        
        // Apply state
        if let Some(modifier) = self.states.get(state) {
            modifier(&mut instance);
        } else {
            return None;
        }
        
        // Apply after-creating hooks
        for hook in &self.after_creating {
            hook(&mut instance);
        }
        
        *self.count.write() += 1;
        
        Some(instance)
    }
    
    /// Create multiple instances
    pub fn create_many(&self, count: usize) -> Vec<T> {
        (0..count).map(|_| self.create()).collect()
    }
    
    /// Create multiple instances with a state
    pub fn create_many_with_state(&self, count: usize, state: &str) -> Vec<T> {
        (0..count)
            .filter_map(|_| self.create_with_state(state))
            .collect()
    }
    
    /// Create instances with different states
    pub fn create_with_states(&self, states: &[&str]) -> Vec<T> {
        states
            .iter()
            .filter_map(|state| self.create_with_state(state))
            .collect()
    }
    
    /// Get the number of instances created
    pub fn created_count(&self) -> usize {
        *self.count.read()
    }
    
    /// Reset the counter
    pub fn reset_count(&self) {
        *self.count.write() = 0;
    }
}

/// Factory registry
pub struct FactoryRegistry {
    factories: Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
}

impl Default for FactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FactoryRegistry {
    pub fn new() -> Self {
        Self {
            factories: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a factory
    pub fn register<T: Clone + Send + Sync + 'static>(&self, name: &str, factory: FactoryDefinition<T>) {
        self.factories.write().insert(name.to_string(), Box::new(factory));
    }
    
    /// Create an instance using a registered factory
    pub fn create<T: Clone + Send + Sync + 'static>(&self, name: &str) -> Option<T> {
        let factories = self.factories.read();
        
        let factory = factories.get(name)?
            .downcast_ref::<FactoryDefinition<T>>()?;
        
        Some(factory.create())
    }
    
    /// Create multiple instances
    pub fn create_many<T: Clone + Send + Sync + 'static>(&self, name: &str, count: usize) -> Vec<T> {
        let factories = self.factories.read();
        
        if let Some(factory) = factories.get(name).and_then(|f| f.downcast_ref::<FactoryDefinition<T>>()) {
            return factory.create_many(count);
        }
        
        Vec::new()
    }
}

/// Fake data generators
pub mod faker {
    use rand::Rng;
    
    /// Generate a random name
    pub fn name() -> String {
        let first_names = ["John", "Jane", "Bob", "Alice", "Charlie", "Diana", "Eve", "Frank", "Grace", "Hank"];
        let last_names = ["Smith", "Doe", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez"];
        
        let mut rng = rand::thread_rng();
        format!(
            "{} {}",
            first_names[rng.gen_range(0..first_names.len())],
            last_names[rng.gen_range(0..last_names.len())]
        )
    }
    
    /// Generate a random email
    pub fn email() -> String {
        let mut rng = rand::thread_rng();
        let name = name().to_lowercase().replace(' ', ".");
        let domains = ["example.com", "test.com", "demo.org", "sample.io", "fake.dev"];
        format!("{}@{}", name, domains[rng.gen_range(0..domains.len())])
    }
    
    /// Generate a random phone number
    pub fn phone() -> String {
        let mut rng = rand::thread_rng();
        format!(
            "+1 ({}) {}-{}",
            rng.gen_range(200..999),
            rng.gen_range(200..999),
            rng.gen_range(1000..9999)
        )
    }
    
    /// Generate a random address
    pub fn address() -> String {
        let mut rng = rand::thread_rng();
        let streets = ["Main St", "Oak Ave", "Maple Dr", "Cedar Ln", "Pine Rd", "Elm St"];
        let cities = ["New York", "Los Angeles", "Chicago", "Houston", "Phoenix", "Philadelphia"];
        
        format!(
            "{} {} St, {}, {} {}",
            rng.gen_range(100..9999),
            streets[rng.gen_range(0..streets.len())],
            cities[rng.gen_range(0..cities.len())],
            rng.gen_range(10000..99999),
            "US"
        )
    }
    
    /// Generate random text
    pub fn text(words: usize) -> String {
        let word_list = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do", "eiusmod", "tempor", "incididunt"];
        
        let mut rng = rand::thread_rng();
        (0..words)
            .map(|_| word_list[rng.gen_range(0..word_list.len())])
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Generate a random sentence
    pub fn sentence() -> String {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(5..15);
        let mut text = text(count);
        if let Some(c) = text.chars().next() {
            text = text.replace(c, &c.to_uppercase().to_string());
        }
        format!("{}.", text)
    }
    
    /// Generate random paragraphs
    pub fn paragraphs(count: usize) -> String {
        (0..count)
            .map(|_| sentence())
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Generate a random UUID
    pub fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Generate a random number
    pub fn number(min: i64, max: i64) -> i64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(min..max)
    }
    
    /// Generate a random float
    pub fn float(min: f64, max: f64) -> f64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(min..max)
    }
    
    /// Generate a random boolean
    pub fn boolean() -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.5)
    }
    
    /// Generate a random date (as timestamp)
    pub fn timestamp(days_ago_max: i64) -> i64 {
        let mut rng = rand::thread_rng();
        let now = chrono::Utc::now().timestamp();
        let days_ago = rng.gen_range(0..days_ago_max);
        now - (days_ago * 86400)
    }
    
    /// Pick a random element from a list
    pub fn random_element<T: Clone>(list: &[T]) -> T {
        let mut rng = rand::thread_rng();
        list[rng.gen_range(0..list.len())].clone()
    }
    
    /// Generate a random URL
    pub fn url() -> String {
        let mut rng = rand::thread_rng();
        let domains = ["example.com", "test.org", "demo.io", "sample.dev"];
        let paths = ["/page", "/article", "/post", "/view", "/api"];
        
        format!(
            "https://{}{}{}",
            domains[rng.gen_range(0..domains.len())],
            paths[rng.gen_range(0..paths.len())],
            rng.gen_range(1..1000)
        )
    }
    
    /// Generate a random IP address
    pub fn ip() -> String {
        let mut rng = rand::thread_rng();
        format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255)
        )
    }
    
    /// Generate a random user agent
    pub fn user_agent() -> String {
        let agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)",
            "Mozilla/5.0 (Android 13; Mobile; rv:109.0)",
        ];
        
        let mut rng = rand::thread_rng();
        agents[rng.gen_range(0..agents.len())].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct User {
        id: i64,
        name: String,
        email: String,
        active: bool,
    }
    
    #[test]
    fn test_factory_create() {
        let factory = FactoryDefinition::new("User", || User {
            id: faker::number(1, 1000),
            name: faker::name(),
            email: faker::email(),
            active: true,
        });
        
        let user = factory.create();
        
        assert!(!user.name.is_empty());
        assert!(!user.email.is_empty());
        assert!(user.active);
    }
    
    #[test]
    fn test_factory_states() {
        let factory = FactoryDefinition::new("User", || User {
            id: faker::number(1, 1000),
            name: faker::name(),
            email: faker::email(),
            active: true,
        })
        .state("inactive", |user| {
            user.active = false;
        })
        .state("admin", |user| {
            user.email = format!("admin@{}", user.email.split('@').nth(1).unwrap_or("example.com"));
        });
        
        let user = factory.create_with_state("inactive").unwrap();
        assert!(!user.active);
        
        let user = factory.create_with_state("admin").unwrap();
        assert!(user.email.starts_with("admin@"));
    }
    
    #[test]
    fn test_factory_many() {
        let factory = FactoryDefinition::new("User", || User {
            id: faker::number(1, 1000),
            name: faker::name(),
            email: faker::email(),
            active: true,
        });
        
        let users = factory.create_many(5);
        
        assert_eq!(users.len(), 5);
        assert_eq!(factory.created_count(), 5);
    }
    
    #[test]
    fn test_faker_name() {
        let name = faker::name();
        assert!(!name.is_empty());
        assert!(name.contains(' '));
    }
    
    #[test]
    fn test_faker_email() {
        let email = faker::email();
        assert!(email.contains('@'));
        assert!(email.contains('.'));
    }
    
    #[test]
    fn test_faker_text() {
        let text = faker::text(10);
        let words: Vec<&str> = text.split(' ').collect();
        assert_eq!(words.len(), 10);
    }
    
    #[test]
    fn test_faker_number() {
        let n = faker::number(1, 100);
        assert!(n >= 1 && n < 100);
    }
    
    #[test]
    fn test_faker_ip() {
        let ip = faker::ip();
        let parts: Vec<&str> = ip.split('.').collect();
        assert_eq!(parts.len(), 4);
    }
    
    #[test]
    fn test_factory_registry() {
        let registry = FactoryRegistry::new();
        
        let factory = FactoryDefinition::new("User", || User {
            id: faker::number(1, 1000),
            name: faker::name(),
            email: faker::email(),
            active: true,
        });
        
        registry.register("User", factory);
        
        let user: Option<User> = registry.create("User");
        assert!(user.is_some());
        
        let users: Vec<User> = registry.create_many("User", 3);
        assert_eq!(users.len(), 3);
    }
}
