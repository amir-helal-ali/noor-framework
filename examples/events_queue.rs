// ============================================================
// Example: Events and Queue
// مثال: الأحداث والطوابير
// ============================================================

use noor::*;
use noor::core::events::EventEmitter;
use noor::core::queue::{Queue, Job, Priority};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n📢 Events & Queue Demo\n");
    
    // 1. Event Emitter
    let emitter = Arc::new(EventEmitter::new());
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Register event handlers
    let counter_clone = counter.clone();
    emitter.on("user.registered", Arc::new(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        println!("  📧 Sending welcome email to new user");
        Ok(())
    }));
    
    let counter_clone2 = counter.clone();
    emitter.on("user.registered", Arc::new(move |_event| {
        counter_clone2.fetch_add(1, Ordering::SeqCst);
        println!("  📊 Updating analytics");
        Ok(())
    }));
    
    println!("Firing 'user.registered' event...");
    emitter.fire("user.registered", serde_json::json!({"user_id": 123}))?;
    
    println!("  Handlers called: {}", counter.load(Ordering::SeqCst));
    
    // 2. Queue with priorities
    let queue = Queue::new();
    
    let email_counter = Arc::new(AtomicUsize::new(0));
    let email_counter_clone = email_counter.clone();
    queue.register("send_email", Arc::new(move |job| {
        email_counter_clone.fetch_add(1, Ordering::SeqCst);
        println!("  📤 Processing email job: {}", job.payload);
        Ok(())
    }));
    
    println!("\nDispatching jobs...");
    
    // Push jobs with different priorities
    queue.push(Job::new("send_email", serde_json::json!({"to": "low@example.com"}))
        .with_priority(Priority::Low))?;
    
    queue.push(Job::new("send_email", serde_json::json!({"to": "critical@example.com"}))
        .with_priority(Priority::Critical))?;
    
    queue.push(Job::new("send_email", serde_json::json!({"to": "normal@example.com"}))
        .with_priority(Priority::Normal))?;
    
    queue.push(Job::new("send_email", serde_json::json!({"to": "high@example.com"}))
        .with_priority(Priority::High))?;
    
    println!("  Pending jobs: {}", queue.pending_count());
    
    // Process all jobs
    println!("\nProcessing jobs...");
    while queue.process_next()? {
        // Continue processing
    }
    
    println!("\n  Jobs processed: {}", email_counter.load(Ordering::SeqCst));
    
    println!("\n✅ Events & Queue demo completed!");
    Ok(())
}
