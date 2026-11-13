use crate::{CheckStatus, ScanReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchDecision {
    pub ready: bool,
    pub reasons: Vec<String>,
}

pub fn decide(
    report: &ScanReport,
    channel_qualified: bool,
    recovery_pending: bool,
) -> LaunchDecision {
    let mut reasons = Vec::new();
    if !report.recognized {
        reasons.push("installationInvalid".into());
    }
    if recovery_pending {
        reasons.push("recoveryRequired".into());
    }
    if !channel_qualified {
        reasons.push("channelUnqualified".into());
    }
    for check in report
        .checks
        .iter()
        .filter(|check| check.status == CheckStatus::Error)
    {
        if !reasons.contains(&check.code) {
            reasons.push(check.code.clone());
        }
    }
    for (code, filename, local_reason) in [
        ("direct3d", "d3d9.dll", "localDirect3dUnverified"),
        (
            "legacyDirectx",
            "d3dx9_43.dll",
            "localLegacyDirectxUnverified",
        ),
    ] {
        let check = report.checks.iter().find(|check| check.code == code);
        if !check.is_some_and(|check| check.status == CheckStatus::Passed) {
            if report
                .binaries
                .iter()
                .any(|pe| pe.name.eq_ignore_ascii_case(filename))
            {
                reasons.push(local_reason.into());
            } else if !reasons.iter().any(|reason| reason == code) {
                reasons.push(format!("{code}Unresolved"));
            }
        }
    }
    if report
        .checks
        .iter()
        .any(|check| check.code == "visualCpp" && check.status == CheckStatus::Warning)
    {
        reasons.push("visualCppUnresolved".into());
    }
    LaunchDecision {
        ready: reasons.is_empty(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Check;

    #[test]
    fn confidence_and_readiness_are_independent_and_optional_wrapper_is_not_required() {
        let mut report: ScanReport = serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"root":"fixture","scannedAt":0,"recognized":true,
            "verification":"UNVERIFIED","repairStatus":"UNKNOWN","files":[],
            "fileCount":0,"totalBytes":0,"pkzCount":0,"binaries":[],"checks":[]
        }))
        .unwrap();
        for code in ["direct3d", "legacyDirectx", "wrapper"] {
            report
                .checks
                .push(Check::new(code, CheckStatus::Passed, None));
        }
        assert!(decide(&report, true, false).ready);
        assert!(!decide(&report, false, false).ready);
        assert!(!decide(&report, true, true).ready);
        report
            .checks
            .push(Check::new("gameLibrary", CheckStatus::Error, None));
        assert_eq!(decide(&report, true, false).reasons, ["gameLibrary"]);
        report.checks.pop();
        report.verification = "VERIFIED".into();
        report.checks[0].status = CheckStatus::Warning;
        assert!(!decide(&report, true, false).ready);
    }

    #[test]
    fn an_unverified_local_wrapper_has_a_distinct_launch_reason() {
        let report: ScanReport = serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"root":"fixture","scannedAt":0,"recognized":true,
            "verification":"UNVERIFIED","repairStatus":"UNKNOWN","files":[],
            "fileCount":0,"totalBytes":0,"pkzCount":1,
            "binaries":[{"name":"D3D9.dll","architecture":"x86","sha256":"fixture",
                "bytes":1,"imports":[],"importsInspected":false,"largeAddressAware":false}],
            "checks":[{"code":"direct3d","status":"unverified","value":null},
                {"code":"legacyDirectx","status":"passed","value":null}]
        }))
        .unwrap();
        assert_eq!(
            decide(&report, true, false).reasons,
            ["localDirect3dUnverified"]
        );
        assert_eq!(report.redacted().binaries[0].name, "D3D9.dll");
    }
}
