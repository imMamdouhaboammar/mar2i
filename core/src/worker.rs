use crate::policy::Mode;
use crate::queue::{BatchQueue, Job};
use crate::runner::{RunOutcome, run_one};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{Instant, UNIX_EPOCH};
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceProfile {
    Low,
    Balanced,
    Maximum,
}

pub struct Control {
    paused: AtomicBool,
    cancelled: AtomicBool,
    wake: Condvar,
    mutex: Mutex<()>,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            wake: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }
}

impl Control {
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        self.wake.notify_all();
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn wait_until_active(&self) -> bool {
        let mut guard = self.mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while self.paused.load(Ordering::SeqCst) && !self.cancelled.load(Ordering::SeqCst) {
            guard = self.wake.wait(guard).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        !self.cancelled.load(Ordering::SeqCst)
    }
}

// Conservative initial budget: at most 2 FFmpeg threads per job, and at least 1.5 GiB
// of currently available RAM reserved per job. This is a bound, not a benchmark optimum.
pub fn worker_limit(profile: ResourceProfile, cores: usize, available_memory_bytes: u64) -> usize {
    let cpu_budget = cores.max(1).div_ceil(2).max(1);
    let per_job_budget = 1536u64 * 1024 * 1024;
    let memory_budget = (available_memory_bytes / per_job_budget).max(1) as usize;
    let preferred = match profile {
        ResourceProfile::Low => 1,
        ResourceProfile::Balanced => 2,
        ResourceProfile::Maximum => 4,
    };
    preferred.min(cpu_budget).min(memory_budget).max(1)
}

pub fn modified_ns(path: &Path) -> Result<i64> {
    let modified = fs::symlink_metadata(path)?.modified()?;
    let since_epoch = modified.duration_since(UNIX_EPOCH)?;
    i64::try_from(since_epoch.as_nanos()).context("source timestamp out of range")
}

fn check_job_source(job: &Job) -> Result<()> {
    let source = Path::new(&job.source);
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!("source changed type or became a symbolic link");
    }
    if metadata.len() != job.input_bytes || modified_ns(source)? != job.modified_ns {
        bail!("source changed since discovery; job not processed");
    }
    Ok(())
}

fn process_next(
    queue: &BatchQueue,
    ffmpeg: &Path,
    ffprobe: &Path,
    output: &Path,
    mode: Mode,
    job: Job,
) -> Result<()> {
    let start = Instant::now();
    let result = check_job_source(&job).and_then(|_| run_one(
        ffmpeg, ffprobe, Path::new(&job.source), Path::new(&job.relative), output, mode
    ));
    let elapsed = start.elapsed().as_millis().min(i64::MAX as u128) as u64;
    match result {
        Ok(RunOutcome::Accepted { output, after, quality, .. }) => {
            queue.finish(job.id, "completed", Some(&output), Some(after), Some(&quality), None, elapsed)
        }
        Ok(RunOutcome::Copied { output, bytes, reason }) => {
            queue.finish(job.id, "skipped", Some(&output), Some(bytes), Some("byte-identical copy"), Some(&reason), elapsed)
        }
        Ok(RunOutcome::Skipped { reason } | RunOutcome::Rejected { reason }) => {
            queue.finish(job.id, "skipped", None, None, None, Some(&reason), elapsed)
        }
        Err(error) => queue.finish(job.id, "failed", None, None, None, Some(&format!("{error:#}")), elapsed),
    }
}

pub fn run_batch(
    queue: &BatchQueue,
    ffmpeg: &Path,
    ffprobe: &Path,
    output: &Path,
    mode: Mode,
    profile: ResourceProfile,
    control: &Control,
) -> Result<()> {
    if !control.wait_until_active() {
        return Ok(());
    }
    let cores = thread::available_parallelism().map(|value| value.get()).unwrap_or(1);
    let mut system = System::new();
    system.refresh_memory();
    let worker_count = worker_limit(profile, cores, system.available_memory());
    thread::scope(|scope| -> Result<()> {
        let handles = (0..worker_count).map(|_| scope.spawn(|| -> Result<()> {
            loop {
                if !control.wait_until_active() {
                    break;
                }
                let Some(job) = queue.claim_next()? else { break };
                process_next(queue, ffmpeg, ffprobe, output, mode, job)?;
            }
            Ok(())
        })).collect::<Vec<_>>();
        for handle in handles {
            handle.join().map_err(|_| anyhow::anyhow!("worker panicked"))??;
        }
        Ok(())
    })
}
