---
name: pkg-owner
description: Determine macOS application and command package ownership, list installed software, or fuzzy-search software using the bundled read-only pkg-owner CLI.
---

Run `python3 <plugin-root>/bin/pkg-owner` with the user's requested name or path.
Use `--list` for an inventory, `--search TEXT` for fuzzy search, and `--json` for structured evidence.
The plugin root is two directories above this skill directory.
Use `--scan-dir PATH` for additional application directories.
Report collector warnings and preserve Unknown results. An installation record is evidence of registration, not proof of which installer last modified the software. Do not install or update software while diagnosing ownership.
