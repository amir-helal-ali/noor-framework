// ============================================================
// مثال: Feature Flags | Example: Feature Flags
// ============================================================

use noor::*;
use noor::core::features::FeatureFlagManager;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🎯 Feature Flags Demo\n");
    
    let manager = FeatureFlagManager::new();
    
    // Register flags
    manager.boolean("new_ui", "New UI Redesign", false);
    manager.boolean("dark_mode", "Dark Mode", true);
    manager.percentage("beta_feature", "Beta Feature", 25, true);
    manager.for_users("admin_panel", "Admin Panel Access", vec![
        "admin1".to_string(),
        "admin2".to_string(),
    ]);
    
    println!("📋 Registered {} feature flags:\n", manager.count());
    
    let flags = manager.list();
    for flag in &flags {
        let status = if flag.enabled { "ON" } else { "OFF" };
        println!("  {} [{}] - {}", flag.name, status, flag.key);
    }
    
    // Check flags
    println!("\n🔍 Checking flags:");
    println!("  new_ui: {}", manager.is_enabled("new_ui"));
    println!("  dark_mode: {}", manager.is_enabled("dark_mode"));
    println!("  beta_feature: {}", manager.is_enabled("beta_feature"));
    
    // Check user-specific flags
    println!("\n👤 User-specific flags:");
    println!("  admin_panel for admin1: {}", manager.is_enabled_for("admin_panel", "admin1"));
    println!("  admin_panel for user1: {}", manager.is_enabled_for("admin_panel", "user1"));
    
    // Toggle a flag
    println!("\n🔄 Toggling 'new_ui' flag...");
    manager.toggle("new_ui");
    println!("  new_ui is now: {}", manager.is_enabled("new_ui"));
    
    // Using the macro
    println!("\n📝 Using the `feature!` macro:");
    if feature!(manager, "dark_mode") {
        println!("  ✓ Dark mode is enabled!");
    }
    
    if feature!(manager, "admin_panel", "admin1") {
        println!("  ✓ Admin1 has admin panel access!");
    }
    
    println!("\n✅ Feature Flags demo completed!");
    Ok(())
}
