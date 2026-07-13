// ============================================================
// Repository Pattern - نمط المستودع
// ============================================================
// Abstraction layer for data access. Separates business logic
// from data persistence concerns.
//
// طبقة تجريد للوصول للبيانات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Repository trait
pub trait Repository<T>: Send + Sync {
    /// Find a record by ID
    fn find(&self, id: i64) -> Option<T>;
    
    /// Find all records
    fn all(&self) -> Vec<T>;
    
    /// Create a new record
    fn create(&self, data: &T) -> crate::NoorResult<T>;
    
    /// Update a record
    fn update(&self, id: i64, data: &T) -> crate::NoorResult<Option<T>>;
    
    /// Delete a record
    fn delete(&self, id: i64) -> bool;
    
    /// Count records
    fn count(&self) -> usize;
    
    /// Find with filters
    fn find_by(&self, filters: &HashMap<String, String>) -> Vec<T>;
    
    /// Paginate results
    fn paginate(&self, page: u32, per_page: u32) -> (Vec<T>, u64);
}

/// In-memory repository (useful for testing and small datasets)
pub struct InMemoryRepository<T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>> {
    items: Arc<RwLock<Vec<T>>>,
    id_extractor: Box<dyn Fn(&T) -> i64 + Send + Sync>,
}

impl<T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>> InMemoryRepository<T> {
    pub fn new<F>(id_extractor: F) -> Self
    where
        F: Fn(&T) -> i64 + Send + Sync + 'static,
    {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            id_extractor: Box::new(id_extractor),
        }
    }
    
    pub fn with_data<F>(id_extractor: F, data: Vec<T>) -> Self
    where
        F: Fn(&T) -> i64 + Send + Sync + 'static,
    {
        Self {
            items: Arc::new(RwLock::new(data)),
            id_extractor: Box::new(id_extractor),
        }
    }
}

impl<T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>> Repository<T> for InMemoryRepository<T> {
    fn find(&self, id: i64) -> Option<T> {
        self.items.read().iter()
            .find(|item| (self.id_extractor)(item) == id)
            .cloned()
    }
    
    fn all(&self) -> Vec<T> {
        self.items.read().clone()
    }
    
    fn create(&self, data: &T) -> crate::NoorResult<T> {
        self.items.write().push(data.clone());
        Ok(data.clone())
    }
    
    fn update(&self, id: i64, data: &T) -> crate::NoorResult<Option<T>> {
        let mut items = self.items.write();
        
        if let Some(pos) = items.iter().position(|item| (self.id_extractor)(item) == id) {
            items[pos] = data.clone();
            return Ok(Some(data.clone()));
        }
        
        Ok(None)
    }
    
    fn delete(&self, id: i64) -> bool {
        let mut items = self.items.write();
        let initial = items.len();
        items.retain(|item| (self.id_extractor)(item) != id);
        items.len() < initial
    }
    
    fn count(&self) -> usize {
        self.items.read().len()
    }
    
    fn find_by(&self, _filters: &HashMap<String, String>) -> Vec<T> {
        // Generic implementation - override for specific filtering
        self.all()
    }
    
    fn paginate(&self, page: u32, per_page: u32) -> (Vec<T>, u64) {
        let items = self.items.read();
        let total = items.len() as u64;
        
        let start = ((page - 1) * per_page) as usize;
        let end = (start + per_page as usize).min(items.len());
        
        let paginated: Vec<T> = if start < items.len() {
            items[start..end].to_vec()
        } else {
            vec![]
        };
        
        (paginated, total)
    }
}

/// Repository factory
pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Create an in-memory repository
    pub fn in_memory<T, F>(id_extractor: F) -> Arc<InMemoryRepository<T>>
    where
        T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        F: Fn(&T) -> i64 + Send + Sync + 'static,
    {
        Arc::new(InMemoryRepository::new(id_extractor))
    }
    
    /// Create an in-memory repository with initial data
    pub fn with_data<T, F>(id_extractor: F, data: Vec<T>) -> Arc<InMemoryRepository<T>>
    where
        T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        F: Fn(&T) -> i64 + Send + Sync + 'static,
    {
        Arc::new(InMemoryRepository::with_data(id_extractor, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct User {
        id: i64,
        name: String,
        email: String,
    }
    
    #[test]
    fn test_repository_crud() {
        let repo = InMemoryRepository::new(|u: &User| u.id);
        
        // Create
        let user = User { id: 1, name: "John".to_string(), email: "john@example.com".to_string() };
        repo.create(&user).unwrap();
        
        // Find
        let found = repo.find(1).unwrap();
        assert_eq!(found.name, "John");
        
        // Update
        let updated = User { id: 1, name: "Jane".to_string(), email: "jane@example.com".to_string() };
        repo.update(1, &updated).unwrap();
        
        let found = repo.find(1).unwrap();
        assert_eq!(found.name, "Jane");
        
        // Count
        assert_eq!(repo.count(), 1);
        
        // Delete
        assert!(repo.delete(1));
        assert_eq!(repo.count(), 0);
    }
    
    #[test]
    fn test_repository_pagination() {
        let data: Vec<User> = (1..=25)
            .map(|i| User {
                id: i,
                name: format!("User {}", i),
                email: format!("user{}@example.com", i),
            })
            .collect();
        
        let repo = InMemoryRepository::with_data(|u: &User| u.id, data);
        
        let (page1, total) = repo.paginate(1, 10);
        assert_eq!(page1.len(), 10);
        assert_eq!(total, 25);
        
        let (page3, _) = repo.paginate(3, 10);
        assert_eq!(page3.len(), 5); // Last page
        
        let (page4, _) = repo.paginate(4, 10);
        assert_eq!(page4.len(), 0); // Beyond total
    }
}
