# Arachnid

Arachnid is an open-source community launcher for **The Amazing Spider-Man (2012) on PC**. Its goal is to preserve the PC release by keeping it playable on modern Windows, restoring broken features, and giving the community a way to share compatibility fixes, performance improvements, and mods.

Arachnid targets Windows 10 and 11 (x64). Bring your own PC installation of the game. The launcher comes separately.

## Getting started

1. Open **Releases** on GitHub to check whether a public Windows installer is available. Download and run the setup executable when one is published.
2. Open Arachnid and select the folder containing `Game.exe` and `Data`. Arachnid checks the installation structure and reports compatibility issues.
3. On **Home**, click **Play**. Arachnid checks the selected installation again and launches `Game.exe` directly when it passes the launch checks.

The Windows installer checks for required legacy DirectX, Visual C++, and .NET components. Installing missing Windows components may require an internet connection or administrator approval. Mods included in the installer are available offline.

Starting with **Arachnid 0.1.1**, signed releases check GitHub for newer stable versions at startup and offer to install them in the launcher. Close the game before updating. If you use 0.1.0, download and install the first updater-enabled release manually.

## What you can do

| Section         | What it does                                                                                                                                                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Home**        | Launch the game, check its status, and choose the language for Arachnid and the game.                                                                                                                         |
| **Graphics**    | Change resolution, display mode, and supported graphics options. Use the original presets **Very Low**, **Low**, **Medium**, **High**, and **Amazing**, or adjust individual settings for a **Custom** setup. |
| **Mods**        | Toggle Fix Enhancers, ReShade, and DXVK individually. Choose optional costume textures and switch back to **Original textures** to remove them.                                                               |
| **Saves**       | Use SaveGuard to back up changes automatically, create manual snapshots, or restore an earlier save. Backups live in Arachnid’s local app data, outside the game's save folder.                               |
| **Diagnostics** | Check the installation, inspect missing dependencies and file warnings, or export a report for troubleshooting.                                                                                               |
| **Settings**    | Change the game folder or point SaveGuard to a different save folder if your edition stores saves somewhere else.                                                                                             |

Arachnid applies **Fix Enhancers** automatically once it recognizes a compatible game installation and can safely apply the mod. You can turn it off under **Mods**. ReShade and DXVK are optional. In **Graphics** and **Mods**, make your selections and click **Apply changes** to apply and verify them together.

On a fresh Windows profile, Arachnid creates the missing display and graphics settings before the first launch. The initial settings are 1920 × 1080 in fullscreen with the Medium preset. Choose your preferred settings under **Graphics**.

SaveGuard detects changes to save contents and groups automatic backups in five-minute windows. To restore a snapshot, close the game, choose the snapshot in **Saves**, review the changes, and confirm.

## Supported languages

**English · French (Français) · German (Deutsch) · Italian (Italiano) · Spanish (Español)**

The language selector on **Home** controls both the launcher and the game. **Automatic** follows your device language and uses English if that language is unsupported. Arachnid checks which language files are present in your copy of the game. If the requested game language is missing, Arachnid can fall back to English when its files are available.

## Troubleshooting

- **The game crashes or closes in the same spot.** In **Mods**, disable **ReShade** first if it is enabled, click **Apply changes**, and retry the same scene. If it still crashes, try disabling the other graphics mods one at a time. You can also test with all three disabled to establish a baseline.
- **Play is unavailable or the game fails to start.** Open **Diagnostics** and check the game folder, executable build, and runtime warnings. Arachnid checks launch compatibility separately from folder recognition, so a recognized folder can still have an unsupported executable or missing dependencies.
- **The game stutters or renders incorrectly.** Try a lower preset in **Graphics**, apply it, and compare the same scene. If DXVK is active, test with it disabled. The results depend on your GPU and driver.
- **A modded costume looks unchanged.** Some texture packs target another suit, such as Scarlet Spider. Equip the corresponding suit in the game's wardrobe when available. Select **Original textures** and apply the change to revert.
- **SaveGuard finds no saves.** Check which save folder your installation uses and update the source in **Settings**. If you need to recover progress, use the snapshots listed in **Saves**.

If the problem persists, open a GitHub issue with the Arachnid version, your Windows version, GPU and driver, enabled mods, and steps to reproduce the problem. Include the exact error or exit code if one appears, and attach a diagnostics report if you’re comfortable sharing it.

## Contributing

You can help preserve TASM1 by reproducing bugs, testing on different Windows versions and GPUs, improving translations and documentation, restoring broken graphics or gameplay features, or developing compatibility fixes, optimizations, and mods.

- **Report a bug.** Describe what happened, what you expected, and where it happens in the game. Include the active graphics preset and mods, and say whether you reproduced it with the mods disabled.
- **Propose a fix or mod.** Open an issue describing the problem it solves, which game builds it targets, and how to test, apply, and remove it. For an existing community project, include the original source, author, version, and license.
- **Test a performance change.** Use the same scene, resolution, and settings before and after the change. Share your hardware, driver, active mods, and the measurements you collected.
- **Submit a pull request.** Keep it focused and include the tests you ran. Use English identifiers, follow the repository's [development guidelines](AGENTS.md), and keep game files, personal saves, and secrets out of commits.

### Develop on Windows

You’ll need Node.js 26 LTS (26.13.0 or later within the 26.x series), pnpm 11.25.0, the Rust MSVC toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml), Visual Studio C++ Build Tools with the Windows SDK, and WebView2.

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

Run the project's checks before opening a pull request.

```powershell
pnpm check
pnpm format:check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The desktop app uses **Tauri 2**, **Rust**, **Vue 3**, and **TypeScript**. The Vue UI is in `src/`, desktop commands are in `src-tauri/`, and Rust modules are in `crates/`. The automated tests use fixtures. For manual testing, point the launcher at a game installation outside the repository.

Start with [CONTRIBUTING.md](CONTRIBUTING.md) for the recommended reading order and how to prepare a contribution. The [contributor guide](docs/README.md) indexes the documentation by topic.

## License

Arachnid's source code is licensed under [GPL-2.0-only](LICENSE). Third-party mods, fonts, and assets retain their respective licenses.
