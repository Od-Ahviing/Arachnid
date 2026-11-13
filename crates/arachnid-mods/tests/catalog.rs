use arachnid_mods::{MAX_CATALOG_BYTES, MissingEvidence, bundled_catalog, parse_catalog};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "schemaVersion": 1,
        "game": "tasm1-pc-2012",
        "groups": [],
        "entries": [{
            "id": "synthetic-fix",
            "name": "Synthetic fixture",
            "author": "Arachnid tests",
            "source": "https://example.invalid/source",
            "kind": "wrapper",
            "defaultEnabled": false,
            "version": "1.0.0",
            "license": {
                "name": "Synthetic test only",
                "source": "https://example.invalid/license"
            },
            "artifact": {
                "url": "https://example.invalid/fixture.zip",
                "sha256": "a".repeat(64),
                "archiveBytes": 100,
                "extractedBytes": 200
            },
            "compatibility": [{ "gameSha256": "b".repeat(64), "evidence": "Synthetic only" }],
            "files": [{
                "source": "x32/d3d9.dll",
                "target": "d3d9.dll",
                "sha256": "c".repeat(64),
                "bytes": 100,
                "preimage": { "kind": "absent" }
            }],
            "dependencies": [],
            "conflicts": [],
            "removal": "restorePreimages"
        }]
    })
}

fn rejected(value: &Value) {
    assert_eq!(
        parse_catalog(&serde_json::to_vec(value).unwrap())
            .unwrap_err()
            .code,
        "invalidModCatalog"
    );
}

#[test]
fn bundled_candidates_remain_informational_without_invented_artifacts_or_builds() {
    let catalog = bundled_catalog().unwrap();
    assert_eq!(catalog.entries.len(), 5);
    for recipe in &catalog.entries {
        let missing = recipe.missing_evidence();
        assert!(missing.contains(&MissingEvidence::Artifact));
        assert!(missing.contains(&MissingEvidence::Compatibility));
        assert!(missing.contains(&MissingEvidence::Files));
        assert!(missing.contains(&MissingEvidence::Removal));
    }
    assert!(
        catalog.entries[0]
            .missing_evidence()
            .contains(&MissingEvidence::License)
    );
}

#[test]
fn requested_defaults_do_not_grant_installation_or_remove_unresolved_conflicts() {
    let catalog = bundled_catalog().unwrap();
    let selected: Vec<_> = catalog
        .entries
        .iter()
        .filter(|r| r.default_enabled)
        .map(|r| r.id.as_str())
        .collect();
    assert!(selected.is_empty());
    let optional: Vec<_> = catalog
        .entries
        .iter()
        .filter(|r| !r.default_enabled)
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(
        optional,
        [
            "fix-enhancers",
            "dxvk",
            "reshade",
            "tasm1-skin-pack",
            "arachnoman"
        ]
    );
    assert_eq!(catalog.groups.len(), 1);
    let pack = &catalog.groups[0];
    assert_eq!(pack.id, "performance-pack");
    assert!(pack.default_enabled);
    assert_eq!(pack.members, ["fix-enhancers", "reshade", "dxvk"]);
    assert!(!catalog.entries.iter().any(|r| r.id == "tasm1-mod-import"));
    for recipe in &catalog.entries {
        assert!(recipe.artifact.is_none());
        assert!(!recipe.missing_evidence().is_empty());
    }
    assert!(catalog.entries[0].conflicts.iter().any(|id| id == "dxvk"));
    assert!(
        catalog.entries[2]
            .conflicts
            .iter()
            .any(|id| id == "fix-enhancers")
    );
}

#[test]
fn groups_reject_missing_members_duplicate_ownership_and_individual_defaults() {
    let value: Value = serde_json::to_value(bundled_catalog().unwrap()).unwrap();
    let mut bad = value.clone();
    bad["groups"][0]["members"] = json!(["missing"]);
    rejected(&bad);
    let mut bad = value.clone();
    bad["groups"][0]["members"] = json!(["fix-enhancers", "fix-enhancers"]);
    rejected(&bad);
    let mut bad = value.clone();
    bad["entries"][0]["defaultEnabled"] = json!(true);
    rejected(&bad);
    let mut bad = value.clone();
    let mut second = bad["groups"][0].clone();
    second["id"] = json!("other-pack");
    bad["groups"].as_array_mut().unwrap().push(second);
    rejected(&bad);
    let mut bad = value;
    bad["groups"][0]["members"] = json!(["tasm1-skin-pack"]);
    rejected(&bad);
}

#[test]
fn missing_or_malformed_defaults_and_unknown_mod_kinds_are_rejected() {
    let mut value = fixture();
    value["entries"][0]["defaultEnabled"] = json!("true");
    rejected(&value);
    value["entries"][0]
        .as_object_mut()
        .unwrap()
        .remove("defaultEnabled");
    rejected(&value);
    let mut value = fixture();
    value["entries"][0]["kind"] = json!("externalTool");
    rejected(&value);
}

#[test]
fn complete_synthetic_recipe_describes_exact_preimages_and_round_trips() {
    let mut value = fixture();
    value["entries"][0]["files"][0]["preimage"] =
        json!({ "kind": "sha256", "sha256": "d".repeat(64) });
    let catalog = parse_catalog(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(catalog.entries[0].missing_evidence().is_empty());
    let encoded = serde_json::to_vec(&catalog).unwrap();
    assert!(
        parse_catalog(&encoded).unwrap().entries[0]
            .missing_evidence()
            .is_empty()
    );
}

#[test]
fn missing_evidence_does_not_become_an_implicit_default() {
    for (field, empty, expected) in [
        ("version", Value::Null, MissingEvidence::Version),
        ("license", Value::Null, MissingEvidence::License),
        ("artifact", Value::Null, MissingEvidence::Artifact),
        ("compatibility", json!([]), MissingEvidence::Compatibility),
        ("files", json!([]), MissingEvidence::Files),
        ("removal", Value::Null, MissingEvidence::Removal),
    ] {
        let mut value = fixture();
        value["entries"][0][field] = empty;
        let parsed = parse_catalog(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(parsed.entries[0].missing_evidence().contains(&expected));
    }
}

#[test]
fn unknown_versions_games_fields_duplicate_ids_and_excessive_input_are_rejected() {
    for (field, bad) in [
        ("schemaVersion", json!(2)),
        ("game", json!("tasm2")),
        ("installCommand", json!("anything")),
    ] {
        let mut value = fixture();
        value[field] = bad;
        rejected(&value);
    }
    let mut value = fixture();
    value["entries"][0]["files"][0]["command"] = json!("not allowed");
    rejected(&value);
    let mut value = fixture();
    let entry = value["entries"][0].clone();
    value["entries"].as_array_mut().unwrap().push(entry.clone());
    rejected(&value);
    value["entries"] = json!(vec![entry; 33]);
    rejected(&value);
    assert!(parse_catalog(&vec![b' '; MAX_CATALOG_BYTES + 1]).is_err());
}

#[test]
fn urls_cannot_request_shell_local_files_credentials_or_insecure_transport() {
    for bad in [
        "http://example.invalid/fix",
        "file:///C:/private",
        "javascript:alert(1)",
        "https://user:secret@example.invalid/fix",
        "https://example.invalid:444/fix",
        "https://example.invalid/fix#fragment",
        "https://example.invalid\\fix",
        "https://example.invalid/\nfix",
    ] {
        for field in ["source", "artifact", "license"] {
            let mut value = fixture();
            match field {
                "artifact" => value["entries"][0]["artifact"]["url"] = json!(bad),
                "license" => value["entries"][0]["license"]["source"] = json!(bad),
                _ => value["entries"][0]["source"] = json!(bad),
            }
            rejected(&value);
        }
    }
}

#[test]
fn windows_escape_paths_aliases_and_device_names_are_rejected() {
    for bad in [
        "../d3d9.dll",
        "/d3d9.dll",
        "x//d3d9.dll",
        "C:/d3d9.dll",
        "x\\d3d9.dll",
        "d3d9.dll:stream",
        "NUL",
        "con.txt",
        "x/LPT1.dll",
        "COM9.ini",
        "x/CONOUT$",
        "x./a.dll",
        "x /a.dll",
        "x/a?.dll",
        "x/a\u{7f}.dll",
        "x/é.dll",
    ] {
        for field in ["source", "target"] {
            let mut value = fixture();
            value["entries"][0]["files"][0][field] = json!(bad);
            rejected(&value);
        }
    }
}

#[test]
fn case_collisions_and_parent_file_collisions_are_rejected() {
    for (source, target) in [
        ("other.dll", "D3D9.DLL"),
        ("X32/D3D9.DLL", "other.dll"),
        ("other.dll", "d3d9.dll/child.dll"),
        ("x32/d3d9.dll/child.dll", "other.dll"),
    ] {
        let mut value = fixture();
        let mut second = value["entries"][0]["files"][0].clone();
        second["source"] = json!(source);
        second["target"] = json!(target);
        value["entries"][0]["files"]
            .as_array_mut()
            .unwrap()
            .push(second);
        rejected(&value);
    }
}

#[test]
fn hashes_sizes_and_unbounded_payloads_are_rejected() {
    for (field, bad) in [
        ("sha256", json!("unverified")),
        ("archiveBytes", json!(0)),
        ("archiveBytes", json!(134217729)),
        ("extractedBytes", json!(99)),
        ("extractedBytes", json!(268435457)),
    ] {
        let mut value = fixture();
        value["entries"][0]["artifact"][field] = bad;
        rejected(&value);
    }
    let mut value = fixture();
    value["entries"][0]["files"][0]["preimage"] = json!({ "kind": "sha256", "sha256": "short" });
    rejected(&value);
    value["entries"][0]["files"][0]["preimage"] = json!({ "kind": "anything" });
    rejected(&value);
}

#[test]
fn metadata_and_self_dependencies_are_bounded() {
    for (field, bad) in [
        ("id", json!("UPPERCASE")),
        ("author", json!("")),
        ("name", json!("x".repeat(161))),
        ("version", json!("\ninvalid")),
        (
            "dependencies",
            json!([{ "id": "synthetic-fix", "version": "1.0.0" }]),
        ),
        ("conflicts", json!(["synthetic-fix"])),
    ] {
        let mut value = fixture();
        value["entries"][0][field] = bad;
        rejected(&value);
    }
}
