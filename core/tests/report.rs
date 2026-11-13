use mar2i_core::queue::Job;
use mar2i_core::report::{report_csv, report_html, summarize};

fn item(id: i64, filename: &str, status: &str, before: u64, after: Option<u64>) -> Job {
    Job {
        id,
        relative: filename.to_owned(),
        source: format!("/source/{filename}"),
        input_bytes: before,
        modified_ns: 1,
        status: status.to_owned(),
        output: after.map(|_| format!("/output/{filename}")),
        output_bytes: after,
        reason: None,
        quality: Some("SSIM=0.99000".into()),
        elapsed_ms: Some(900),
    }
}

fn item_with_stale_output_bytes(
    id: i64,
    filename: &str,
    status: &str,
    before: u64,
    stale_output_bytes: u64,
) -> Job {
    // Simulates a failed job that has a stale output_bytes value (the bug scenario)
    Job {
        id,
        relative: filename.to_owned(),
        source: format!("/source/{filename}"),
        input_bytes: before,
        modified_ns: 1,
        status: status.to_owned(),
        output: None, // failed jobs have no published output path
        output_bytes: Some(stale_output_bytes),
        reason: Some("source identity changed since discovery".into()),
        quality: None,
        elapsed_ms: Some(500),
    }
}

#[test]
fn savings_include_only_accepted_smaller_outputs() {
    let rows = vec![
        item(1, "movie.mp4", "completed", 100, Some(40)),
        item(2, "already.mp4", "skipped", 50, Some(50)),
        item(3, "unprocessed.mp4", "skipped", 20, None),
        item(4, "failed.mov", "failed", 10, None),
    ];
    let totals = summarize(&rows);
    assert_eq!(totals.files, 4);
    assert_eq!(totals.completed, 1);
    assert_eq!(totals.skipped, 2);
    assert_eq!(totals.failed, 1);
    assert_eq!(totals.original_bytes, 180);
    assert_eq!(totals.published_bytes, 90);
    assert_eq!(totals.verified_savings_bytes, 60);
}

#[test]
fn published_bytes_excludes_stale_output_bytes_on_failed_jobs() {
    // A failed job may have a stale output_bytes value from a previous enqueue cycle.
    // published_bytes must NOT include bytes from failed or interrupted jobs.
    let rows = vec![
        item(1, "good.mp4", "completed", 200, Some(80)),
        item_with_stale_output_bytes(2, "bad.mp4", "failed", 100, 95),
        item_with_stale_output_bytes(3, "interrupted.mp4", "interrupted", 150, 140),
    ];
    let totals = summarize(&rows);
    assert_eq!(totals.completed, 1);
    assert_eq!(totals.failed, 1);
    // published_bytes must be 80 (only the completed job's output)
    // NOT 80 + 95 + 140 = 315 (which the old bug would produce)
    assert_eq!(
        totals.published_bytes, 80,
        "failed and interrupted jobs must not contribute to published_bytes"
    );
    assert_eq!(totals.verified_savings_bytes, 120); // 200 - 80
}

#[test]
fn published_bytes_includes_skipped_jobs_with_actual_output() {
    // Skipped (byte-identical copy) jobs have a real published output path and bytes.
    let rows = vec![
        item(1, "copy.mp4", "skipped", 50, Some(50)), // byte-identical copy has output
        item(2, "rejected.mp4", "skipped", 30, None), // rejected/skipped without output
    ];
    let totals = summarize(&rows);
    // Only the copy (skipped-with-output) counts toward published_bytes
    assert_eq!(totals.published_bytes, 50);
    assert_eq!(totals.verified_savings_bytes, 0); // copies have no savings
}

#[test]
fn csv_export_blocks_formula_injection_and_quotes_fields() {
    let rows = vec![item(
        1,
        "=HYPERLINK(\"bad\",\"run\")",
        "completed",
        100,
        Some(40),
    )];
    let csv = report_csv(&rows);
    assert!(csv.starts_with('\u{FEFF}'));
    assert!(csv.contains("'=HYPERLINK("));
    assert!(csv.contains("\"\"bad\"\""));
    assert!(csv.contains("60"));
}

#[test]
fn html_report_escapes_filenames() {
    let rows = vec![item(
        1,
        "<script>alert('x')</script>.mov",
        "completed",
        100,
        Some(40),
    )];
    let html = report_html(&rows);
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("60"));
}
