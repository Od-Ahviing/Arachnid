use arachnid_mods::selection::{Selection, resolve};
use arachnid_mods::{Dependency, Preimage, RecipeFile, bundled_catalog};

fn synthetic() -> arachnid_mods::Catalog {
    let mut catalog = bundled_catalog().unwrap();
    for (index, recipe) in catalog.entries.iter_mut().enumerate() {
        recipe.conflicts.clear();
        recipe.version = Some("test-1".into());
        recipe.files = vec![RecipeFile {
            source: format!("fixture-{index}.dll"),
            target: format!("fixture-{index}.dll"),
            sha256: "a".repeat(64),
            bytes: 1,
            preimage: Preimage::Absent,
        }];
    }
    catalog
}

#[test]
fn legacy_catalog_selection_preserves_grouped_preferences_for_migration() {
    let catalog = synthetic();
    let mut selection = Selection::defaults(&catalog);
    assert_eq!(
        selection.enabled.iter().collect::<Vec<_>>(),
        vec!["performance-pack"]
    );
    for member in ["fix-enhancers", "dxvk", "reshade"] {
        for enabled in [true, false] {
            assert_eq!(
                selection.set(&catalog, member, enabled).unwrap_err().code,
                "invalidModSelection"
            );
        }
    }
    assert_eq!(resolve(&catalog, &selection).unwrap().order.len(), 3);
    selection.set(&catalog, "performance-pack", false).unwrap();
    assert!(resolve(&catalog, &selection).unwrap().order.is_empty());
    selection.set(&catalog, "performance-pack", true).unwrap();
    assert_eq!(resolve(&catalog, &selection).unwrap().order.len(), 3);
    assert!(selection.set(&catalog, "unknown", false).is_err());
}

#[test]
fn grouped_preference_does_not_waive_real_conflicts() {
    let catalog = bundled_catalog().unwrap();
    assert_eq!(
        resolve(&catalog, &Selection::defaults(&catalog))
            .unwrap_err()
            .code,
        "modConflict"
    );
}

#[test]
fn dependency_order_is_deterministic_and_versions_are_exact() {
    let mut catalog = synthetic();
    catalog.entries[0].dependencies.push(Dependency {
        id: "reshade".into(),
        version: "test-1".into(),
    });
    let selection = Selection::defaults(&catalog);
    assert_eq!(
        resolve(&catalog, &selection).unwrap().order,
        vec!["dxvk", "reshade", "fix-enhancers"]
    );
    catalog.entries.reverse();
    assert_eq!(
        resolve(&catalog, &selection).unwrap().order,
        vec!["dxvk", "reshade", "fix-enhancers"]
    );
    catalog
        .entries
        .iter_mut()
        .find(|r| r.id == "reshade")
        .unwrap()
        .version = Some("test-2".into());
    assert_eq!(
        resolve(&catalog, &selection).unwrap_err().code,
        "modDependencyVersion"
    );
}

#[test]
fn missing_dependencies_cycles_and_partial_groups_are_refused() {
    let mut catalog = synthetic();
    catalog.entries[0].dependencies.push(Dependency {
        id: "missing".into(),
        version: "test-1".into(),
    });
    let selection = Selection::defaults(&catalog);
    assert_eq!(
        resolve(&catalog, &selection).unwrap_err().code,
        "modDependencyMissing"
    );
    catalog.entries[0].dependencies[0].id = "dxvk".into();
    catalog.entries[1].dependencies.push(Dependency {
        id: "fix-enhancers".into(),
        version: "test-1".into(),
    });
    assert_eq!(
        resolve(&catalog, &selection).unwrap_err().code,
        "modDependencyCycle"
    );
    catalog.entries[0].dependencies.clear();
    catalog.entries[1].dependencies.clear();
    catalog.entries[3].dependencies.push(Dependency {
        id: "dxvk".into(),
        version: "test-1".into(),
    });
    let mut selection = Selection::defaults(&catalog);
    selection.set(&catalog, "performance-pack", false).unwrap();
    selection.set(&catalog, "tasm1-skin-pack", true).unwrap();
    assert_eq!(
        resolve(&catalog, &selection).unwrap_err().code,
        "modGroupRequired"
    );
}

#[test]
fn duplicate_and_parent_paths_conflict_across_members() {
    for path in ["FIXTURE-0.DLL", "fixture-0.dll/child", "folder"] {
        let mut catalog = synthetic();
        if path == "folder" {
            catalog.entries[0].files[0].target = "folder/child".into();
        }
        catalog.entries[1].files[0].target = path.into();
        assert_eq!(
            resolve(&catalog, &Selection::defaults(&catalog))
                .unwrap_err()
                .code,
            "modPathConflict"
        );
    }
}

#[cfg(windows)]
#[test]
fn persisted_group_choice_survives_restart_and_corruption_is_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = synthetic();
    let mut selection = Selection::load(directory.path(), &catalog).unwrap();
    assert!(selection.enabled.contains("performance-pack"));
    selection.set(&catalog, "performance-pack", false).unwrap();
    selection.save(directory.path(), &catalog).unwrap();
    assert_eq!(
        Selection::load(directory.path(), &catalog).unwrap(),
        selection
    );
    let path = directory.path().join("mod-selection.json");
    for invalid in [
        b"{broken".to_vec(),
        vec![b' '; 8193],
        br#"{"schemaVersion":1,"enabled":["dxvk"]}"#.to_vec(),
    ] {
        std::fs::write(&path, &invalid).unwrap();
        assert!(selection.save(directory.path(), &catalog).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), invalid);
    }
}
