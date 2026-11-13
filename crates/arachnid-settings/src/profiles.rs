use super::{LOCAL_BUILD, NativeSetting, dword, replace_bits};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Action, Image, OperationPlan, Resources, Target};
use serde::{Deserialize, Serialize};

pub const QUALITY_MASK: u32 = 0xf | 0x30 | 0x1c0 | 0x1800 | 0x30000 | 0x40000 | 0x180000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Profile {
    VeryLow,
    Low,
    Medium,
    High,
    Amazing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStatus {
    pub name: Profile,
    pub available: bool,
    pub settings: Vec<super::SettingChange>,
}

impl Profile {
    pub const ALL: [Self; 5] = [
        Self::VeryLow,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Amazing,
    ];
    pub fn settings(self) -> Vec<super::SettingChange> {
        let values = self.values();
        let mut settings = values
            .iter()
            .filter(|(_, name, _)| super::is_simple(name))
            .map(|(_, name, value)| super::SettingChange {
                name: (*name).into(),
                value: *value,
            })
            .collect::<Vec<_>>();
        for field in super::advanced::FIELDS {
            let Some((_, _, source)) = values.iter().find(|(_, name, _)| *name == field.source)
            else {
                continue;
            };
            if let Some((value, _)) = field
                .values
                .iter()
                .find(|(_, bits)| source & field.mask == *bits)
            {
                settings.push(super::SettingChange {
                    name: field.name.into(),
                    value: *value,
                });
            }
        }
        settings
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::VeryLow => "veryLow",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Amazing => "amazing",
        }
    }
    pub(crate) fn values(self) -> Vec<(&'static str, &'static str, u32)> {
        // Native XML recipes; resolution, language, edge detection and CMB are preserved.
        let (preset, aniso, blur, quality, rendering, density) = match self {
            Self::VeryLow => (0, 0, 0, 1 | 0x10 | 0x40 | 0x800 | 0x10000, 7, 1),
            Self::Low => (1, 2, 0, 2 | 0x10 | 0x80 | 0x800 | 0x10000, 7, 1),
            Self::Medium => (2, 4, 1, 4 | 0x10 | 0x40 | 0x1000 | 0x40000, 7, 2),
            Self::High => (3, 8, 1, 8 | 0x20 | 0x80 | 0x1000 | 0x40000 | 0x80000, 2, 2),
            Self::Amazing => (
                4,
                16,
                1,
                8 | 0x20 | 0x100 | 0x1000 | 0x20000 | 0x40000 | 0x100000,
                0,
                3,
            ),
        };
        vec![
            ("Display", "VSync", 1),
            ("Display", "MaxAniso", aniso),
            (
                "Display",
                "AntiAliasing",
                match self {
                    Self::VeryLow => 0,
                    Self::Amazing => 3,
                    _ => 1,
                },
            ),
            ("Display", "MotionBlur", blur),
            ("Display", "QualitySettings", quality),
            ("Display", "RenderingFlags", rendering),
            ("GameConfig", "GraphicsPreset", preset),
            ("GameConfig", "FxPreset", preset.max(1)),
            ("GameConfig", "LightningPreset", preset),
            ("GameConfig", "ShadowsPreset", preset),
            ("GameConfig", "CityLifeDensity", density),
        ]
    }
}

pub fn current(settings: &[NativeSetting]) -> &'static str {
    for profile in Profile::ALL {
        if profile.values().iter().all(|(section, name, expected)| {
            let mask = match *name {
                "QualitySettings" => QUALITY_MASK,
                "RenderingFlags" => 7,
                _ => u32::MAX,
            };
            settings
                .iter()
                .find(|field| field.section == *section && field.name == *name)
                .and_then(|field| field.value)
                .is_some_and(|value| value & mask == *expected)
        }) {
            return profile.name();
        }
    }
    "custom"
}

pub(crate) fn available(settings: &[NativeSetting]) -> bool {
    Profile::Low.values().iter().all(|(section, name, _)| {
        settings
            .iter()
            .find(|field| field.section == *section && field.name == *name)
            .is_some_and(|field| field.value.is_some() || field.value_type.is_none())
    })
}

pub fn plan(
    installation: String,
    build: &str,
    profile: Profile,
    resources: &impl Resources,
) -> AppResult<OperationPlan> {
    let plan = profile_plan(installation, build, profile, resources)?;
    super::validate_plan(&plan, &plan.installation)?;
    Ok(plan)
}

fn profile_plan(
    installation: String,
    build: &str,
    profile: Profile,
    resources: &impl Resources,
) -> AppResult<OperationPlan> {
    if build != LOCAL_BUILD {
        return Err(AppError::new("unsupportedSetting"));
    }
    let mut actions = Vec::new();
    for (section, name, selected) in profile.values() {
        let target = Target::Registry {
            section: section.into(),
            name: name.into(),
        };
        let before = resources.read(&target)?;
        if before.as_ref().is_some_and(|image| dword(image).is_none()) {
            return Err(AppError::new("unsupportedSetting"));
        }
        let value = match name {
            "QualitySettings" | "RenderingFlags" => {
                let original = before.as_ref().and_then(dword).unwrap_or_default();
                replace_bits(
                    original,
                    if name == "QualitySettings" {
                        QUALITY_MASK
                    } else {
                        7
                    },
                    selected,
                )?
            }
            _ => selected,
        };
        let after = Some(Image {
            value_type: 4,
            bytes: value.to_le_bytes().to_vec(),
        });
        if before != after {
            actions.push(Action {
                target,
                before,
                after,
            });
        }
    }
    actions.sort_by(|a, b| a.target.cmp(&b.target));
    let plan = OperationPlan {
        schema_version: 1,
        installation,
        actions,
    };
    Ok(plan)
}

pub fn plan_with_changes(
    installation: String,
    build: &str,
    profile: Profile,
    changes: Vec<super::SettingChange>,
    resources: &impl Resources,
) -> AppResult<OperationPlan> {
    let base = profile_plan(installation.clone(), build, profile, resources)?;
    if changes.is_empty() {
        super::validate_plan(&base, &base.installation)?;
        return Ok(base);
    }
    struct Preview<'a, R> {
        base: &'a OperationPlan,
        resources: &'a R,
    }
    impl<R: Resources> Resources for Preview<'_, R> {
        fn read(&self, target: &Target) -> AppResult<Option<Image>> {
            match self
                .base
                .actions
                .iter()
                .find(|action| &action.target == target)
            {
                Some(action) => Ok(action.after.clone()),
                None => self.resources.read(target),
            }
        }
        fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
            Err(AppError::new("invalidPlan"))
        }
    }
    let overrides = super::plan(
        installation,
        build,
        changes,
        &Preview {
            base: &base,
            resources,
        },
    )?;
    let mut actions = base
        .actions
        .into_iter()
        .map(|action| (action.target.clone(), action))
        .collect::<std::collections::BTreeMap<_, _>>();
    for action in overrides.actions {
        if let Some(existing) = actions.get_mut(&action.target) {
            existing.after = action.after;
        } else {
            actions.insert(action.target.clone(), action);
        }
    }
    let plan = OperationPlan {
        schema_version: 1,
        installation: overrides.installation,
        actions: actions
            .into_values()
            .filter(|action| action.before != action.after)
            .collect(),
    };
    super::validate_plan(&plan, &plan.installation)?;
    Ok(plan)
}

pub(crate) fn valid_action(section: &str, name: &str, before: Option<u32>, after: u32) -> bool {
    match (section, name) {
        ("Display", "VSync" | "MaxAniso") if before.is_none() => super::valid_value(name, after),
        ("Display", "QualitySettings") => {
            let original = before.unwrap_or_default();
            original & !QUALITY_MASK == after & !QUALITY_MASK
                && [1, 2, 4, 8].contains(&(after & 0xf))
                && [0, 0x10, 0x20].contains(&(after & 0x30))
                && [0x40, 0x80, 0x100].contains(&(after & 0x1c0))
                && [0x800, 0x1000].contains(&(after & 0x1800))
                && [0, 0x10000, 0x20000].contains(&(after & 0x30000))
                && [0, 0x80000, 0x100000].contains(&(after & 0x180000))
        }
        ("Display", "RenderingFlags") => before.unwrap_or_default() & !7 == after & !7,
        ("GameConfig", "GraphicsPreset" | "FxPreset" | "LightningPreset" | "ShadowsPreset") => {
            after <= 4
        }
        ("GameConfig", "CityLifeDensity") => (1..=3).contains(&after),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_fresh_registry_can_use_presets_without_overwriting_opaque_values() {
        struct Empty;
        impl Resources for Empty {
            fn read(&self, _: &Target) -> AppResult<Option<Image>> {
                Ok(None)
            }
            fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
                panic!("planning is read-only")
            }
        }
        let mut settings = super::super::fields()
            .into_iter()
            .map(|(section, name)| NativeSetting {
                name: name.into(),
                section: section.into(),
                value: None,
                value_type: None,
                editable: false,
            })
            .collect::<Vec<_>>();
        assert!(available(&settings));
        for profile in Profile::ALL {
            let plan = plan("reference".into(), LOCAL_BUILD, profile, &Empty).unwrap();
            assert_eq!(plan.actions.len(), profile.values().len());
            assert!(plan.actions.iter().all(|action| action.before.is_none()));
        }
        settings
            .iter_mut()
            .find(|field| field.name == "GraphicsPreset")
            .unwrap()
            .value_type = Some(1);
        assert!(!available(&settings));
    }
    #[test]
    fn profile_only_and_overrides_of_an_existing_profile_are_supported() {
        let base = plan("reference".into(), LOCAL_BUILD, Profile::High, &Snapshot).unwrap();
        assert_eq!(
            plan_with_changes(
                "reference".into(),
                LOCAL_BUILD,
                Profile::High,
                vec![],
                &Snapshot
            )
            .unwrap(),
            base,
        );
        struct ExistingProfile;
        impl Resources for ExistingProfile {
            fn read(&self, target: &Target) -> AppResult<Option<Image>> {
                let Target::Registry { name, .. } = target else {
                    panic!("registry only")
                };
                let value = Profile::High
                    .values()
                    .iter()
                    .find(|(_, field, _)| *field == name)
                    .map(|(_, _, value)| *value)
                    .unwrap_or(1920);
                Ok(Some(Image {
                    value_type: 4,
                    bytes: value.to_le_bytes().to_vec(),
                }))
            }
            fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
                panic!("read-only planning")
            }
        }
        let plan = plan_with_changes(
            "reference".into(),
            LOCAL_BUILD,
            Profile::High,
            vec![super::super::SettingChange {
                name: "Width".into(),
                value: 1280,
            }],
            &ExistingProfile,
        )
        .unwrap();
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].before.as_ref().and_then(dword), Some(1920));
        assert_eq!(plan.actions[0].after.as_ref().and_then(dword), Some(1280));
    }
    #[test]
    fn profile_draft_and_overrides_share_recipes_and_restore_original_preimages() {
        let plan = plan_with_changes(
            "reference".into(),
            LOCAL_BUILD,
            Profile::High,
            vec![
                super::super::SettingChange {
                    name: "Width".into(),
                    value: 1280,
                },
                super::super::SettingChange {
                    name: "ShadowDetail".into(),
                    value: 1,
                },
                super::super::SettingChange {
                    name: "VSync".into(),
                    value: 0,
                },
            ],
            &Snapshot,
        )
        .unwrap();
        let find = |name: &str| {
            plan.actions.iter().find(|action| matches!(&action.target, Target::Registry { name: field, .. } if field == name)).unwrap()
        };
        assert_eq!(find("Width").after.as_ref().and_then(dword), Some(1280));
        assert_eq!(find("VSync").after.as_ref().and_then(dword), Some(0));
        let quality = find("QualitySettings");
        assert_eq!(quality.before.as_ref().and_then(dword), Some(0xa000_4000));
        assert_eq!(quality.after.as_ref().and_then(dword).unwrap() & 0xf, 1);
        assert_eq!(
            quality.after.as_ref().and_then(dword).unwrap() & !QUALITY_MASK,
            0xa000_4000 & !QUALITY_MASK
        );
        for profile in Profile::ALL {
            let fields = profile.settings();
            assert!(
                !fields
                    .iter()
                    .any(|field| matches!(field.name.as_str(), "Width" | "Height" | "Language"))
            );
            let expected_shadow = match profile {
                Profile::VeryLow => 1,
                Profile::Low => 2,
                Profile::Medium => 3,
                Profile::High | Profile::Amazing => 4,
            };
            assert_eq!(
                fields
                    .iter()
                    .find(|field| field.name == "ShadowDetail")
                    .unwrap()
                    .value,
                expected_shadow
            );
        }
    }
    struct Snapshot;
    impl Resources for Snapshot {
        fn read(&self, _: &Target) -> AppResult<Option<Image>> {
            Ok(Some(Image {
                value_type: 4,
                bytes: 0xa000_4000u32.to_le_bytes().to_vec(),
            }))
        }
        fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
            panic!("planning is read-only")
        }
    }

    #[test]
    fn extreme_presets_follow_native_subpreset_references_and_effects() {
        let very_low = Profile::VeryLow.values();
        assert!(very_low.contains(&("GameConfig", "GraphicsPreset", 0)));
        assert!(very_low.contains(&("GameConfig", "FxPreset", 1)));
        assert!(very_low.contains(&("GameConfig", "LightningPreset", 0)));
        assert!(very_low.contains(&("Display", "AntiAliasing", 0)));
        assert!(very_low.contains(&("Display", "MaxAniso", 0)));
        let amazing = Profile::Amazing.settings();
        for (name, value) in [
            ("MaxAniso", 16),
            ("AntiAliasing", 3),
            ("ShadowDetail", 4),
            ("Ssao", 3),
            ("DrawDistance", 3),
            ("LightShafts", 3),
            ("Bloom", 1),
            ("Heat", 1),
            ("DepthOfField", 1),
        ] {
            assert!(
                amazing
                    .iter()
                    .any(|setting| setting.name == name && setting.value == value)
            );
        }
    }
    #[test]
    fn profile_and_language_transactions_restore_preimages_and_initial_absence() {
        use std::{collections::BTreeMap, sync::atomic::AtomicBool};
        #[derive(Clone)]
        struct Memory(BTreeMap<Target, Image>);
        impl Resources for Memory {
            fn read(&self, target: &Target) -> AppResult<Option<Image>> {
                Ok(self.0.get(target).cloned())
            }
            fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
                if let Some(image) = image {
                    self.0.insert(target.clone(), image.clone());
                } else {
                    self.0.remove(target);
                }
                Ok(())
            }
        }
        let initial = Memory(
            [
                ("QualitySettings", 0xa000_4000u32),
                ("RenderingFlags", 0xa000_0000),
                ("VSync", 0),
                ("MaxAniso", 2),
                ("UnknownVendorValue", 42),
            ]
            .into_iter()
            .map(|(name, value)| {
                (
                    Target::Registry {
                        section: "Display".into(),
                        name: name.into(),
                    },
                    Image {
                        value_type: 4,
                        bytes: value.to_le_bytes().to_vec(),
                    },
                )
            })
            .collect(),
        );
        for profile in [
            Some(Profile::VeryLow),
            Some(Profile::Low),
            Some(Profile::Medium),
            Some(Profile::High),
            Some(Profile::Amazing),
            None,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("journal.json");
            let mut resources = initial.clone();
            let plan = if let Some(profile) = profile {
                plan("reference".into(), LOCAL_BUILD, profile, &resources).unwrap()
            } else {
                super::super::plan(
                    "reference".into(),
                    LOCAL_BUILD,
                    vec![super::super::SettingChange {
                        name: "Language".into(),
                        value: 2,
                    }],
                    &resources,
                )
                .unwrap()
            };
            let mut journal = arachnid_repair::prepare(plan, &resources, &path).unwrap();
            let digest = journal.digest.clone();
            arachnid_repair::apply(
                &mut journal,
                &digest,
                &mut resources,
                &path,
                &AtomicBool::new(false),
                || Ok(()),
            )
            .unwrap();
            assert_ne!(resources.0, initial.0);
            arachnid_repair::rollback(&mut journal, &mut resources, &path).unwrap();
            assert_eq!(resources.0, initial.0);
        }
    }
    #[test]
    fn every_profile_is_deterministic_preserves_unknown_bits_and_excludes_language_and_resolution()
    {
        for profile in Profile::ALL {
            let first = plan("reference".into(), LOCAL_BUILD, profile, &Snapshot).unwrap();
            assert_eq!(
                first.digest().unwrap(),
                plan("reference".into(), LOCAL_BUILD, profile, &Snapshot)
                    .unwrap()
                    .digest()
                    .unwrap()
            );
            let mut fields = Vec::new();
            for action in &first.actions {
                let Target::Registry { section, name } = &action.target else {
                    panic!()
                };
                assert!(!["Language", "Width", "Height", "Fullscreen"].contains(&name.as_str()));
                let after = action.after.as_ref().and_then(dword).unwrap();
                if name == "QualitySettings" {
                    assert_eq!(after & !QUALITY_MASK, 0xa000_4000 & !QUALITY_MASK);
                }
                fields.push(NativeSetting {
                    name: name.clone(),
                    section: section.clone(),
                    value: Some(after),
                    value_type: Some(4),
                    editable: false,
                });
            }
            for (section, name, _) in profile.values() {
                if !fields
                    .iter()
                    .any(|field| field.section == section && field.name == name)
                {
                    fields.push(NativeSetting {
                        name: name.into(),
                        section: section.into(),
                        value: Some(0xa000_4000),
                        value_type: Some(4),
                        editable: false,
                    });
                }
            }
            assert_eq!(current(&fields), profile.name());
            fields
                .iter_mut()
                .find(|field| field.name == "VSync")
                .unwrap()
                .value = Some(0);
            assert_eq!(current(&fields), "custom");
        }
        assert!(plan("reference".into(), "unknown", Profile::Low, &Snapshot).is_err());
    }
}
