// ============================================================
// Connection Pool - تجمع الاتصالات
// ============================================================
// Manages a pool of database connections for efficient reuse.
// Reduces connection overhead and limits max connections.
//
// يدير تجمع اتصالات قاعدة البيانات لإعادة الاستخدام.
// ============================================================

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// Connection trait
pub trait Connection: Send + 'static {
    /// Check if the connection is still alive
    fn is_alive(&self) -> bool;
    
    /// Close the connection
    fn close(&self);
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
        }
    }
}

/// Pooled connection wrapper
pub struct PooledConnection<C: Connection> {
    connection: Option<C>,
    pool: Arc<ConnectionPoolInner<C>>,
    created_at: Instant,
}

impl<C: Connection> PooledConnection<C> {
    /// Get a reference to the connection
    pub fn connection(&self) -> Option<&C> {
        self.connection.as_ref()
    }
    
    /// Get a mutable reference to the connection
    pub fn connection_mut(&mut self) -> Option<&mut C> {
        self.connection.as_mut()
    }
}

impl<C: Connection> Drop for PooledConnection<C> {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            // Return connection to pool if still alive
            if conn.is_alive() {
                let mut idle = self.pool.idle.lock();
                idle.push_back((conn, Instant::now()));
            }
            
            self.pool.active.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Connection pool inner state
struct ConnectionPoolInner<C: Connection> {
    idle: Mutex<VecDeque<(C, Instant)>>,
    active: AtomicU32,
    total_created: AtomicU32,
    closed: AtomicBool,
    config: PoolConfig,
    factory: Box<dyn Fn() -> C + Send + Sync>,
}

/// Connection pool
pub struct ConnectionPool<C: Connection> {
    inner: Arc<ConnectionPoolInner<C>>,
}

impl<C: Connection> ConnectionPool<C> {
    /// Create a new connection pool
    pub fn new<F>(config: PoolConfig, factory: F) -> Self
    where
        F: Fn() -> C + Send + Sync + 'static,
    {
        let inner = Arc::new(ConnectionPoolInner {
            idle: Mutex::new(VecDeque::new()),
            active: AtomicU32::new(0),
            total_created: AtomicU32::new(0),
            closed: AtomicBool::new(false),
            config: config.clone(),
            factory: Box::new(factory),
        });
        
        let pool = Self { inner: Arc::clone(&inner) };
        
        // Pre-create minimum connections
        for _ in 0..config.min_connections {
            let conn = (inner.factory)();
            inner.idle.lock().push_back((conn, Instant::now()));
            inner.total_created.fetch_add(1, Ordering::SeqCst);
        }
        
        pool
    }
    
    /// Acquire a connection from the pool
    pub fn acquire(&self) -> crate::NoorResult<PooledConnection<C>> {
        if self.inner.closed.load(Ordering::SeqCst) {
            return Err(crate::NoorError::Internal("Pool is closed".to_string()));
        }
        
        // Try to get an idle connection
        loop {
            let mut idle = self.inner.idle.lock();
            
            if let Some((conn, created_at)) = idle.pop_front() {
                // Check if connection is still alive and not too old
                if conn.is_alive() && created_at.elapsed() < self.inner.config.max_lifetime {
                    drop(idle);
                    
                    self.inner.active.fetch_add(1, Ordering::SeqCst);
                    
                    return Ok(PooledConnection {
                        connection: Some(conn),
                        pool: Arc::clone(&self.inner),
                        created_at,
                    });
                } else {
                    // Connection is dead or too old, close it
                    drop(idle);
                    conn.close();
                    
                    // Try again
                    continue;
                }
            }
            
            drop(idle);
            
            // No idle connections, try to create a new one
            let active = self.inner.active.load(Ordering::SeqCst) as usize;
            
            if active < self.inner.config.max_connections {
                let conn = (self.inner.factory)();
                self.inner.total_created.fetch_add(1, Ordering::SeqCst);
                self.inner.active.fetch_add(1, Ordering::SeqCst);
                
                return Ok(PooledConnection {
                    connection: Some(conn),
                    pool: Arc::clone(&self.inner),
                    created_at: Instant::now(),
                });
            }
            
            // Pool is exhausted, wait and retry
            std::thread::sleep(Duration::from_millis(10));
            
            // Check timeout
            // In a real implementation, we'd track wait time
        }
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let idle_count = self.inner.idle.lock().len();
        let active_count = self.inner.active.load(Ordering::SeqCst) as usize;
        let total_created = self.inner.total_created.load(Ordering::SeqCst) as usize;
        
        PoolStats {
            idle: idle_count,
            active: active_count,
            total_created,
            max: self.inner.config.max_connections,
        }
    }
    
    /// Close the pool and all connections
    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::SeqCst);
        
        let mut idle = self.inner.idle.lock();
        
        while let Some((conn, _)) = idle.pop_front() {
            conn.close();
        }
    }
    
    /// Clean up idle connections that have been idle too long
    pub fn clean_idle(&self) -> usize {
        let mut cleaned = 0;
        let mut idle = self.inner.idle.lock();
        
        let now = Instant::now();
        let retain = idle.retain(|(_, created_at)| {
            if now.duration_since(*created_at) > self.inner.config.idle_timeout {
                cleaned += 1;
                false
            } else {
                true
            }
        });
        
        let _ = retain;
        cleaned
    }
}

impl<C: Connection> Drop for ConnectionPool<C> {
    fn drop(&mut self) {
        self.close();
    }
}

/// Pool statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    pub idle: usize,
    pub active: usize,
    pub total_created: usize,
    pub max: usize,
}

impl PoolStats {
    /// Get the total number of connections (idle + active)
    pub fn total(&self) -> usize {
        self.idle + self.active
    }
    
    /// Get the utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.max == 0 {
            0.0
        } else {
            (self.active as f64 / self.max as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    
    struct TestConnection {
        id: u32,
        alive: bool,
    }
    
    impl Connection for TestConnection {
        fn is_alive(&self) -> bool {
            self.alive
        }
        
        fn close(&self) {
            // Mark as closed
        }
    }
    
    static CONN_ID: AtomicU32 = AtomicU32::new(0);
    
    #[test]
    fn test_pool_creation() {
        let config = PoolConfig {
            min_connections: 2,
            max_connections: 5,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config, || TestConnection {
            id: CONN_ID.fetch_add(1, Ordering::SeqCst),
            alive: true,
        });
        
        let stats = pool.stats();
        assert_eq!(stats.idle, 2);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.total(), 2);
    }
    
    #[test]
    fn test_acquire_release() {
        let config = PoolConfig {
            min_connections: 1,
            max_connections: 3,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config, || TestConnection {
            id: CONN_ID.fetch_add(1, Ordering::SeqCst),
            alive: true,
        });
        
        // Acquire a connection
        let conn1 = pool.acquire().unwrap();
        assert_eq!(pool.stats().active, 1);
        assert_eq!(pool.stats().idle, 0);
        
        // Release it (drop)
        drop(conn1);
        
        // Should be back in the pool
        assert_eq!(pool.stats().active, 0);
        assert_eq!(pool.stats().idle, 1);
    }
    
    #[test]
    fn test_max_connections() {
        let config = PoolConfig {
            min_connections: 1,
            max_connections: 2,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config, || TestConnection {
            id: CONN_ID.fetch_add(1, Ordering::SeqCst),
            alive: true,
        });
        
        let _conn1 = pool.acquire().unwrap();
        let _conn2 = pool.acquire().unwrap();
        
        assert_eq!(pool.stats().active, 2);
        assert_eq!(pool.stats().total(), 2);
        assert_eq!(pool.stats().max, 2);
    }
    
    #[test]
    fn test_utilization() {
        let config = PoolConfig {
            min_connections: 0,
            max_connections: 10,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config, || TestConnection {
            id: CONN_ID.fetch_add(1, Ordering::SeqCst),
            alive: true,
        });
        
        let _conn1 = pool.acquire().unwrap();
        let _conn2 = pool.acquire().unwrap();
        let _conn3 = pool.acquire().unwrap();
        
        let stats = pool.stats();
        assert_eq!(stats.active, 3);
        assert_eq!(stats.utilization(), 30.0);
    }
}
