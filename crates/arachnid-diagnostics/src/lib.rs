use arachnid_core::{Check, CheckStatus, ScanReport};

pub fn evaluate(report: &mut ScanReport) {
    inspect_display(report);
    inspect_dependencies(report);
    if let Some(game) = report
        .binaries
        .iter()
        .find(|pe| pe.name.eq_ignore_ascii_case("Game.exe"))
    {
        report.checks.push(Check::new(
            "gameBuild",
            if arachnid_core::game_build::supported(&game.sha256) {
                CheckStatus::Passed
            } else {
                CheckStatus::Warning
            },
            None,
        ));
    }
    report
        .checks
        .push(Check::new("runtimeUntested", CheckStatus::Unverified, None));
    report.finalize();
}

#[cfg(windows)]
fn inspect_display(report: &mut ScanReport) {
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY},
    };
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Activision\ASM\Settings\Display",
        KEY_READ | KEY_WOW64_32KEY,
    );
    match key {
        Ok(key) => {
            let width = key.get_value::<u32, _>("Width");
            let height = key.get_value::<u32, _>("Height");
            match (width, height) {
                (Ok(w), Ok(h)) if (320..=16384).contains(&w) && (200..=16384).contains(&h) => {
                    report.checks.push(Check::new(
                        "displaySettings",
                        CheckStatus::Passed,
                        Some(format!("{w} × {h}")),
                    ))
                }
                _ => report
                    .checks
                    .push(Check::new("displaySettings", CheckStatus::Warning, None)),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => report
            .checks
            .push(Check::new("displayAbsent", CheckStatus::Unverified, None)),
        Err(_) => report
            .checks
            .push(Check::new("displayUnreadable", CheckStatus::Warning, None)),
    }
}

#[cfg(not(windows))]
fn inspect_display(report: &mut ScanReport) {
    report
        .checks
        .push(Check::new("windowsOnly", CheckStatus::Skipped, None));
}

#[cfg(windows)]
fn inspect_dependencies(report: &mut ScanReport) {
    let windows = std::env::var_os("SystemRoot").map(std::path::PathBuf::from);
    for (name, code) in [("d3d9.dll", "direct3d"), ("d3dx9_43.dll", "legacyDirectx")] {
        let local = report
            .binaries
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name));
        let system = windows.as_ref().map(|p| p.join("SysWOW64").join(name));
        let status = match local {
            Some(pe) if pe.architecture == "x86" => CheckStatus::Unverified,
            Some(_) => CheckStatus::Error,
            None if system.as_ref().is_some_and(|p| {
                arachnid_detector::inspect_pe(p, name, &std::sync::atomic::AtomicBool::new(false))
                    .is_ok_and(|pe| pe.architecture == "x86")
            }) =>
            {
                CheckStatus::Passed
            }
            None => CheckStatus::Warning,
        };
        report
            .checks
            .push(Check::new(code, status, local.map(|_| name.to_string())));
    }
    let imports: std::collections::BTreeSet<_> = report
        .binaries
        .iter()
        .filter(|pe| pe.name.eq_ignore_ascii_case("Game.exe"))
        .flat_map(|pe| pe.imports.iter())
        .map(|name| name.to_ascii_lowercase())
        .filter(|name| {
            name.ends_with(".dll")
                && (name.starts_with("msvcr")
                    || name.starts_with("msvcp")
                    || name.starts_with("vcruntime"))
        })
        .collect();
    let missing = imports
        .iter()
        .filter(|name| {
            let local = report
                .binaries
                .iter()
                .any(|pe| pe.name.eq_ignore_ascii_case(name) && pe.architecture == "x86");
            let system = windows
                .as_ref()
                .map(|root| root.join("SysWOW64").join(name));
            !local
                && !system.is_some_and(|path| {
                    arachnid_detector::inspect_pe(
                        &path,
                        name,
                        &std::sync::atomic::AtomicBool::new(false),
                    )
                    .is_ok_and(|pe| pe.architecture == "x86")
                })
        })
        .map(String::as_str)
        .collect::<Vec<_>>();
    report.checks.push(Check::new(
        "visualCpp",
        if imports.is_empty() {
            CheckStatus::Skipped
        } else if !missing.is_empty() {
            CheckStatus::Warning
        } else {
            CheckStatus::Passed
        },
        (!missing.is_empty()).then(|| missing.join(", ")),
    ));
}

#[cfg(not(windows))]
fn inspect_dependencies(_: &mut ScanReport) {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_never_claim_runtime_validation() {
        let mut report = ScanReport {
            schema_version: 1,
            root: "test".into(),
            scanned_at: 0,
            recognized: true,
            verification: String::new(),
            repair_status: String::new(),
            files: vec![],
            file_count: 0,
            total_bytes: 0,
            pkz_count: 1,
            binaries: vec![],
            checks: vec![],
        };
        evaluate(&mut report);
        assert_eq!(report.verification, "UNVERIFIED");
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.code == "runtimeUntested" && c.status == CheckStatus::Unverified)
        );
    }
}
