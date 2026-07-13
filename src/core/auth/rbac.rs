// ============================================================
// RBAC - Role-Based Access Control
// التحكم في الوصول القائم على الأدوار
// ============================================================
// Hierarchical role and permission system with:
// - Roles (admin, editor, user, etc.)
// - Permissions (create, read, update, delete)
// - Role inheritance
// - Resource-level permissions
//
// نظام أدوار وصلاحيات هرمي.
// ============================================================

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;

/// RBAC manager
/// مدير RBAC
pub struct Rbac {
    /// Role -> Permissions mapping
    roles: Arc<RwLock<HashMap<String, Role>>>,
    /// User -> Roles mapping
    user_roles: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

/// A role definition
/// تعريف دور
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub permissions: HashSet<String>,
    pub parent: Option<String>, // For role inheritance
}

impl Rbac {
    pub fn new() -> Self {
        let mut roles = HashMap::new();
        
        // Default roles
        roles.insert("super_admin".to_string(), Role {
            name: "super_admin".to_string(),
            description: "Full access to everything".to_string(),
            permissions: vec!["*".to_string()].into_iter().collect(),
            parent: None,
        });
        
        roles.insert("admin".to_string(), Role {
            name: "admin".to_string(),
            description: "Administrator with most permissions".to_string(),
            permissions: vec![
                "users.read", "users.write", "users.delete",
                "posts.read", "posts.write", "posts.delete",
                "settings.read", "settings.write",
            ].into_iter().map(|s| s.to_string()).collect(),
            parent: None,
        });
        
        roles.insert("editor".to_string(), Role {
            name: "editor".to_string(),
            description: "Can manage content".to_string(),
            permissions: vec![
                "posts.read", "posts.write",
                "users.read",
            ].into_iter().map(|s| s.to_string()).collect(),
            parent: None,
        });
        
        roles.insert("user".to_string(), Role {
            name: "user".to_string(),
            description: "Regular user".to_string(),
            permissions: vec![
                "posts.read",
            ].into_iter().map(|s| s.to_string()).collect(),
            parent: None,
        });
        
        Self {
            roles: Arc::new(RwLock::new(roles)),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a custom role
    /// إضافة دور مخصص
    pub fn add_role(&self, role: Role) {
        self.roles.write().insert(role.name.clone(), role);
    }
    
    /// Assign a role to a user
    /// تعيين دور لمستخدم
    pub fn assign_role(&self, user_id: &str, role: &str) {
        self.user_roles
            .write()
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(role.to_string());
    }
    
    /// Remove a role from a user
    /// إزالة دور من مستخدم
    pub fn revoke_role(&self, user_id: &str, role: &str) {
        if let Some(roles) = self.user_roles.write().get_mut(user_id) {
            roles.remove(role);
        }
    }
    
    /// Get all roles for a user
    /// الحصول على جميع أدوار المستخدم
    pub fn get_user_roles(&self, user_id: &str) -> HashSet<String> {
        self.user_roles.read().get(user_id).cloned().unwrap_or_default()
    }
    
    /// Get all permissions for a user (including inherited)
    /// الحصول على جميع صلاحيات المستخدم
    pub fn get_user_permissions(&self, user_id: &str) -> HashSet<String> {
        let mut permissions = HashSet::new();
        let roles = self.get_user_roles(user_id);
        
        for role_name in &roles {
            self.collect_permissions(role_name, &mut permissions);
        }
        
        permissions
    }
    
    /// Recursively collect permissions (with inheritance)
    /// جمع الصلاحيات بشكل متكرر (مع الوراثة)
    fn collect_permissions(&self, role_name: &str, permissions: &mut HashSet<String>) {
        let roles = self.roles.read();
        if let Some(role) = roles.get(role_name) {
            for perm in &role.permissions {
                permissions.insert(perm.clone());
            }
            
            if let Some(parent) = role.parent.clone() {
                drop(roles);
                self.collect_permissions(&parent, permissions);
            }
        }
    }
    
    /// Check if a user has a permission
    /// فحص إذا كان المستخدم يملك صلاحية
    pub fn can(&self, user_id: &str, permission: &str) -> bool {
        let permissions = self.get_user_permissions(user_id);
        
        // Super admin check
        if permissions.contains("*") {
            return true;
        }
        
        // Direct permission check
        if permissions.contains(permission) {
            return true;
        }
        
        // Wildcard check (e.g., "posts.*" matches "posts.read")
        let parts: Vec<&str> = permission.split('.').collect();
        if parts.len() > 1 {
            let wildcard = format!("{}.*", parts[0]);
            if permissions.contains(&wildcard) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if a user has any of the given permissions
    /// فحص إذا كان المستخدم يملك أي من الصلاحيات
    pub fn can_any(&self, user_id: &str, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.can(user_id, p))
    }
    
    /// Check if a user has all of the given permissions
    /// فحص إذا كان المستخدم يملك جميع الصلاحيات
    pub fn can_all(&self, user_id: &str, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.can(user_id, p))
    }
    
    /// Check if a user has a role
    /// فحص إذا كان المستخدم يملك دور
    pub fn has_role(&self, user_id: &str, role: &str) -> bool {
        self.get_user_roles(user_id).contains(role)
    }
}

impl Default for Rbac {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rbac() {
        let rbac = Rbac::new();
        
        rbac.assign_role("user1", "admin");
        
        assert!(rbac.can("user1", "users.read"));
        assert!(rbac.can("user1", "posts.write"));
        assert!(!rbac.can("user1", "nonexistent.permission"));
    }
    
    #[test]
    fn test_super_admin() {
        let rbac = Rbac::new();
        
        rbac.assign_role("super", "super_admin");
        
        assert!(rbac.can("super", "anything"));
        assert!(rbac.can("super", "everything.here"));
    }
}
