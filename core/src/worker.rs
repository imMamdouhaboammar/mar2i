use crate::policy::Mode;
use crate::queue::{BatchQueue, Job, JobCompletion};
use crate::runner::{RunOutcome, run_one_with_control};
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

pub struct ProcessGuard<'a> {
    control: &'a Control,
    pid: u32,
}

impl Drop for ProcessGuard<'_> {
    fn drop(&mut self) {
        self.control.unregister_process(self.pid);
    }
}

pub struct Control {
    paused: AtomicBool,
    cancelled: AtomicBool,
    wake: Condvar,
    mutex: Mutex<()>,
    active_processes: Mutex<Vec<u32>>,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            wake: Condvar::new(),
            mutex: Mutex::new(()),
            active_processes: Mutex::new(Vec::new()),
        }
    }
}

#[cfg(unix)]
fn terminate_pid_graceful(pid: u32) {
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
}

#[cfg(not(unix))]
fn terminate_pid_graceful(_pid: u32) {}

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
        let pids = self.active_pids();
        for pid in pids {
            terminate_pid_graceful(pid);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn register_process(&self, pid: u32) -> ProcessGuard<'_> {
        let mut list = self
            .active_processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        list.push(pid);
        ProcessGuard { control: self, pid }
    }

    pub fn unregister_process(&self, pid: u32) {
        let mut list = self
            .active_processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(pos) = list.iter().position(|&p| p == pid) {
            list.swap_remove(pos);
        }
    }

    pub fn active_pids(&self) -> Vec<u32> {
        self.active_processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn wait_until_active(&self) -> bool {
        let mut guard = self
            .mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while self.paused.load(Ordering::SeqCst) && !self.cancelled.load(Ordering::SeqCst) {
            guard = self
                .wake
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    control: &Control,
) -> Result<()> {
    let start = Instant::now();
    let result = check_job_source(&job).and_then(|_| {
        run_one_with_control(
            ffmpeg,
            ffprobe,
            Path::new(&job.source),
            Path::new(&job.relative),
            output,
            mode,
            Some(control),
        )
    });
    let elapsed = start.elapsed().as_millis().min(i64::MAX as u128) as u64;
    match result {
        Ok(RunOutcome::Accepted {
            output,
            after,
            quality,
            ..
        }) => queue.finish(
            job.id,
            JobCompletion {
                status: "completed",
                output: Some(&output),
                bytes: Some(after),
                quality: Some(&quality),
                reason: None,
                elapsed_ms: elapsed,
            },
        ),
        Ok(RunOutcome::Copied {
            output,
            bytes,
            reason,
        }) => queue.finish(
            job.id,
            JobCompletion {
                status: "skipped",
                output: Some(&output),
                bytes: Some(bytes),
                quality: Some("byte-identical copy"),
                reason: Some(&reason),
                elapsed_ms: elapsed,
            },
        ),
        Ok(RunOutcome::Skipped { reason } | RunOutcome::Rejected { reason }) => queue.finish(
            job.id,
            JobCompletion {
                status: "skipped",
                output: None,
                bytes: None,
                quality: None,
                reason: Some(&reason),
                elapsed_ms: elapsed,
            },
        ),
        Err(error) => {
            if control.is_cancelled() {
                queue.finish(
                    job.id,
                    JobCompletion {
                        status: "interrupted",
                        output: None,
                        bytes: None,
                        quality: None,
                        reason: Some("batch cancelled by user during execution"),
                        elapsed_ms: elapsed,
                    },
                )
            } else {
                queue.finish(
                    job.id,
                    JobCompletion {
                        status: "failed",
                        output: None,
                        bytes: None,
                        quality: None,
                        reason: Some(&format!("{error:#}")),
                        elapsed_ms: elapsed,
                    },
                )
            }
        }
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
    // Prevent OS idle sleep during batch encoding
    let _keep_awake = crate::keep_awake::KeepAwake::acquire();
    let cores = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut system = System::new();
    system.refresh_memory();
    let worker_count = worker_limit(profile, cores, system.available_memory());
    thread::scope(|scope| -> Result<()> {
        let handles = (0..worker_count)
            .map(|_| {
                scope.spawn(|| -> Result<()> {
                    loop {
                        if !control.wait_until_active() {
                            break;
                        }
                        let Some(job) = queue.claim_next()? else {
                            break;
                        };
                        process_next(queue, ffmpeg, ffprobe, output, mode, job, control)?;
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("worker panicked"))??;
        }
        Ok(())
    })
}
