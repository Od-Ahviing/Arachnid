# Development rules

The [contributor documentation](docs/README.md) covers the architecture, code workflows, testing, and builds. The [README](README.md) introduces the launcher and explains how to use it.

## Scope and contributions

- Target The Amazing Spider-Man (2012) for PC on Windows 10 and 11. Players supply their own game installation. TASM 2 is outside the project scope.
- Keep changes focused. Reuse existing modules and justify new dependencies. Update the documentation when behavior or procedures change.
- Write code identifiers and commit messages in English. Keep code readable and maintainable. Avoid unnecessary abstractions. Add a comment when the code alone cannot explain the intent or constraint.
- Omit optional semicolons in JavaScript, TypeScript, and Vue. Keep them where syntax requires them or where they make multiple statements on one line easier to read.
- Translate new user-facing text into English, French, German, Italian, and Spanish. Preserve the shared launcher and game language setting, automatic language selection, and English fallback. Reuse the approved components, fonts, and logo.

## Game installation and player data

- Keep system operations in Rust behind Tauri commands. Validate paths, current state, and game build compatibility in the backend. Keep diagnostics read-only.
- Keep Graphics and Mods changes in a draft until the player selects **Apply changes**. Check preconditions, preserve originals, log writes, verify the result, and support rollback. Keep explicit approval for restores and recovery. Language synchronization changes only the designated native setting and requires the game to be closed.
- Protect personal saves. Store SaveGuard backups outside the monitored source folder and test restores using disposable data.
- Prefer verified native settings and targeted, reversible fixes. Keep bundled mods usable offline. Respect third-party licenses and the code's GPL-2.0-only license.
- Keep game binaries, game archives, extracted assets, personal saves, and secrets out of the repository. Keep Windows protections enabled. Exclude unverified binaries, universal injectors, and DRM bypasses.

## Verification

- Add or update tests for each feature you change. Cover rejected operations, partial writes, concurrency, and recovery where relevant.
- Use fixtures and the repository's test tools for automated tests. Verify Windows workflows and in-game behavior on a trusted installation when needed.
- Report the checks you ran and their limitations. Support compatibility or performance claims with the relevant tests or measurements.
