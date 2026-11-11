use crate::queue::Job;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    pub files: usize,
    pub completed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub original_bytes: u64,
    pub published_bytes: u64,
    pub verified_savings_bytes: u64,
}

fn savings(row: &Job) -> u64 {
    if row.status == "completed" {
        row.output_bytes.map(|bytes| row.input_bytes.saturating_sub(bytes)).unwrap_or(0)
    } else {
        0
    }
}

pub fn summarize(jobs: &[Job]) -> Summary {
    let mut summary = Summary { files: jobs.len(), ..Summary::default() };
    for row in jobs {
        summary.original_bytes = summary.original_bytes.saturating_add(row.input_bytes);
        summary.published_bytes = summary.published_bytes.saturating_add(row.output_bytes.unwrap_or(0));
        summary.verified_savings_bytes = summary.verified_savings_bytes.saturating_add(savings(row));
        match row.status.as_str() {
            "completed" => summary.completed += 1,
            "skipped" => summary.skipped += 1,
            "failed" => summary.failed += 1,
            _ => {}
        }
    }
    summary
}

fn csv_cell(value: &str) -> String {
    let sanitized = value.replace(['\r', '\n', '\t'], " ");
    let potentially_executable = matches!(sanitized.trim_start().chars().next(), Some('=' | '+' | '-' | '@'));
    let safe = if potentially_executable {
        format!("'{sanitized}")
    } else {
        sanitized
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

pub fn report_csv(jobs: &[Job]) -> String {
    let mut csv = String::from("\u{FEFF}");
    csv.push_str("File,Relative source,Destination,Original bytes,Accepted output bytes,Verified saved bytes,Savings %,Duration ms,Throughput bytes/sec,Quality,Status,Reason\n");
    for row in jobs {
        let file = std::path::Path::new(&row.relative)
            .file_name().and_then(|value| value.to_str()).unwrap_or(&row.relative);
        let saved = savings(row);
        let percent = if row.input_bytes == 0 { 0.0 } else {
            100.0 * saved as f64 / row.input_bytes as f64
        };
        let speed = match row.elapsed_ms {
            Some(milliseconds) if milliseconds > 0 => (row.input_bytes as f64 * 1000.0 / milliseconds as f64).round() as u64,
            _ => 0,
        };
        let fields = [
            csv_cell(file),
            csv_cell(&row.relative),
            csv_cell(row.output.as_deref().unwrap_or("")),
            row.input_bytes.to_string(),
            row.output_bytes.map(|value| value.to_string()).unwrap_or_default(),
            saved.to_string(),
            format!("{percent:.2}"),
            row.elapsed_ms.map(|value| value.to_string()).unwrap_or_default(),
            speed.to_string(),
            csv_cell(row.quality.as_deref().unwrap_or("")),
            csv_cell(&row.status),
            csv_cell(row.reason.as_deref().unwrap_or("")),
        ];
        csv.push_str(&fields.join(","));
        csv.push('\n');
    }
    csv
}

fn html_cell(value: &str) -> String {
    value.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn report_html(jobs: &[Job]) -> String {
    let totals = summarize(jobs);
    let mut html = format!(
        "<!doctype html><html lang=\"ar\" dir=\"rtl\"><meta charset=\"utf-8\"><title>مرئي: تقرير الدفعة</title>\
         <style>body{{font-family:Tahoma,Arial,sans-serif;margin:36px;color:#20392f}}\
         table{{width:100%;border-collapse:collapse}}td,th{{padding:9px;border-bottom:1px solid #ccd5ce;text-align:right}}\
         th{{background:#eef3ef}}.stats{{margin:20px 0}}</style>\
         <h1>تقرير مرئي</h1><p class=\"stats\">الملفات: {} | مكتمل: {} | دون ضغط: {} | فشل: {} | المساحة الموفرة المتحقق منها: {} بايت</p>\
         <table><thead><tr><th>الملف</th><th>المصدر</th><th>المخرجات</th><th>قبل (بايت)</th><th>بعد (بايت)</th><th>توفير (بايت)</th><th>الحالة</th><th>التحقق</th><th>السبب</th></tr></thead><tbody>",
        totals.files, totals.completed, totals.skipped, totals.failed, totals.verified_savings_bytes
    );
    for job in jobs {
        let name = std::path::Path::new(&job.relative)
            .file_name().and_then(|value| value.to_str()).unwrap_or(&job.relative);
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_cell(name), html_cell(&job.relative), html_cell(job.output.as_deref().unwrap_or("")),
            job.input_bytes, job.output_bytes.map(|value| value.to_string()).unwrap_or_default(),
            savings(job), html_cell(&job.status), html_cell(job.quality.as_deref().unwrap_or("")),
            html_cell(job.reason.as_deref().unwrap_or(""))
        ));
    }
    html.push_str("</tbody></table></html>");
    html
}
