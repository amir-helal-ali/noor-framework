// ============================================================
// مثال: تعدد المستأجرين | Example: Multi-tenancy
// ============================================================

use noor::*;
use noor::core::tenancy::{TenantManager, Tenant, TenantPlan, TenantStatus, ResolutionStrategy};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🏢 Multi-tenancy Demo\n");
    
    let manager = TenantManager::new(ResolutionStrategy::Subdomain);
    
    // Create tenants
    let acme = Tenant::new("Acme Corp")
        .with_subdomain("acme")
        .with_domain("acme.example.com")
        .with_plan(TenantPlan::Pro);
    manager.register(acme);
    
    let tech = Tenant::new("Tech Inc")
        .with_subdomain("tech")
        .with_plan(TenantPlan::Enterprise);
    manager.register(tech);
    
    let free = Tenant::new("Free User");
    manager.register(free);
    
    println!("📋 Registered {} tenants:\n", manager.count());
    
    for tenant in manager.list() {
        let plan = match tenant.plan {
            TenantPlan::Free => "Free",
            TenantPlan::Starter => "Starter",
            TenantPlan::Pro => "Pro",
            TenantPlan::Enterprise => "Enterprise",
            TenantPlan::Custom(_) => "Custom",
        };
        let subdomain = tenant.subdomain.unwrap_or("-".to_string());
        println!("  {} ({}) - Plan: {}, Subdomain: {}", 
            tenant.name, tenant.id, plan, subdomain);
    }
    
    // Activate tenants
    println!("\n✅ Activating tenants...");
    for tenant in manager.list() {
        manager.update(&tenant.id, |t| {
            t.status = TenantStatus::Active;
        });
    }
    
    // Check feature access
    println!("\n🔍 Feature access check:");
    let tenants = manager.list();
    
    for tenant in &tenants {
        println!("\n  {}:", tenant.name);
        println!("    basic_features: {}", manager.has_feature(&tenant.id, "basic_features"));
        println!("    api_access: {}", manager.has_feature(&tenant.id, "api_access"));
        println!("    white_label: {}", manager.has_feature(&tenant.id, "white_label"));
        println!("    custom_domain: {}", manager.has_feature(&tenant.id, "custom_domain"));
    }
    
    // Resolve tenant from request
    println!("\n🌐 Resolving tenant from request:");
    
    let mut request = noor::core::http::Request::new(
        noor::core::http::Method::Get,
        "/api/data".to_string(),
    );
    request.headers.insert("host".to_string(), "acme.example.com".to_string());
    
    // Note: For subdomain strategy, we'd extract from host
    // For demo, let's use header strategy
    
    let header_manager = TenantManager::new(ResolutionStrategy::Header);
    for tenant in manager.list() {
        let t = Tenant::new(&tenant.name);
        header_manager.register(Tenant {
            id: tenant.id.clone(),
            name: tenant.name.clone(),
            ..Tenant::new(&tenant.name)
        });
    }
    
    println!("\n✅ Multi-tenancy demo completed!");
    Ok(())
}
