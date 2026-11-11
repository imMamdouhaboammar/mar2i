use anyhow::{Context, Result, bail};
use mar2i_core::discovery::{discover, validate_directories};
use mar2i_core::media::inspect;
use mar2i_core::policy::{Decision, Mode, choose_plan};
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{report_csv, report_html, summarize};
use mar2i_core::worker::{Control, ResourceProfile, modified_ns, run_batch};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

fn mode_from(text: &str) -> Result<Mode> {
    match text {
        "publish" | "publishing" => Ok(Mode::Publishing),
        "archive" | "archival" => Ok(Mode::Archival),
        _ => bail!("mode must be publish or archive"),
    }
}

fn profile_from(text: &str) -> Result<ResourceProfile> {
    match text {
        "low" => Ok(ResourceProfile::Low),
        "balanced" => Ok(ResourceProfile::Balanced),
        "maximum" | "max" => Ok(ResourceProfile::Maximum),
        _ => bail!("resource profile must be low, balanced or maximum"),
    }
}

fn atomic_report(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().context("missing report directory")?;
    let mut file = NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut file, contents)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn command_line() -> Result<()> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 5 || args[1] == "--help" {
        println!("مرئي | Offline video processing\n\
Usage: mar2i <scan|run|report> <input-dir> <output-dir> <publish|archive> [low|balanced|maximum] [ffmpeg-bin] [ffprobe-bin]\n\
The original media files are never changed. Jobs resume from a local SQLite database.");
        return if args.get(1).is_some_and(|a| a == "--help") { Ok(()) } else { bail!("missing arguments") };
    }
    let action = args[1].to_string_lossy();
    let input = PathBuf::from(&args[2]);
    let output = PathBuf::from(&args[3]);
    let mode = mode_from(&args[4].to_string_lossy())?;
    let profile = profile_from(&args.get(5).map(|value| value.to_string_lossy()).unwrap_or("balanced".into()))?;
    let ffmpeg = args.get(6).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let ffprobe = args.get(7).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffprobe"));

    validate_directories(&input, &output)?;
    let files = discover(&input)?;
    if action == "scan" {
        println!("Discovered {} candidate video files", files.len());
        for file in files {
            match inspect(&ffprobe, &file.absolute) {
                Ok(info) => {
                    let plan = choose_plan(&info.profile, mode);
                    let label = match plan {
                        Decision::Copy { ref reason } => format!("COPY: {reason}"),
                        Decision::Encode { codec, container, .. } => format!("ENCODE: {codec} -> {container}"),
                        Decision::Skip { ref reason } => format!("SKIP: {reason}"),
                    };
                    println!("{}\t{}\t{}", file.relative.display(), info.profile.codec, label);
                }
                Err(error) => println!("{}\tUNSUPPORTED: {error:#}", file.relative.display()),
            }
        }
        return Ok(());
    }

    if action != "run" && action != "report" {
        bail!("action must be scan, run or report");
    }
    fs::create_dir_all(&output)?;
    let db_dir = output.join(".mar2i");
    fs::create_dir_all(&db_dir)?;
    let queue = BatchQueue::open(&db_dir.join("batch.sqlite"), &input, &output, mode)?;
    if action == "run" {
        let interrupted = queue.recover()?;
        if interrupted != 0 {
            println!("Recovered {interrupted} interrupted jobs; unfinished encodes restart from the beginning");
        }
        for file in files {
            queue.enqueue(
                &file.relative.to_string_lossy(), &file.absolute, file.size,
                modified_ns(&file.absolute)?,
            )?;
        }
        let control = Control::default();
        run_batch(&queue, &ffmpeg, &ffprobe, &output, mode, profile, &control)?;
    }
    let jobs = queue.list()?;
    atomic_report(&db_dir.join("report.csv"), report_csv(&jobs).as_bytes())?;
    atomic_report(&db_dir.join("report.html"), report_html(&jobs).as_bytes())?;
    let summary = summarize(&jobs);
    println!(
        "files={} completed={} skipped={} failed={} verified_savings_bytes={}",
        summary.files, summary.completed, summary.skipped, summary.failed,
        summary.verified_savings_bytes
    );
    println!("Reports: {}", db_dir.display());
    if summary.failed > 0 {
        bail!("{} media files failed; inspect the report for reasons", summary.failed);
    }
    Ok(())
}

fn main() {
    if let Err(error) = command_line() {
        eprintln!("mar2i: {error:#}");
        std::process::exit(2);
    }
}
