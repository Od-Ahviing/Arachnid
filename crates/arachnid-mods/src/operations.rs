use crate::{
    Catalog, Preimage,
    archive::PreparedArchive,
    payloads::{Payload, PayloadResources},
    selection::{Selection, resolve},
};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Action, OperationPlan, Resources, Target};
use std::collections::BTreeMap;

/// The caller must authenticate the bundled catalogue and hold the installation lease.
/// Preparing a plan writes backups outside the game; applying it requires separate exact consent.
pub fn plan_installation(
    catalog: &Catalog,
    selection: &Selection,
    game_sha256: &str,
    installation: &str,
    resources: &PayloadResources,
    archives: &BTreeMap<String, PreparedArchive>,
) -> AppResult<OperationPlan> {
    if !crate::hash(game_sha256) || !crate::hash(installation) {
        return Err(AppError::new("modBuildUnsupported"));
    }
    let resolution = resolve(catalog, selection)?;
    let mut actions = Vec::new();
    let mut file_count = 0usize;
    let mut payload_bytes = 0u64;
    // Check every selected recipe before staging any preimages.
    for id in &resolution.order {
        let recipe = catalog
            .entries
            .iter()
            .find(|r| &r.id == id)
            .ok_or_else(|| AppError::new("invalidModCatalog"))?;
        if !recipe.missing_evidence().is_empty() {
            return Err(AppError::new("modRecipeUnqualified"));
        }
        if !recipe
            .compatibility
            .iter()
            .any(|c| c.game_sha256.eq_ignore_ascii_case(game_sha256))
        {
            return Err(AppError::new("modBuildUnsupported"));
        }
        if !archives.contains_key(id) {
            return Err(AppError::new("modPayloadMissing"));
        }
        file_count += recipe.files.len();
        payload_bytes += recipe.files.iter().map(|file| file.bytes).sum::<u64>();
        if file_count > 32 || payload_bytes > 64 * 1024 * 1024 {
            return Err(AppError::new("operationLimit"));
        }
    }
    for id in &resolution.order {
        let recipe = catalog
            .entries
            .iter()
            .find(|r| &r.id == id)
            .ok_or_else(|| AppError::new("invalidModCatalog"))?;
        let archive = &archives[id];
        for file in &recipe.files {
            let target = Target::File {
                path: file.target.clone(),
            };
            let before = resources.read(&target)?;
            let matches = match (&file.preimage, &before) {
                (Preimage::Absent, None) => true,
                (Preimage::Sha256 { sha256 }, Some(image)) => Payload::from_image(image)?
                    .sha256
                    .eq_ignore_ascii_case(sha256),
                _ => false,
            };
            if !matches {
                return Err(AppError::new("modPreimageConflict"));
            }
            let source = archive
                .file(&file.target)
                .ok_or_else(|| AppError::new("modPayloadMissing"))?;
            let after = Some(resources.import(
                &source,
                &Payload {
                    sha256: file.sha256.to_ascii_lowercase(),
                    bytes: file.bytes,
                },
            )?);
            if before == after {
                return Err(AppError::new("modAlreadyPresent"));
            }
            actions.push(Action {
                target,
                before,
                after,
            });
        }
    }
    let plan = OperationPlan {
        schema_version: 1,
        installation: installation.into(),
        actions,
    };
    PayloadResources::validate_plan(&plan)?;
    Ok(plan)
}
