use mar2i_core::discovery::{discover, validate_directories};
use mar2i_core::media::inspect;
use mar2i_core::policy::{CodecPreference, Decision, Mode, choose_plan_with_preference};
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{summarize, write_reports};
use mar2i_core::worker::{
    BatchOptions, Control, ResourceProfile, modified_ns, run_batch_with_options,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone)]
struct Selection {
    input: PathBuf,
    output: PathBuf,
    mode: Mode,
}

#[derive(Default)]
struct Runtime {
    running: AtomicBool,
    control: Mutex<Option<Arc<Control>>>,
    selection: Mutex<Option<Selection>>,
    error: Mutex<Option<String>>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ScanResult {
    total: usize,
    total_bytes: u64,
    supported: usize,
    skipped: usize,
    details: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JobView {
    relative: String,
    status: String,
    input_bytes: u64,
    output_bytes: Option<u64>,
    output: Option<String>,
    reason: Option<String>,
    quality: Option<String>,
    elapsed_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResult {
    running: bool,
    paused: bool,
    cancelled: bool,
    error: Option<String>,
    files: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    queued: usize,
    active: usize,
    input_bytes: u64,
    output_bytes: u64,
    verified_savings_bytes: u64,
    jobs: Vec<JobView>,
}

fn parse_mode(mode: &str) -> Result<Mode, String> {
    match mode {
        "publish" => Ok(Mode::Publishing),
        "archive" => Ok(Mode::Archival),
        _ => Err("غرض المعالجة غير معروف".into()),
    }
}

fn parse_profile(profile: &str) -> Result<ResourceProfile, String> {
    match profile {
        "low" => Ok(ResourceProfile::Low),
        "balanced" => Ok(ResourceProfile::Balanced),
        "maximum" => Ok(ResourceProfile::Maximum),
        _ => Err("ملف استهلاك الموارد غير معروف".into()),
    }
}

fn parse_codec(codec: &str) -> Result<CodecPreference, String> {
    match codec {
        "auto" => Ok(CodecPreference::Auto),
        "smart_abr" | "smart" | "abr" | "yalashbab" => Ok(CodecPreference::SmartAbr),
        "h264" => Ok(CodecPreference::H264),
        "hevc" => Ok(CodecPreference::Hevc),
        "av1" => Ok(CodecPreference::Av1),
        "vp9" => Ok(CodecPreference::Vp9),
        "ffv1" => Ok(CodecPreference::Ffv1),
        _ => Err("الترميز المحدد غير مدعوم".into()),
    }
}

pub fn resolve_media_binary(name: &str, current_exe: Option<&Path>) -> PathBuf {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };

    // 1. Explicit environment variable overrides
    let env_var = format!("MAR2I_{}_PATH", name.to_uppercase());
    if let Ok(override_path) = std::env::var(&env_var) {
        let p = PathBuf::from(override_path);
        if p.is_file() {
            return p;
        }
    }
    if let Ok(media_dir) = std::env::var("MAR2I_MEDIA_DIR") {
        let candidate = Path::new(&media_dir).join(&filename);
        if candidate.is_file() {
            return candidate;
        }
    }

    // 2. Bundled and resource paths relative to executable (offline installation)
    if let Some(executable) = current_exe
        && let Some(parent) = executable.parent()
    {
        for candidate in [
            parent.join(&filename),
            parent.join("bin").join(&filename),
            parent.join("../Resources").join(&filename),
            parent.join("../Resources/bin").join(&filename),
        ] {
            if candidate.is_file() {
                return std::fs::canonicalize(&candidate).unwrap_or(candidate);
            }
        }
    }

    // 3. Known system locations (development / local machine)
    for root in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        let path = Path::new(root).join(&filename);
        if path.is_file() {
            return path;
        }
    }

    // 4. Fallback to bare filename for PATH lookup
    PathBuf::from(filename)
}

fn media_binary(name: &str) -> PathBuf {
    let current_exe = std::env::current_exe().ok();
    resolve_media_binary(name, current_exe.as_deref())
}

fn validate_paths(input: &Path, output: &Path) -> Result<(), String> {
    validate_directories(input, output).map_err(|error| error.to_string())
}

#[tauri::command]
async fn analyze_directory(
    input: String,
    output: String,
    mode: String,
    codec: Option<String>,
) -> Result<ScanResult, String> {
    let mode = parse_mode(&mode)?;
    let preference = codec
        .as_deref()
        .map(parse_codec)
        .transpose()?
        .unwrap_or(CodecPreference::Auto);
    let input = PathBuf::from(input);
    let output = PathBuf::from(output);
    validate_paths(&input, &output)?;
    let discovered = discover(&input).map_err(|error| error.to_string())?;
    let ffprobe = media_binary("ffprobe");
    let mut supported = 0;
    let mut details = Vec::new();
    let mut total_bytes = 0u64;
    for file in &discovered {
        total_bytes = total_bytes.saturating_add(file.size);
        match inspect(&ffprobe, &file.absolute) {
            Ok(media) => {
                match choose_plan_with_preference(&media.profile, mode, preference, None) {
                    Decision::Skip { reason } => {
                        if details.len() < 15 {
                            details.push(format!("{}: {reason}", file.relative.display()));
                        }
                    }
                    _ => supported += 1,
                }
            }
            Err(error) => {
                if details.len() < 15 {
                    details.push(format!("{}: {error:#}", file.relative.display()));
                }
            }
        }
    }
    Ok(ScanResult {
        total: discovered.len(),
        total_bytes,
        supported,
        skipped: discovered.len().saturating_sub(supported),
        details,
    })
}

#[tauri::command]
fn start_batch(
    input: String,
    output: String,
    mode: String,
    profile: String,
    codec: String,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    let mode = parse_mode(&mode)?;
    let profile = parse_profile(&profile)?;
    let preference = parse_codec(&codec)?;
    let input = PathBuf::from(input);
    let output = PathBuf::from(output);
    validate_paths(&input, &output)?;
    if runtime.running.swap(true, Ordering::SeqCst) {
        return Err("توجد دفعة قيد التشغيل بالفعل".into());
    }
    let runtime = Arc::clone(runtime.inner());
    let control = Arc::new(Control::default());
    *runtime.control.lock().unwrap() = Some(Arc::clone(&control));
    *runtime.selection.lock().unwrap() = Some(Selection {
        input: input.clone(),
        output: output.clone(),
        mode,
    });
    *runtime.error.lock().unwrap() = None;

    std::thread::spawn(move || {
        let result = (|| -> anyhow::Result<()> {
            std::fs::create_dir_all(&output)?;
            let state_dir = output.join(".mar2i");
            std::fs::create_dir_all(&state_dir)?;
            let queue = BatchQueue::open(&state_dir.join("batch.sqlite"), &input, &output, mode)?;
            queue.recover()?;
            let _ = queue.reset_skipped();
            let files = discover(&input)?;
            for file in files {
                queue.enqueue(
                    &file.relative.to_string_lossy(),
                    &file.absolute,
                    file.size,
                    modified_ns(&file.absolute)?,
                )?;
            }
            run_batch_with_options(
                &queue,
                &media_binary("ffmpeg"),
                &media_binary("ffprobe"),
                &output,
                BatchOptions {
                    mode,
                    profile,
                    preference,
                },
                &control,
            )?;
            let jobs = queue.list()?;
            write_reports(&state_dir, &jobs)?;
            Ok(())
        })();
        if let Err(error) = result {
            *runtime.error.lock().unwrap() = Some(error.to_string());
        }
        runtime.running.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
fn batch_status(runtime: State<'_, Arc<Runtime>>) -> Result<StatusResult, String> {
    let selection = runtime.selection.lock().unwrap().clone();
    let control = runtime.control.lock().unwrap().clone();
    let jobs = if let Some(selected) = selection {
        let db = selected.output.join(".mar2i/batch.sqlite");
        if db.is_file() {
            let queue = BatchQueue::open(&db, &selected.input, &selected.output, selected.mode)
                .map_err(|error| error.to_string())?;
            queue.list().map_err(|error| error.to_string())?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let summary = summarize(&jobs);
    let queued = jobs
        .iter()
        .filter(|job| matches!(job.status.as_str(), "queued" | "interrupted"))
        .count();
    let active = jobs
        .iter()
        .filter(|job| matches!(job.status.as_str(), "processing" | "verifying"))
        .count();
    let views = jobs
        .into_iter()
        .map(|job| JobView {
            relative: job.relative,
            status: job.status,
            input_bytes: job.input_bytes,
            output_bytes: job.output_bytes,
            output: job.output,
            reason: job.reason,
            quality: job.quality,
            elapsed_ms: job.elapsed_ms,
        })
        .collect();
    Ok(StatusResult {
        running: runtime.running.load(Ordering::SeqCst),
        paused: control.as_ref().is_some_and(|value| value.is_paused()),
        cancelled: control.as_ref().is_some_and(|value| value.is_cancelled()),
        error: runtime.error.lock().unwrap().clone(),
        files: summary.files,
        completed: summary.completed,
        skipped: summary.skipped,
        failed: summary.failed,
        queued,
        active,
        input_bytes: summary.original_bytes,
        output_bytes: summary.published_bytes,
        verified_savings_bytes: summary.verified_savings_bytes,
        jobs: views,
    })
}

#[tauri::command]
fn control_batch(action: String, runtime: State<'_, Arc<Runtime>>) -> Result<(), String> {
    let control = runtime
        .control
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "لا توجد دفعة بدأت معالجتها بعد".to_owned())?;
    match action.as_str() {
        "pause" => control.pause(),
        "resume" => control.resume(),
        "cancel" => control.cancel(),
        _ => return Err("إجراء غير معروف".into()),
    }
    Ok(())
}

#[tauri::command]
fn open_output_directory(path: Option<String>, runtime: State<'_, Arc<Runtime>>) -> Result<(), String> {
    let target = if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        PathBuf::from(p)
    } else {
        let selected = runtime
            .selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "لم يتم تحديد مجلد النتائج بعد".to_owned())?;
        selected.output
    };

    if !target.exists() {
        std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    }

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&target).status();
    #[cfg(target_os = "windows")]
    let result = {
        let path_str = target.to_string_lossy().to_string();
        let clean = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
        std::process::Command::new("explorer").arg(clean).status()
    };
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&target).status();

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err("تعذر فتح مجلد النتائج عبر نظام التشغيل".into()),
        Err(error) => Err(error.to_string()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchSavingsEstimate {
    original_bytes: u64,
    estimated_bytes: u64,
    savings_bytes: i64,
    savings_percent: i32,
    files_count: usize,
}

#[tauri::command]
fn detect_hardware_encoders() -> Result<mar2i_core::hardware::HardwareReport, String> {
    let ffmpeg = media_binary("ffmpeg");
    Ok(mar2i_core::hardware::detect_hardware(&ffmpeg))
}

#[tauri::command]
fn get_ministry_profiles() -> Result<Vec<mar2i_core::profiles::MinistryProfile>, String> {
    Ok(mar2i_core::profiles::standard_profiles())
}

#[tauri::command]
fn fingerprint_media(path: String) -> Result<mar2i_core::fingerprint::MediaFingerprint, String> {
    mar2i_core::fingerprint::compute_fingerprint(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn estimate_batch_savings(
    input: String,
    mode: String,
    codec: String,
    crf: Option<u8>,
) -> Result<BatchSavingsEstimate, String> {
    let mode_enum = parse_mode(&mode)?;
    let preference = parse_codec(&codec).unwrap_or(CodecPreference::Auto);
    let input_path = PathBuf::from(input);
    let discovered = discover(&input_path).map_err(|e| e.to_string())?;
    let ffprobe = media_binary("ffprobe");
    let ffmpeg = media_binary("ffmpeg");
    let crf_val = crf.unwrap_or(22);

    let mut total_original = 0u64;
    let mut total_estimated = 0u64;
    let mut files_counted = 0usize;

    for file in &discovered {
        total_original = total_original.saturating_add(file.size);
        if let Ok(media) = inspect(&ffprobe, &file.absolute) {
            match choose_plan_with_preference(&media.profile, mode_enum, preference, None) {
                Decision::Skip { .. } => {
                    total_estimated = total_estimated.saturating_add(file.size);
                }
                Decision::Copy { .. } => {
                    total_estimated = total_estimated.saturating_add(file.size);
                    files_counted += 1;
                }
                Decision::EncodeAbr { target_kbps, .. } => {
                    let duration = media.duration;
                    let audio_bps = media
                        .audio_streams
                        .first()
                        .and_then(|a| a.bit_rate)
                        .unwrap_or(128_000);
                    let est_total_bps = (target_kbps as u64 * 1000) + audio_bps;
                    let est_bytes = ((est_total_bps as f64 * duration) / 8.0) as u64;
                    total_estimated = total_estimated.saturating_add(est_bytes.min(file.size));
                    files_counted += 1;
                }
                Decision::Encode { .. } => {
                    let est = mar2i_core::estimate::estimate_video_size(
                        &ffmpeg,
                        &file.absolute,
                        &media,
                        &codec,
                        crf_val,
                        mode_enum,
                    );
                    match est {
                        Ok(res) => {
                            total_estimated =
                                total_estimated.saturating_add(res.estimated_output_bytes);
                            files_counted += 1;
                        }
                        Err(_) => {
                            total_estimated = total_estimated.saturating_add(file.size);
                        }
                    }
                }
            }
        } else {
            total_estimated = total_estimated.saturating_add(file.size);
        }
    }

    let savings_bytes = total_original as i64 - total_estimated as i64;
    let savings_percent = if total_original > 0 {
        ((savings_bytes as f64 / total_original as f64) * 100.0).round() as i32
    } else {
        0
    };

    Ok(BatchSavingsEstimate {
        original_bytes: total_original,
        estimated_bytes: total_estimated,
        savings_bytes,
        savings_percent,
        files_count: files_counted,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectedMedia {
    pub path: String,
    pub file_size: u64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
    pub video_codec: String,
    pub video_bitrate: Option<u64>,
    pub total_bitrate: Option<u64>,
    pub audio_codec: Option<String>,
    pub audio_bitrate: Option<u64>,
    pub target_abr_kbps: Option<u32>,
    pub recommended_action: String,
}

#[tauri::command]
fn inspect_media_file(path: String) -> Result<InspectedMedia, String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err("الملف غير موجود".into());
    }
    let file_size = std::fs::metadata(&p).map_err(|e| e.to_string())?.len();
    let ffprobe = media_binary("ffprobe");
    let media = inspect(&ffprobe, &p).map_err(|e| e.to_string())?;

    let plan = choose_plan_with_preference(
        &media.profile,
        Mode::Publishing,
        CodecPreference::SmartAbr,
        None,
    );
    let (target_abr_kbps, recommended_action) = match plan {
        Decision::EncodeAbr { target_kbps, .. } => (
            Some(target_kbps),
            format!("ضغط ذكي 50% بنظام 2-Pass ({target_kbps} kbps)"),
        ),
        Decision::Copy { reason } => (None, format!("إبقاء دون تعديل: {reason}")),
        Decision::Skip { reason } => (None, format!("تخطي: {reason}")),
        _ => (None, "معالجة قياسية".into()),
    };

    let audio_bitrate = media.audio_streams.first().and_then(|a| a.bit_rate);
    let total_bitrate = if media.duration > 0.0 {
        Some(((file_size as f64 * 8.0) / media.duration).round() as u64)
    } else {
        None
    };

    Ok(InspectedMedia {
        path,
        file_size,
        width: media.profile.width,
        height: media.profile.height,
        fps: media.profile.frame_rate,
        duration_seconds: media.duration,
        video_codec: media.profile.codec,
        video_bitrate: media.profile.video_bitrate,
        total_bitrate,
        audio_codec: media.profile.audio_codecs.first().cloned(),
        audio_bitrate,
        target_abr_kbps,
        recommended_action,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionSnapshotReport {
    pub source_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub size_reduction_percent: f64,
    pub original_video_bitrate: Option<u64>,
    pub compressed_video_bitrate: Option<u64>,
    pub original_resolution: String,
    pub compressed_resolution: String,
    pub original_fps: f64,
    pub compressed_fps: f64,
    pub video_codec: String,
    pub audio_codec: String,
    pub audio_action: String,
    pub container: String,
    pub faststart: bool,
    pub ssim: Option<f64>,
    pub psnr: Option<f64>,
    pub elapsed_seconds: f64,
}

#[tauri::command]
async fn compress_single_video(
    source_path: String,
    output_dir: Option<String>,
    _target_ratio: Option<f64>,
) -> Result<CompressionSnapshotReport, String> {
    let source = PathBuf::from(&source_path);
    if !source.is_file() {
        return Err("ملف الفيديو المصدر غير موجود".into());
    }
    let dest_dir = if let Some(out) = output_dir.filter(|s| !s.trim().is_empty()) {
        PathBuf::from(out)
    } else {
        source
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let filename = source
        .file_name()
        .ok_or_else(|| "اسم الملف غير صالح".to_string())?;
    let ffmpeg = media_binary("ffmpeg");
    let ffprobe = media_binary("ffprobe");

    let original_meta = inspect(&ffprobe, &source).map_err(|e| e.to_string())?;
    let original_size = std::fs::metadata(&source).map_err(|e| e.to_string())?.len();

    let start = std::time::Instant::now();
    let available_encoders = mar2i_core::hardware::probe_available_encoders(&ffmpeg);

    let outcome = mar2i_core::runner::run_one_with_options(
        &ffmpeg,
        &ffprobe,
        &source,
        Path::new(filename),
        &dest_dir,
        mar2i_core::runner::RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::SmartAbr,
            available_encoders: Some(&available_encoders),
        },
        None,
    )
    .map_err(|e| e.to_string())?;

    let elapsed_seconds = start.elapsed().as_secs_f64();

    let (published_path, final_size) = match outcome {
        mar2i_core::runner::RunOutcome::Accepted { output, after, .. } => (output, after),
        mar2i_core::runner::RunOutcome::Copied { output, bytes, .. } => (output, bytes),
        mar2i_core::runner::RunOutcome::Skipped { reason } => {
            return Err(format!("تم تخطي الملف: {reason}"));
        }
        mar2i_core::runner::RunOutcome::Rejected { reason } => {
            return Err(format!(
                "تم رفض الضغط لعدم استيفاء معايير الجودة: {reason}"
            ));
        }
    };

    let comp_meta = inspect(&ffprobe, &published_path).ok();
    let compressed_video_bitrate = comp_meta.as_ref().and_then(|m| m.profile.video_bitrate);
    let compressed_resolution = comp_meta
        .as_ref()
        .map(|m| format!("{}x{}", m.profile.width, m.profile.height))
        .unwrap_or_else(|| {
            format!(
                "{}x{}",
                original_meta.profile.width, original_meta.profile.height
            )
        });
    let compressed_fps = comp_meta
        .as_ref()
        .map(|m| m.profile.frame_rate)
        .unwrap_or(original_meta.profile.frame_rate);
    let out_audio = comp_meta
        .as_ref()
        .and_then(|m| m.profile.audio_codecs.first().cloned())
        .unwrap_or_else(|| "aac".into());

    let ssim = mar2i_core::runner::ssim_score(&ffmpeg, &source, &published_path, None).ok();
    let psnr = mar2i_core::runner::psnr_score(&ffmpeg, &source, &published_path, None).ok();

    let reduction = if original_size > 0 && final_size < original_size {
        ((original_size - final_size) as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    let faststart = if let Ok(bytes) = std::fs::read(&published_path) {
        let window = &bytes[..bytes.len().min(4096)];
        window.windows(4).any(|w| w == b"moov")
    } else {
        false
    };

    Ok(CompressionSnapshotReport {
        source_path,
        output_path: published_path.to_string_lossy().into_owned(),
        original_size,
        compressed_size: final_size,
        size_reduction_percent: (reduction * 10.0).round() / 10.0,
        original_video_bitrate: original_meta.profile.video_bitrate,
        compressed_video_bitrate,
        original_resolution: format!(
            "{}x{}",
            original_meta.profile.width, original_meta.profile.height
        ),
        compressed_resolution,
        original_fps: original_meta.profile.frame_rate,
        compressed_fps,
        video_codec: "H.264 (libx264 2-Pass ABR)".into(),
        audio_codec: out_audio,
        audio_action: if original_meta
            .profile
            .audio_codecs
            .iter()
            .all(|c| c == "aac")
        {
            "نسخ مباشر دون إعادة ترميز (-c:a copy)".into()
        } else {
            "تحويل مباشر إلى AAC 128k".into()
        },
        container: "MP4 (ISO/IEC 14496-14)".into(),
        faststart,
        ssim,
        psnr,
        elapsed_seconds: (elapsed_seconds * 10.0).round() / 10.0,
    })
}

#[tauri::command]
fn report_content(format: String, runtime: State<'_, Arc<Runtime>>) -> Result<String, String> {
    let selected = runtime
        .selection
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "لم تبدأ أي دفعة بعد".to_owned())?;
    let file = match format.as_str() {
        "csv" => "report.csv",
        "html" => "report.html",
        _ => return Err("صيغة التقرير غير مدعومة حاليًا".into()),
    };
    std::fs::read_to_string(selected.output.join(".mar2i").join(file))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_report_file(
    format: String,
    destination: String,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<String, String> {
    let selected = runtime
        .selection
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "لم تبدأ أي دفعة بعد".to_owned())?;
    let file = match format.as_str() {
        "csv" | "excel" => "report.csv",
        "html" => "report.html",
        _ => return Err("صيغة التقرير غير مدعومة حاليًا".into()),
    };
    let state_dir = selected.output.join(".mar2i");
    let src = state_dir.join(file);
    if !src.is_file() {
        let db = state_dir.join("batch.sqlite");
        if db.is_file()
            && let Ok(queue) = BatchQueue::open(&db, &selected.input, &selected.output, selected.mode)
            && let Ok(jobs) = queue.list()
        {
            let _ = write_reports(&state_dir, &jobs);
        }
    }
    if !src.is_file() {
        return Err("ملف التقرير غير متوفر في مجلد الدفعة".into());
    }
    let dest_path = PathBuf::from(&destination);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&src, &dest_path).map_err(|e| format!("فشل نسخ التقرير إلى الوجهة المحددة: {e}"))?;
    Ok(destination)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Runtime::default()))
        .invoke_handler(tauri::generate_handler![
            analyze_directory,
            start_batch,
            batch_status,
            control_batch,
            open_output_directory,
            report_content,
            save_report_file,
            detect_hardware_encoders,
            get_ministry_profiles,
            fingerprint_media,
            estimate_batch_savings,
            inspect_media_file,
            compress_single_video
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building Tauri application");
}

#[cfg(test)]
mod tests {
    use super::{analyze_directory, resolve_media_binary};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn analyze_directory_handles_empty_folder() {
        let tmp = tempdir().unwrap();
        let in_dir = tmp.path().join("in");
        let out_dir = tmp.path().join("out");
        fs::create_dir_all(&in_dir).unwrap();
        fs::create_dir_all(&out_dir).unwrap();
        let res = tauri::async_runtime::block_on(analyze_directory(
            in_dir.to_str().unwrap().to_string(),
            out_dir.to_str().unwrap().to_string(),
            "publish".to_string(),
            None,
        )).unwrap();
        assert_eq!(res.total, 0);
    }

    #[test]
    fn test_analyze_user_dirs() {
        use std::path::Path;
        if Path::new("/Users/mamdouhaboammar/Documents/videos/in").exists() {
            let res = tauri::async_runtime::block_on(analyze_directory(
                "/Users/mamdouhaboammar/Documents/videos/in".into(),
                "/Users/mamdouhaboammar/Documents/videos/out".into(),
                "publish".into(),
                Some("hevc".into()),
            )).unwrap();
            println!("Analyze result details: {:?}", res);
            assert_eq!(res.supported, 1);
        }
    }

    #[test]
    fn resolve_media_binary_prioritizes_bundled_resources() {
        let tmp = tempdir().unwrap();
        let app_dir = tmp.path().join("Mar2i.app/Contents/MacOS");
        let res_bin_dir = tmp.path().join("Mar2i.app/Contents/Resources/bin");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&res_bin_dir).unwrap();

        let fake_exe = app_dir.join("Mar2i");
        fs::write(&fake_exe, b"executable").unwrap();

        let bundled_ffmpeg = res_bin_dir.join("ffmpeg");
        fs::write(&bundled_ffmpeg, b"bundled ffmpeg binary").unwrap();

        let resolved = resolve_media_binary("ffmpeg", Some(&fake_exe));
        assert_eq!(
            std::fs::canonicalize(resolved).unwrap(),
            std::fs::canonicalize(bundled_ffmpeg).unwrap(),
            "offline bundled resource binary must take precedence over system PATH"
        );
    }

    #[test]
    fn resolve_media_binary_prioritizes_explicit_env_override() {
        let tmp = tempdir().unwrap();
        let custom_ffmpeg = tmp.path().join("custom_ffmpeg");
        fs::write(&custom_ffmpeg, b"custom binary").unwrap();

        // Safety: tests run single-threaded in this module
        unsafe {
            std::env::set_var("MAR2I_FFMPEG_PATH", &custom_ffmpeg);
        }

        let resolved = resolve_media_binary("ffmpeg", None);
        assert_eq!(resolved, custom_ffmpeg);

        unsafe {
            std::env::remove_var("MAR2I_FFMPEG_PATH");
        }
    }
}
