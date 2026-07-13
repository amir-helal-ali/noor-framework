// ============================================================
// مثال: DI Container | Example: Dependency Injection
// ============================================================

use noor::*;
use noor::core::container::Container;
use std::sync::Arc;

struct Database {
    url: String,
}

struct UserRepository {
    db: Arc<Database>,
}

struct UserService {
    repo: Arc<UserRepository>,
}

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n💉 Dependency Injection Demo\n");
    
    let container = Container::new();
    
    // Register services
    container.singleton(|_| Database { 
        url: "sqlite://app.db".to_string() 
    });
    
    container.singleton(|c| UserRepository {
        db: c.expect::<Database>(),
    });
    
    container.singleton(|c| UserService {
        repo: c.expect::<UserRepository>(),
    });
    
    // Resolve
    let service = container.expect::<UserService>();
    
    println!("✓ Database URL: {}", service.repo.db.url);
    println!("✓ Services registered: {}", container.count());
    
    // Test singleton - same instance
    let db1 = container.resolve::<Database>().unwrap();
    let db2 = container.resolve::<Database>().unwrap();
    println!("✓ Singleton check (same instance): {}", Arc::ptr_eq(&db1, &db2));
    
    println!("\n✅ DI demo completed!");
    Ok(())
}
