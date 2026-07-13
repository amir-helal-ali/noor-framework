// ============================================================
// مثال: HTTP Client | Example: HTTP Client
// ============================================================

use noor::*;
use noor::core::http_client::HttpClient;
use std::time::Duration;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🌐 HTTP Client Demo\n");
    
    // Create a client with defaults
    let client = HttpClient::new()
        .timeout(Duration::from_secs(30))
        .header("User-Agent", "Noor/1.5")
        .bearer_token("my_api_token");
    
    println!("✓ Created HTTP client with auth");
    
    // Build a GET request
    let request = client
        .get("https://api.example.com/users")
        .query("page", "1")
        .query("limit", "10")
        .header("Accept", "application/json")
        .build();
    
    println!("✓ Built GET request: {} {}", request.method.as_str(), request.url);
    println!("  Query params: {:?}", request.query_params);
    println!("  Headers: {} total", request.headers.len());
    
    // Build a POST request with JSON body
    let post_request = HttpClient::new()
        .post("https://api.example.com/users")
        .json(&serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com"
        }))?
        .build();
    
    println!("\n✓ Built POST request with JSON body");
    println!("  Content-Type: {}", post_request.headers.get("Content-Type").unwrap());
    
    // Build a request with Basic Auth
    let auth_request = HttpClient::new()
        .basic_auth("username", "password")
        .get("https://api.example.com/protected")
        .build();
    
    println!("\n✓ Built request with Basic Auth");
    println!("  Authorization: {}", auth_request.headers.get("Authorization").unwrap());
    
    // Convenience functions
    println!("\n📋 Convenience functions available:");
    println!("  get(url), post(url), put(url), delete(url)");
    
    println!("\n✅ HTTP Client demo completed!");
    Ok(())
}
