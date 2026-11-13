use serde::{Deserialize, Serialize};

pub mod coordination;
pub mod game_build;
pub mod readiness;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
}

impl AppError {
    pub fn new(code: &str) -> Self {
        Self { code: code.into() }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Auto,
    En,
    Fr,
    De,
    It,
    Es,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub language: Language,
    pub installation_path: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            language: Language::Auto,
            installation_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Passed,
    Warning,
    Error,
    Unverified,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub code: String,
    pub status: CheckStatus,
    pub value: Option<String>,
}

impl Check {
    pub fn new(code: &str, status: CheckStatus, value: Option<String>) -> Self {
        Self {
            code: code.into(),
            status,
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeInfo {
    pub name: String,
    pub architecture: String,
    pub sha256: String,
    pub bytes: u64,
    pub imports: Vec<String>,
    pub imports_inspected: bool,
    pub large_address_aware: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub schema_version: u32,
    pub root: String,
    pub scanned_at: u64,
    pub recognized: bool,
    pub verification: String,
    pub repair_status: String,
    pub files: Vec<FileEntry>,
    pub file_count: usize,
    pub total_bytes: u64,
    pub pkz_count: usize,
    pub binaries: Vec<PeInfo>,
    pub checks: Vec<Check>,
}

impl ScanReport {
    pub fn finalize(&mut self) {
        self.file_count = self.files.len();
        self.verification = if self.recognized {
            "UNVERIFIED"
        } else {
            "INVALID"
        }
        .into();
        self.repair_status = if self.checks.iter().any(|c| c.status == CheckStatus::Error) {
            "REPAIR_REQUIRED"
        } else {
            "UNKNOWN"
        }
        .into();
    }

    pub fn redacted(&self) -> Self {
        let mut report = self.clone();
        report.root = "<installation>".into();
        report.files.clear();
        report.binaries.retain(|binary| {
            ["Game.exe", "d3d9.dll", "d3dx9_43.dll"]
                .iter()
                .any(|name| binary.name.eq_ignore_ascii_case(name))
        });
        for check in &mut report.checks {
            if matches!(check.code.as_str(), "localBinary" | "binaryArchitecture") {
                check.value = None;
            }
        }
        report
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub stage: String,
    pub files_scanned: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_never_promotes_verification() {
        let mut report = ScanReport {
            schema_version: 1,
            root: "private".into(),
            scanned_at: 0,
            recognized: true,
            verification: "VERIFIED".into(),
            repair_status: String::new(),
            files: vec![],
            file_count: 0,
            total_bytes: 0,
            pkz_count: 0,
            binaries: vec![],
            checks: vec![],
        };
        report.finalize();
        assert_eq!(report.verification, "UNVERIFIED");
        assert_eq!(report.repair_status, "UNKNOWN");
        assert_eq!(report.redacted().root, "<installation>");
    }

    #[test]
    fn preferences_reject_unknown_language_and_preserve_schema() {
        assert!(serde_json::from_str::<Language>("\"unknown\"").is_err());
        for code in ["fr", "en", "de", "it", "es", "auto"] {
            let encoded = format!("\"{code}\"");
            let language = serde_json::from_str::<Language>(&encoded).unwrap();
            assert_eq!(serde_json::to_string(&language).unwrap(), encoded);
        }
        let prefs = Preferences::default();
        assert_eq!(
            serde_json::from_str::<Preferences>(&serde_json::to_string(&prefs).unwrap()).unwrap(),
            prefs
        );
    }
}
