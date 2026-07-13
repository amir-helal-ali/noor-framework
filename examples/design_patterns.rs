// ============================================================
// مثال: أنماط التصميم | Example: Design Patterns
// ============================================================

use noor::*;
use noor::core::{
    container::Container,
    repository::{Repository, InMemoryRepository, RepositoryFactory},
    state_machine::{StateMachine, StateMachineInstance},
    observer::{Observer, ObserverManager, ObserverRegistry},
    circuit_breaker::{CircuitBreaker, CircuitConfig, CircuitState},
    env::{EnvLoader, env, env_or},
    cookies::{Cookie, CookieJar, SignedCookieManager},
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: i64,
    name: String,
    email: String,
}

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🎨 Design Patterns Demo\n");
    
    // 1. Dependency Injection
    println!("1️⃣  Dependency Injection:");
    let container = Container::new();
    container.singleton(|_| "Database Connection".to_string());
    let db = container.resolve::<String>().unwrap();
    println!("   ✓ Resolved: {}", db);
    
    // 2. Repository Pattern
    println!("\n2️⃣  Repository Pattern:");
    let repo = RepositoryFactory::in_memory::<User, _>(|u: &User| u.id);
    
    repo.create(&User { id: 1, name: "John".to_string(), email: "john@example.com".to_string() })?;
    repo.create(&User { id: 2, name: "Jane".to_string(), email: "jane@example.com".to_string() })?;
    
    println!("   ✓ Total users: {}", repo.count());
    let user = repo.find(1).unwrap();
    println!("   ✓ Found user: {} ({})", user.name, user.email);
    
    let (page, total) = repo.paginate(1, 10);
    println!("   ✓ Paginated: {} items of {}", page.len(), total);
    
    // 3. State Machine
    println!("\n3️⃣  State Machine (Order Workflow):");
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum OrderState { Pending, Paid, Shipped, Delivered }
    impl std::fmt::Display for OrderState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
    }
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum OrderEvent { Pay, Ship, Deliver }
    impl std::fmt::Display for OrderEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
    }
    
    let machine = StateMachine::new(OrderState::Pending)
        .transition(OrderState::Pending, OrderEvent::Pay, OrderState::Paid)
        .transition(OrderState::Paid, OrderEvent::Ship, OrderState::Shipped)
        .transition(OrderState::Shipped, OrderEvent::Deliver, OrderState::Delivered);
    
    let mut order = StateMachineInstance::new("order-001", OrderState::Pending);
    println!("   ✓ Initial state: {:?}", order.state());
    
    order.transition(&machine, OrderEvent::Pay).unwrap();
    println!("   ✓ After Pay: {:?}", order.state());
    
    order.transition(&machine, OrderEvent::Ship).unwrap();
    println!("   ✓ After Ship: {:?}", order.state());
    
    order.transition(&machine, OrderEvent::Deliver).unwrap();
    println!("   ✓ After Deliver: {:?}", order.state());
    
    println!("   ✓ History entries: {}", order.history().len());
    
    // 4. Observer Pattern
    println!("\n4️⃣  Observer Pattern:");
    
    struct UserObserver {
        counter: Arc<AtomicUsize>,
    }
    
    impl Observer for UserObserver {
        fn on_creating(&self, _model: &mut serde_json::Value) -> bool {
            self.counter.fetch_add(1, Ordering::SeqCst);
            println!("   📝 User is being created...");
            true
        }
        
        fn on_created(&self, _model: &serde_json::Value) {
            self.counter.fetch_add(1, Ordering::SeqCst);
            println!("   ✅ User was created");
        }
    }
    
    let registry = ObserverRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));
    registry.observe("User", Arc::new(UserObserver { counter: counter.clone() }));
    
    let user_manager = registry.for_model("User");
    let mut model = serde_json::json!({"name": "John"});
    user_manager.fire_creating(&mut model);
    user_manager.fire_created(&model);
    
    println!("   ✓ Observer called {} times", counter.load(Ordering::SeqCst));
    
    // 5. Circuit Breaker
    println!("\n5️⃣  Circuit Breaker:");
    
    let config = CircuitConfig {
        failure_threshold: 3,
        ..Default::default()
    };
    let breaker = CircuitBreaker::new("api_service", config);
    
    println!("   ✓ Initial state: {}", breaker.state());
    
    // Record failures
    breaker.record_failure();
    breaker.record_failure();
    println!("   ✓ After 2 failures: {} (failures: {})", breaker.state(), breaker.failure_count());
    
    breaker.record_failure();
    println!("   ✓ After 3 failures: {} (tripped: {})", breaker.state(), breaker.is_tripped());
    
    // Try to execute
    let result: Result<(), _> = breaker.execute(|| Err(NoorError::Internal("Service unavailable".to_string())));
    println!("   ✓ Execute when open: {:?}", result.is_err());
    
    // 6. Environment Loader
    println!("\n6️⃣  Environment Loader:");
    
    let loader = EnvLoader::new();
    loader.set("APP_NAME", "Noor Demo");
    loader.set("APP_DEBUG", "true");
    loader.set("APP_PORT", "8080");
    
    println!("   ✓ APP_NAME: {}", env_or("APP_NAME", "Default"));
    println!("   ✓ APP_DEBUG: {:?}", loader.get_bool("APP_DEBUG"));
    println!("   ✓ APP_PORT: {:?}", loader.get_int("APP_PORT"));
    
    // 7. Signed Cookies
    println!("\n7️⃣  Signed Cookies:");
    
    let cookie_manager = SignedCookieManager::new("secret_key");
    let cookie = cookie_manager.sign("session", "user123")?;
    
    println!("   ✓ Created signed cookie: {}={}", cookie.name, &cookie.value[..20]);
    
    let verified = cookie_manager.verify(&cookie)
        .ok_or_else(|| NoorError::Internal("Cookie verification failed".to_string()))?;
    println!("   ✓ Verified value: {}", verified);
    
    // Cookie Jar
    let mut jar = CookieJar::new();
    jar.add(Cookie::new("user", "john"));
    jar.add(Cookie::new("theme", "dark"));
    
    println!("   ✓ Cookie jar has {} cookies", jar.count());
    println!("   ✓ User cookie: {}", jar.get_value("user").unwrap());
    
    println!("\n✅ Design Patterns demo completed!");
    Ok(())
}
