use crate::media::{InspectedMedia, inspect};
use crate::policy::{
    CodecPreference, Decision, Mode, choose_plan_with_preference, validate_container_compatibility,
};
use crate::worker::Control;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use tempfile::{Builder, NamedTempFile};

pub trait PartialOutputRegistry {
    fn register_partial(&self, path: &Path) -> Result<()>;
    fn unregister_partial(&self, path: &Path) -> Result<()>;
}

pub struct RunExecution<'a> {
    pub control: Option<&'a Control>,
    pub registry: Option<&'a dyn PartialOutputRegistry>,
}

struct RegisteredTemporary<'a> {
    file: Option<NamedTempFile>,
    registry: Option<&'a dyn PartialOutputRegistry>,
}

impl<'a> RegisteredTemporary<'a> {
    fn new(file: NamedTempFile, registry: Option<&'a dyn PartialOutputRegistry>) -> Result<Self> {
        if let Some(registry) = registry {
            registry.register_partial(file.path())?;
        }
        Ok(Self {
            file: Some(file),
            registry,
        })
    }

    fn path(&self) -> &Path {
        self.file
            .as_ref()
            .expect("temporary output exists until guard drop")
            .path()
    }

    fn publish(&mut self, destination: &Path) -> Result<()> {
        self.file
            .as_ref()
            .context("temporary output is no longer available")?
            .as_file()
            .sync_all()
            .context("failed to synchronize verified output before publication")?;

        let file = self
            .file
            .take()
            .context("temporary output is no longer available")?;
        let temporary_path = file.path().to_path_buf();
        match file.persist_noclobber(destination) {
            Ok(_) => {
                if !temporary_path.exists()
                    && let Some(registry) = self.registry
                {
                    // An absent temporary path is safely pruned on recovery if this fails.
                    let _ = registry.unregister_partial(&temporary_path);
                }
                Ok(())
            }
            Err(error) => {
                self.file = Some(error.file);
                Err(error.error).context("verified output could not be published without overwrite")
            }
        }
    }
}

impl Drop for RegisteredTemporary<'_> {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let path = file.path().to_path_buf();
            if file.close().is_ok()
                && let Some(registry) = self.registry
            {
                let _ = registry.unregister_partial(&path);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RunOutcome {
    Accepted {
        output: PathBuf,
        before: u64,
        after: u64,
        quality: String,
    },
    Copied {
        output: PathBuf,
        bytes: u64,
        reason: String,
    },
    Skipped {
        reason: String,
    },
    Rejected {
        reason: String,
    },
}

fn checked_command(result: Output, operation: &str) -> Result<Output> {
    if !result.status.success() {
        let error = String::from_utf8_lossy(&result.stderr);
        let truncated = error.chars().take(2000).collect::<String>();
        bail!("{operation} failed: {truncated}");
    }
    Ok(result)
}

pub fn terminate_child_bounded(child: &mut std::process::Child) -> Result<()> {
    if let Ok(Some(_)) = child.try_wait() {
        return Ok(());
    }

    let pid = child.id();

    #[cfg(unix)]
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
    #[cfg(windows)]
    {
        let _ = child.kill();
    }

    let grace_limit = Duration::from_millis(1000);
    let start = Instant::now();
    let poll = Duration::from_millis(20);

    while start.elapsed() < grace_limit {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(poll),
            Err(_) => return Ok(()),
        }
    }

    let _ = child.kill();

    let reap_deadline = Instant::now() + Duration::from_millis(2000);
    while Instant::now() < reap_deadline {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(poll),
            Err(_) => return Ok(()),
        }
    }
    let _ = child.wait();
    Ok(())
}

pub fn run_command_cancellable(
    command: &mut Command,
    operation: &str,
    control: Option<&Control>,
) -> Result<Output> {
    if control.is_some_and(|c| c.is_cancelled()) {
        bail!("batch cancelled before starting {operation}");
    }

    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn process for {operation}"))?;
    let pid = child.id();
    let _guard = control.map(|ctrl| ctrl.register_process(pid));

    let mut stdout_handle = child.stdout.take();
    let mut stderr_handle = child.stderr.take();

    let stdout_thread = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        if let Some(mut stream) = stdout_handle.take() {
            stream.read_to_end(&mut buffer)?;
        }
        Ok(buffer)
    });

    let stderr_thread = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        if let Some(mut stream) = stderr_handle.take() {
            stream.read_to_end(&mut buffer)?;
        }
        Ok(buffer)
    });

    let poll_interval = Duration::from_millis(30);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if control.is_some_and(|c| c.is_cancelled()) {
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    bail!("batch cancelled by user during {operation}");
                }
                break status;
            }
            Ok(None) => {
                if control.is_some_and(|c| c.is_cancelled()) {
                    terminate_child_bounded(&mut child)?;
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    bail!("batch cancelled by user during {operation}");
                }
                std::thread::sleep(poll_interval);
            }
            Err(err) => {
                let _ = terminate_child_bounded(&mut child);
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(err).context(format!("error polling process for {operation}"));
            }
        }
    };

    let stdout = stdout_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stdout reader thread panicked"))?
        .context("failed to read process stdout")?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stderr reader thread panicked"))?
        .context("failed to read process stderr")?;

    let output = Output {
        status,
        stdout,
        stderr,
    };

    checked_command(output, operation)
}

fn validated_relative(relative: &Path) -> Result<()> {
    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
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
    if current.file_type().is_symlink() || !current.file_type().is_file() {
        return Ok(true);
    }
    Ok(current.len() != starting.len() || current.modified()? != starting.modified()?)
}

fn make_output_name(relative: &Path, extension: &str, copy: bool) -> Result<PathBuf> {
    let filename = relative
        .file_name()
        .context("relative path has no filename")?;
    if copy {
        return Ok(relative.to_path_buf());
    }
    let mut output = filename.to_os_string();
    output.push(format!(".mar2i.{extension}"));
    Ok(relative.with_file_name(output))
}

fn verify_decode(ffmpeg: &Path, output: &Path, control: Option<&Control>) -> Result<()> {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-nostdin", "-v", "error", "-xerror", "-i"])
        .arg(output)
        .args(["-map", "0:v:0", "-f", "null", "-"]);
    run_command_cancellable(&mut command, "output decode verification", control)?;
    Ok(())
}

fn frame_hashes(ffmpeg: &Path, source: &Path, control: Option<&Control>) -> Result<Vec<String>> {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-nostdin", "-v", "error", "-i"])
        .arg(source)
        .args(["-map", "0:v:0", "-f", "framemd5", "-"]);
    let result = run_command_cancellable(&mut command, "frame hash verification", control)?;
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

fn ssim_score(
    ffmpeg: &Path,
    source: &Path,
    output: &Path,
    control: Option<&Control>,
) -> Result<f64> {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(source)
        .arg("-i")
        .arg(output)
        .args([
            "-filter_complex",
            "[0:v:0]setpts=PTS-STARTPTS,format=yuv420p[ref];[1:v:0]setpts=PTS-STARTPTS,format=yuv420p[test];[ref][test]ssim",
            "-an",
            "-f",
            "null",
            "-",
        ]);
    let result = run_command_cancellable(&mut command, "SSIM measurement", control)?;
    let log = String::from_utf8_lossy(&result.stderr);
    let value = log
        .rsplit("All:")
        .next()
        .context("SSIM score unavailable")?;
    let literal: String = value
        .chars()
        .take_while(|letter| letter.is_ascii_digit() || *letter == '.')
        .collect();
    let score = literal.parse::<f64>().context("invalid SSIM measurement")?;
    if !(0.0..=1.0).contains(&score) {
        bail!("SSIM score out of range");
    }
    Ok(score)
}

fn verify_stream_contract(
    source: &InspectedMedia,
    accepted: &InspectedMedia,
    mode: Mode,
) -> Result<()> {
    let original = &source.profile;
    let output = &accepted.profile;
    if original.width != output.width || original.height != output.height {
        bail!("output resolution changed");
    }
    if (source.duration - accepted.duration).abs() > (2.0 / original.frame_rate.max(1.0)).max(0.15)
    {
        bail!("duration exceeds permitted timing difference");
    }
    if (original.frame_rate - output.frame_rate).abs() > 0.01 {
        bail!("frame rate changed");
    }
    if source.audio_count != accepted.audio_count
        || source.subtitle_count != accepted.subtitle_count
    {
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

#[derive(Debug, Clone, Copy, Default)]
pub struct RunOptions<'a> {
    pub mode: Mode,
    pub preference: CodecPreference,
    pub available_encoders: Option<&'a [String]>,
}

impl From<Mode> for RunOptions<'_> {
    fn from(mode: Mode) -> Self {
        Self {
            mode,
            preference: CodecPreference::Auto,
            available_encoders: None,
        }
    }
}

pub fn run_one(
    ffmpeg: &Path,
    ffprobe: &Path,
    source: &Path,
    relative: &Path,
    output_root: &Path,
    mode: Mode,
) -> Result<RunOutcome> {
    run_one_with_control(ffmpeg, ffprobe, source, relative, output_root, mode, None)
}

pub fn run_one_with_control(
    ffmpeg: &Path,
    ffprobe: &Path,
    source: &Path,
    relative: &Path,
    output_root: &Path,
    mode: Mode,
    control: Option<&Control>,
) -> Result<RunOutcome> {
    run_one_with_options(
        ffmpeg,
        ffprobe,
        source,
        relative,
        output_root,
        RunOptions::from(mode),
        control,
    )
}

pub fn run_one_with_options(
    ffmpeg: &Path,
    ffprobe: &Path,
    source: &Path,
    relative: &Path,
    output_root: &Path,
    options: RunOptions<'_>,
    control: Option<&Control>,
) -> Result<RunOutcome> {
    run_one_with_registry(
        ffmpeg,
        ffprobe,
        source,
        relative,
        output_root,
        options,
        RunExecution {
            control,
            registry: None,
        },
    )
}

pub fn run_one_with_registry(
    ffmpeg: &Path,
    ffprobe: &Path,
    source: &Path,
    relative: &Path,
    output_root: &Path,
    options: RunOptions<'_>,
    execution: RunExecution<'_>,
) -> Result<RunOutcome> {
    let control = execution.control;
    let registry = execution.registry;
    if control.is_some_and(|c| c.is_cancelled()) {
        bail!("batch cancelled before processing");
    }
    validated_relative(relative)?;
    let input = inspect(ffprobe, source)?;
    let starting_metadata = fs::symlink_metadata(source)?;
    let decision = choose_plan_with_preference(
        &input.profile,
        options.mode,
        options.preference,
        options.available_encoders,
    );

    let (extension, copy) = match &decision {
        Decision::Skip { reason } => {
            return Ok(RunOutcome::Skipped {
                reason: reason.clone(),
            });
        }
        Decision::Copy { .. } => ("", true),
        Decision::Encode { container, .. } => (*container, false),
    };
    if let Decision::Encode {
        codec, container, ..
    } = &decision
    {
        validate_container_compatibility(codec, container)
            .map_err(|e| anyhow::anyhow!("container compatibility error: {e}"))?;
    }
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

    let suffix = if copy {
        ".copy"
    } else {
        match extension {
            "mkv" => ".mkv",
            "webm" => ".webm",
            _ => ".mp4",
        }
    };
    let tempfile = Builder::new()
        .prefix(".mar2i-partial-")
        .suffix(suffix)
        .tempfile_in(parent)?;
    let mut tempfile = RegisteredTemporary::new(tempfile, registry)?;
    let temporary = tempfile.path();
    let quality = match decision {
        Decision::Copy { ref reason } => {
            if control.is_some_and(|c| c.is_cancelled()) {
                bail!("batch cancelled by user during copy");
            }
            fs::copy(source, temporary)?;
            if copy_hash(source)? != copy_hash(temporary)? {
                bail!("copied bytes differ from original");
            }
            format!("byte-for-byte copy: {reason}")
        }
        Decision::Encode { codec: "ffv1", .. } => {
            // Use explicit argv, never invoke a shell; passthrough retains timestamps.
            let mut command = Command::new(ffmpeg);
            command
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-map",
                    "0",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "-c:v",
                    "ffv1",
                    "-level",
                    "3",
                    "-coder",
                    "1",
                    "-context",
                    "1",
                    "-slicecrc",
                    "1",
                    "-threads",
                    "2",
                    "-fps_mode",
                    "passthrough",
                    "-c:a",
                    "copy",
                    "-c:s",
                    "copy",
                ])
                .arg(temporary);
            run_command_cancellable(&mut command, "lossless archival encoding", control)?;
            verify_decode(ffmpeg, temporary, control)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, options.mode)?;
            if frame_hashes(ffmpeg, source, control)? != frame_hashes(ffmpeg, temporary, control)? {
                bail!("decoded archival frames are not identical");
            }
            "decoded-frame hashes identical".to_owned()
        }
        Decision::Encode {
            codec: "libx264",
            crf,
            ..
        } => {
            let mut command = Command::new(ffmpeg);
            command
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "medium",
                    "-crf",
                ])
                .arg(crf.to_string())
                .args([
                    "-fps_mode",
                    "passthrough",
                    "-threads",
                    "2",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "copy",
                    "-movflags",
                    "+faststart",
                ])
                .arg(temporary);
            run_command_cancellable(&mut command, "publishing encoding", control)?;
            verify_decode(ffmpeg, temporary, control)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, options.mode)?;
            let score = ssim_score(ffmpeg, source, temporary, control)?;
            if score < 0.965 {
                return Ok(RunOutcome::Rejected {
                    reason: format!(
                        "SSIM {score:.5} below provisional 0.965 gate; manual quality review needed"
                    ),
                });
            }
            format!("SSIM={score:.5} (automated gate, not proof of visual equivalence)")
        }
        Decision::Encode {
            codec: "libx265",
            crf,
            container,
        } => {
            let mut command = Command::new(ffmpeg);
            command
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "-c:v",
                    "libx265",
                    "-preset",
                    "medium",
                    "-crf",
                ])
                .arg(crf.to_string())
                .args([
                    "-tag:v",
                    "hvc1",
                    "-fps_mode",
                    "passthrough",
                    "-threads",
                    "2",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "copy",
                ]);
            if container == "mp4" {
                command.args(["-movflags", "+faststart"]);
            }
            command.arg(temporary);
            run_command_cancellable(&mut command, "HEVC encoding", control)?;
            verify_decode(ffmpeg, temporary, control)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, options.mode)?;
            let score = ssim_score(ffmpeg, source, temporary, control)?;
            if score < 0.965 {
                return Ok(RunOutcome::Rejected {
                    reason: format!(
                        "SSIM {score:.5} below provisional 0.965 gate; manual quality review needed"
                    ),
                });
            }
            format!("SSIM={score:.5} (automated gate, not proof of visual equivalence)")
        }
        Decision::Encode {
            codec: "libsvtav1",
            crf,
            container,
        } => {
            let mut command = Command::new(ffmpeg);
            command
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "-c:v",
                    "libsvtav1",
                    "-preset",
                    "7",
                    "-crf",
                ])
                .arg(crf.to_string())
                .args([
                    "-fps_mode",
                    "passthrough",
                    "-threads",
                    "2",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "copy",
                ]);
            if container == "mp4" {
                command.args(["-movflags", "+faststart"]);
            }
            command.arg(temporary);
            run_command_cancellable(&mut command, "AV1 encoding", control)?;
            verify_decode(ffmpeg, temporary, control)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, options.mode)?;
            let score = ssim_score(ffmpeg, source, temporary, control)?;
            if score < 0.965 {
                return Ok(RunOutcome::Rejected {
                    reason: format!(
                        "SSIM {score:.5} below provisional 0.965 gate; manual quality review needed"
                    ),
                });
            }
            format!("SSIM={score:.5} (automated gate, not proof of visual equivalence)")
        }
        Decision::Encode {
            codec: "libvpx-vp9",
            crf,
            ..
        } => {
            let mut command = Command::new(ffmpeg);
            command
                .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "-c:v",
                    "libvpx-vp9",
                    "-b:v",
                    "0",
                    "-crf",
                ])
                .arg(crf.to_string())
                .args([
                    "-deadline",
                    "good",
                    "-cpu-used",
                    "2",
                    "-fps_mode",
                    "passthrough",
                    "-threads",
                    "2",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "copy",
                ])
                .arg(temporary);
            run_command_cancellable(&mut command, "VP9 encoding", control)?;
            verify_decode(ffmpeg, temporary, control)?;
            let accepted = inspect(ffprobe, temporary)?;
            verify_stream_contract(&input, &accepted, options.mode)?;
            let score = ssim_score(ffmpeg, source, temporary, control)?;
            if score < 0.965 {
                return Ok(RunOutcome::Rejected {
                    reason: format!(
                        "SSIM {score:.5} below provisional 0.965 gate; manual quality review needed"
                    ),
                });
            }
            format!("SSIM={score:.5} (automated gate, not proof of visual equivalence)")
        }
        Decision::Encode { .. } => bail!("unsupported encoding plan"),
        Decision::Skip { .. } => unreachable!(),
    };

    if control.is_some_and(|c| c.is_cancelled()) {
        bail!("batch cancelled by user before publication");
    }

    if source_changed(source, &starting_metadata)? {
        bail!("source changed during processing; discarding temporary output");
    }
    let after = fs::metadata(temporary)?.len();
    if after == 0 {
        bail!("empty output rejected");
    }
    if !copy && after >= input.input_bytes {
        return Ok(RunOutcome::Rejected {
            reason: format!(
                "output would be {} bytes, not smaller than {} bytes",
                after, input.input_bytes
            ),
        });
    }
    tempfile.publish(&destination)?;

    if let Decision::Copy { reason } = decision {
        Ok(RunOutcome::Copied {
            output: destination,
            bytes: after,
            reason,
        })
    } else {
        Ok(RunOutcome::Accepted {
            output: destination,
            before: input.input_bytes,
            after,
            quality,
        })
    }
}

#[cfg(test)]
mod publication_tests {
    use super::RegisteredTemporary;
    use std::fs;
    use tempfile::{Builder, tempdir};

    #[test]
    fn publication_refuses_destination_created_after_temporary_file() {
        let directory = tempdir().unwrap();
        let temporary = Builder::new().tempfile_in(directory.path()).unwrap();
        let mut publication = RegisteredTemporary::new(temporary, None).unwrap();
        fs::write(publication.path(), b"verified_output").unwrap();
        let destination = directory.path().join("output.mkv");
        fs::write(&destination, b"unrelated_user_file").unwrap();

        assert!(publication.publish(&destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated_user_file");

        let path = publication.path().to_path_buf();
        drop(publication);
        assert!(
            !path.exists(),
            "failed publication must clean its temporary file"
        );
    }

    #[test]
    fn successful_publication_leaves_only_the_final_file() {
        let directory = tempdir().unwrap();
        let temporary = Builder::new().tempfile_in(directory.path()).unwrap();
        let mut publication = RegisteredTemporary::new(temporary, None).unwrap();
        fs::write(publication.path(), b"verified_output").unwrap();
        let temporary_path = publication.path().to_path_buf();
        let destination = directory.path().join("output.mkv");

        publication.publish(&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"verified_output");
        assert!(
            !temporary_path.exists(),
            "published temporary file must be removed"
        );
    }
}
