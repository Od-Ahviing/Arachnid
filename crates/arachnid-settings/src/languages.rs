use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Journal, OperationPlan, Resources, Target};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSync {
    pub requested: String,
    pub applied: String,
    pub changed: bool,
}

fn validate_sync_journal(journal: &Journal, installation: &str) -> AppResult<()> {
    super::validate_plan(&journal.plan, installation)?;
    if journal.plan.actions.len() != 1
        || !matches!(
            &journal.plan.actions[0].target,
            Target::Registry { section, name } if section == "Install" && name == "Language"
        )
    {
        return Err(AppError::new("invalidPlan"));
    }
    Ok(())
}

pub fn synchronize(
    installation: &str,
    build: &str,
    root: &Path,
    requested: &str,
    resources: &mut impl Resources,
    path: &Path,
    gate: impl Fn() -> AppResult<()>,
) -> AppResult<LanguageSync> {
    gate()?;
    if build != super::LOCAL_BUILD || !["en", "fr", "de", "it", "es"].contains(&requested) {
        return Err(AppError::new("unsupportedSetting"));
    }
    if path.exists() {
        let mut journal = Journal::load(path)?;
        validate_sync_journal(&journal, installation)?;
        if !journal.terminal() {
            arachnid_repair::rollback_guarded(&mut journal, resources, path, &gate)?;
        }
    }
    let languages = inspect(root);
    let selected = languages
        .iter()
        .find(|language| language.code == requested && language.available)
        .or_else(|| {
            languages
                .iter()
                .find(|language| language.code == "en" && language.available)
        })
        .ok_or_else(|| AppError::new("languageResourcesMissing"))?;
    let current = resources.read(&Target::Registry {
        section: "Install".into(),
        name: "Language".into(),
    })?;
    let changed = current.as_ref().and_then(super::dword) != Some(selected.value);
    if changed {
        let plan = super::plan(
            installation.into(),
            build,
            vec![super::SettingChange {
                name: "Language".into(),
                value: selected.value,
            }],
            resources,
        )?;
        validate_selection(root, &plan)?;
        if path.exists() {
            let archive = tempfile::Builder::new()
                .prefix("language-history-")
                .suffix(".json")
                .tempfile_in(
                    path.parent()
                        .ok_or_else(|| AppError::new("journalUnavailable"))?,
                )
                .map_err(|_| AppError::new("journalUnavailable"))?;
            fs::copy(path, archive.path()).map_err(|_| AppError::new("journalUnavailable"))?;
            archive
                .as_file()
                .sync_all()
                .map_err(|_| AppError::new("journalUnavailable"))?;
            archive
                .keep()
                .map_err(|_| AppError::new("journalUnavailable"))?;
        }
        let mut journal = arachnid_repair::prepare(plan, resources, path)?;
        let digest = journal.digest.clone();
        let approved = journal.plan.clone();
        arachnid_repair::apply(
            &mut journal,
            &digest,
            resources,
            path,
            &std::sync::atomic::AtomicBool::new(false),
            || {
                gate()?;
                validate_selection(root, &approved)
            },
        )?;
    }
    Ok(LanguageSync {
        requested: requested.into(),
        applied: selected.code.into(),
        changed,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLanguage {
    pub code: &'static str,
    pub native_name: &'static str,
    pub value: u32,
    pub available: bool,
    pub resource: &'static str,
}

fn ordinary_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.len() > 0 && !arachnid_detector::is_link(&metadata)
    })
}

pub fn inspect(root: &Path) -> Vec<GameLanguage> {
    [
        ("fr", "Français", 2, "Data/igct_fr.bnx"),
        ("en", "English", 1, "Data/igct.bnx"),
        ("de", "Deutsch", 4, "Data/igct_de.bnx"),
        ("it", "Italiano", 3, "Data/igct_it.bnx"),
        ("es", "Español", 5, "Data/igct_sp.bnx"),
    ]
    .into_iter()
    .map(|(code, native_name, value, resource)| {
        let directory_present = fs::symlink_metadata(root.join("Data"))
            .is_ok_and(|metadata| metadata.is_dir() && !arachnid_detector::is_link(&metadata));
        GameLanguage {
            code,
            native_name,
            value,
            available: directory_present && ordinary_file(&root.join(resource)),
            resource,
        }
    })
    .collect()
}

pub fn validate_selection(root: &Path, plan: &OperationPlan) -> AppResult<()> {
    for action in &plan.actions {
        if matches!(&action.target, Target::Registry { section, name } if section == "Install" && name == "Language")
        {
            let value = action
                .after
                .as_ref()
                .and_then(super::dword)
                .ok_or_else(|| AppError::new("unsupportedSetting"))?;
            if !inspect(root)
                .iter()
                .any(|language| language.value == value && language.available)
            {
                return Err(AppError::new("languageResourcesMissing"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Memory {
        image: Option<arachnid_repair::Image>,
        fail_write: bool,
        writes: usize,
    }
    impl Resources for Memory {
        fn read(&self, target: &Target) -> AppResult<Option<arachnid_repair::Image>> {
            assert!(
                matches!(target, Target::Registry { section, name } if section == "Install" && name == "Language")
            );
            Ok(self.image.clone())
        }
        fn write(&mut self, _: &Target, image: &Option<arachnid_repair::Image>) -> AppResult<()> {
            if self.fail_write {
                return Err(AppError::new("writeFailed"));
            }
            self.writes += 1;
            self.image = image.clone();
            Ok(())
        }
    }
    fn fixture() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("Data")).unwrap();
        fs::write(
            directory.path().join("Data/igct_fr.bnx"),
            b"synthetic French resource",
        )
        .unwrap();
        fs::write(
            directory.path().join("Data/igct.bnx"),
            b"synthetic English resource",
        )
        .unwrap();
        directory
    }
    #[test]
    fn synchronization_preserves_absence_skips_noops_and_falls_back_only_to_available_english() {
        let root = fixture();
        let journals = tempfile::tempdir().unwrap();
        let path = journals.path().join("language.json");
        let mut resources = Memory {
            image: None,
            fail_write: false,
            writes: 0,
        };
        let result = synchronize(
            "fixture",
            super::super::LOCAL_BUILD,
            root.path(),
            "fr",
            &mut resources,
            &path,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(result.applied, "fr");
        let first = Journal::load(&path).unwrap();
        assert_eq!(first.plan.actions[0].before, None);
        assert!(
            !synchronize(
                "fixture",
                super::super::LOCAL_BUILD,
                root.path(),
                "fr",
                &mut resources,
                &path,
                || Ok(())
            )
            .unwrap()
            .changed
        );
        assert_eq!(resources.writes, 1);
        assert_eq!(fs::read_dir(journals.path()).unwrap().count(), 1);
        let fallback = synchronize(
            "fixture",
            super::super::LOCAL_BUILD,
            root.path(),
            "de",
            &mut resources,
            &path,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(
            (fallback.requested.as_str(), fallback.applied.as_str()),
            ("de", "en")
        );
        assert_eq!(fs::read_dir(journals.path()).unwrap().count(), 2);
        let mut last = Journal::load(&path).unwrap();
        arachnid_repair::rollback(&mut last, &mut resources, &path).unwrap();
        assert_eq!(
            resources.image.as_ref().and_then(super::super::dword),
            Some(2)
        );
        let mut first = first;
        arachnid_repair::rollback(&mut first, &mut resources, &path).unwrap();
        assert_eq!(resources.image, None);
    }
    #[test]
    fn a_running_game_missing_resources_or_failed_write_never_claims_synchronization() {
        let root = fixture();
        let journals = tempfile::tempdir().unwrap();
        let path = journals.path().join("language.json");
        let mut resources = Memory {
            image: None,
            fail_write: false,
            writes: 0,
        };
        assert_eq!(
            synchronize(
                "fixture",
                super::super::LOCAL_BUILD,
                root.path(),
                "fr",
                &mut resources,
                &path,
                || Err(AppError::new("gameAlreadyRunning"))
            )
            .unwrap_err()
            .code,
            "gameAlreadyRunning"
        );
        assert!(!path.exists());
        resources.fail_write = true;
        assert!(
            synchronize(
                "fixture",
                super::super::LOCAL_BUILD,
                root.path(),
                "fr",
                &mut resources,
                &path,
                || Ok(())
            )
            .is_err()
        );
        assert_eq!(resources.image, None);
        assert_eq!(
            Journal::load(&path).unwrap().phase,
            arachnid_repair::Phase::RolledBack
        );
        fs::remove_file(root.path().join("Data/igct.bnx")).unwrap();
        assert_eq!(
            synchronize(
                "fixture",
                super::super::LOCAL_BUILD,
                root.path(),
                "de",
                &mut resources,
                &path,
                || Ok(())
            )
            .unwrap_err()
            .code,
            "languageResourcesMissing"
        );
        assert_eq!(resources.writes, 0);
    }
    #[test]
    fn interrupted_sync_recovers_only_its_language_key() {
        let root = fixture();
        let journals = tempfile::tempdir().unwrap();
        let path = journals.path().join("language.json");
        let mut resources = Memory {
            image: None,
            fail_write: false,
            writes: 0,
        };
        let plan = super::super::plan(
            "fixture".into(),
            super::super::LOCAL_BUILD,
            vec![super::super::SettingChange {
                name: "Language".into(),
                value: 2,
            }],
            &resources,
        )
        .unwrap();
        let mut journal = arachnid_repair::prepare(plan, &resources, &path).unwrap();
        journal.phase = arachnid_repair::Phase::Applying;
        journal.intent_count = 1;
        journal.save(&path).unwrap();
        resources.image = journal.plan.actions[0].after.clone();
        synchronize(
            "fixture",
            super::super::LOCAL_BUILD,
            root.path(),
            "en",
            &mut resources,
            &path,
            || Ok(()),
        )
        .unwrap();
        let mut journal = Journal::load(&path).unwrap();
        assert_eq!(journal.plan.actions[0].before, None);
        assert_eq!(
            resources.image.as_ref().and_then(super::super::dword),
            Some(1)
        );
        journal.plan.actions[0].target = Target::Registry {
            section: "Display".into(),
            name: "Width".into(),
        };
        journal.plan.actions[0].before = Some(arachnid_repair::Image {
            value_type: 4,
            bytes: 1920u32.to_le_bytes().to_vec(),
        });
        journal.plan.actions[0].after = Some(arachnid_repair::Image {
            value_type: 4,
            bytes: 1280u32.to_le_bytes().to_vec(),
        });
        journal.digest = journal.expected_digest().unwrap();
        journal.save(&path).unwrap();
        let writes = resources.writes;
        assert!(
            synchronize(
                "fixture",
                super::super::LOCAL_BUILD,
                root.path(),
                "fr",
                &mut resources,
                &path,
                || Ok(())
            )
            .is_err()
        );
        assert_eq!(resources.writes, writes);
    }
    #[test]
    fn language_availability_requires_an_ordinary_nonempty_resource_in_the_expected_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Data")).unwrap();
        assert!(
            inspect(root.path())
                .iter()
                .all(|language| !language.available)
        );
        fs::write(
            root.path().join("Data/igct.bnx"),
            b"synthetic resource presence",
        )
        .unwrap();
        fs::write(root.path().join("Data/igct_fr.bnx"), b"synthetic satellite").unwrap();
        fs::write(root.path().join("Data/igct_de.bnx"), b"").unwrap();
        let languages = inspect(root.path());
        assert_eq!(
            languages
                .iter()
                .filter(|language| language.available)
                .map(|language| language.code)
                .collect::<Vec<_>>(),
            ["fr", "en"]
        );
        assert_eq!(
            languages
                .iter()
                .map(|language| language.value)
                .collect::<Vec<_>>(),
            [2, 1, 4, 3, 5]
        );
    }
}
