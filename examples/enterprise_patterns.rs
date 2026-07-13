// ============================================================
// مثال: الأنماط المؤسسية | Example: Enterprise Patterns
// ============================================================

use noor::*;
use noor::core::{
    cqrs::{CqrsSystem, Command, CommandHandler, Query, QueryHandler},
    distributed_lock::{MemoryLockManager, DistributedLock},
    service_discovery::{ServiceRegistry, ServiceInstance, LoadBalancer, LoadBalancerStrategy},
    idempotency::IdempotencyManager,
    saga::{Saga, SagaStep, SagaOrchestrator, SagaResult},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🏢 Enterprise Patterns Demo\n");
    
    // 1. CQRS
    println!("1️⃣  CQRS Pattern:");
    
    #[derive(Clone)]
    struct CreateUser { name: String }
    impl Command for CreateUser {
        type Result = i64;
        fn command_type() -> &'static str { "CreateUser" }
    }
    
    struct CreateUserHandler;
    impl CommandHandler<CreateUser> for CreateUserHandler {
        fn handle(&self, _cmd: CreateUser) -> NoorResult<i64> { Ok(1) }
    }
    
    #[derive(Clone)]
    struct GetUser { id: i64 }
    impl Query for GetUser {
        type Result = String;
        fn query_type() -> &'static str { "GetUser" }
    }
    
    struct GetUserHandler;
    impl QueryHandler<GetUser> for GetUserHandler {
        fn handle(&self, q: GetUser) -> NoorResult<String> {
            Ok(format!("User {}", q.id))
        }
    }
    
    let cqrs = CqrsSystem::new();
    cqrs.command_bus.register(CreateUserHandler);
    cqrs.query_bus.register(GetUserHandler);
    
    let id = cqrs.execute(CreateUser { name: "John".into() })?;
    let user = cqrs.query(GetUser { id })?;
    println!("   ✓ Command: Created user with ID {}", id);
    println!("   ✓ Query: Found '{}'", user);
    
    // 2. Distributed Lock
    println!("\n2️⃣  Distributed Lock:");
    
    let lock_manager = MemoryLockManager::new();
    
    let result = DistributedLock::with_lock(&lock_manager, "resource", "worker1", 60, || {
        println!("   ✓ Lock acquired, doing work...");
        Ok(42)
    })?;
    
    println!("   ✓ Work completed: {}", result);
    
    // 3. Service Discovery
    println!("\n3️⃣  Service Discovery:");
    
    let registry = Arc::new(ServiceRegistry::default());
    
    registry.register(ServiceInstance::new("api-service", "10.0.0.1", 8080));
    registry.register(ServiceInstance::new("api-service", "10.0.0.2", 8080));
    registry.register(ServiceInstance::new("api-service", "10.0.0.3", 8080));
    
    let lb = LoadBalancer::new(registry.clone(), LoadBalancerStrategy::RoundRobin);
    
    for i in 0..3 {
        let instance = lb.next("api-service").unwrap();
        println!("   ✓ Request {} -> {}", i + 1, instance.url());
    }
    
    let stats = registry.stats();
    println!("   ✓ Total services: {}, instances: {}", stats.total_services, stats.total_instances);
    
    // 4. Idempotency
    println!("\n4️⃣  Idempotency:");
    
    let idempotency = IdempotencyManager::default();
    let counter = Arc::new(AtomicUsize::new(0));
    
    let counter_clone = counter.clone();
    let result1: i32 = idempotency.execute("payment-123", || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(100)
    })?;
    
    let counter_clone = counter.clone();
    let result2: i32 = idempotency.execute("payment-123", || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(200) // Should not be called
    })?;
    
    println!("   ✓ First call: {}", result1);
    println!("   ✓ Second call (cached): {}", result2);
    println!("   ✓ Function called: {} time(s)", counter.load(Ordering::SeqCst));
    
    // 5. Saga Pattern
    println!("\n5️⃣  Saga Pattern:");
    
    let saga = Saga::new("order_processing")
        .step(SagaStep::new(
            "create_order",
            || { println!("   ✓ Step 1: Order created"); Ok(()) },
            || { println!("   ↩️  Compensate: Order cancelled"); Ok(()) },
        ))
        .step(SagaStep::new(
            "charge_payment",
            || { println!("   ✓ Step 2: Payment charged"); Ok(()) },
            || { println!("   ↩️  Compensate: Payment refunded"); Ok(()) },
        ))
        .step(SagaStep::new(
            "ship_order",
            || { println!("   ✓ Step 3: Order shipped"); Ok(()) },
            || { println!("   ↩️  Compensate: Shipment cancelled"); Ok(()) },
        ));
    
    let orchestrator = SagaOrchestrator::new();
    let result = orchestrator.execute(saga);
    
    match result {
        SagaResult::Success(status) => {
            println!("   ✓ Saga completed: {} steps", status.completed_steps.len());
        }
        SagaResult::Failed(status) => {
            println!("   ✗ Saga failed at: {:?}", status.failed_step);
        }
    }
    
    // Test failing saga
    println!("\n   Testing failing saga...");
    
    let failing_saga = Saga::new("failing_process")
        .step(SagaStep::new(
            "step1",
            || { println!("   ✓ Step 1: OK"); Ok(()) },
            || { println!("   ↩️  Compensate step1"); Ok(()) },
        ))
        .step(SagaStep::new(
            "step2",
            || { println!("   ✓ Step 2: OK"); Ok(()) },
            || { println!("   ↩️  Compensate step2"); Ok(()) },
        ))
        .step(SagaStep::new(
            "step3_fails",
            || { 
                println!("   ✗ Step 3: FAILED");
                Err(NoorError::Internal("Step 3 failed".to_string()))
            },
            || { println!("   ↩️  Compensate step3"); Ok(()) },
        ));
    
    let result = orchestrator.execute(failing_saga);
    
    match result {
        SagaResult::Failed(status) => {
            println!("   ✓ Saga failed and compensated {} steps", status.completed_steps.len());
        }
        _ => panic!("Expected failure"),
    }
    
    println!("\n✅ Enterprise Patterns demo completed!");
    Ok(())
}
