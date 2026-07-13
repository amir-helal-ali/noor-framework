// ============================================================
// Pagination & Sorting - التصفح والترتيب
// ============================================================
// Helpers for paginating and sorting query results.
// مساعدات لتصفح وترتيب نتائج الاستعلام.
// ============================================================

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Pagination metadata
/// معلومات التصفح
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub current_page: u32,
    pub per_page: u32,
    pub total: u64,
    pub last_page: u32,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub has_more_pages: bool,
}

impl Pagination {
    /// Create pagination from total count and current page
    pub fn from(total: u64, current_page: u32, per_page: u32) -> Self {
        let last_page = if per_page == 0 {
            1
        } else {
            ((total as f64 / per_page as f64).ceil() as u32).max(1)
        };
        
        let from = if total == 0 {
            None
        } else {
            Some(((current_page - 1) * per_page) as u64 + 1)
        };
        
        let to = if total == 0 {
            None
        } else {
            Some(std::cmp::min((current_page * per_page) as u64, total))
        };
        
        Self {
            current_page,
            per_page,
            total,
            last_page,
            from,
            to,
            has_more_pages: current_page < last_page,
        }
    }
    
    /// Generate pagination links
    pub fn links(&self, base_url: &str) -> HashMap<String, String> {
        let mut links = HashMap::new();
        
        links.insert("first".to_string(), format!("{}?page=1", base_url));
        links.insert("last".to_string(), format!("{}?page={}", base_url, self.last_page));
        
        if self.current_page > 1 {
            links.insert("prev".to_string(), format!("{}?page={}", base_url, self.current_page - 1));
        }
        
        if self.has_more_pages {
            links.insert("next".to_string(), format!("{}?page={}", base_url, self.current_page + 1));
        }
        
        links
    }
}

/// Paginated result
/// نتيجة مصفحة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
    pub links: PaginationLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub current_page: u32,
    pub per_page: u32,
    pub total: u64,
    pub last_page: u32,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
    pub first: String,
    pub last: String,
    pub prev: Option<String>,
    pub next: Option<String>,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result
    pub fn new(
        data: Vec<T>,
        total: u64,
        current_page: u32,
        per_page: u32,
        path: &str,
    ) -> Self {
        let pagination = Pagination::from(total, current_page, per_page);
        let last_page = pagination.last_page;
        let from = pagination.from;
        let to = pagination.to;
        let has_more = pagination.has_more_pages;
        
        let prev = if current_page > 1 {
            Some(format!("{}?page={}", path, current_page - 1))
        } else {
            None
        };
        
        let next = if has_more {
            Some(format!("{}?page={}", path, current_page + 1))
        } else {
            None
        };
        
        Self {
            data,
            meta: PaginationMeta {
                current_page,
                per_page,
                total,
                last_page,
                from,
                to,
                path: path.to_string(),
            },
            links: PaginationLinks {
                first: format!("{}?page=1", path),
                last: format!("{}?page={}", path, last_page),
                prev,
                next,
            },
        }
    }
    
    /// Map the data to a different type
    pub fn map<U, F>(self, f: F) -> PaginatedResult<U>
    where
        F: FnMut(T) -> U,
    {
        let data: Vec<U> = self.data.into_iter().map(f).collect();
        
        PaginatedResult {
            data,
            meta: self.meta,
            links: self.links,
        }
    }
}

/// Sort parameters
/// معاملات الترتيب
#[derive(Debug, Clone)]
pub struct Sort {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Sort {
    pub fn new(column: &str, direction: SortDirection) -> Self {
        Self {
            column: column.to_string(),
            direction,
        }
    }
    
    /// Parse from query string (e.g., "name" or "-name" for desc)
    pub fn from_query(sort_param: &str) -> Self {
        if sort_param.starts_with('-') {
            Self::new(&sort_param[1..], SortDirection::Desc)
        } else {
            Self::new(sort_param, SortDirection::Asc)
        }
    }
    
    /// Convert to SQL ORDER BY clause
    pub fn to_sql(&self) -> String {
        let dir = match self.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        format!("{} {}", self.column, dir)
    }
}

/// Helper to extract pagination parameters from a request
pub struct PaginationParams {
    pub page: u32,
    pub per_page: u32,
    pub sort: Option<Sort>,
}

impl PaginationParams {
    /// Extract from query parameters
    pub fn from_query(query: &std::collections::HashMap<String, String>) -> Self {
        let page = query
            .get("page")
            .and_then(|p| p.parse().ok())
            .unwrap_or(1)
            .max(1);
        
        let per_page = query
            .get("per_page")
            .or_else(|| query.get("limit"))
            .and_then(|p| p.parse().ok())
            .unwrap_or(15)
            .min(100); // Max 100 per page
        
        let sort = query
            .get("sort")
            .map(|s| Sort::from_query(s));
        
        Self { page, per_page, sort }
    }
    
    /// Calculate the offset for SQL LIMIT/OFFSET
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.per_page
    }
    
    /// Calculate the limit for SQL LIMIT/OFFSET
    pub fn limit(&self) -> u32 {
        self.per_page
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pagination_calculation() {
        let pagination = Pagination::from(100, 1, 15);
        
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.per_page, 15);
        assert_eq!(pagination.total, 100);
        assert_eq!(pagination.last_page, 7); // ceil(100/15) = 7
        assert_eq!(pagination.from, Some(1));
        assert_eq!(pagination.to, Some(15));
        assert!(pagination.has_more_pages);
    }
    
    #[test]
    fn test_pagination_last_page() {
        let pagination = Pagination::from(100, 7, 15);
        
        assert_eq!(pagination.current_page, 7);
        assert_eq!(pagination.from, Some(91));
        assert_eq!(pagination.to, Some(100));
        assert!(!pagination.has_more_pages);
    }
    
    #[test]
    fn test_pagination_empty() {
        let pagination = Pagination::from(0, 1, 15);
        
        assert_eq!(pagination.last_page, 1);
        assert_eq!(pagination.from, None);
        assert_eq!(pagination.to, None);
        assert!(!pagination.has_more_pages);
    }
    
    #[test]
    fn test_paginated_result() {
        let data = vec![1, 2, 3, 4, 5];
        let result = PaginatedResult::new(data, 100, 1, 5, "/api/posts");
        
        assert_eq!(result.data.len(), 5);
        assert_eq!(result.meta.total, 100);
        assert_eq!(result.meta.last_page, 20);
        assert!(result.links.next.is_some());
        assert!(result.links.prev.is_none());
    }
    
    #[test]
    fn test_sort_parsing() {
        let sort = Sort::from_query("name");
        assert_eq!(sort.column, "name");
        assert_eq!(sort.direction, SortDirection::Asc);
        
        let sort = Sort::from_query("-created_at");
        assert_eq!(sort.column, "created_at");
        assert_eq!(sort.direction, SortDirection::Desc);
    }
    
    #[test]
    fn test_sort_to_sql() {
        let sort = Sort::new("name", SortDirection::Asc);
        assert_eq!(sort.to_sql(), "name ASC");
        
        let sort = Sort::new("created_at", SortDirection::Desc);
        assert_eq!(sort.to_sql(), "created_at DESC");
    }
    
    #[test]
    fn test_pagination_params() {
        let mut query = HashMap::new();
        query.insert("page".to_string(), "3".to_string());
        query.insert("per_page".to_string(), "20".to_string());
        query.insert("sort".to_string(), "-name".to_string());
        
        let params = PaginationParams::from_query(&query);
        
        assert_eq!(params.page, 3);
        assert_eq!(params.per_page, 20);
        assert_eq!(params.offset(), 40);
        assert_eq!(params.limit(), 20);
        assert!(params.sort.is_some());
    }
}
