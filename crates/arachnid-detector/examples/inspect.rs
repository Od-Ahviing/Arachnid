use std::{path::PathBuf, sync::atomic::AtomicBool};

fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: cargo run -p arachnid-detector --example inspect -- <installation>");
        std::process::exit(2);
    };
    match arachnid_detector::scan(&PathBuf::from(path), &AtomicBool::new(false), |_| {}) {
        Ok(report) => {
            println!(
                "recognized={} verification={} files={} pkz={} binaries={}",
                report.recognized,
                report.verification,
                report.files.len(),
                report.pkz_count,
                report.binaries.len()
            );
            for check in report.checks {
                println!("{}: {:?}", check.code, check.status);
            }
            if !report.recognized {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{}", error.code);
            std::process::exit(1);
        }
    }
}
