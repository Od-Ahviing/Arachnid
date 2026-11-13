use crate::payloads::{Payload, PayloadResources};
use arachnid_core::{AppError, AppResult, game_build};
use arachnid_repair::{Action, Journal, Phase, Target, filesystem::DirectoryGuard};
use std::{fs, path::Path};

// Retained only to restore backups made before the user withdrew the FPS patch.
const LEGACY_PATCHED: &str = "033b51e9b4b0a1592733b8140bc14f1e7cfa7c148c6f96a22f473913230ac956";

struct Recipe<'a> {
    bytes: u64,
    original: &'a str,
    patched: &'a str,
}
const RECIPE: Recipe<'static> = Recipe {
    bytes: 6_895_912,
    original: game_build::ORIGINAL,
    patched: LEGACY_PATCHED,
};

fn descriptor(hash: &str, recipe: &Recipe<'_>) -> Payload {
    Payload {
        sha256: hash.into(),
        bytes: recipe.bytes,
    }
}
fn target() -> Target {
    Target::File {
        path: "Game.exe".into(),
    }
}

pub fn restore(
    root: &Path,
    state: &Path,
    installation: &str,
    stopped: impl Fn() -> AppResult<()>,
) -> AppResult<()> {
    restore_recipe(root, state, installation, &RECIPE, stopped)
}

fn restore_recipe(
    root: &Path,
    state: &Path,
    installation: &str,
    recipe: &Recipe<'_>,
    stopped: impl Fn() -> AppResult<()>,
) -> AppResult<()> {
    let _guard = DirectoryGuard::acquire(state)?;
    let path = state.join("frame-limit.json");
    let metadata = fs::symlink_metadata(&path).map_err(|_| AppError::new("noOperation"))?;
    if !metadata.is_file() || arachnid_detector::is_link(&metadata) {
        return Err(AppError::new("recoveryRequired"));
    }
    let mut journal = Journal::load(&path)?;
    if journal.plan.installation != installation
        || journal.phase != Phase::Committed
        || journal.plan.actions
            != vec![Action {
                target: target(),
                before: Some(descriptor(recipe.original, recipe).image()?),
                after: Some(descriptor(recipe.patched, recipe).image()?),
            }]
        || journal.context != Some(serde_json::json!({"domain":"arachnid-frame-limit-v1"}))
    {
        return Err(AppError::new("recoveryRequired"));
    }
    let mut resources = PayloadResources::new(root, &state.join("blobs"))?;
    stopped()?;
    arachnid_repair::rollback_guarded(&mut journal, &mut resources, &path, stopped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arachnid_repair::OperationPlan;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn legacy_restore_preserves_original_bytes_and_refuses_external_changes() {
        for external in [false, true] {
            let game = tempfile::tempdir().unwrap();
            let state = tempfile::tempdir().unwrap();
            let source = tempfile::tempdir().unwrap();
            let original = b"original fixture";
            let patched = b"modified fixture";
            let original_hash = crate::fix_adapter::digest(original);
            let patched_hash = crate::fix_adapter::digest(patched);
            let recipe = Recipe {
                bytes: original.len() as u64,
                original: &original_hash,
                patched: &patched_hash,
            };
            fs::create_dir(state.path().join("blobs")).unwrap();
            fs::write(game.path().join("Game.exe"), original).unwrap();
            fs::write(source.path().join("fixture"), patched).unwrap();
            let mut resources =
                PayloadResources::new(game.path(), &state.path().join("blobs")).unwrap();
            let before = resources
                .import(
                    &game.path().join("Game.exe"),
                    &descriptor(&original_hash, &recipe),
                )
                .unwrap();
            let after = resources
                .import(
                    &source.path().join("fixture"),
                    &descriptor(&patched_hash, &recipe),
                )
                .unwrap();
            let plan = OperationPlan {
                schema_version: 1,
                installation: "fixture".into(),
                actions: vec![Action {
                    target: target(),
                    before: Some(before),
                    after: Some(after),
                }],
            };
            let path = state.path().join("frame-limit.json");
            let mut journal = arachnid_repair::prepare_with_context(
                plan,
                &resources,
                &path,
                Some(serde_json::json!({"domain":"arachnid-frame-limit-v1"})),
            )
            .unwrap();
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
            drop(resources);
            if external {
                fs::write(game.path().join("Game.exe"), b"personal changes").unwrap();
            }
            let result = restore_recipe(game.path(), state.path(), "fixture", &recipe, || Ok(()));
            if external {
                assert!(result.is_err());
                assert_eq!(
                    fs::read(game.path().join("Game.exe")).unwrap(),
                    b"personal changes"
                );
            } else {
                result.unwrap();
                assert_eq!(fs::read(game.path().join("Game.exe")).unwrap(), original);
            }
        }
    }
}
