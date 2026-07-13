// ============================================================
// GraphQL Support - دعم GraphQL
// ============================================================
// Simple GraphQL-like query resolver for Noor applications.
// Supports queries, mutations, and schema definition.
//
// دعم GraphQL بسيط لتطبيقات نور.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// GraphQL field type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphQLType {
    String,
    Int,
    Float,
    Boolean,
    ID,
    Custom(String),
    List(Box<GraphQLType>),
    Nullable(Box<GraphQLType>),
}

impl GraphQLType {
    pub fn to_str(&self) -> String {
        match self {
            Self::String => "String".to_string(),
            Self::Int => "Int".to_string(),
            Self::Float => "Float".to_string(),
            Self::Boolean => "Boolean".to_string(),
            Self::ID => "ID".to_string(),
            Self::Custom(name) => name.clone(),
            Self::List(inner) => format!("[{}]", inner.to_str()),
            Self::Nullable(inner) => inner.to_str(),
        }
    }
}

/// GraphQL field definition
#[derive(Debug, Clone)]
pub struct GraphQLField {
    pub name: String,
    pub field_type: GraphQLType,
    pub description: Option<String>,
    pub arguments: Vec<GraphQLArgument>,
}

/// GraphQL argument definition
#[derive(Debug, Clone)]
pub struct GraphQLArgument {
    pub name: String,
    pub arg_type: GraphQLType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
}

/// GraphQL type definition
#[derive(Debug, Clone)]
pub struct GraphQLType_ {
    pub name: String,
    pub fields: Vec<GraphQLField>,
    pub description: Option<String>,
}

/// GraphQL schema
#[derive(Debug, Clone)]
pub struct GraphQLSchema {
    pub query_type: Option<GraphQLType_>,
    pub mutation_type: Option<GraphQLType_>,
    pub types: Vec<GraphQLType_>,
}

/// Resolver function type
type ResolverFn = Arc<dyn Fn(&HashMap<String, serde_json::Value>) -> crate::NoorResult<serde_json::Value> + Send + Sync>;

/// GraphQL resolver registry
pub struct GraphQLResolver {
    resolvers: Arc<RwLock<HashMap<String, ResolverFn>>>,
    schema: Arc<RwLock<GraphQLSchema>>,
}

impl Default for GraphQLResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphQLResolver {
    pub fn new() -> Self {
        Self {
            resolvers: Arc::new(RwLock::new(HashMap::new())),
            schema: Arc::new(RwLock::new(GraphQLSchema {
                query_type: None,
                mutation_type: None,
                types: vec![],
            })),
        }
    }
    
    /// Register a resolver for a field
    pub fn resolver<F>(&self, field_name: &str, resolver: F) -> &Self
    where
        F: Fn(&HashMap<String, serde_json::Value>) -> crate::NoorResult<serde_json::Value> + Send + Sync + 'static,
    {
        self.resolvers
            .write()
            .insert(field_name.to_string(), Arc::new(resolver));
        self
    }
    
    /// Set the schema
    pub fn set_schema(&self, schema: GraphQLSchema) {
        *self.schema.write() = schema;
    }
    
    /// Execute a query
    pub fn execute(&self, query: &str, variables: HashMap<String, serde_json::Value>) -> crate::NoorResult<GraphQLResponse> {
        // Parse query (simplified - real impl would use a proper parser)
        let parsed = Self::parse_query(query)?;
        
        let mut data = serde_json::Map::new();
        let mut errors = Vec::new();
        
        for field in &parsed.fields {
            if let Some(resolver) = self.resolvers.read().get(&field.name) {
                match resolver(&field.arguments) {
                    Ok(value) => {
                        data.insert(field.name.clone(), value);
                    }
                    Err(e) => {
                        errors.push(GraphQLError {
                            message: e.to_string(),
                            locations: None,
                            path: Some(vec![field.name.clone()]),
                        });
                    }
                }
            } else {
                errors.push(GraphQLError {
                    message: format!("Cannot query field '{}' on type 'Query'", field.name),
                    locations: None,
                    path: Some(vec![field.name.clone()]),
                });
            }
        }
        
        Ok(GraphQLResponse {
            data: if data.is_empty() { None } else { Some(serde_json::Value::Object(data)) },
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }
    
    /// Parse a GraphQL query (simplified)
    fn parse_query(query: &str) -> crate::NoorResult<ParsedQuery> {
        // This is a very simplified parser
        // In production, use a proper GraphQL parser like `async-graphql`
        
        let mut fields = Vec::new();
        let mut in_query = false;
        let mut current_field = String::new();
        let mut in_field_name = false;
        
        for ch in query.chars() {
            match ch {
                '{' => {
                    in_query = true;
                    in_field_name = true;
                }
                '}' => {
                    if !current_field.is_empty() {
                        fields.push(ParsedField {
                            name: current_field.trim().to_string(),
                            arguments: HashMap::new(),
                        });
                        current_field.clear();
                    }
                    in_query = false;
                }
                _ if in_query && in_field_name => {
                    if ch.is_whitespace() && !current_field.is_empty() {
                        fields.push(ParsedField {
                            name: current_field.trim().to_string(),
                            arguments: HashMap::new(),
                        });
                        current_field.clear();
                    } else if !ch.is_whitespace() {
                        current_field.push(ch);
                    }
                }
                _ => {}
            }
        }
        
        Ok(ParsedQuery {
            operation_type: OperationType::Query,
            fields,
        })
    }
    
    /// Generate schema in SDL (Schema Definition Language)
    pub fn to_sdl(&self) -> String {
        let schema = self.schema.read();
        let mut sdl = String::new();
        
        if let Some(ref query_type) = schema.query_type {
            sdl.push_str(&format!("type {} {{\n", query_type.name));
            for field in &query_type.fields {
                sdl.push_str(&format!("  {}", field.name));
                
                if !field.arguments.is_empty() {
                    sdl.push('(');
                    let args: Vec<String> = field.arguments.iter().map(|arg| {
                        let required = if arg.required { "!" } else { "" };
                        format!("{}: {}{}", arg.name, arg.arg_type.to_str(), required)
                    }).collect();
                    sdl.push_str(&args.join(", "));
                    sdl.push(')');
                }
                
                sdl.push_str(&format!(": {}\n", field.field_type.to_str()));
            }
            sdl.push_str("}\n");
        }
        
        for type_def in &schema.types {
            sdl.push_str(&format!("\ntype {} {{\n", type_def.name));
            for field in &type_def.fields {
                sdl.push_str(&format!("  {}: {}\n", field.name, field.field_type.to_str()));
            }
            sdl.push_str("}\n");
        }
        
        sdl
    }
}

/// Parsed GraphQL query
#[derive(Debug)]
struct ParsedQuery {
    operation_type: OperationType,
    fields: Vec<ParsedField>,
}

#[derive(Debug)]
enum OperationType {
    Query,
    Mutation,
    Subscription,
}

#[derive(Debug)]
struct ParsedField {
    name: String,
    arguments: HashMap<String, serde_json::Value>,
}

/// GraphQL response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<SourceLocation>>,
    pub path: Option<Vec<String>>,
}

/// Source location in a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

/// Schema builder for constructing GraphQL schemas
pub struct SchemaBuilder {
    query_type: Option<GraphQLType_>,
    mutation_type: Option<GraphQLType_>,
    types: Vec<GraphQLType_>,
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            query_type: None,
            mutation_type: None,
            types: vec![],
        }
    }
    
    pub fn query_type(mut self, type_def: GraphQLType_) -> Self {
        self.query_type = Some(type_def);
        self
    }
    
    pub fn mutation_type(mut self, type_def: GraphQLType_) -> Self {
        self.mutation_type = Some(type_def);
        self
    }
    
    pub fn type_def(mut self, type_def: GraphQLType_) -> Self {
        self.types.push(type_def);
        self
    }
    
    pub fn build(self) -> GraphQLSchema {
        GraphQLSchema {
            query_type: self.query_type,
            mutation_type: self.mutation_type,
            types: self.types,
        }
    }
}

/// Field builder helper
pub struct FieldBuilder {
    field: GraphQLField,
}

impl FieldBuilder {
    pub fn new(name: &str, field_type: GraphQLType) -> Self {
        Self {
            field: GraphQLField {
                name: name.to_string(),
                field_type,
                description: None,
                arguments: vec![],
            },
        }
    }
    
    pub fn description(mut self, desc: &str) -> Self {
        self.field.description = Some(desc.to_string());
        self
    }
    
    pub fn argument(mut self, name: &str, arg_type: GraphQLType, required: bool) -> Self {
        self.field.arguments.push(GraphQLArgument {
            name: name.to_string(),
            arg_type,
            default_value: None,
            required,
        });
        self
    }
    
    pub fn build(self) -> GraphQLField {
        self.field
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_graphql_resolver() {
        let resolver = GraphQLResolver::new();
        
        resolver.resolver("hello", |_| {
            Ok(serde_json::json!("Hello, World!"))
        });
        
        let response = resolver.execute("{ hello }", HashMap::new()).unwrap();
        
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        
        let data = response.data.unwrap();
        assert_eq!(data["hello"], "Hello, World!");
    }
    
    #[test]
    fn test_graphql_missing_field() {
        let resolver = GraphQLResolver::new();
        
        let response = resolver.execute("{ nonexistent }", HashMap::new()).unwrap();
        
        assert!(response.errors.is_some());
        let errors = response.errors.unwrap();
        assert!(errors[0].message.contains("Cannot query field"));
    }
    
    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new()
            .query_type(GraphQLType_ {
                name: "Query".to_string(),
                fields: vec![
                    FieldBuilder::new("user", GraphQLType::Custom("User".to_string()))
                        .argument("id", GraphQLType::ID, true)
                        .build(),
                    FieldBuilder::new("posts", GraphQLType::List(Box::new(GraphQLType::Custom("Post".to_string()))))
                        .build(),
                ],
                description: None,
            })
            .build();
        
        assert!(schema.query_type.is_some());
        let query = schema.query_type.unwrap();
        assert_eq!(query.fields.len(), 2);
    }
    
    #[test]
    fn test_type_to_str() {
        assert_eq!(GraphQLType::String.to_str(), "String");
        assert_eq!(GraphQLType::Int.to_str(), "Int");
        assert_eq!(
            GraphQLType::List(Box::new(GraphQLType::String)).to_str(),
            "[String]"
        );
    }
}
