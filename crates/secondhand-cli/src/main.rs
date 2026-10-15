//! `secondhand-cli [--format ai|pdf|svg] [--json REPORT] PATH…`
//!
//! Converts every FreeHand drawing in the given files and folders and writes the result next to
//! each original. Exits 0 when nothing failed, 1 when a file failed, 2 on bad arguments.

use std::path::PathBuf;

use secondhand_core::{Event, Format, Outcome};

fn main() {
    let mut format = Format::Ai;
    let mut json = None::<PathBuf>;
    let mut paths = Vec::new();
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--format" | "-f") => match args.next().and_then(|v| v.to_str().and_then(Format::parse)) {
                Some(f) => format = f,
                None => usage("--format needs ai, pdf or svg"),
            },
            Some("--json") => json = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage("--json needs a file"))),
            Some("--help" | "-h") => usage(""),
            Some("--version" | "-V") => {
                println!("secondhand-cli {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        usage("give at least one file or folder");
    }
    let summary = secondhand_core::run(&paths, format, |e| match e {
        Event::Start { total } => eprintln!("{total} file(s) to convert"),
        Event::Begin { index, total, input } => eprintln!("[{}/{total}] {}", index + 1, input.display()),
        Event::Done { outcome, .. } => match outcome {
            Outcome::Converted { outputs, .. } => eprintln!("    ok → {}", outputs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")),
            Outcome::Skipped { reason, .. } => eprintln!("    skipped: {reason}"),
            Outcome::Failed { reason, .. } => eprintln!("    failed: {reason}"),
        },
    });
    print!("{}", summary.report());
    if let Some(path) = json {
        let text = serde_json::to_string_pretty(&summary).unwrap_or_default();
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("couldn't write {}: {e}", path.display());
        }
    }
    std::process::exit(if summary.failed.is_empty() { 0 } else { 1 });
}

fn usage(msg: &str) -> ! {
    if !msg.is_empty() {
        eprintln!("secondhand-cli: {msg}\n");
    }
    eprintln!("usage: secondhand-cli [--format ai|pdf|svg] [--json REPORT.json] FILE_OR_FOLDER…");
    eprintln!("Converts FreeHand 8, 9, 10 and MX drawings; writes Name.ai next to each one, never overwriting.");
    std::process::exit(if msg.is_empty() { 0 } else { 2 });
}
