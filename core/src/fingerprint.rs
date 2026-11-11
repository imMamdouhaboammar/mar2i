//! Sparse SHA-256 media fingerprinting and archival registry.
//!
//! High-volume Hajj and Umrah media operations ingest hundreds of gigabytes per session.
//! Computing a full SHA-256 across entire 50GB 4K files is slow and wastes disk I/O.
//!
//! This module computes an O(1) sparse media fingerprint:
//! SHA-256( size_le_bytes || first_1MB || last_1MB )
//!
//! This is a quick similarity hint, not a content-integrity proof: files with
//! equal size, first MiB and last MiB may differ in the unsampled middle.
//! The optional SQLite registry is not wired into batch deduplication.

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const SAMPLE_BYTES: u64 = 1024 * 1024; // 1 MiB sample window

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaFingerprint {
    pub hash: String,
    pub size_bytes: u64,
}

/// Computes the sparse SHA-256 fingerprint of a media file.
pub fn compute_fingerprint(path: &Path) -> Result<MediaFingerprint> {
    let mut file = File::open(path).with_context(|| {
        format!(
            "unable to open media file for fingerprinting: {}",
            path.display()
        )
    })?;

    let metadata = file
        .metadata()
        .with_context(|| format!("unable to read metadata for: {}", path.display()))?;

    if !metadata.is_file() {
        bail!("path is not a regular file: {}", path.display());
    }

    let size = metadata.len();
    let mut hasher = Sha256::new();

    // 1. Hash the file size in 8-byte LE
    hasher.update(size.to_le_bytes());

    // 2. Hash the head (up to 1 MiB)
    let head_read = SAMPLE_BYTES.min(size) as usize;
    let mut head_buf = vec![0u8; head_read];
    file.read_exact(&mut head_buf)
        .with_context(|| "failed to read head bytes")?;
    hasher.update(&head_buf);

    // 3. Hash the tail (up to 1 MiB) if the file is larger than 1 MiB
    if size > SAMPLE_BYTES {
        let tail_size = SAMPLE_BYTES.min(size - SAMPLE_BYTES);
        file.seek(SeekFrom::End(-(tail_size as i64)))
            .with_context(|| "failed to seek to tail bytes")?;
        let mut tail_buf = vec![0u8; tail_size as usize];
        file.read_exact(&mut tail_buf)
            .with_context(|| "failed to read tail bytes")?;
        hasher.update(&tail_buf);
    }

    let hash_bytes = hasher.finalize();
    let hash = format!("{:x}", hash_bytes);

    Ok(MediaFingerprint {
        hash,
        size_bytes: size,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintRecord {
    pub fingerprint: String,
    pub source_path: String,
    pub output_path: String,
    pub mode: String,
    pub codec: String,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub created_at: String,
}

/// Persistent registry of processed media fingerprints stored in SQLite.
pub struct FingerprintRegistry {
    conn: Connection,
}

impl FingerprintRegistry {
    pub fn open_or_create(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).with_context(|| {
            format!(
                "failed to open fingerprint database at: {}",
                db_path.display()
            )
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS processed_fingerprints (
                fingerprint TEXT PRIMARY KEY,
                source_path TEXT NOT NULL,
                output_path TEXT NOT NULL,
                mode TEXT NOT NULL,
                codec TEXT NOT NULL,
                input_bytes INTEGER NOT NULL,
                output_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_fp_source ON processed_fingerprints (source_path);",
        )?;

        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS processed_fingerprints (
                fingerprint TEXT PRIMARY KEY,
                source_path TEXT NOT NULL,
                output_path TEXT NOT NULL,
                mode TEXT NOT NULL,
                codec TEXT NOT NULL,
                input_bytes INTEGER NOT NULL,
                output_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn register(&self, record: &FingerprintRecord) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO processed_fingerprints (
                fingerprint, source_path, output_path, mode, codec, input_bytes, output_bytes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.fingerprint,
                record.source_path,
                record.output_path,
                record.mode,
                record.codec,
                record.input_bytes as i64,
                record.output_bytes as i64
            ],
        )?;
        Ok(())
    }

    pub fn lookup(&self, fingerprint: &str) -> Result<Option<FingerprintRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT fingerprint, source_path, output_path, mode, codec, input_bytes, output_bytes, created_at
             FROM processed_fingerprints WHERE fingerprint = ?1",
        )?;

        let mut rows = stmt.query(params![fingerprint])?;
        if let Some(row) = rows.next()? {
            let in_bytes: i64 = row.get(5)?;
            let out_bytes: i64 = row.get(6)?;
            Ok(Some(FingerprintRecord {
                fingerprint: row.get(0)?,
                source_path: row.get(1)?,
                output_path: row.get(2)?,
                mode: row.get(3)?,
                codec: row.get(4)?,
                input_bytes: in_bytes as u64,
                output_bytes: out_bytes as u64,
                created_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }
}
