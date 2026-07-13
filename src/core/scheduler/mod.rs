// ============================================================
// Scheduler Module - المجدول
// ============================================================
// Cron-like task scheduling for periodic jobs.
// Supports cron expressions and simple intervals.
//
// جدولة المهام الدورية مثل cron.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Task type
/// نوع المهمة
pub type ScheduledTask = Arc<dyn Fn() -> crate::NoorResult<()> + Send + Sync>;

/// Schedule frequency
/// تكرار الجدولة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// Every N seconds
    EverySeconds(u64),
    /// Every N minutes
    EveryMinutes(u64),
    /// Every N hours
    EveryHours(u64),
    /// Daily at specific hour:minute
    Daily { hour: u32, minute: u32 },
    /// Weekly at specific day:hour:minute
    Weekly { day: u32, hour: u32, minute: u32 },
    /// Monthly at specific day:hour:minute
    Monthly { day: u32, hour: u32, minute: u32 },
    /// Cron expression (simplified: "minute hour day month weekday")
    Cron(String),
}

/// A scheduled job
/// مهمة مجدولة
struct ScheduledJob {
    name: String,
    schedule: Schedule,
    task: ScheduledTask,
    last_run: Option<i64>,
    next_run: i64,
    enabled: bool,
}

/// Task scheduler
/// مجدول المهام
pub struct Scheduler {
    jobs: Arc<RwLock<Vec<ScheduledJob>>>,
    running: Arc<RwLock<bool>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Schedule a task
    /// جدولة مهمة
    pub fn schedule(&self, name: &str, schedule: Schedule, task: ScheduledTask) -> &Self {
        let next_run = Self::calculate_next_run(&schedule, None);
        
        self.jobs.write().push(ScheduledJob {
            name: name.to_string(),
            schedule,
            task,
            last_run: None,
            next_run,
            enabled: true,
        });
        
        self
    }
    
    /// Run a task every minute
    pub fn every_minute(&self, name: &str, task: ScheduledTask) -> &Self {
        self.schedule(name, Schedule::EveryMinutes(1), task)
    }
    
    /// Run a task every N minutes
    pub fn every_n_minutes(&self, name: &str, minutes: u64, task: ScheduledTask) -> &Self {
        self.schedule(name, Schedule::EveryMinutes(minutes), task)
    }
    
    /// Run a task every hour
    pub fn hourly(&self, name: &str, task: ScheduledTask) -> &Self {
        self.schedule(name, Schedule::EveryHours(1), task)
    }
    
    /// Run a task daily at specific time
    pub fn daily_at(&self, name: &str, hour: u32, minute: u32, task: ScheduledTask) -> &Self {
        self.schedule(name, Schedule::Daily { hour, minute }, task)
    }
    
    /// Run a task weekly
    pub fn weekly(&self, name: &str, day: u32, hour: u32, minute: u32, task: ScheduledTask) -> &Self {
        self.schedule(name, Schedule::Weekly { day, hour, minute }, task)
    }
    
    /// Disable a scheduled task
    pub fn disable(&self, name: &str) {
        let mut jobs = self.jobs.write();
        if let Some(job) = jobs.iter_mut().find(|j| j.name == name) {
            job.enabled = false;
        }
    }
    
    /// Enable a scheduled task
    pub fn enable(&self, name: &str) {
        let mut jobs = self.jobs.write();
        if let Some(job) = jobs.iter_mut().find(|j| j.name == name) {
            job.enabled = true;
        }
    }
    
    /// Remove a scheduled task
    pub fn remove(&self, name: &str) {
        self.jobs.write().retain(|j| j.name != name);
    }
    
    /// Check for and run due tasks
    /// فحص وتشغيل المهام المستحقة
    pub fn run_due(&self) -> crate::NoorResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let mut run_count = 0;
        
        let mut jobs = self.jobs.write();
        
        for job in jobs.iter_mut() {
            if !job.enabled {
                continue;
            }
            
            if now >= job.next_run {
                // Run the task
                if let Err(e) = (job.task)() {
                    tracing::error!("Scheduled task '{}' failed: {}", job.name, e);
                } else {
                    tracing::debug!("Scheduled task '{}' completed", job.name);
                }
                
                job.last_run = Some(now);
                job.next_run = Self::calculate_next_run(&job.schedule, Some(now));
                run_count += 1;
            }
        }
        
        Ok(run_count)
    }
    
    /// Start the scheduler (blocks current thread)
    /// تشغيل المجدول (يحجب الخيط الحالي)
    pub fn start(&self, check_interval_secs: u64) {
        *self.running.write() = true;
        
        let running = self.running.clone();
        let jobs = self.jobs.clone();
        
        std::thread::spawn(move || {
            while *running.read() {
                let now = chrono::Utc::now().timestamp();
                
                let mut jobs = jobs.write();
                for job in jobs.iter_mut() {
                    if !job.enabled {
                        continue;
                    }
                    
                    if now >= job.next_run {
                        if let Err(e) = (job.task)() {
                            tracing::error!("Scheduled task '{}' failed: {}", job.name, e);
                        }
                        
                        job.last_run = Some(now);
                        job.next_run = Self::calculate_next_run(&job.schedule, Some(now));
                    }
                }
                drop(jobs);
                
                std::thread::sleep(Duration::from_secs(check_interval_secs));
            }
        });
    }
    
    /// Stop the scheduler
    pub fn stop(&self) {
        *self.running.write() = false;
    }
    
    /// Calculate the next run timestamp
    pub fn calculate_next_run(schedule: &Schedule, last_run: Option<i64>) -> i64 {
        let now = chrono::Utc::now();
        let base = last_run
            .map(chrono::DateTime::from_timestamp_secs)
            .flatten()
            .unwrap_or(now);
        
        match schedule {
            Schedule::EverySeconds(secs) => base.timestamp() + *secs as i64,
            Schedule::EveryMinutes(mins) => base.timestamp() + (mins * 60) as i64,
            Schedule::EveryHours(hours) => base.timestamp() + (hours * 3600) as i64,
            Schedule::Daily { hour, minute } => {
                let mut next = base.date_naive()
                    .and_hms_opt(*hour, *minute, 0)
                    .unwrap();
                if next <= base.naive_utc() {
                    next = next + chrono::Duration::days(1);
                }
                next.and_utc().timestamp()
            }
            Schedule::Weekly { day, hour, minute } => {
                let mut next = base.date_naive()
                    .and_hms_opt(*hour, *minute, 0)
                    .unwrap();
                let current_weekday = base.weekday().num_days_from_monday();
                let target_weekday = *day;
                
                let days_ahead = if target_weekday > current_weekday {
                    target_weekday - current_weekday
                } else if target_weekday < current_weekday {
                    7 - (current_weekday - target_weekday)
                } else {
                    if next <= base.naive_utc() { 7 } else { 0 }
                };
                
                next = next + chrono::Duration::days(days_ahead as i64);
                next.and_utc().timestamp()
            }
            Schedule::Monthly { day, hour, minute } => {
                let base_date = base.date_naive();
                let next_month = if base_date.month() == 12 {
                    base_date.with_month(1).unwrap()
                } else {
                    base_date.with_month(base_date.month() + 1).unwrap()
                };
                
                let next = next_month
                    .with_day(*day)
                    .unwrap()
                    .and_hms_opt(*hour, *minute, 0)
                    .unwrap();
                next.and_utc().timestamp()
            }
            Schedule::Cron(_expr) => {
                // Simplified - in production, use a cron parser library
                base.timestamp() + 60 // Default to 1 minute
            }
        }
    }
    
    /// List all scheduled jobs
    pub fn list_jobs(&self) -> Vec<(String, bool, Option<i64>, i64)> {
        self.jobs
            .read()
            .iter()
            .map(|j| (j.name.clone(), j.enabled, j.last_run, j.next_run))
            .collect()
    }
}

use chrono::{Datelike, Timelike};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_schedule_every_minute() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        scheduler.every_minute("test_task", Arc::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        
        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].0, "test_task");
    }
    
    #[test]
    fn test_calculate_next_run() {
        let schedule = Schedule::EveryMinutes(5);
        let next = Scheduler::calculate_next_run(&schedule, None);
        let now = chrono::Utc::now().timestamp();
        assert!(next > now);
    }
}
