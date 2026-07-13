// ============================================================
// مثال: OpenAPI/Swagger | Example: API Documentation
// ============================================================

use noor::*;
use noor::core::openapi::{OpenApiBuilder, OperationBuilder, SchemaBuilder};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n📖 OpenAPI/Swagger Demo\n");
    
    let builder = OpenApiBuilder::new("Noor Blog API", "1.0.0")
        .description("A sample blog API with authentication and CRUD operations")
        .contact("Noor Team", "support@noor.dev", Some("https://noor.dev"))
        .license("MIT", "https://opensource.org/licenses/MIT")
        .server("https://api.noor.dev", "Production")
        .server("http://localhost:8080", "Development")
        .with_jwt_security()
        // Users endpoints
        .operation(
            "/users",
            "get",
            OperationBuilder::new("listUsers")
                .tag("Users")
                .summary("Get all users")
                .description("Retrieve a paginated list of all users")
                .parameter("page", "query", false, "Page number", "integer")
                .parameter("per_page", "query", false, "Items per page (max 100)", "integer")
                .json_response("200", "Successful response",
                    SchemaBuilder::new("object").build()
                )
                .with_jwt()
                .build()
        )
        .operation(
            "/users/{id}",
            "get",
            OperationBuilder::new("getUser")
                .tag("Users")
                .summary("Get a user by ID")
                .parameter("id", "path", true, "User ID", "integer")
                .response("200", "User found")
                .response("404", "User not found")
                .with_jwt()
                .build()
        )
        // Posts endpoints
        .operation(
            "/posts",
            "get",
            OperationBuilder::new("listPosts")
                .tag("Posts")
                .summary("Get all posts")
                .parameter("page", "query", false, "Page number", "integer")
                .parameter("category", "query", false, "Filter by category", "string")
                .json_response("200", "Successful response",
                    SchemaBuilder::new("object").build()
                )
                .build()
        )
        .operation(
            "/posts",
            "post",
            OperationBuilder::new("createPost")
                .tag("Posts")
                .summary("Create a new post")
                .json_request("Post data",
                    SchemaBuilder::new("object")
                        .description("Post creation data")
                        .build(),
                    true
                )
                .json_response("201", "Post created",
                    SchemaBuilder::new("object").build()
                )
                .json_response("422", "Validation error",
                    SchemaBuilder::new("object").build()
                )
                .with_jwt()
                .build()
        );
    
    // Generate Swagger UI HTML
    let html = builder.to_swagger_ui();
    println!("✓ Swagger UI generated ({} bytes)", html.len());
    
    // Export spec
    let spec = builder.build();
    let spec_json = serde_json::to_string_pretty(&spec)?;
    println!("✓ OpenAPI spec generated ({} bytes)", spec_json.len());
    
    println!("\n📄 API Endpoints:");
    for (path, methods) in &spec.paths {
        for (method, operation) in methods {
            println!("  {} {} - {}", method.to_uppercase(), path, operation.summary);
        }
    }
    
    println!("\n🌐 Visit /api/docs in your browser to see the interactive Swagger UI!");
    println!("\n✅ OpenAPI demo completed!");
    Ok(())
}
