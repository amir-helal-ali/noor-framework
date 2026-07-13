// ============================================================
// Example: Basic REST API
// مثال: REST API أساسي
// ============================================================
// Demonstrates how to build a complete REST API with Noor
// يوضح كيفية بناء REST API كامل مع نور
// ============================================================

use noor::*;
use noor::{Application, Config, Router};
use noor::core::http::{Request, Response, StatusCode};
use noor::core::pagination::PaginationParams;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

fn main() -> NoorResult<()> {
    println!("{}", banner());
    
    let config = Config::default();
    let mut router = Router::new();
    
    // GET /api/users - List users with pagination
    router.get("/api/users", |req: Request| {
        let params = PaginationParams::from_query(&req.query_params);
        
        // In real app, fetch from database
        let users = vec![
            User { id: 1, name: "John".to_string(), email: "john@example.com".to_string() },
            User { id: 2, name: "Jane".to_string(), email: "jane@example.com".to_string() },
        ];
        
        Ok(Response::ok().json(&serde_json::json!({
            "data": users,
            "meta": {
                "current_page": params.page,
                "per_page": params.per_page,
            }
        }))?)
    });
    
    // GET /api/users/{id} - Get single user
    router.get("/api/users/{id}", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        
        Ok(Response::ok().json(&serde_json::json!({
            "data": {
                "id": id,
                "name": "User",
                "email": "user@example.com"
            }
        }))?)
    });
    
    // POST /api/users - Create user
    router.post("/api/users", |req: Request| {
        let body: serde_json::Value = req.json()?;
        
        let name = body.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoorError::Validation("Name is required".to_string()))?;
        
        let email = body.get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NoorError::Validation("Email is required".to_string()))?;
        
        // Validate email
        noor::core::security::Validator::email(email, "email")
            .map_err(|e| NoorError::Validation(e.message))?;
        
        Ok(Response::new(StatusCode::CREATED).json(&serde_json::json!({
            "message": "User created",
            "data": {
                "id": 1,
                "name": name,
                "email": email,
            }
        }))?)
    });
    
    // PUT /api/users/{id} - Update user
    router.put("/api/users/{id}", |req: Request| {
        let id = req.param("id").unwrap_or("0");
        let body: serde_json::Value = req.json()?;
        
        Ok(Response::ok().json(&serde_json::json!({
            "message": "User updated",
            "id": id,
            "data": body,
        }))?)
    });
    
    // DELETE /api/users/{id} - Delete user
    router.delete("/api/users/{id}", |req: Request| {
        let id = req.param("id").unwrap_or("0");
        
        Ok(Response::new(StatusCode::NO_CONTENT))
    });
    
    let app = Application::new(config, router);
    app.run()
}
