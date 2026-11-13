# Architecture and runtime

## Code map

| Location                                                                                                                                           | Responsibility                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [src/main.ts](../src/main.ts), [src/App.vue](../src/App.vue), [src/router/index.ts](../src/router/index.ts)                                        | Vue startup, launcher shell, and navigation. The router requires a recognized installation outside Setup.                                                                 |
| [src/views](../src/views), [src/stores/installation.ts](../src/stores/installation.ts)                                                             | Page workflows and shared installation state.                                                                                                                             |
| [src/services/desktop.ts](../src/services/desktop.ts), [src/types](../src/types)                                                                   | Typed TypeScript calls to Tauri and their data contracts.                                                                                                                 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs), [src-tauri/src/commands/mod.rs](../src-tauri/src/commands/mod.rs)                                 | Command registration, shared state, authorization, and orchestration. Mods, costumes, saves, storage, and window lifecycle also have dedicated modules in this directory. |
| [arachnid-core](../crates/arachnid-core), [arachnid-detector](../crates/arachnid-detector), [arachnid-diagnostics](../crates/arachnid-diagnostics) | Shared types, game build identity, coordination and launch readiness, file inspection, and diagnostic findings.                                                           |
| [arachnid-launch](../crates/arachnid-launch), [arachnid-settings](../crates/arachnid-settings), [arachnid-repair](../crates/arachnid-repair)       | Process and runtime management, native settings, and journaled transactions for files and the registry.                                                                   |
| [arachnid-mods](../crates/arachnid-mods), [arachnid-saveguard](../crates/arachnid-saveguard)                                                       | Prepared mod compositions and costumes, save discovery, snapshots, and restores.                                                                                          |
| [manifests](../manifests)                                                                                                                          | Installation reference metadata and inventories for community packages, effects, and costumes.                                                                            |

The normal path for a system operation is a Vue view calling `desktop`, a Tauri command validating the request and coordinating shared state, then a Rust crate carrying out the operation. Keep Windows paths, registry access, process control, and file mutations in Rust. Update both sides of the command contract and their tests when changing a payload or an error code.

## Installation, diagnostics, and launch

1. Setup opens a folder picker and calls `scan_installation`. The backend inspects the executable and resources in read-only mode, evaluates diagnostics, emits scan progress, and saves the folder in preferences after recognition.
2. **Play** calls `start_game`. The backend takes the operation lock and checks for a running game. Language synchronization checks for incomplete transactions before it can write. The launch path then checks recovery state, scans again, and validates the executable build, diagnostics, and active mod runtime before starting `Game.exe`.
3. [lifecycle.rs](../src-tauri/src/lifecycle.rs) waits for the supervised game's visible window before destroying the launcher WebView. The Rust backend stays alive, including SaveGuard when enabled. On exit, Arachnid recreates the interface. If the game exits before displaying a window, the interface stays available.
4. Launch status and crash advice are returned to the UI. The advice for the recognized graphics crash depends on which graphics mods were active and points to ReShade first when it was enabled.

Recognition, launch eligibility, and mod compatibility are separate checks. A missing optional wrapper is normal for a clean installation. Use [game_build.rs](../crates/arachnid-core/src/game_build.rs), [readiness.rs](../crates/arachnid-core/src/readiness.rs), and the launch tests when working on build support.

## Settings and language

[GraphicsView.vue](../src/views/GraphicsView.vue) keeps edits as a local draft. **Apply changes** prepares and applies a checked plan, then refreshes the displayed state. [draftNavigation.ts](../src/composables/draftNavigation.ts) handles leaving a page with unapplied changes. Preset definitions and supported settings live in [arachnid-settings](../crates/arachnid-settings). The original presets are Very Low, Low, Medium, High, and Amazing. Manual edits can produce a Custom configuration.

[GameLanguageControl.vue](../src/components/game/GameLanguageControl.vue) exposes one preference for the UI and game on Home and during Setup. [i18n.ts](../src/composables/i18n.ts) resolves the device locale in Automatic mode. English is the interface fallback. [languages.rs](../crates/arachnid-settings/src/languages.rs) handles the game's `Install/Language` value and checks the installed resources. Game changes are deferred while it runs, and English is a game fallback only when its resources are present.

## Mods and recovery

- [ModsView.vue](../src/views/ModsView.vue) keeps graphics selections and the costume choice in a draft. Apply prepares and commits the requested transactions, then refreshes the installation state. Graphics mods and costumes use separate transactions, so an error in one can follow a successful change in the other.
- [composition.rs](../crates/arachnid-mods/src/composition.rs) models the eight combinations of Fix Enhancers, ReShade, and DXVK, including all off. Fix Enhancers alone is preselected on a fresh setup. [performance.rs](../crates/arachnid-mods/src/performance.rs) defines prepared graphics recipes, and [mods.rs](../src-tauri/src/mods.rs) connects them to Tauri and the launch runtime. ReShade uses Direct3D 9 without DXVK and a per-session Vulkan layer with DXVK.
- [skins.rs](../crates/arachnid-mods/src/skins.rs) manages the optional texture packs. Some replace the base suit and others target a different suit in the wardrobe. **Original textures** removes the Arachnid-managed costume change.
- [arachnid-repair](../crates/arachnid-repair) supplies the common transaction machinery. A prepared plan identifies the intended changes and their expected starting state. Applying the approved plan rechecks its digest and current state, preserves the originals, and records recovery information. Files changed outside Arachnid require review. Deactivation restores the pre-mod state, which can include an existing wrapper.

## SaveGuard

[saves.rs](../src-tauri/src/saves.rs) wires SaveGuard to desktop commands and the background watcher. [discovery.rs](../crates/arachnid-saveguard/src/discovery.rs) looks for known save locations using the installation and files it finds. Settings provides a manual source override.

The [SaveGuard crate](../crates/arachnid-saveguard/src/lib.rs) checks stable file contents before copying. Automatic backups group changes into fixed five-minute windows. Later changes update that window's backup without extending the deadline. Manual and pinned snapshots have their own lifecycle. Restore checks the snapshot, preserves the current state for rollback, and requires the game to be stopped before applying an approved plan. Test that a restored save actually loads when changing this workflow. A matching hash verifies the copied bytes.

## Local data

Tauri resolves the app's directory under Windows Local AppData. [storage.rs](../src-tauri/src/storage.rs) manages `preferences.json` and `launcher.log`. SaveGuard keeps its profile in `saveguard-profile.json` and stores snapshots in `saveguard/` under the app data directory, outside the save source. The diagnostics report is exported to a destination chosen by the player.
