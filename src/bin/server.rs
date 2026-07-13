// ============================================================
// Noor Server Binary - خادم نور
// ============================================================

use noor::{Application, Config, Router};
use noor::core::http::{Response, StatusCode};

fn main() -> noor::NoorResult<()> {
    // Load configuration
    let config_path = std::path::Path::new("noor.toml");
    let config = Config::load(config_path)?;
    
    // Build the router
    let mut router = Router::new();
    
    router.get("/", |_req| {
        Ok(Response::ok().html(
            r#"<html>
<head><title>Noor Framework</title></head>
<body>
    <h1>✨ Welcome to Noor Framework</h1>
    <p>Your application is running successfully!</p>
    <p><a href="/blog">Visit the Blog Demo</a></p>
</body>
</html>"#
        ))
    });
    
    router.get("/health", |_req| {
        Ok(Response::ok().json(&serde_json::json!({
            "status": "ok",
            "framework": "noor",
            "version": noor::VERSION,
        }))?)
    });
    
    // Create and run the application
    let app = Application::new(config, router);
    app.run()
}
