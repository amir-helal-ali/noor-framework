// ============================================================
// Search Engine - محرك البحث
// ============================================================
// Full-text search with indexing, ranking, and highlighting.
// Supports in-memory and SQLite FTS5 backends.
//
// بحث كامل بالنص مع الفهرسة والترتيب.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Search document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub metadata: HashMap<String, String>,
    pub score: f64,
    pub highlights: Vec<String>,
}

impl SearchDocument {
    pub fn new(id: &str, title: &str, content: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            url: None,
            metadata: HashMap::new(),
            score: 0.0,
            highlights: vec![],
        }
    }
    
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
    
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Search query
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub fields: Vec<String>,
    pub limit: usize,
    pub offset: usize,
    pub filters: HashMap<String, String>,
    pub sort_by: Option<String>,
    pub sort_order: SortOrder,
    pub min_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            fields: vec!["title".to_string(), "content".to_string()],
            limit: 20,
            offset: 0,
            filters: HashMap::new(),
            sort_by: None,
            sort_order: SortOrder::Desc,
            min_score: 0.0,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub documents: Vec<SearchDocument>,
    pub total: usize,
    pub query: String,
    pub took_ms: u64,
    pub max_score: f64,
}

/// Search index
pub struct SearchIndex {
    name: String,
    documents: Arc<RwLock<HashMap<String, SearchDocument>>>,
    /// Inverted index: term -> [(doc_id, frequency)]
    inverted_index: Arc<RwLock<HashMap<String, Vec<(String, usize)>>>>,
}

impl SearchIndex {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            documents: Arc::new(RwLock::new(HashMap::new())),
            inverted_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add or update a document
    pub fn index(&self, doc: SearchDocument) {
        let doc_id = doc.id.clone();
        let title = doc.title.clone();
        let content = doc.content.clone();
        
        // Remove old document from inverted index if exists
        self.remove_from_inverted_index(&doc_id);
        
        // Index title terms
        let title_terms = Self::tokenize(&title);
        for term in &title_terms {
            let mut index = self.inverted_index.write();
            let entry = index.entry(term.clone()).or_insert_with(Vec::new);
            
            if let Some((_, freq)) = entry.iter_mut().find(|(id, _)| id == &doc_id) {
                *freq += 2; // Title terms weighted higher
            } else {
                entry.push((doc_id.clone(), 2));
            }
        }
        
        // Index content terms
        let content_terms = Self::tokenize(&content);
        for term in &content_terms {
            let mut index = self.inverted_index.write();
            let entry = index.entry(term.clone()).or_insert_with(Vec::new);
            
            if let Some((_, freq)) = entry.iter_mut().find(|(id, _)| id == &doc_id) {
                *freq += 1;
            } else {
                entry.push((doc_id.clone(), 1));
            }
        }
        
        // Store document
        self.documents.write().insert(doc_id, doc);
    }
    
    /// Remove a document
    pub fn remove(&self, doc_id: &str) -> bool {
        self.remove_from_inverted_index(doc_id);
        self.documents.write().remove(doc_id).is_some()
    }
    
    fn remove_from_inverted_index(&self, doc_id: &str) {
        let mut index = self.inverted_index.write();
        for entries in index.values_mut() {
            entries.retain(|(id, _)| id != doc_id);
        }
    }
    
    /// Search the index
    pub fn search(&self, query: &SearchQuery) -> SearchResult {
        let start = std::time::Instant::now();
        
        let terms = Self::tokenize(&query.query);
        
        if terms.is_empty() {
            return SearchResult {
                documents: vec![],
                total: 0,
                query: query.query.clone(),
                took_ms: start.elapsed().as_millis() as u64,
                max_score: 0.0,
            };
        }
        
        // Calculate document scores
        let mut scores: HashMap<String, f64> = HashMap::new();
        let index = self.inverted_index.read();
        
        for term in &terms {
            if let Some(entries) = index.get(term) {
                let idf = Self::calculate_idf(
                    self.documents.read().len() as f64,
                    entries.len() as f64,
                );
                
                for (doc_id, tf) in entries {
                    let score = (*tf as f64) * idf;
                    *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                }
            }
        }
        
        drop(index);
        
        // Get matching documents
        let documents = self.documents.read();
        let mut results: Vec<SearchDocument> = scores
            .iter()
            .filter_map(|(doc_id, score)| {
                if *score < query.min_score {
                    return None;
                }
                
                let mut doc = documents.get(doc_id)?.clone();
                doc.score = *score;
                
                // Apply filters
                for (key, value) in &query.filters {
                    if doc.metadata.get(key) != Some(value) {
                        return None;
                    }
                }
                
                // Add highlights
                doc.highlights = Self::highlight(&doc.content, &terms);
                
                Some(doc)
            })
            .collect();
        
        // Sort by score
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let total = results.len();
        let max_score = results.first().map(|d| d.score).unwrap_or(0.0);
        
        // Paginate
        let paginated: Vec<SearchDocument> = results
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();
        
        SearchResult {
            documents: paginated,
            total,
            query: query.query.clone(),
            took_ms: start.elapsed().as_millis() as u64,
            max_score,
        }
    }
    
    /// Get the number of indexed documents
    pub fn document_count(&self) -> usize {
        self.documents.read().len()
    }
    
    /// Clear the index
    pub fn clear(&self) {
        self.documents.write().clear();
        self.inverted_index.write().clear();
    }
    
    /// Tokenize text into terms
    fn tokenize(text: &str) -> Vec<String> {
        text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Calculate Inverse Document Frequency
    fn calculate_idf(total_docs: f64, docs_with_term: f64) -> f64 {
        if docs_with_term == 0.0 {
            return 0.0;
        }
        (total_docs / docs_with_term).ln() + 1.0
    }
    
    /// Highlight matching terms in text
    fn highlight(text: &str, terms: &[String]) -> Vec<String> {
        let lower_text = text.to_lowercase();
        let mut highlights = Vec::new();
        
        for term in terms {
            if let Some(pos) = lower_text.find(term) {
                let start = pos.saturating_sub(30);
                let end = (pos + term.len() + 30).min(text.len());
                
                let snippet = format!(
                    "...{}...",
                    &text[start..end]
                );
                
                highlights.push(snippet);
            }
        }
        
        highlights
    }
}

/// Search engine managing multiple indices
pub struct SearchEngine {
    indices: Arc<RwLock<HashMap<String, Arc<SearchIndex>>>>,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            indices: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create or get an index
    pub fn index(&self, name: &str) -> Arc<SearchIndex> {
        let mut indices = self.indices.write();
        indices
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(SearchIndex::new(name)))
            .clone()
    }
    
    /// Delete an index
    pub fn delete_index(&self, name: &str) -> bool {
        self.indices.write().remove(name).is_some()
    }
    
    /// List all indices
    pub fn list_indices(&self) -> Vec<String> {
        self.indices.read().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_index_and_search() {
        let index = SearchIndex::new("test");
        
        index.index(SearchDocument::new("1", "Rust Programming", "Rust is a systems programming language"));
        index.index(SearchDocument::new("2", "Python Guide", "Python is a popular programming language"));
        index.index(SearchDocument::new("3", "Web Development", "Building web applications with Rust"));
        
        let query = SearchQuery {
            query: "rust programming".to_string(),
            ..Default::default()
        };
        
        let result = index.search(&query);
        
        assert!(result.total > 0);
        assert_eq!(result.documents[0].id, "1"); // Should be highest scored
    }
    
    #[test]
    fn test_tokenization() {
        let tokens = SearchIndex::tokenize("Hello, World! This is a Test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }
    
    #[test]
    fn test_remove_document() {
        let index = SearchIndex::new("test");
        
        index.index(SearchDocument::new("1", "Test", "Content"));
        assert_eq!(index.document_count(), 1);
        
        assert!(index.remove("1"));
        assert_eq!(index.document_count(), 0);
    }
    
    #[test]
    fn test_search_with_filters() {
        let index = SearchIndex::new("test");
        
        index.index(
            SearchDocument::new("1", "Post 1", "Content about Rust")
                .with_metadata("category", "tech")
        );
        index.index(
            SearchDocument::new("2", "Post 2", "Content about Rust")
                .with_metadata("category", "news")
        );
        
        let query = SearchQuery {
            query: "rust".to_string(),
            filters: {
                let mut f = HashMap::new();
                f.insert("category".to_string(), "tech".to_string());
                f
            },
            ..Default::default()
        };
        
        let result = index.search(&query);
        assert_eq!(result.total, 1);
        assert_eq!(result.documents[0].id, "1");
    }
    
    #[test]
    fn test_highlight() {
        let highlights = SearchIndex::highlight(
            "This is a long text about Rust programming language",
            &["rust".to_string()],
        );
        
        assert!(!highlights.is_empty());
        assert!(highlights[0].to_lowercase().contains("rust"));
    }
    
    #[test]
    fn test_search_engine_multiple_indices() {
        let engine = SearchEngine::new();
        
        let posts_index = engine.index("posts");
        let users_index = engine.index("users");
        
        posts_index.index(SearchDocument::new("1", "Post", "Content"));
        users_index.index(SearchDocument::new("1", "User", "Bio"));
        
        assert_eq!(engine.list_indices().len(), 2);
    }
}
