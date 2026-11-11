use anyhow::{Context, Result, bail};
use mar2i_core::discovery::{discover, validate_directories};
use mar2i_core::estimate::estimate_video_size;
use mar2i_core::fingerprint::compute_fingerprint;
use mar2i_core::hardware::detect_hardware;
use mar2i_core::mcp::run_mcp_server;
use mar2i_core::media::inspect;
use mar2i_core::policy::{Decision, Mode, choose_plan};
use mar2i_core::profiles::standard_profiles;
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{report_csv, report_html, summarize};
use mar2i_core::worker::{Control, ResourceProfile, modified_ns, run_batch};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
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

fn print_help() {
    println!(
        "مرئي | Offline Video Processing & Archival Suite (Saudi Ministry of Hajj & Umrah)\n\
        \n\
        Usage:\n\
          mar2i <scan|run|report> <input-dir> <output-dir> <publish|archive> [low|balanced|maximum] [ffmpeg-bin] [ffprobe-bin]\n\
          mar2i mcp [ffmpeg-bin]\n\
          mar2i probe-hw [ffmpeg-bin]\n\
          mar2i fingerprint <file-path>\n\
          mar2i profiles\n\
          mar2i estimate <file-path> [ffmpeg-bin] [ffprobe-bin] [codec] [crf]\n\
        \n\
        The original media files are never changed. Jobs resume from a local SQLite database."
    );
}

fn command_line() -> Result<()> {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_help();
        return if args.get(1).is_some_and(|a| a == "--help" || a == "-h") {
            Ok(())
        } else {
            bail!("missing arguments")
        };
    }

    let action = args[1].to_string_lossy();

    // MCP Server mode
    if action == "mcp" {
        let ffmpeg = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let stdin = io::stdin();
        let stdout = io::stdout();
        run_mcp_server(stdin.lock(), stdout.lock(), &ffmpeg);
        return Ok(());
    }

    // Hardware acceleration probe mode
    if action == "probe-hw" {
        let ffmpeg = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let report = detect_hardware(&ffmpeg);
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    // Sparse SHA-256 media fingerprint mode
    if action == "fingerprint" {
        if args.len() < 3 {
            bail!("usage: mar2i fingerprint <file-path>");
        }
        let file_path = PathBuf::from(&args[2]);
        let fp = compute_fingerprint(&file_path)?;
        println!("{}", serde_json::to_string_pretty(&fp)?);
        return Ok(());
    }

    // Ministry profiles list mode
    if action == "profiles" {
        let profiles = standard_profiles();
        println!("{}", serde_json::to_string_pretty(&profiles)?);
        return Ok(());
    }

    // Empirical size estimate mode
    if action == "estimate" {
        if args.len() < 3 {
            bail!("usage: mar2i estimate <file-path> [ffmpeg-bin] [ffprobe-bin] [codec] [crf]");
        }
        let file_path = PathBuf::from(&args[2]);
        let ffmpeg = args.get(3).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let ffprobe = args.get(4).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffprobe"));
        let codec = args.get(5).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "hevc".into());
        let crf = args.get(6).and_then(|s| s.to_string_lossy().parse::<u8>().ok()).unwrap_or(22);

        let inspected = inspect(&ffprobe, &file_path)?;
        let est = estimate_video_size(&ffmpeg, &file_path, &inspected, &codec, crf, Mode::Publishing)?;
        println!("{}", serde_json::to_string_pretty(&est)?);
        return Ok(());
    }

    if args.len() < 5 {
        print_help();
        bail!("missing arguments for action '{}'", action);
    }

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
                &file.relative.to_string_lossy(),
                &file.absolute,
                file.size,
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
        "Batch complete: total files {}, completed {}, skipped {}, failed {}, verified savings {:.2} GB",
        summary.files,
        summary.completed,
        summary.skipped,
        summary.failed,
        summary.verified_savings_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );

    Ok(())
}

fn main() {
    if let Err(error) = command_line() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}
