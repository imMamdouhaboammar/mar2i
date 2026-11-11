use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
    pub absolute: PathBuf,
    pub relative: PathBuf,
    pub size: u64,
}

const EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "mxf", "avi", "webm", "mts", "m2ts", "m4v", "mpg", "mpeg", "ts", "3gp",
    "wmv",
];
const MAX_FILES: usize = 1_000_000;

fn canonical_candidate(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path)
            .with_context(|| format!("cannot resolve {}", path.display()));
    }

    let mut missing = Vec::new();
    let mut ancestor = path;
    while !ancestor.exists() {
        let filename = ancestor
            .file_name()
            .context("path has no existing parent")?;
        if filename == ".." || filename == "." {
            bail!("ambiguous directory path");
        }
        missing.push(filename.to_os_string());
        ancestor = ancestor.parent().context("path has no parent")?;
    }
    let mut resolved = fs::canonicalize(ancestor)?;
    for part in missing.iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

pub fn validate_directories(input: &Path, output: &Path) -> Result<()> {
    let source = canonical_candidate(input)?;
    if !source.is_dir() {
        bail!("input is not a directory");
    }
    let destination = canonical_candidate(output)?;
    if source.starts_with(&destination) || destination.starts_with(&source) {
        bail!("input/output directories overlap");
    }
    if source
        .components()
        .any(|part| matches!(part, Component::ParentDir))
        || destination
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        bail!("parent traversal is not allowed");
    }
    Ok(())
}

pub fn discover(input: &Path) -> Result<Vec<DiscoveredFile>> {
    let root = fs::canonicalize(input).context("input directory unavailable")?;
    if !root.is_dir() {
        bail!("input is not a directory");
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&root).follow_links(false).sort_by_file_name() {
        let entry = entry.with_context(|| format!("cannot inspect {}", root.display()))?;
        if entry.file_type().is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        let allowed = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
        if !allowed {
            continue;
        }
        let relative = entry.path().strip_prefix(&root)?.to_path_buf();
        let size = entry.metadata()?.len();
        files.push(DiscoveredFile {
            absolute: entry.path().to_path_buf(),
            relative,
            size,
        });
        if files.len() > MAX_FILES {
            bail!("too many media files in batch");
        }
    }
    files.sort_unstable_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}
