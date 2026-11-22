# Development and testing

## Run the launcher locally

On Windows, use Node.js 26 LTS (26.13.0 or later within the 26.x series), pnpm 11.25.0, the [pinned Rust toolchain](../rust-toolchain.toml) with the MSVC target, Visual Studio C++ Build Tools and Windows SDK, and WebView2. GitHub Actions reads the Node.js major version from [.node-version](../.node-version), and [package.json](../package.json) defines the supported version range. Run `node --version` before installing dependencies to check which version your terminal uses.

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

Keep the game outside the repository. The automated suite uses fixtures and original test executables. To inspect a local game folder without starting it, pass its absolute path to the detector example.

```powershell
cargo run -p arachnid-detector --example inspect -- 'C:\Games\TASM1'
```

## Choose the right tests

| Change                                | What to check                                                                                                                                                                                                           |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| View, translation, or desktop payload | Extend [frontend tests](../tests/frontend). Check all supported languages for new strings and cover loading, failure, and retry or recovery as relevant. For draft pages, test Apply, discard, and staying on the page. |
| Detector, settings, or repairs        | Use synthetic files and test the rejected path as well as the expected one. Cover stale input, partial failure, and rollback if the change writes to disk or the registry.                                              |
| Launch or window lifecycle            | Use the [process harness](../tests/process-harness) for process tracking, exit codes, and window release and recreation. Recheck with a trusted game installation before reporting in-game behavior.                    |
| Graphics mod or costume               | Test apply, switch, disable, and recovery. Test the affected combinations and confirm the original state is restored. For an in-game test, record the actual suit or scene, game build, GPU, and driver.                |
| SaveGuard                             | Use disposable fixtures for changed contents, incomplete writes, and restore. Use a disposable game profile to verify in-game reload.                                                                                   |
| Installer                             | Run the [prerequisite script tests](../scripts/release/test-prerequisites.ps1). For installation changes, verify on the affected Windows versions and check the case where required components are missing.             |

Run the same checks as the [Windows CI workflow](../.github/workflows/ci.yml) before submitting a cross-cutting change.

For save-source diagnosis, `cargo run -p arachnid-saveguard --example inspect_saves --locked -- "C:\path\to\save-folder"` reports file count, total bytes and recognized native play time, then verifies a backup in a temporary directory. It reads the source without modifying it and never restores into it. Run it after the game has finished saving; a changing source causes verification to fail. Restore tests must still use disposable fixtures.

```powershell
pnpm check
pnpm format:check
./scripts/release/test-prerequisites.ps1
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
pnpm tauri build --debug --no-bundle
```

## Contribute a fix or mod

Start with a reproducible case. Describe the game build, scene, settings, active mods, expected behavior, and observed result. For an optimization, record the same scene, resolution, settings, hardware and driver before and after the change. Include the upstream source, author, version, and license for a third-party component.

Identify the correct integration point before editing. Native settings belong in `arachnid-settings`. Graphics compositions and costume recipes belong in `arachnid-mods`. File replacement and rollback use the existing repair infrastructure. If a mod introduces a new shared destination or runtime component, include its compatibility with existing combinations and a test for removing it. Keep the change and its tests in one focused pull request.

Submit contributions as pull requests for review. Only the repository owner, Od_Ahviing, merges pull requests into `main` and publishes launcher updates.

For UI changes, put strings in [src/locales](../src/locales) or the relevant translated messages module in [src/composables](../src/composables). Keep the dark graphite and crimson palette in [tokens.css](../src/assets/styles/tokens.css). The UI uses Unbounded for prominent titles and Instrument Sans for body and navigation. Reuse the approved [logo](../src/assets/brand/arachnid-logo.png) and shared components in [src/components/ui](../src/components/ui). Check keyboard access, reduced motion, and long translated labels.

## Build the Windows installer

- `pnpm tauri build --debug --no-bundle` builds a desktop executable for development.
- `pnpm mods:verify-prepared` verifies the prepared graphics combinations, shaders and costumes expected by the full installer. The prepared payloads are supplied separately from the source checkout.
- `pnpm build:offline:local` builds **Arachnid Local Test** with a separate application identifier. This is the package for local installer testing.
- `pnpm build:offline` verifies the prepared resources and runtime qualification for the graphics pack and costumes before building a standalone NSIS installer.

The [offline bundle configuration](../src-tauri/tauri.offline.conf.json) lists the prepared resource folders. [The verifier](../crates/arachnid-mods/examples/prepared.rs) checks those resources. The installer [hook](../src-tauri/installer/hooks.nsh) runs the [prerequisites script](../src-tauri/installer/tasm-prerequisites.ps1) for missing legacy DirectX and Visual C++ x86 components and .NET Framework 3.5. The script selects the .NET installation method for the Windows build. Offline refers to bundled mods. Fetching missing Windows components can require a connection.

The NSIS installer uses the Arachnid icon and sidebar image. The installer hook sets `Od_Ahviing` in Windows installed-app metadata and preserves the registry key that Tauri uses to find existing installations. Windows identifies the verified publisher in its security prompt through a separate trusted Authenticode signature.
