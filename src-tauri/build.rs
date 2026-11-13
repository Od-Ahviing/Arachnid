fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("resources/test-app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-examples=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "check_app_update",
            "install_app_update",
            "load_preferences",
            "mod_status",
            "select_mod",
            "prepare_pack",
            "apply_pack",
            "skin_status",
            "prepare_skin",
            "apply_skin",
            "save_language",
            "pick_game_directory",
            "scan_installation",
            "cancel_scan",
            "forget_installation",
            "export_report",
            "launch_status",
            "start_game",
            "synchronize_game_language",
            "native_settings",
            "prepare_settings",
            "apply_settings",
            "restore_settings",
            "settings_operation",
            "save_status",
            "open_save_folder",
            "delete_save",
            "detect_save_source",
            "choose_save_source",
            "configure_saveguard",
            "snapshot_saves",
            "pin_save",
            "prepare_save_restore",
            "apply_save_restore",
        ]),
    ))
    .expect("failed to prepare Arachnid desktop resources");
}
