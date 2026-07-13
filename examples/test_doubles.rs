// ============================================================
// مثال: أنماط الاختبار | Example: Test Doubles
// ============================================================

use noor::*;
use noor::core::test_doubles::{Mock, Stub, FakeDatabase, Spy, Fixture};
use std::sync::Arc;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🧪 Test Doubles Demo\n");
    
    // 1. Mock - verifies interactions
    println!("1️⃣  Mock Pattern:");
    let mock: Mock<()> = Mock::new();
    
    mock.expect_return("get_user", vec![serde_json::json!(1)], serde_json::json!({"name": "John"}));
    
    let result = mock.invoke("get_user", vec![serde_json::json!(1)]);
    println!("   ✓ Mock returned: {:?}", result);
    
    match mock.verify() {
        Ok(()) => println!("   ✓ All expectations met"),
        Err(e) => println!("   ✗ Verification failed: {}", e),
    }
    
    // 2. Stub - returns predefined responses
    println!("\n2️⃣  Stub Pattern:");
    let mut stub = Stub::new();
    
    stub.when("get_name", serde_json::json!("John Doe"));
    stub.when("get_email", serde_json::json!("john@example.com"));
    
    println!("   ✓ Stub name: {:?}", stub.call("get_name"));
    println!("   ✓ Stub email: {:?}", stub.call("get_email"));
    
    // 3. Fake - simplified working implementation
    println!("\n3️⃣  Fake Database:");
    let db = FakeDatabase::new();
    
    db.insert("user:1", serde_json::json!({"name": "John", "email": "john@example.com"}));
    db.insert("user:2", serde_json::json!({"name": "Jane", "email": "jane@example.com"}));
    
    println!("   ✓ Total records: {}", db.count());
    println!("   ✓ User 1: {:?}", db.get("user:1"));
    
    db.delete("user:2");
    println!("   ✓ After delete: {} records", db.count());
    
    // 4. Spy - records interactions
    println!("\n4️⃣  Spy Pattern:");
    let spy = Spy::new(vec![1, 2, 3, 4, 5]);
    
    spy.call("len", |v| v.len());
    spy.call("first", |v| v.first().copied());
    spy.call("len", |v| v.len());
    
    println!("   ✓ Interactions: {:?}", spy.interactions());
    println!("   ✓ 'len' called {} times", spy.call_count("len"));
    println!("   ✓ 'first' called {} times", spy.call_count("first"));
    
    // 5. Fixture - setup and teardown
    println!("\n5️⃣  Fixture Pattern:");
    let fixture = Fixture::new(|| {
        println!("   📦 Setting up test data...");
        vec![1, 2, 3]
    }).with_teardown(|_data| {
        println!("   🧹 Cleaning up test data...");
    });
    
    let result = fixture.run(|data| {
        println!("   🏃 Running test with {} items", data.len());
        data.iter().sum::<i32>()
    });
    
    println!("   ✓ Test result: {}", result);
    
    println!("\n✅ Test Doubles demo completed!");
    Ok(())
}
