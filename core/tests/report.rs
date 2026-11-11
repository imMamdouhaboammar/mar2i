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
