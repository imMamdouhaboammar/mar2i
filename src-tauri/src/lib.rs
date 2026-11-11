use mar2i_core::discovery::{discover, validate_directories};
use mar2i_core::media::inspect;
use mar2i_core::policy::{Decision, Mode, choose_plan};
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{report_csv, report_html, summarize};
use mar2i_core::worker::{Control, ResourceProfile, modified_ns, run_batch};
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

#[derive(Serialize)]
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

fn media_binary(name: &str) -> PathBuf {
    let filename = if cfg!(windows) { format!("{name}.exe") } else { name.to_owned() };
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            for candidate in [
                parent.join(&filename),
                parent.join("bin").join(&filename),
                parent.join("../Resources").join(&filename),
            ] {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    for root in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        let path = Path::new(root).join(&filename);
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from(filename)
}

fn validate_paths(input: &Path, output: &Path) -> Result<(), String> {
    validate_directories(input, output).map_err(|error| error.to_string())
}

#[tauri::command]
fn analyze_directory(input: String, output: String, mode: String) -> Result<ScanResult, String> {
    let mode = parse_mode(&mode)?;
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
            Ok(media) => match choose_plan(&media.profile, mode) {
                Decision::Skip { reason } => {
                    if details.len() < 15 {
                        details.push(format!("{}: {reason}", file.relative.display()));
                    }
                }
                _ => supported += 1,
            },
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
    if codec != "auto" {
        return Err("الترميز اليدوي غير مفعل بعد في النسخة الحالية. اختر «تلقائي» للمعالجة الفعلية".into());
    }
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
            let files = discover(&input)?;
            for file in files {
                queue.enqueue(
                    &file.relative.to_string_lossy(),
                    &file.absolute,
                    file.size,
                    modified_ns(&file.absolute)?,
                )?;
            }
            run_batch(
                &queue, &media_binary("ffmpeg"), &media_binary("ffprobe"),
                &output, mode, profile, &control,
            )?;
            let jobs = queue.list()?;
            std::fs::write(state_dir.join("report.csv"), report_csv(&jobs))?;
            std::fs::write(state_dir.join("report.html"), report_html(&jobs))?;
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
    let queued = jobs.iter().filter(|job| matches!(job.status.as_str(), "queued" | "interrupted")).count();
    let active = jobs.iter().filter(|job| matches!(job.status.as_str(), "processing" | "verifying")).count();
    let views = jobs.into_iter().map(|job| JobView {
        relative: job.relative,
        status: job.status,
        input_bytes: job.input_bytes,
        output_bytes: job.output_bytes,
        output: job.output,
        reason: job.reason,
        quality: job.quality,
        elapsed_ms: job.elapsed_ms,
    }).collect();
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
    let control = runtime.control.lock().unwrap().clone()
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
fn open_output_directory(runtime: State<'_, Arc<Runtime>>) -> Result<(), String> {
    let selected = runtime.selection.lock().unwrap().clone()
        .ok_or_else(|| "لم يتم تحديد مجلد النتائج بعد".to_owned())?;
    let folder = std::fs::canonicalize(selected.output).map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&folder).status();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&folder).status();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&folder).status();
    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err("تعذر فتح مجلد النتائج".into()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
fn report_content(format: String, runtime: State<'_, Arc<Runtime>>) -> Result<String, String> {
    let selected = runtime.selection.lock().unwrap().clone()
        .ok_or_else(|| "لم تبدأ أي دفعة بعد".to_owned())?;
    let file = match format.as_str() {
        "csv" => "report.csv",
        "html" => "report.html",
        _ => return Err("صيغة التقرير غير مدعومة حاليًا".into()),
    };
    std::fs::read_to_string(selected.output.join(".mar2i").join(file))
        .map_err(|error| error.to_string())
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
            report_content
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
