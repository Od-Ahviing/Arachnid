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

For UI changes, put strings in [src/locales](../src/locales) or the relevant translated messages module in [src/composables](../src/composables). Keep the dark graphite and crimson palette in [tokens.css](../src/assets/styles/tokens.css). The UI uses Unbounded for prominent titles and Instrument Sans for body and navigation. Reuse the approved [logo](../src/assets/brand/arachnid-logo.png) and shared components in [src/components/ui](../src/components/ui). Check keyboard access, reduced motion, and long translated labels.

## Build the Windows installer

- `pnpm tauri build --debug --no-bundle` builds a desktop executable for development.
- `pnpm mods:verify-prepared` verifies the prepared graphics combinations, shaders and costumes expected by the full installer. The prepared payloads are supplied separately from the source checkout.
- `pnpm build:offline:local` builds **Arachnid Local Test** with a separate application identifier. This is the package for local installer testing.
- `pnpm build:offline` verifies the prepared resources and runtime qualification for the graphics pack and costumes before building a standalone NSIS installer. Use the signed release workflow below to produce an updater-enabled installer.

The [offline bundle configuration](../src-tauri/tauri.offline.conf.json) lists the prepared resource folders. [The verifier](../crates/arachnid-mods/examples/prepared.rs) checks those resources. The installer [hook](../src-tauri/installer/hooks.nsh) runs the [prerequisites script](../src-tauri/installer/tasm-prerequisites.ps1) for missing legacy DirectX and Visual C++ x86 components and .NET Framework 3.5. The script selects the .NET installation method for the Windows build. Offline refers to bundled mods. Fetching missing Windows components can require a connection.

## Publish a Windows update

Set up signing and resources once in the public repository's **Settings → Secrets and variables → Actions**.

1. Run `pnpm tauri signer generate --write-keys "$env:USERPROFILE\arachnid-update.key"` to generate a Tauri signing key outside the repository. Set a password at the prompt and keep a backup of the key and password. Use the same key for subsequent releases so installed copies can verify them.
2. Set the repository variable `ARACHNID_UPDATER_PUBLIC_KEY` to the contents of the generated `.pub` file. Set the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to the private key file contents and its password.
3. From the project root, verify the prepared resources and create an archive outside the repository.

   ```powershell
   pnpm mods:verify
   $archive = Join-Path $env:TEMP 'arachnid-resources-1.zip'
   Compress-Archive -Path src-tauri/resources/performance-preview-3,src-tauri/resources/reshade-effects,src-tauri/resources/skins-preview -DestinationPath $archive
   Get-FileHash -LiteralPath $archive -Algorithm SHA256
   ```

   Host the versioned ZIP at an HTTPS URL the GitHub Windows runner can download. Set `ARACHNID_RESOURCES_URL` to that URL and `ARACHNID_RESOURCES_SHA256` to the hash. Update both values when the prepared resources change. The ZIP contains the prepared mod folders only. Keep it separate from the app release feed.

For each stable release, align the versions in `package.json`, `src-tauri/tauri.conf.json`, and the Cargo workspace, and refresh `Cargo.lock` when the workspace version changes. Push the version commit, check CI, then push the matching tag, such as `v0.1.1`. The [release workflow](../.github/workflows/release.yml) downloads and verifies the resource archive, builds the NSIS installer, and creates a draft with the installer, its signature and `latest.json`. Test the draft's installer before publishing it as the latest stable release. The GitHub latest-release URL then serves its manifest to installed launchers. Keep the three assets together.

Arachnid 0.1.0 users install the first updater-enabled release manually. For later releases, test updating from the previous stable version on Windows with disposable saves. Confirm the banner shows the newer version, the launcher remains usable offline, installation is blocked while the game is open, and the new version restarts with the same preferences and SaveGuard snapshots.
