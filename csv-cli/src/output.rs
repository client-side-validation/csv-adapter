//! Output formatting helpers

use colored::Colorize;

pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

pub fn warning(msg: &str) {
    println!("{} {}", "⚠".yellow().bold(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "ℹ".blue().bold(), msg);
}

pub fn header(title: &str) {
    println!("\n{}", title.bold().underline());
    println!("{}", "─".repeat(60).dimmed());
}

pub fn kv(key: &str, value: &str) {
    println!("  {:<25} {}", format!("{}:", key).bold(), value);
}

pub fn kv_hash(key: &str, hash: &[u8]) {
    println!("  {:<25} {}", format!("{}:", key).bold(), hex::encode(hash).dimmed());
}

pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Print headers
    let header_line: String = headers.iter().enumerate()
        .map(|(i, h)| format!("  {:<width$}", h, width = widths[i]))
        .collect();
    println!("{}", header_line.bold());
    println!("{}", "─".repeat(widths.iter().sum::<usize>() + widths.len() * 2).dimmed());

    // Print rows
    for row in rows {
        let row_line: String = row.iter().enumerate()
            .map(|(i, c)| format!("  {:<width$}", c, width = widths[i]))
            .collect();
        println!("{}", row_line);
    }
}

pub fn json<T: serde::Serialize>(data: &T) {
    match serde_json::to_string_pretty(data) {
        Ok(s) => println!("{}", s),
        Err(e) => error(&format!("Failed to serialize: {}", e)),
    }
}

pub fn progress(step: usize, total: usize, msg: &str) {
    let bar = format!("[{}/{}]", step, total).dimmed();
    println!("  {} {}", bar, msg);
}
