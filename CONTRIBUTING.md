# Contributing to Vulture

First off — thanks! Vulture exists so Windows stays fast, and every fix counts.

## Ground rules

1. **`src/core/` holds engine code, `src/ui/` holds presentation.**
   Engine code must never depend on egui types; UI code must never call Win32 directly.
2. **Safety first.** Anything that deletes user data must go through the Recycle Bin (`trash`),
   never `std::fs::remove_file` on user content. Startup changes must back up first.
3. **No telemetry, ever.** Vulture makes no network calls except "open the GitHub page".
   PRs adding analytics of any kind will not be merged.
4. **Windows only for now.** macOS/Linux support is welcome, but gate it behind `#[cfg]`.

## Dev setup

```powershell
git clone https://github.com/aethory/vulture.git
cd vulture
cargo run
```

Requires Rust stable + the MSVC C++ build tools (`winget install Microsoft.VisualStudio.2022.BuildTools`
with the VCTools workload).

## Before you open a PR

```powershell
cargo fmt
cargo clippy --all-targets
cargo test
```

CI runs the same trio on every push — please keep it green.

## Architecture in 30 seconds

- `core::scanner` — junk categories, scan, safe recycle-bin clean
- `core::browsers` — per-browser cache/cookie/history detection & wipe
- `core::startup` — registry Run keys + Startup folder, with JSON backups in `%APPDATA%\Vulture`
- `core::memory` — sysinfo stats + `EmptyWorkingSet` boost
- `ui::<tab>` — one module per screen, message-passing via `std::sync::mpsc` for background work

## Reporting bugs

Use the issue templates — attach your `%APPDATA%\Vulture\logs\` output when
something in a clean fails. Screenshots of the UI help a lot too.
