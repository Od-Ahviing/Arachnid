# Contributing to Arachnid

Arachnid helps keep **The Amazing Spider-Man (2012) for PC** playable on Windows 10 and 11. Contributions that fix launcher bugs, improve compatibility, or add well-tested optimizations and mods are welcome.

## Where to start

1. Read the [contributor guide](docs/README.md) for the project philosophy and links to the documentation for each area of the launcher.
2. Read the [development rules](AGENTS.md) before editing code. They cover code style, translations, changes to game files, save protection, and testing.
3. Use the [architecture guide](docs/ARCHITECTURE.md) to find the modules responsible for the behavior you want to change. The code map shows where the Vue interface, Tauri commands, and Rust crates fit together.
4. Follow the [development and testing guide](docs/DEVELOPMENT.md) to set up your environment, run the launcher, choose the right tests, and build the Windows installer.

The [main README](README.md) explains how to install, use, and troubleshoot the launcher from a player's perspective.

## Prepare a change

- For a bug fix, record the steps to reproduce it, the expected behavior, and any relevant graphics settings or active mods. For a feature, describe what the player should be able to do and how you will verify it.
- For changes to community fixes or mods, follow the [guide to contributing a fix or mod](docs/DEVELOPMENT.md#contribute-a-fix-or-mod).
- In your pull request, summarize what changed, list the checks you ran and their results, and mention anything that still needs testing on Windows or in game. Update the relevant documentation if the change affects behavior or contributor workflows.
