use crate::policy::Mode;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: i64,
    pub relative: String,
    pub source: String,
    pub input_bytes: u64,
    pub modified_ns: i64,
    pub status: String,
    pub output: Option<String>,
    pub output_bytes: Option<u64>,
    pub reason: Option<String>,
    pub quality: Option<String>,
    pub elapsed_ms: Option<u64>,
}

/// Final state and measured values for one claimed job; validated when persisted.
#[derive(Debug, Clone)]
pub struct JobCompletion<'a> {
    pub status: &'a str,
    pub output: Option<&'a Path>,
    pub bytes: Option<u64>,
    pub quality: Option<&'a str>,
    pub reason: Option<&'a str>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BatchQueue {
    pub db_path: PathBuf,
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Publishing => "publishing",
        Mode::Archival => "archival",
    }
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get(0)?,
        relative: row.get(1)?,
        source: row.get(2)?,
        input_bytes: row.get::<_, i64>(3)? as u64,
        modified_ns: row.get(4)?,
        status: row.get(5)?,
        output: row.get(6)?,
        output_bytes: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
        reason: row.get(8)?,
        quality: row.get(9)?,
        elapsed_ms: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
    })
}

const JOB_COLUMNS: &str = "id, relative, source, input_bytes, modified_ns, status, output, output_bytes, reason, quality, elapsed_ms";

impl BatchQueue {
    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.db_path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        Ok(connection)
    }

    pub fn open(db: &Path, input: &Path, output: &Path, mode: Mode) -> Result<Self> {
        let input = fs::canonicalize(input).context("input directory missing")?;
        let output = fs::canonicalize(output).context("output directory missing")?;
        if !input.is_dir() || !output.is_dir() {
            bail!("batch directories must exist");
        }
        let queue = Self {
            db_path: db.to_path_buf(),
        };
        let connection = queue.connect()?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS batch (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                input_root TEXT NOT NULL,
                output_root TEXT NOT NULL,
                mode TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY,
                relative TEXT NOT NULL UNIQUE,
                source TEXT NOT NULL,
                input_bytes INTEGER NOT NULL CHECK (input_bytes >= 0),
                modified_ns INTEGER NOT NULL,
                status TEXT NOT NULL CHECK (status IN (
                    'discovered','analyzed','queued','processing','verifying',
                    'completed','skipped','failed','interrupted'
                )),
                output TEXT,
                output_bytes INTEGER,
                reason TEXT,
                quality TEXT,
                elapsed_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status, id);
        ",
        )?;
        let expected = (
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
            mode_name(mode).to_owned(),
        );
        connection.execute(
            "INSERT OR IGNORE INTO batch(id, input_root, output_root, mode) VALUES (1, ?1, ?2, ?3)",
            params![expected.0, expected.1, expected.2],
        )?;
        let actual: (String, String, String) = connection.query_row(
            "SELECT input_root, output_root, mode FROM batch WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if actual != expected {
            bail!("existing batch belongs to different source, destination or mode");
        }
        Ok(queue)
    }

    pub fn enqueue(
        &self,
        relative: &str,
        source: &Path,
        size: u64,
        modified_ns: i64,
    ) -> Result<()> {
        if relative.is_empty() || size > i64::MAX as u64 {
            bail!("invalid source identity");
        }
        let connection = self.connect()?;
        let absolute = fs::canonicalize(source)?;
        connection.execute(
            "INSERT OR IGNORE INTO jobs(relative, source, input_bytes, modified_ns, status)
             VALUES(?1, ?2, ?3, ?4, 'queued')",
            params![
                relative,
                absolute.to_string_lossy(),
                size as i64,
                modified_ns
            ],
        )?;
        // Never silently reinterpret a changed source as the same completed job.
        connection.execute(
            "UPDATE jobs SET status='failed', reason='source identity changed since discovery'
             WHERE relative=?1 AND (source != ?2 OR input_bytes != ?3 OR modified_ns != ?4)
             AND status NOT IN ('processing','verifying')",
            params![
                relative,
                absolute.to_string_lossy(),
                size as i64,
                modified_ns
            ],
        )?;
        Ok(())
    }

    pub fn claim_next(&self) -> Result<Option<Job>> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let job: Option<Job> = {
            let mut statement = transaction.prepare(&format!(
                "SELECT {JOB_COLUMNS} FROM jobs WHERE status IN ('queued','interrupted') ORDER BY id LIMIT 1"
            ))?;
            statement.query_row([], job_from_row).optional()?
        };
        if let Some(mut job) = job {
            let rows = transaction.execute(
                "UPDATE jobs SET status='processing' WHERE id=?1 AND status IN ('queued','interrupted')",
                [job.id],
            )?;
            if rows != 1 {
                bail!("job was claimed by another worker");
            }
            transaction.commit()?;
            job.status = "processing".into();
            Ok(Some(job))
        } else {
            transaction.commit()?;
            Ok(None)
        }
    }

    pub fn finish(&self, id: i64, completion: JobCompletion<'_>) -> Result<()> {
        if !matches!(completion.status, "completed" | "skipped" | "failed")
            || completion
                .bytes
                .is_some_and(|value| value > i64::MAX as u64)
            || completion.elapsed_ms > i64::MAX as u64
        {
            bail!("invalid job outcome");
        }
        let connection = self.connect()?;
        let rows = connection.execute(
            "UPDATE jobs SET status=?2, output=?3, output_bytes=?4,
             quality=?5, reason=?6, elapsed_ms=?7
             WHERE id=?1 AND status IN ('processing','verifying')",
            params![
                id,
                completion.status,
                completion
                    .output
                    .map(|value| value.to_string_lossy().into_owned()),
                completion.bytes.map(|value| value as i64),
                completion.quality,
                completion.reason,
                completion.elapsed_ms as i64
            ],
        )?;
        if rows != 1 {
            bail!("job is not active or no longer exists");
        }
        Ok(())
    }

    pub fn recover(&self) -> Result<usize> {
        let connection = self.connect()?;
        let count = connection.execute(
            "UPDATE jobs SET status='interrupted', reason='previous process stopped during execution'
             WHERE status IN ('processing','verifying')",
            [],
        )?;
        Ok(count)
    }

    pub fn list(&self) -> Result<Vec<Job>> {
        let connection = self.connect()?;
        let mut statement =
            connection.prepare(&format!("SELECT {JOB_COLUMNS} FROM jobs ORDER BY id"))?;
        let jobs = statement
            .query_map([], job_from_row)?
            .collect::<rusqlite::Result<Vec<Job>>>()?;
        Ok(jobs)
    }
}
