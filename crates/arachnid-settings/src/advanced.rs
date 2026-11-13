use super::{NativeSetting, SettingChange, dword, replace_bits};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Action, Image, Resources, Target};
use std::collections::BTreeMap;

pub struct BitSetting {
    pub name: &'static str,
    pub source: &'static str,
    pub mask: u32,
    pub values: &'static [(u32, u32)],
}

pub const FIELDS: &[BitSetting] = &[
    BitSetting {
        name: "ShadowDetail",
        source: "QualitySettings",
        mask: 0xf,
        values: &[(1, 1), (2, 2), (3, 4), (4, 8)],
    },
    BitSetting {
        name: "Particles",
        source: "QualitySettings",
        mask: 0x30,
        values: &[(0, 0), (1, 0x10), (3, 0x20)],
    },
    BitSetting {
        name: "LightShafts",
        source: "QualitySettings",
        mask: 0x1c0,
        values: &[(1, 0x40), (2, 0x80), (3, 0x100)],
    },
    BitSetting {
        name: "Halo",
        source: "QualitySettings",
        mask: 0x1800,
        values: &[(0, 0x800), (1, 0x1000)],
    },
    BitSetting {
        name: "DrawDistance",
        source: "QualitySettings",
        mask: 0x30000,
        values: &[(1, 0x10000), (2, 0), (3, 0x20000)],
    },
    BitSetting {
        name: "Godrays",
        source: "QualitySettings",
        mask: 0x40000,
        values: &[(0, 0), (1, 0x40000)],
    },
    BitSetting {
        name: "Ssao",
        source: "QualitySettings",
        mask: 0x180000,
        values: &[(0, 0), (1, 0x80000), (3, 0x100000)],
    },
    BitSetting {
        name: "DepthOfField",
        source: "RenderingFlags",
        mask: 1,
        values: &[(0, 1), (1, 0)],
    },
    BitSetting {
        name: "Bloom",
        source: "RenderingFlags",
        mask: 2,
        values: &[(0, 2), (1, 0)],
    },
    BitSetting {
        name: "Heat",
        source: "RenderingFlags",
        mask: 4,
        values: &[(0, 4), (1, 0)],
    },
];

pub fn append(settings: &mut Vec<NativeSetting>, supported: bool) {
    for field in FIELDS {
        let value = settings
            .iter()
            .find(|item| item.name == field.source)
            .and_then(|item| item.value)
            .and_then(|original| {
                field
                    .values
                    .iter()
                    .find(|(_, bits)| original & field.mask == *bits)
                    .map(|(value, _)| *value)
            });
        settings.push(NativeSetting {
            name: field.name.into(),
            section: "Display".into(),
            value,
            value_type: value.map(|_| 4),
            editable: supported && value.is_some(),
        });
    }
}

pub fn merge(
    change: &SettingChange,
    resources: &impl Resources,
    actions: &mut BTreeMap<Target, Action>,
) -> AppResult<bool> {
    let Some(field) = FIELDS.iter().find(|field| field.name == change.name) else {
        return Ok(false);
    };
    let selected = field
        .values
        .iter()
        .find(|(value, _)| *value == change.value)
        .map(|(_, bits)| *bits)
        .ok_or_else(|| AppError::new("unsupportedSetting"))?;
    let target = Target::Registry {
        section: "Display".into(),
        name: field.source.into(),
    };
    let before = if let Some(action) = actions.get(&target) {
        action.before.clone()
    } else {
        resources.read(&target)?
    };
    let original = before
        .as_ref()
        .and_then(dword)
        .ok_or_else(|| AppError::new("unsupportedSetting"))?;
    if !field
        .values
        .iter()
        .any(|(_, bits)| original & field.mask == *bits)
    {
        return Err(AppError::new("unsupportedSetting"));
    }
    let previous = actions
        .get(&target)
        .and_then(|action| action.after.as_ref())
        .and_then(dword)
        .unwrap_or(original);
    let value = replace_bits(previous, field.mask, selected)?;
    actions.insert(
        target.clone(),
        Action {
            target,
            before,
            after: Some(Image {
                value_type: 4,
                bytes: value.to_le_bytes().to_vec(),
            }),
        },
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Snapshot;
    impl Resources for Snapshot {
        fn read(&self, target: &Target) -> AppResult<Option<Image>> {
            let value = if matches!(target, Target::Registry { name, .. } if name == "RenderingFlags")
            {
                0xa000_0000u32
            } else {
                0xa000_4000 | 1 | 0x10 | 0x40 | 0x800
            };
            Ok(Some(Image {
                value_type: 4,
                bytes: value.to_le_bytes().to_vec(),
            }))
        }
        fn write(&mut self, _: &Target, _: &Option<Image>) -> AppResult<()> {
            panic!("planning is read-only")
        }
    }
    #[test]
    fn shared_bitfield_changes_merge_deterministically_without_losing_unrelated_bits() {
        let changes = || {
            vec![
                SettingChange {
                    name: "ShadowDetail".into(),
                    value: 4,
                },
                SettingChange {
                    name: "Ssao".into(),
                    value: 3,
                },
                SettingChange {
                    name: "Bloom".into(),
                    value: 0,
                },
            ]
        };
        let first = super::super::plan(
            "reference".into(),
            super::super::LOCAL_BUILD,
            changes(),
            &Snapshot,
        )
        .unwrap();
        let mut reversed = changes();
        reversed.reverse();
        assert_eq!(
            first.digest().unwrap(),
            super::super::plan(
                "reference".into(),
                super::super::LOCAL_BUILD,
                reversed,
                &Snapshot
            )
            .unwrap()
            .digest()
            .unwrap()
        );
        assert_eq!(first.actions.len(), 2);
        for action in first.actions {
            let original = action.before.as_ref().and_then(dword).unwrap();
            let changed = action.after.as_ref().and_then(dword).unwrap();
            let Target::Registry { name, .. } = action.target else {
                panic!()
            };
            let mask = if name == "QualitySettings" {
                0xf | 0x180000
            } else {
                2
            };
            assert_eq!(original & !mask, changed & !mask);
            if name == "RenderingFlags" {
                assert_eq!(changed & 2, 2);
            }
        }
    }
}
