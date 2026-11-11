use crate::media::{InspectedMedia, inspect};
use crate::policy::{Decision, Mode, choose_plan};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use tempfile::Builder;

#[derive(Debug, Clone)]
pub enum RunOutcome {
    Accepted { output: PathBuf, before: u64, after: u64, quality: String },
    Copied { output: PathBuf, bytes: u64, reason: String },
    Skipped { reason: String },
    Rejected { reason: String },
}

fn checked_command(result: Output, operation: &str) -> Result<Output> {
    if !result.status.success() {
        let error = String::from_utf8_lossy(&result.stderr);
        let truncated = error.chars().take(2000).collect::<String>();
        bail!("{operation} failed: {truncated}");
    }
    Ok(result)
}

fn validated_relative(relative: &Path) -> Result<()> {
    if relative.as_os_str().is_empty()
        || !relative.components().all(|part| matches!(part, Component::Normal(_)))
    {
        bail!("source-relative path contains traversal or absolute components");
    }
    Ok(())
}

fn copy_hash(path: &Path) -> Result<Vec<u8>> {
    let mut file = BufReader::new(File::open(path)?);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().to_vec())
}

fn source_changed(source: &Path, starting: &fs::Metadata) -> Result<bool> {
    let current = fs::symlink_metadata(source)?;
    Ok(current.len() != starting.len() || current.modified()? != starting.modified()?)
}

fn make_output_name(relative: &Path, extension: &str, copy: bool) -> Result<PathBuf> {
    let filename = relative.file_name().context("relative path has no filename")?;
    if copy {
        return Ok(relative.to_path_buf());
    }
    let mut output = filename.to_os_string();
    output.push(format!(".mar2i.{extension}"));
    Ok(relative.with_file_name(output))
}

fn verify_decode(ffmpeg: &Path, output: &Path) -> Result<()> {
    let result = Command::new(ffmpeg)
        .args(["-hide_banner", "-nostdin", "-v", "error", "-xerror", "-i"])
        .arg(output)
        .args(["-map", "0:v:0", "-f", "null", "-"])
        .output()?;
    checked_command(result, "output decode verification")?;
    Ok(())
}

fn frame_hashes(ffmpeg: &Path, source: &Path) -> Result<Vec<String>> {
    let result = Command::new(ffmpeg)
        .args(["-hide_banner", "-nostdin", "-v", "error", "-i"])
        .arg(source)
        .args(["-map", "0:v:0", "-f", "framemd5", "-"])
        .output()?;
    let result = checked_command(result, "frame hash verification")?;
    let hashes: Vec<String> = String::from_utf8(result.stdout)?
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.rsplit(',').next().map(|hash| hash.trim().to_owned()))
        .collect();
    if hashes.is_empty() {
        bail!("no decoded frames to compare");
    }
    Ok(hashes)
}

fn ssim_score(ffmpeg: &Path, source: &Path, output: &Path) -> Result<f64> {
    let result = Command::new(ffmpeg)
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(source)
        .arg("-i")
        .arg(output)
        .args(["-filter_complex", "[0:v:0]setpts=PTS-STARTPTS,format=yuv420p[ref];[1:v:0]setpts=PTS-STARTPTS,format=yuv420p[test];[ref][test]ssim", "-an", "-f", "null", "-"])
        .output()?;
    let result = checked_command(result, "SSIM measurement")?;
    let log = String::from_utf8_lossy(&result.stderr);
    let value = log.rsplit("All:").next().context("SSIM score unavailable")?;
    let literal: String = value.chars()
        .take_while(|letter| letter.is_ascii_digit() || *letter == '.')
        .collect();
    let score = literal.parse::<f64>().context("invalid SSIM measurement")?;
    if !(0.0..=1.0).contains(&score) {
        bail!("SSIM score out of range");
    }
    Ok(score)
}

fn verify_stream_contract(source: &InspectedMedia, accepted: &InspectedMedia, mode: Mode) -> Result<()> {
    let original = &source.profile;
    let output = &accepted.profile;
    if original.width != output.width || original.height != output.height {
        bail!("output resolution changed");
    }
    if (source.duration - accepted.duration).abs() > (2.0 / original.frame_rate.max(1.0)).max(0.15) {
        bail!("duration exceeds permitted timing difference");
    }
    if (original.frame_rate - output.frame_rate).abs() > 0.01 {
        bail!("frame rate changed");
    }
    if source.audio_count != accepted.audio_count || source.subtitle_count != accepted.subtitle_count {
        bail!("audio or subtitle stream count changed");
    }
    if mode == Mode::Archival {
        if original.pix_fmt != output.pix_fmt {
            bail!("archival pixel format changed");
        }
        if original.color_transfer != output.color_transfer {
            bail!("archival color transfer metadata changed");
        }
        if source.chapter_count != accepted.chapter_count {
            bail!("archival chapter count changed");
        }
    }
    Ok(())
}

pub fn run_one(
    ffmpeg: &Path,
    ffprobe: &Path,
    source: &Path,
    relative: &Path,
    output_root: &Path,
    mode: Mode,
) -> Result<RunOutcome> {
    validated_relative(relative)?;
    let input = inspect(ffprobe, source)?;
    let starting_metadata = fs::symlink_metadata(source)?;
    let decision = choose_plan(&input.profile, mode);

    let (extension, copy) = match &decision {
        Decision::Skip { reason } => return Ok(RunOutcome::Skipped { reason: reason.clone() }),
        Decision::Copy { .. } => ("", true),
        Decision::Encode { container, .. } => (*container, false),
    };
    let destination_relative = make_output_name(relative, extension, copy)?;
    let root = fs::canonicalize(output_root).context("output directory must exist")?;
    if !root.is_dir() {
        bail!("output path is not a directory");
    }
    let destination = root.join(&destination_relative);
    let parent = destination.parent().context("output file has no parent")?;
    fs::create_dir_all(parent)?;
    let canonical_parent = fs::canonicalize(parent)?;
    if !canonical_parent.starts_with(&root) {
        bail!("output directory path escapes its selected root");
    }
    if fs::symlink_metadata(&destination).is_ok() {
        bail!("output file already exists; refusing to overwrite");
    }

    let suffix = if copy { ".copy" } else if extension == "mkv" { ".mkv" } else { ".mp4" };
    let tempfile = Builder::new().prefix(".mar2i-partial-").suffix(suffix).tempfile_in(parent)?;
    let temporary = tempfile.path();
    let quality = match decision {
        Decision::Copy { ref reason } => {
            fs::copy(source, temporary)?;
            if copy_hash(source)? != copy_hash(temporary)? {
                bail!("copied bytes differ from original");
            }
            format!("byte-for-byte copy: {reason}")
        }
        Decision::Encode { codec: "ffv1", .. } => {
            // Use explicit argv, never invoke a shell; passthrough retains timestamps.
            let result = Command::new(ffmpeg)
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args(["-map", "0", "-map_metadata", "0", "-map_chapters", "0",
                    "-c:v", "ffv1", "-level", "3", "-coder", "1", "-context", "1",
                    "-slicecrc", "1", "-threads", "2", "-fps_mode", "passthrough", "-c:a", "copy", "-c:s", "copy"])
                .arg(temporary).output()?;
            checked_command(result, "lossless archival encoding")?;
            verify_decode(ffmpeg, temporary)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, mode)?;
            if frame_hashes(ffmpeg, source)? != frame_hashes(ffmpeg, temporary)? {
                bail!("decoded archival frames are not identical");
            }
            "decoded-frame hashes identical".to_owned()
        }
        Decision::Encode { codec: "libx264", crf, .. } => {
            let result = Command::new(ffmpeg)
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args(["-map", "0:v:0", "-map", "0:a?", "-map_metadata", "0", "-map_chapters", "0",
                    "-c:v", "libx264", "-preset", "medium", "-crf"])
                .arg(crf.to_string())
                .args(["-fps_mode", "passthrough", "-threads", "2", "-pix_fmt", "yuv420p",
                    "-c:a", "copy", "-movflags", "+faststart"])
                .arg(temporary).output()?;
            checked_command(result, "publishing encoding")?;
            verify_decode(ffmpeg, temporary)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, mode)?;
            let score = ssim_score(ffmpeg, source, temporary)?;
            if score < 0.965 {
                return Ok(RunOutcome::Rejected {
                    reason: format!("SSIM {score:.5} below provisional 0.965 gate; manual quality review needed"),
                });
            }
            format!("SSIM={score:.5} (automated gate, not proof of visual equivalence)")
        }
        Decision::Encode { .. } => bail!("unsupported encoding plan"),
        Decision::Skip { .. } => unreachable!(),
    };

    if source_changed(source, &starting_metadata)? {
        bail!("source changed during processing; discarding temporary output");
    }
    let after = fs::metadata(temporary)?.len();
    if after == 0 {
        bail!("empty output rejected");
    }
    if !copy && after >= input.input_bytes {
        return Ok(RunOutcome::Rejected {
            reason: format!("output would be {} bytes, not smaller than {} bytes", after, input.input_bytes),
        });
    }
    // Same-directory hard link publishes only if destination is absent; unlike rename,
    // it cannot replace an unrelated file created concurrently.
    fs::hard_link(temporary, &destination).with_context(|| {
        format!("cannot atomically publish without overwrite at {}", destination.display())
    })?;

    if let Decision::Copy { reason } = decision {
        Ok(RunOutcome::Copied { output: destination, bytes: after, reason })
    } else {
        Ok(RunOutcome::Accepted {
            output: destination,
            before: input.input_bytes,
            after,
            quality,
        })
    }
}
