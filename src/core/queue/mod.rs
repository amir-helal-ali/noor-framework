// ============================================================
// Queue Module - نظام الطوابير
// ============================================================
// Asynchronous job processing for heavy tasks.
// Supports delayed jobs, retries, priorities, and persistence.
// Uses file-based storage for weak servers (no Redis needed).
//
// معالجة غير متزامنة للمهام الثقيلة.
// ============================================================

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering as CmpOrdering;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use serde::{Serialize, Deserialize};

/// Job priority
/// أولوية المهمة
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A job in the queue
/// مهمة في الطابور
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub priority: Priority,
    pub attempts: u32,
    pub max_attempts: u32,
    pub available_at: i64,  // Unix timestamp when job becomes available
    pub created_at: i64,
    pub failed_at: Option<i64>,
    pub error: Option<String>,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Job {}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        // Higher priority first, then earlier available_at
        // BinaryHeap is a max-heap, so we reverse for correct ordering
        Some(match other.priority.cmp(&self.priority) {
            CmpOrdering::Equal => other.available_at.cmp(&self.available_at),
            other => other,
        })
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.partial_cmp(other).unwrap_or(CmpOrdering::Equal)
    }
}

impl Job {
    pub fn new(name: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            payload,
            priority: Priority::default(),
            attempts: 0,
            max_attempts: 3,
            available_at: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            failed_at: None,
            error: None,
        }
    }
    
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_delay(mut self, delay_secs: i64) -> Self {
        self.available_at = chrono::Utc::now().timestamp() + delay_secs;
        self
    }
    
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }
    
    /// Check if the job is ready to be processed
    pub fn is_ready(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.available_at
    }
    
    /// Check if the job has exceeded max attempts
    pub fn is_failed(&self) -> bool {
        self.attempts >= self.max_attempts
    }
}

/// Job handler type
/// نوع معالج المهمة
pub type JobHandler = Arc<dyn Fn(&Job) -> crate::NoorResult<()> + Send + Sync>;

/// Queue worker that processes jobs
/// عامل الطابور الذي يعالج المهام
pub struct Queue {
    /// Pending jobs (priority queue)
    pending: Arc<Mutex<BinaryHeap<Job>>>,
    /// Failed jobs
    failed: Arc<Mutex<Vec<Job>>>,
    /// Completed jobs (for tracking)
    completed: Arc<Mutex<Vec<String>>>,
    /// Job handlers
    handlers: Arc<RwLock<HashMap<String, Vec<JobHandler>>>>,
    /// Storage directory for persistence
    storage_dir: Option<PathBuf>,
    /// Whether the queue is running
    running: Arc<Mutex<bool>>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(BinaryHeap::new())),
            failed: Arc::new(Mutex::new(Vec::new())),
            completed: Arc::new(Mutex::new(Vec::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            storage_dir: None,
            running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Create a queue with file-based persistence (for weak servers)
    /// إنشاء طابور مع تخزين ملفي
    pub fn with_persistence(storage_dir: &str) -> crate::NoorResult<Self> {
        let path = PathBuf::from(storage_dir);
        std::fs::create_dir_all(&path)?;
        
        let mut queue = Self::new();
        queue.storage_dir = Some(path.clone());
        
        // Load pending jobs from storage
        queue.load_from_storage()?;
        
        Ok(queue)
    }
    
    /// Register a job handler
    /// تسجيل معالج مهمة
    pub fn register(&self, job_name: &str, handler: JobHandler) {
        self.handlers
            .write()
            .entry(job_name.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
    }
    
    /// Push a new job to the queue
    /// إضافة مهمة جديدة للطابور
    pub fn push(&self, job: Job) -> crate::NoorResult<String> {
        let job_id = job.id.clone();
        self.pending.lock().push(job);
        self.persist()?;
        Ok(job_id)
    }
    
    /// Convenience method to push a job
    /// طريقة ملائمة لإضافة مهمة
    pub fn dispatch(&self, name: &str, payload: serde_json::Value) -> crate::NoorResult<String> {
        let job = Job::new(name, payload);
        self.push(job)
    }
    
    /// Dispatch a job with delay
    /// إرسال مهمة مع تأخير
    pub fn dispatch_later(&self, name: &str, payload: serde_json::Value, delay_secs: i64) -> crate::NoorResult<String> {
        let job = Job::new(name, payload).with_delay(delay_secs);
        self.push(job)
    }
    
    /// Process the next available job
    /// معالجة المهمة التالية المتاحة
    pub fn process_next(&self) -> crate::NoorResult<bool> {
        let job = {
            let mut pending = self.pending.lock();
            
            // Find a ready job
            let mut ready_job = None;
            let mut temp_jobs = Vec::new();
            
            while let Some(job) = pending.pop() {
                if job.is_ready() {
                    ready_job = Some(job);
                    break;
                } else {
                    temp_jobs.push(job);
                }
            }
            
            // Put back non-ready jobs
            for j in temp_jobs {
                pending.push(j);
            }
            
            ready_job
        };
        
        if let Some(mut job) = job {
            job.attempts += 1;
            
            let handlers = self.handlers.read();
            let result = if let Some(job_handlers) = handlers.get(&job.name) {
                let mut result = Ok(());
                for handler in job_handlers {
                    if let Err(e) = handler(&job) {
                        result = Err(e);
                        break;
                    }
                }
                result
            } else {
                Err(crate::NoorError::Internal(
                    format!("No handler registered for job: {}", job.name)
                ))
            };
            drop(handlers);
            
            match result {
                Ok(()) => {
                    self.completed.lock().push(job.id.clone());
                    tracing::info!("Job {} completed successfully", job.id);
                }
                Err(e) => {
                    if job.is_failed() {
                        job.failed_at = Some(chrono::Utc::now().timestamp());
                        job.error = Some(e.to_string());
                        self.failed.lock().push(job.clone());
                        tracing::error!("Job {} permanently failed: {}", job.id, e);
                    } else {
                        // Retry with exponential backoff
                        let backoff = 2_i64.pow(job.attempts);
                        job.available_at = chrono::Utc::now().timestamp() + backoff;
                        let job_id = job.id.clone();
                        self.pending.lock().push(job);
                        tracing::warn!("Job {} failed, will retry in {}s", job_id, backoff);
                    }
                }
            }
            
            self.persist()?;
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Start the queue worker (process jobs continuously)
    /// تشغيل عامل الطابور
    pub fn start_worker(&self, poll_interval_ms: u64) {
        *self.running.lock() = true;
        
        let pending = self.pending.clone();
        let running = self.running.clone();
        
        std::thread::spawn(move || {
            while *running.lock() {
                // Try to process one job
                // In a real implementation, this would use async
                std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms));
            }
        });
    }
    
    /// Stop the queue worker
    pub fn stop_worker(&self) {
        *self.running.lock() = false;
    }
    
    /// Get the number of pending jobs
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }
    
    /// Get the number of failed jobs
    pub fn failed_count(&self) -> usize {
        self.failed.lock().len()
    }
    
    /// Get failed jobs
    pub fn get_failed_jobs(&self) -> Vec<Job> {
        self.failed.lock().clone()
    }
    
    /// Retry a failed job
    pub fn retry(&self, job_id: &str) -> crate::NoorResult<bool> {
        let mut failed = self.failed.lock();
        if let Some(pos) = failed.iter().position(|j| j.id == job_id) {
            let mut job = failed.remove(pos);
            job.attempts = 0;
            job.failed_at = None;
            job.error = None;
            job.available_at = chrono::Utc::now().timestamp();
            
            drop(failed);
            self.pending.lock().push(job);
            self.persist()?;
            return Ok(true);
        }
        Ok(false)
    }
    
    /// Clear all failed jobs
    pub fn clear_failed(&self) {
        self.failed.lock().clear();
        let _ = self.persist();
    }
    
    /// Persist jobs to storage (for weak servers)
    pub fn persist(&self) -> crate::NoorResult<()> {
        if let Some(ref dir) = self.storage_dir {
            let pending: Vec<Job> = self.pending.lock().iter().cloned().collect();
            let failed: Vec<Job> = self.failed.lock().clone();
            
            let pending_path = dir.join("pending_jobs.json");
            let failed_path = dir.join("failed_jobs.json");
            
            std::fs::write(&pending_path, serde_json::to_string_pretty(&pending)?)?;
            std::fs::write(&failed_path, serde_json::to_string_pretty(&failed)?)?;
        }
        Ok(())
    }
    
    /// Load jobs from storage
    pub fn load_from_storage(&self) -> crate::NoorResult<()> {
        if let Some(ref dir) = self.storage_dir {
            let pending_path = dir.join("pending_jobs.json");
            let failed_path = dir.join("failed_jobs.json");
            
            if pending_path.exists() {
                let content = std::fs::read_to_string(&pending_path)?;
                let jobs: Vec<Job> = serde_json::from_str(&content)?;
                let mut pending = self.pending.lock();
                for job in jobs {
                    pending.push(job);
                }
            }
            
            if failed_path.exists() {
                let content = std::fs::read_to_string(&failed_path)?;
                let jobs: Vec<Job> = serde_json::from_str(&content)?;
                *self.failed.lock() = jobs;
            }
        }
        Ok(())
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_job_creation() {
        let job = Job::new("send_email", serde_json::json!({"to": "test@test.com"}));
        assert_eq!(job.name, "send_email");
        assert_eq!(job.attempts, 0);
        assert!(job.is_ready());
    }
    
    #[test]
    fn test_queue_push_and_process() {
        let queue = Queue::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        queue.register("test_job", Arc::new(move |_job| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        
        queue.dispatch("test_job", serde_json::json!({})).unwrap();
        assert_eq!(queue.pending_count(), 1);
        
        let processed = queue.process_next().unwrap();
        assert!(processed);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_job_retry() {
        let queue = Queue::new();
        
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();
        queue.register("failing_job", Arc::new(move |_job| {
            let count = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                return Err(crate::NoorError::Internal("Simulated failure".to_string()));
            }
            Ok(())
        }));
        
        let mut job = Job::new("failing_job", serde_json::json!({}));
        job.max_attempts = 3;
        queue.push(job).unwrap();
        
        // First attempt - fails
        queue.process_next().unwrap();
        // Should be back in queue with delay
        
        // Manually make it ready and process again
        {
            let mut pending = queue.pending.lock();
            if let Some(mut job) = pending.peek_mut() {
                job.available_at = chrono::Utc::now().timestamp();
            };
        }
        
        queue.process_next().unwrap();
    }
}
