# win-control

Minimal utility to control aspects of a Windows PC (currently focused on audio output devices). It can:
- List active audio output devices (plain or JSON)
- Set the default audio output device by ID

## Core Dependencies
- **windows**: Win32 COM + multimedia APIs for enumerating and switching audio devices.
- **clap**: Command‑line flag parsing (e.g. --list-audio-devices, --set-audio-device).
- **serde / serde_json**: JSON serialization for structured device output.
- **schemars**: (Planned) JSON Schema generation for MCP / structured interfaces.

## Status
Early experimental; flags and data shapes may change.
