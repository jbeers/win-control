# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres (as feasible) to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-27
### Added
- Initial Windows audio device control capabilities.
  - List active render (output) devices (`--list-audio-devices`).
  - JSON output variant for device listing (`--list-audio-devices-json`).
  - Retrieve device IDs and (planned) friendly names.
  - Set default output device by ID (`--set-audio-device <DEVICE_ID>`).
- Command line interface built with `clap`.
- MCP (Model Context Protocol) server scaffold for future AI-driven control.
- GUI bootstrap using `eframe` / `egui` (launches when no CLI action flags are supplied).
- Basic COM initialization + error handling paths for Windows multimedia APIs.

### Notes
- This is an early experimental build; APIs and CLI flags may change.
- Friendly device name resolution is in progress; some names may appear as raw IDs.

[0.1.0]: https://github.com/<your-user-or-org>/win-control/releases/tag/v0.1.0
