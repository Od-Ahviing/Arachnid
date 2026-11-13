# Contributor guide

Arachnid is an open-source Windows launcher that helps keep **The Amazing Spider-Man (2012) for PC** playable. It gives players one place to start the game, diagnose problems, adjust settings, manage community fixes and mods, and protect their progress. The player supplies the game installation.

## Project philosophy

- **Keep the path to Play short.** The player selects a folder and sees whether the game can launch. Diagnostics explain what needs attention and where to go next.
- **Preserve the original installation and the player's choices.** Check compatibility before changing anything. Keep backups and recovery available. Optional mods should be easy to disable, and a fresh-install default should preserve choices already made by existing users.
- **Prefer reversible fixes.** Use verified native settings and established community fixes where they solve the problem. Keep any launcher-specific fix narrow and tied to a tested game build.
- **Keep mods available offline.** Prepared mod resources ship with the appropriate installer build. Players can use the included mods without relying on a live download from a mod site.
- **Make compatibility claims testable.** An installation scan establishes what was found. A play test establishes what worked on a particular build and machine. For performance changes, report measurements from repeatable conditions.

## Find your starting point

| If you want to…                                                      | Start here                                                                                                                             |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Understand what happens between clicking Play and launching the game | [Architecture and runtime](ARCHITECTURE.md#installation-diagnostics-and-launch)                                                        |
| Work on the UI, translations, or graphics settings                   | [Architecture and runtime](ARCHITECTURE.md#settings-and-language) and [Development and testing](DEVELOPMENT.md#choose-the-right-tests) |
| Improve mod compatibility or add a mod                               | [Mod handling](ARCHITECTURE.md#mods-and-recovery) and [Contributing a fix or mod](DEVELOPMENT.md#contribute-a-fix-or-mod)              |
| Work on save discovery or restores                                   | [SaveGuard](ARCHITECTURE.md#saveguard)                                                                                                 |
| Run the app, tests, or installer build                               | [Development and testing](DEVELOPMENT.md)                                                                                              |
| Check the project's development rules                                | [AGENTS.md](../AGENTS.md)                                                                                                              |

The [main README](../README.md) is the player guide. The rest of this documentation explains how to change the launcher and verify the result.
