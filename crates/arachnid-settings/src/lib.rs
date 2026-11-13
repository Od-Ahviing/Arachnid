use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Action, Image, OperationPlan, Resources, Target, registry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub mod advanced;
pub mod languages;
pub mod profiles;

pub const LOCAL_BUILD: &str = "facb480a78b1d5dfd2cdbba31be89be9cf3cd01e1eadd524de35f23ea436d60d";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSetting {
    pub name: String,
    pub section: String,
    pub value: Option<u32>,
    pub value_type: Option<u32>,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsReport {
    pub settings: Vec<NativeSetting>,
    pub build_supported: bool,
    pub runtime_qualified: bool,
    pub capabilities: Vec<Capability>,
    pub game_languages: Vec<languages::GameLanguage>,
    pub profiles: Vec<profiles::ProfileStatus>,
    pub current_profile: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub name: String,
    pub kind: &'static str,
    pub state: &'static str,
    pub reason: &'static str,
}

pub fn capabilities(build: &str, settings: &[NativeSetting]) -> Vec<Capability> {
    let mut capabilities = settings
        .iter()
        .map(|setting| {
            let (state, reason) = if build != LOCAL_BUILD {
                ("readOnly", "channelUnqualified")
            } else if setting.name == "Language" && setting.editable {
                ("experimental", "languageResourceEvidence")
            } else if setting.value_type.is_none() {
                ("readOnly", "settingAbsent")
            } else if setting.value.is_none() {
                ("readOnly", "settingOpaque")
            } else if setting.editable && setting.name == "Fullscreen" {
                ("localUserTest", "localDisplayTest")
            } else if setting.editable && matches!(setting.name.as_str(), "Width" | "Height") {
                ("localUserTest", "localResolutionTest")
            } else if setting.editable {
                ("experimental", "needsInGameValidation")
            } else if setting.name == "Language" {
                ("readOnly", "languageMappingUnknown")
            } else {
                ("readOnly", "needsInGameValidation")
            };
            Capability {
                name: setting.name.clone(),
                kind: "native",
                state,
                reason,
            }
        })
        .collect::<Vec<_>>();
    for name in ["FpsLimit", "FieldOfView", "Borderless"] {
        capabilities.push(Capability {
            name: name.into(),
            kind: "external",
            state: "unavailable",
            reason: "noQualifiedExtension",
        });
    }
    for name in [
        "VeryLowProfile",
        "LowProfile",
        "MediumProfile",
        "HighProfile",
        "AmazingProfile",
    ] {
        capabilities.push(Capability {
            name: name.into(),
            kind: "derived",
            state: if build == LOCAL_BUILD {
                "experimental"
            } else {
                "unavailable"
            },
            reason: "needsMeasurements",
        });
    }
    capabilities
}

pub fn fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Display", "Width"),
        ("Display", "Height"),
        ("Display", "Fullscreen"),
        ("Display", "VSync"),
        ("Display", "MaxAniso"),
        ("Display", "AntiAliasing"),
        ("Display", "MotionBlur"),
        ("Display", "QualitySettings"),
        ("Display", "RenderingFlags"),
        ("Display", "AspectRatio"),
        ("Display", "RefreshRate"),
        ("GameConfig", "GraphicsPreset"),
        ("GameConfig", "FxPreset"),
        ("GameConfig", "LightningPreset"),
        ("GameConfig", "ShadowsPreset"),
        ("GameConfig", "CityLifeDensity"),
        ("Install", "Language"),
    ]
}

pub fn read(build: &str, root: &Path) -> AppResult<SettingsReport> {
    let game_languages = languages::inspect(root);
    let language_key_exists = registry::section_exists("Install")?;
    let mut settings = Vec::new();
    for (section, name) in fields() {
        let image = registry::read(section, name)?;
        let value = image.as_ref().and_then(dword);
        settings.push(NativeSetting {
            name: name.into(),
            section: section.into(),
            value,
            value_type: image.as_ref().map(|image| image.value_type),
            editable: build == LOCAL_BUILD
                && ((value.is_some() && is_simple(name))
                    || (name == "Language"
                        && language_key_exists
                        && (image.is_none() || value.is_some())
                        && game_languages.iter().any(|language| language.available))),
        });
    }
    let current_profile = profiles::current(&settings);
    let profiles_available = build == LOCAL_BUILD
        && registry::section_exists("GameConfig")?
        && ["QualitySettings", "RenderingFlags", "VSync", "MaxAniso"]
            .iter()
            .all(|name| {
                settings
                    .iter()
                    .any(|field| field.name == *name && field.value.is_some())
            })
        && settings
            .iter()
            .filter(|field| field.section == "GameConfig")
            .all(|field| field.value_type.is_none() || field.value.is_some());
    advanced::append(&mut settings, build == LOCAL_BUILD);
    let mut capability_list = capabilities(build, &settings);
    if !profiles_available {
        for capability in &mut capability_list {
            if capability.name.ends_with("Profile") {
                capability.state = "unavailable";
                capability.reason = "unsupportedSetting";
            }
        }
    }
    Ok(SettingsReport {
        current_profile,
        profiles: profiles::Profile::ALL
            .into_iter()
            .map(|name| profiles::ProfileStatus {
                name,
                available: profiles_available,
                settings: name.settings(),
            })
            .collect(),
        capabilities: capability_list,
        settings,
        build_supported: build == LOCAL_BUILD,
        runtime_qualified: false,
        game_languages,
    })
}

fn dword(image: &Image) -> Option<u32> {
    if image.value_type != 4 {
        return None;
    }
    Some(u32::from_le_bytes(image.bytes.as_slice().try_into().ok()?))
}

fn is_simple(name: &str) -> bool {
    matches!(
        name,
        "Width" | "Height" | "Fullscreen" | "VSync" | "MaxAniso" | "AntiAliasing" | "MotionBlur"
    )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingChange {
    pub name: String,
    pub value: u32,
}

fn valid_value(name: &str, value: u32) -> bool {
    match name {
        "Width" => (640..=7680).contains(&value),
        "Height" => (480..=4320).contains(&value),
        "Fullscreen" | "VSync" | "MotionBlur" => value <= 1,
        "MaxAniso" => [0, 2, 4, 8, 16].contains(&value),
        "AntiAliasing" => [0, 1, 3].contains(&value),
        "Language" => (1..=5).contains(&value),
        _ => false,
    }
}

pub fn validate_plan(plan: &OperationPlan, installation: &str) -> AppResult<()> {
    plan.validate()?;
    if plan.installation != installation || plan.actions.len() > 16 {
        return Err(AppError::new("invalidPlan"));
    }
    for action in &plan.actions {
        let Target::Registry { section, name } = &action.target else {
            return Err(AppError::new("invalidPlan"));
        };
        let language = section == "Install" && name == "Language";
        if let Some(after) = action.after.as_ref().and_then(dword)
            && profiles::valid_action(section, name, action.before.as_ref().and_then(dword), after)
        {
            if action
                .before
                .as_ref()
                .is_some_and(|image| dword(image).is_none())
            {
                return Err(AppError::new("invalidPlan"));
            }
            continue;
        }
        if (!language && (section != "Display" || name == "Language"))
            || (!language
                && action.before.is_none()
                && !["AntiAliasing", "MotionBlur"].contains(&name.as_str()))
            || action
                .before
                .as_ref()
                .is_some_and(|image| dword(image).is_none())
            || !action
                .after
                .as_ref()
                .and_then(dword)
                .is_some_and(|value| valid_value(name, value))
        {
            return Err(AppError::new("invalidPlan"));
        }
    }
    Ok(())
}

pub fn plan(
    installation: String,
    build: &str,
    changes: Vec<SettingChange>,
    resources: &impl Resources,
) -> AppResult<OperationPlan> {
    if build != LOCAL_BUILD || changes.is_empty() || changes.len() > 24 {
        return Err(AppError::new("unsupportedSetting"));
    }
    let mut actions = BTreeMap::new();
    let mut names = BTreeSet::new();
    for change in changes {
        if !names.insert(change.name.clone()) {
            return Err(AppError::new("invalidPlan"));
        }
        if advanced::merge(&change, resources, &mut actions)? {
            continue;
        }
        if !valid_value(&change.name, change.value) {
            return Err(AppError::new("unsupportedSetting"));
        }
        let target = Target::Registry {
            section: if change.name == "Language" {
                "Install"
            } else {
                "Display"
            }
            .into(),
            name: change.name,
        };
        let before = resources.read(&target)?;
        let language = matches!(&target, Target::Registry { name, .. } if name == "Language");
        if (!language && before.is_none())
            || before.as_ref().is_some_and(|image| dword(image).is_none())
        {
            return Err(AppError::new("unsupportedSetting"));
        }
        let after = Some(Image {
            value_type: 4,
            bytes: change.value.to_le_bytes().to_vec(),
        });
        if before != after {
            actions.insert(
                target.clone(),
                Action {
                    target,
                    before,
                    after,
                },
            );
        }
    }
    let plan = OperationPlan {
        schema_version: 1,
        installation,
        actions: actions
            .into_values()
            .filter(|action| action.before != action.after)
            .collect(),
    };
    validate_plan(&plan, &plan.installation)?;
    Ok(plan)
}

pub fn replace_bits(original: u32, mask: u32, selected: u32) -> AppResult<u32> {
    if selected & !mask != 0 {
        return Err(AppError::new("unsupportedSetting"));
    }
    Ok((original & !mask) | selected)
}

#[cfg(test)]
mod tests {
    #[test]
    fn withdrawn_frame_patch_does_not_keep_native_setting_capabilities() {
        assert_ne!(
            super::capabilities(super::LOCAL_BUILD, &[]),
            super::capabilities(
                "033b51e9b4b0a1592733b8140bc14f1e7cfa7c148c6f96a22f473913230ac956",
                &[]
            )
        );
    }
    use super::*;
    struct Snapshot;
    impl Resources for Snapshot {
        fn read(&self, _: &Target) -> AppResult<Option<Image>> {
            Ok(Some(Image {
                value_type: 4,
                bytes: 1u32.to_le_bytes().to_vec(),
            }))
        }
        fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
            panic!("planning must be read-only")
        }
    }

    #[test]
    fn language_plan_preserves_absence_and_revalidates_resources_before_application() {
        struct MissingLanguage;
        impl Resources for MissingLanguage {
            fn read(&self, _: &Target) -> AppResult<Option<Image>> {
                Ok(None)
            }
            fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
                panic!("planning is read-only")
            }
        }
        let root = tempfile::tempdir().unwrap();
        let plan = plan(
            "reference".into(),
            LOCAL_BUILD,
            vec![SettingChange {
                name: "Language".into(),
                value: 2,
            }],
            &MissingLanguage,
        )
        .unwrap();
        assert_eq!(
            plan.actions[0].target,
            Target::Registry {
                section: "Install".into(),
                name: "Language".into()
            }
        );
        assert!(plan.actions[0].before.is_none());
        assert_eq!(plan.actions[0].after.as_ref().and_then(dword), Some(2));
        assert_eq!(
            languages::validate_selection(root.path(), &plan)
                .unwrap_err()
                .code,
            "languageResourcesMissing"
        );
        std::fs::create_dir(root.path().join("fr")).unwrap();
        std::fs::write(
            root.path().join("fr/ASMLauncher.resources.dll"),
            b"synthetic resource",
        )
        .unwrap();
        languages::validate_selection(root.path(), &plan).unwrap();
        std::fs::remove_file(root.path().join("fr/ASMLauncher.resources.dll")).unwrap();
        assert!(languages::validate_selection(root.path(), &plan).is_err());
        let mut altered = plan;
        if let Target::Registry { section, .. } = &mut altered.actions[0].target {
            *section = "Display".into();
        }
        assert!(validate_plan(&altered, "reference").is_err());
    }
    #[test]
    fn plan_is_deterministic_and_does_not_invent_unsupported_values() {
        let changes = || {
            vec![
                SettingChange {
                    name: "Width".into(),
                    value: 1920,
                },
                SettingChange {
                    name: "Height".into(),
                    value: 1080,
                },
            ]
        };
        let first = plan("test".into(), LOCAL_BUILD, changes(), &Snapshot).unwrap();
        let mut reversed = changes();
        reversed.reverse();
        assert_eq!(
            first.digest().unwrap(),
            plan("test".into(), LOCAL_BUILD, reversed, &Snapshot)
                .unwrap()
                .digest()
                .unwrap()
        );
        for (name, value) in [
            ("FOV", 90),
            ("QualitySettings", 0),
            ("Width", 0),
            ("AntiAliasing", 2),
        ] {
            assert!(
                plan(
                    "test".into(),
                    LOCAL_BUILD,
                    vec![SettingChange {
                        name: name.into(),
                        value
                    }],
                    &Snapshot
                )
                .is_err()
            );
        }
        assert!(plan("test".into(), "other", changes(), &Snapshot).is_err());
    }
    #[test]
    fn bitfield_update_preserves_every_unknown_bit() {
        for original in [0, u32::MAX, 275025, 0x80000000] {
            assert_eq!(replace_bits(original, 15, 2).unwrap() & !15, original & !15);
        }
        assert!(replace_bits(0, 15, 16).is_err());
    }

    #[test]
    fn capabilities_separate_local_evidence_from_absence_and_unknown_builds() {
        let mut settings =
            ["Fullscreen", "VSync", "AntiAliasing", "QualitySettings"].map(|name| NativeSetting {
                name: name.into(),
                section: "Display".into(),
                value: Some(1),
                value_type: Some(4),
                editable: is_simple(name),
            });
        settings[2].value = None;
        settings[2].value_type = None;
        settings[2].editable = false;
        let local = capabilities(LOCAL_BUILD, &settings);
        settings[1].name = "Width".into();
        assert_eq!(
            capabilities(LOCAL_BUILD, &settings)[1].reason,
            "localResolutionTest"
        );
        assert_eq!(local[0].state, "localUserTest");
        assert_eq!(local[1].state, "experimental");
        assert_eq!(local[2].reason, "settingAbsent");
        assert_eq!(local[3].state, "readOnly");
        assert_eq!(
            local
                .iter()
                .find(|item| item.name == "FpsLimit")
                .unwrap()
                .reason,
            "noQualifiedExtension"
        );
        assert_eq!(
            local
                .iter()
                .find(|item| item.name == "LowProfile")
                .unwrap()
                .reason,
            "needsMeasurements"
        );
        assert!(
            capabilities("unknown", &settings)
                .iter()
                .all(|item| item.state != "localUserTest" && item.state != "experimental")
        );
    }

    #[test]
    fn persisted_settings_plan_cannot_expand_into_file_or_unqualified_registry_mutations() {
        let valid = plan(
            "reference".into(),
            LOCAL_BUILD,
            vec![SettingChange {
                name: "Width".into(),
                value: 1280,
            }],
            &Snapshot,
        )
        .unwrap();
        validate_plan(&valid, "reference").unwrap();
        assert!(validate_plan(&valid, "other").is_err());
        for target in [
            Target::File {
                path: "user.ini".into(),
            },
            Target::Registry {
                section: "Install".into(),
                name: "Language".into(),
            },
            Target::Registry {
                section: "Display".into(),
                name: "QualitySettings".into(),
            },
        ] {
            let mut changed = valid.clone();
            changed.actions[0].target = target;
            assert!(validate_plan(&changed, "reference").is_err());
        }
        let mut changed = valid.clone();
        changed.actions[0].before = None;
        assert!(validate_plan(&changed, "reference").is_err());
        let mut changed = valid;
        changed.actions[0].after.as_mut().unwrap().bytes = 0u32.to_le_bytes().to_vec();
        assert!(validate_plan(&changed, "reference").is_err());
    }
}
