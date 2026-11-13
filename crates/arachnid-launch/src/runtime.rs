use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::DirectoryGuard;
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};

pub struct VulkanRuntime {
    dxvk: bool,
    reshade: bool,
    layer_directory: PathBuf,
    base_directory: PathBuf,
    _directories: Vec<DirectoryGuard>,
    _files: Vec<File>,
    skin_config: Option<PathBuf>,
}

impl VulkanRuntime {
    /// Resource hashes and game ownership must be verified by the reviewed backend recipe first.
    pub fn new(layer: &Path, base: &Path, files: Vec<File>) -> AppResult<Self> {
        Self::for_components(layer, base, files, true, true)
    }

    pub fn for_components(
        layer: &Path,
        base: &Path,
        files: Vec<File>,
        dxvk: bool,
        reshade: bool,
    ) -> AppResult<Self> {
        let directories = vec![
            DirectoryGuard::acquire(layer)?,
            DirectoryGuard::acquire(base)?,
        ];
        let layer_directory = layer
            .canonicalize()
            .map_err(|_| AppError::new("invalidPath"))?;
        let base_directory = base
            .canonicalize()
            .map_err(|_| AppError::new("invalidPath"))?;
        // Vulkan and DXVK consume these paths through their own Windows path handling.
        let layer_directory = dunce::simplified(&layer_directory).to_path_buf();
        let base_directory = dunce::simplified(&base_directory).to_path_buf();
        if layer_directory.to_string_lossy().contains(';')
            || base_directory.to_string_lossy().contains(';')
        {
            return Err(AppError::new("invalidPath"));
        }
        Ok(Self {
            dxvk,
            reshade,
            layer_directory,
            base_directory,
            _directories: directories,
            _files: files,
            skin_config: None,
        })
    }

    pub fn with_skin(
        runtime: Option<Self>,
        game: &Path,
        assets: &Path,
        files: Vec<File>,
    ) -> AppResult<Self> {
        let mut runtime = match runtime {
            Some(runtime) => runtime,
            None => Self::for_components(game, game, vec![], false, false)?,
        };
        runtime._directories.push(DirectoryGuard::acquire(game)?);
        runtime._directories.push(DirectoryGuard::acquire(assets)?);
        runtime._files.extend(files);
        runtime.skin_config = Some(
            dunce::simplified(
                &game
                    .canonicalize()
                    .map_err(|_| AppError::new("invalidPath"))?,
            )
            .join("retex.ini"),
        );
        Ok(runtime)
    }

    pub(super) fn configure(&self, command: &mut Command) {
        if let Some(path) = &self.skin_config {
            command.env("RETEX_CONFIG", path).env_remove("RETEX_PACK");
        }
        if self.dxvk {
            command.env("DXVK_LOG_PATH", &self.base_directory);
        }
        if self.reshade {
            command
                .env("RESHADE_BASE_PATH_OVERRIDE", &self.base_directory)
                .env_remove("DISABLE_VK_LAYER_reshade_1")
                .env_remove("RESHADE_DISABLE_LOADING_CHECK");
        }
        if self.dxvk && self.reshade {
            command
                .env("VK_LAYER_PATH", &self.layer_directory)
                .env("VK_INSTANCE_LAYERS", "VK_LAYER_reshade")
                .env("RESHADE_BASE_PATH_OVERRIDE", &self.base_directory)
                .env("RESHADE_DISABLE_GRAPHICS_HOOK", "1")
                .env("DXVK_LOG_PATH", &self.base_directory)
                .env_remove("DISABLE_VK_LAYER_reshade_1")
                .env_remove("RESHADE_DISABLE_LOADING_CHECK");
        } else if self.reshade {
            command.env_remove("RESHADE_DISABLE_GRAPHICS_HOOK");
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn skin_environment_is_scoped_and_does_not_enable_graphics_layers() {
        let game = tempfile::tempdir().unwrap();
        let assets = tempfile::tempdir().unwrap();
        let runtime = VulkanRuntime::with_skin(None, game.path(), assets.path(), vec![]).unwrap();
        let mut command = Command::new("original-fixture.exe");
        runtime.configure(&mut command);
        let environment: std::collections::BTreeMap<_, _> = command.get_envs().collect();
        assert!(
            environment[std::ffi::OsStr::new("RETEX_CONFIG")]
                .unwrap()
                .to_string_lossy()
                .ends_with("retex.ini")
        );
        assert_eq!(environment[std::ffi::OsStr::new("RETEX_PACK")], None);
        assert!(!environment.contains_key(std::ffi::OsStr::new("VK_INSTANCE_LAYERS")));
        assert!(!environment.contains_key(std::ffi::OsStr::new("RESHADE_BASE_PATH_OVERRIDE")));
    }
    #[test]
    fn direct3d_reshade_and_no_reshade_do_not_request_a_vulkan_layer() {
        let layer = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        for (dxvk, reshade) in [(false, false), (true, false), (false, true)] {
            let runtime =
                VulkanRuntime::for_components(layer.path(), base.path(), vec![], dxvk, reshade)
                    .unwrap();
            let mut command = Command::new("original-fixture.exe");
            runtime.configure(&mut command);
            let environment: std::collections::BTreeMap<_, _> = command.get_envs().collect();
            assert!(!environment.contains_key(std::ffi::OsStr::new("VK_INSTANCE_LAYERS")));
            assert_eq!(
                environment.contains_key(std::ffi::OsStr::new("DXVK_LOG_PATH")),
                dxvk
            );
            if reshade {
                assert_eq!(
                    environment[std::ffi::OsStr::new("RESHADE_DISABLE_GRAPHICS_HOOK")],
                    None
                );
            }
        }
    }
    #[test]
    fn vulkan_configuration_is_scoped_to_the_command_and_keeps_directx_hooks_disabled() {
        let layer = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        let runtime = VulkanRuntime::new(layer.path(), base.path(), Vec::new()).unwrap();
        let inherited = std::env::var_os("VK_LAYER_PATH");
        let mut command = Command::new("original-fixture.exe");
        runtime.configure(&mut command);
        let environment: std::collections::BTreeMap<_, _> = command
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().to_string(),
                    v.map(|s| s.to_string_lossy().to_string()),
                )
            })
            .collect();
        assert_eq!(
            environment["VK_INSTANCE_LAYERS"].as_deref(),
            Some("VK_LAYER_reshade")
        );
        assert_eq!(
            environment["RESHADE_DISABLE_GRAPHICS_HOOK"].as_deref(),
            Some("1")
        );
        assert_eq!(environment["DISABLE_VK_LAYER_reshade_1"], None);
        for name in [
            "VK_LAYER_PATH",
            "RESHADE_BASE_PATH_OVERRIDE",
            "DXVK_LOG_PATH",
        ] {
            let path = environment[name].as_ref().unwrap();
            assert!(!path.starts_with(r"\\?\"));
            assert!(std::path::Path::new(path).is_absolute());
        }
        assert_eq!(std::env::var_os("VK_LAYER_PATH"), inherited);
        assert!(
            Command::new("another-fixture.exe")
                .get_envs()
                .next()
                .is_none()
        );
    }
}
