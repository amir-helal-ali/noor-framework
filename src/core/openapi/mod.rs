// ============================================================
// OpenAPI/Swagger Documentation Generator
// مولد توثيق OpenAPI/Swagger
// ============================================================
// Automatically generates OpenAPI 3.0 specification from routes.
// Provides Swagger UI for interactive API documentation.
//
// يولد مواصفات OpenAPI 3.0 تلقائياً من المسارات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// OpenAPI specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub servers: Vec<OpenApiServer>,
    pub paths: HashMap<String, HashMap<String, OpenApiOperation>>,
    pub components: OpenApiComponents,
    pub security: Vec<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Option<OpenApiContact>,
    pub license: Option<OpenApiLicense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiContact {
    pub name: String,
    pub email: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiLicense {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    pub tags: Vec<String>,
    pub summary: String,
    pub description: String,
    pub operation_id: String,
    pub parameters: Vec<OpenApiParameter>,
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: HashMap<String, OpenApiResponse>,
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiParameter {
    pub name: String,
    pub location: String,  // query, path, header, cookie
    pub description: String,
    pub required: bool,
    pub schema: OpenApiSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiRequestBody {
    pub description: String,
    pub content: HashMap<String, OpenApiMediaType>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiMediaType {
    pub schema: OpenApiSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    pub description: String,
    pub content: Option<HashMap<String, OpenApiMediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub items: Option<Box<OpenApiSchema>>,
    pub properties: Option<HashMap<String, OpenApiSchema>>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    pub example: Option<serde_json::Value>,
    #[serde(rename = "$ref")]
    pub ref_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiComponents {
    pub schemas: HashMap<String, OpenApiSchema>,
    pub security_schemes: HashMap<String, OpenApiSecurityScheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub description: Option<String>,
}

impl Default for OpenApiComponents {
    fn default() -> Self {
        let mut security_schemes = HashMap::new();
        security_schemes.insert(
            "bearerAuth".to_string(),
            OpenApiSecurityScheme {
                scheme_type: "http".to_string(),
                scheme: Some("bearer".to_string()),
                bearer_format: Some("JWT".to_string()),
                description: Some("JWT Bearer token authentication".to_string()),
            },
        );
        
        Self {
            schemas: HashMap::new(),
            security_schemes,
        }
    }
}

/// OpenAPI documentation builder
pub struct OpenApiBuilder {
    spec: OpenApiSpec,
}

impl Default for OpenApiBuilder {
    fn default() -> Self {
        Self::new("Noor API", "1.0.0")
    }
}

impl OpenApiBuilder {
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            spec: OpenApiSpec {
                openapi: "3.0.3".to_string(),
                info: OpenApiInfo {
                    title: title.to_string(),
                    description: String::new(),
                    version: version.to_string(),
                    contact: None,
                    license: None,
                },
                servers: vec![],
                paths: HashMap::new(),
                components: OpenApiComponents::default(),
                security: vec![],
            },
        }
    }
    
    pub fn description(mut self, desc: &str) -> Self {
        self.spec.info.description = desc.to_string();
        self
    }
    
    pub fn contact(mut self, name: &str, email: &str, url: Option<&str>) -> Self {
        self.spec.info.contact = Some(OpenApiContact {
            name: name.to_string(),
            email: email.to_string(),
            url: url.map(|s| s.to_string()),
        });
        self
    }
    
    pub fn license(mut self, name: &str, url: &str) -> Self {
        self.spec.info.license = Some(OpenApiLicense {
            name: name.to_string(),
            url: url.to_string(),
        });
        self
    }
    
    pub fn server(mut self, url: &str, description: &str) -> Self {
        self.spec.servers.push(OpenApiServer {
            url: url.to_string(),
            description: description.to_string(),
        });
        self
    }
    
    /// Add an operation to a path
    pub fn operation(mut self, path: &str, method: &str, operation: OpenApiOperation) -> Self {
        self.spec.paths
            .entry(path.to_string())
            .or_insert_with(HashMap::new)
            .insert(method.to_lowercase(), operation);
        self
    }
    
    /// Add a schema component
    pub fn schema(mut self, name: &str, schema: OpenApiSchema) -> Self {
        self.spec.components.schemas.insert(name.to_string(), schema);
        self
    }
    
    /// Enable JWT security globally
    pub fn with_jwt_security(mut self) -> Self {
        let mut security = HashMap::new();
        security.insert("bearerAuth".to_string(), vec![]);
        self.spec.security.push(security);
        self
    }
    
    /// Build the spec
    pub fn build(self) -> OpenApiSpec {
        self.spec
    }
    
    /// Generate Swagger UI HTML
    pub fn to_swagger_ui(&self) -> String {
        let spec_json = serde_json::to_string_pretty(&self.spec).unwrap_or_default();
        
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{} - API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
    <style>
        html {{ box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }}
        *, *:before, *:after {{ box-sizing: inherit; }}
        body {{ margin: 0; background: #fafafa; }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        const spec = {};
        window.onload = () => {{
            window.ui = SwaggerUIBundle({{
                spec: spec,
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [SwaggerUIBundle.presets.apis],
                layout: 'BaseLayout',
            }});
        }};
    </script>
</body>
</html>"#,
            self.spec.info.title,
            spec_json
        )
    }
}

/// Operation builder helper
pub struct OperationBuilder {
    operation: OpenApiOperation,
}

impl OperationBuilder {
    pub fn new(operation_id: &str) -> Self {
        Self {
            operation: OpenApiOperation {
                tags: vec![],
                summary: String::new(),
                description: String::new(),
                operation_id: operation_id.to_string(),
                parameters: vec![],
                request_body: None,
                responses: HashMap::new(),
                security: None,
                deprecated: false,
            },
        }
    }
    
    pub fn tag(mut self, tag: &str) -> Self {
        self.operation.tags.push(tag.to_string());
        self
    }
    
    pub fn summary(mut self, summary: &str) -> Self {
        self.operation.summary = summary.to_string();
        self
    }
    
    pub fn description(mut self, desc: &str) -> Self {
        self.operation.description = desc.to_string();
        self
    }
    
    pub fn parameter(mut self, name: &str, location: &str, required: bool, description: &str, schema_type: &str) -> Self {
        self.operation.parameters.push(OpenApiParameter {
            name: name.to_string(),
            location: location.to_string(),
            description: description.to_string(),
            required,
            schema: OpenApiSchema {
                schema_type: Some(schema_type.to_string()),
                format: None,
                description: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                example: None,
                ref_path: None,
            },
        });
        self
    }
    
    pub fn response(mut self, status: &str, description: &str) -> Self {
        self.operation.responses.insert(status.to_string(), OpenApiResponse {
            description: description.to_string(),
            content: None,
        });
        self
    }
    
    pub fn json_response(mut self, status: &str, description: &str, schema: OpenApiSchema) -> Self {
        let mut content = HashMap::new();
        content.insert("application/json".to_string(), OpenApiMediaType { schema });
        
        self.operation.responses.insert(status.to_string(), OpenApiResponse {
            description: description.to_string(),
            content: Some(content),
        });
        self
    }
    
    pub fn json_request(mut self, description: &str, schema: OpenApiSchema, required: bool) -> Self {
        let mut content = HashMap::new();
        content.insert("application/json".to_string(), OpenApiMediaType { schema });
        
        self.operation.request_body = Some(OpenApiRequestBody {
            description: description.to_string(),
            content,
            required,
        });
        self
    }
    
    pub fn with_jwt(mut self) -> Self {
        let mut security = HashMap::new();
        security.insert("bearerAuth".to_string(), vec![]);
        self.operation.security = Some(vec![security]);
        self
    }
    
    pub fn deprecated(mut self) -> Self {
        self.operation.deprecated = true;
        self
    }
    
    pub fn build(self) -> OpenApiOperation {
        self.operation
    }
}

/// Schema builder helper
pub struct SchemaBuilder {
    schema: OpenApiSchema,
}

impl SchemaBuilder {
    pub fn new(schema_type: &str) -> Self {
        Self {
            schema: OpenApiSchema {
                schema_type: Some(schema_type.to_string()),
                format: None,
                description: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                example: None,
                ref_path: None,
            },
        }
    }
    
    pub fn format(mut self, format: &str) -> Self {
        self.schema.format = Some(format.to_string());
        self
    }
    
    pub fn description(mut self, desc: &str) -> Self {
        self.schema.description = Some(desc.to_string());
        self
    }
    
    pub fn ref_to(mut self, ref_path: &str) -> Self {
        self.schema.ref_path = Some(format!("#/components/schemas/{}", ref_path));
        self.schema.schema_type = None;
        self
    }
    
    pub fn build(self) -> OpenApiSchema {
        self.schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_openapi_builder() {
        let spec = OpenApiBuilder::new("Test API", "1.0.0")
            .description("A test API")
            .server("https://api.example.com", "Production")
            .build();
        
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
        assert_eq!(spec.servers.len(), 1);
    }
    
    #[test]
    fn test_operation_builder() {
        let operation = OperationBuilder::new("getUsers")
            .tag("Users")
            .summary("Get all users")
            .parameter("page", "query", false, "Page number", "integer")
            .response("200", "Successful response")
            .build();
        
        assert_eq!(operation.operation_id, "getUsers");
        assert_eq!(operation.tags, vec!["Users"]);
        assert_eq!(operation.parameters.len(), 1);
        assert!(operation.responses.contains_key("200"));
    }
    
    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new("string")
            .format("email")
            .description("User email address")
            .build();
        
        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert_eq!(schema.format, Some("email".to_string()));
    }
    
    #[test]
    fn test_swagger_ui_generation() {
        let builder = OpenApiBuilder::new("Test API", "1.0.0");
        let html = builder.to_swagger_ui();
        
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("Test API"));
    }
}
