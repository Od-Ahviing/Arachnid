use crate::{
    offline::{Payload, publish_payload},
    performance,
};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::{self, DirectoryGuard};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bundle {
    schema_version: u32,
    files: Vec<EffectFile>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectFile {
    name: String,
    folder: String,
    bytes: u64,
    sha256: String,
    source: String,
}

fn manifest() -> AppResult<Bundle> {
    let bundle: Bundle =
        serde_json::from_str(include_str!("../../../manifests/reshade-effects.json"))
            .map_err(|_| invalid())?;
    let mut names = BTreeSet::new();
    if bundle.schema_version != 1 || bundle.files.is_empty() || bundle.files.len() > 32 {
        return Err(invalid());
    }
    for file in &bundle.files {
        filesystem::validate_relative(&file.name)?;
        if file.name.contains('/')
            || !["shaders", "textures", "notices"].contains(&file.folder.as_str())
            || file.bytes == 0
            || file.bytes > 1024 * 1024
            || !crate::hash(&file.sha256)
            || !file
                .source
                .starts_with("https://raw.githubusercontent.com/")
            || !names.insert(file.name.to_ascii_lowercase())
        {
            return Err(invalid());
        }
    }
    Ok(bundle)
}

pub fn deploy(source: &Path, destination: &Path) -> AppResult<()> {
    let bundle = manifest()?;
    deploy_files(source, destination, &bundle)
}

fn deploy_files(source: &Path, destination: &Path, bundle: &Bundle) -> AppResult<()> {
    let _source = DirectoryGuard::acquire(source)?;
    let _destination = DirectoryGuard::acquire(destination)?;
    let descriptors: Vec<_> = bundle
        .files
        .iter()
        .map(|f| Payload {
            file_name: f.name.clone(),
            bytes: f.bytes,
            sha256: f.sha256.clone(),
        })
        .collect();
    let _files = performance::verified_files(source, &descriptors)?;
    for (file, payload) in bundle.files.iter().zip(&descriptors) {
        let folder = filesystem::resolve(destination, &file.folder)?;
        fs::create_dir_all(&folder).map_err(|_| invalid())?;
        let _folder = DirectoryGuard::acquire(&folder)?;
        let mut input = fs::File::open(source.join(&file.name)).map_err(|_| invalid())?;
        publish_payload(&folder, payload, &mut input)?;
    }
    Ok(())
}

pub fn verify_prepared(source: &Path) -> AppResult<()> {
    let bundle = manifest()?;
    let names: Vec<_> = bundle.files.iter().map(|file| file.name.as_str()).collect();
    performance::exact_directory(source, &names)?;
    performance::verified_files(
        source,
        &bundle
            .files
            .into_iter()
            .map(|f| Payload {
                file_name: f.name,
                bytes: f.bytes,
                sha256: f.sha256,
            })
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}

fn invalid() -> AppError {
    AppError::new("invalidShaderBundle")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_deployment_checks_content_and_never_overwrites_personal_files() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let bytes = b"synthetic shader";
        fs::write(source.path().join("Test.fx"), bytes).unwrap();
        let bundle = Bundle {
            schema_version: 1,
            files: vec![EffectFile {
                name: "Test.fx".into(),
                folder: "shaders".into(),
                bytes: bytes.len() as u64,
                sha256: crate::fix_adapter::digest(bytes),
                source: "https://raw.githubusercontent.com/fixture/commit/Test.fx".into(),
            }],
        };
        deploy_files(source.path(), destination.path(), &bundle).unwrap();
        deploy_files(source.path(), destination.path(), &bundle).unwrap();
        let output = destination.path().join("shaders/Test.fx");
        assert_eq!(fs::read(&output).unwrap(), bytes);
        fs::write(&output, b"personal shader").unwrap();
        assert!(deploy_files(source.path(), destination.path(), &bundle).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"personal shader");
        fs::write(source.path().join("Test.fx"), b"bad source").unwrap();
        assert!(deploy_files(source.path(), destination.path(), &bundle).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"personal shader");
    }
    #[test]
    fn manifest_lists_required_effects_and_dependencies() {
        let bundle = manifest().unwrap();
        for name in [
            "SMAA.fx",
            "SMAA.fxh",
            "LumaSharpen.fx",
            "qUINT_mxao.fx",
            "qUINT_common.fxh",
            "AreaTex.png",
            "SearchTex.png",
            "ReShade.fxh",
            "ReShadeUI.fxh",
        ] {
            assert!(bundle.files.iter().any(|f| f.name == name));
        }
    }
}
