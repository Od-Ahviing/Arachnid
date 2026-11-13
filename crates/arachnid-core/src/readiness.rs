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
    if report
        .checks
        .iter()
        .any(|check| check.status == CheckStatus::Error)
    {
        reasons.push("diagnosticBlocking".into());
    }
    for code in ["direct3d", "legacyDirectx"] {
        if !report
            .checks
            .iter()
            .any(|check| check.code == code && check.status == CheckStatus::Passed)
        {
            reasons.push(format!("{code}Unresolved"));
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
        report.verification = "VERIFIED".into();
        report.checks[0].status = CheckStatus::Warning;
        assert!(!decide(&report, true, false).ready);
    }
}
