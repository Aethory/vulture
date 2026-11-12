# 🦅 Vulture

**All-in-one PC cleaner and optimizer for Windows — built 100% in Rust.**

Vulture hunts down the junk slowing your machine down and clears it out.
One-click cleaning, a startup manager, a RAM booster, disk analysis, duplicate
detection, an uninstaller and browser privacy cleaning — wrapped in a fast,
modern, native interface.

> Files are always moved to the **Recycle Bin**, never permanently deleted,
> so you can undo any clean if you need to.

## Features

| Module | What it does |
| --- | --- |
| 📊 **Dashboard** | Live CPU, RAM, disk and uptime stats plus a one-click Smart Clean |
| 🧹 **Smart Clean** | Remove Windows temp files, update caches, prefetch, thumbnail caches, crash dumps, shader caches and more |
| 🚀 **Startup Manager** | See everything that runs at boot (registry Run keys + Startup folder) and disable/re-enable entries |
| ⚡ **RAM Booster** | Trim the working set of every background process you choose, and end memory-hogging tasks |
| 💽 **Disk Analyzer** | Find out where your disk space went — folder breakdown with bars and your 40 largest files |
| 🧬 **Duplicate Finder** | Content-hash (BLAKE3) comparison finds true duplicates across any folder |
| 🗑️ **Uninstaller** | Every installed program with size, publisher and one-click uninstall |
| 🍪 **Privacy Cleaner** | Wipe caches (and optionally cookies/history) for Chrome, Edge, Brave, Opera and Firefox |
| ⏰ **Auto Clean** | Daily scheduled `--auto-clean` runs via Windows Task Scheduler |
| ⚙️ **Settings** | Accent themes, data folder access, stats reset and about |

## Why Vulture?

- **100% safe by default** — cleaning moves files to the Recycle Bin, startup entries are backed up before removal, system/critical processes are protected.
- **Fast & native** — a tiny, single binary. No Electron, no web view, no 300 MB install to save you 300 MB of junk.
- **Private** — zero telemetry, zero network calls, open source under the MIT license.
- **Transparent** — every action shows exactly what it removed and what it freed.

## Install

### From source

Requires [Rust](https://rustup.rs/) 1.85+ (stable) and the Windows C++ build tools.

```powershell
git clone https://github.com/aethory/vulture.git
cd vulture
cargo run --release
```

The binary lands in `target\release\vulture.exe`.

### Prebuilt binaries

Watch the [Releases](https://github.com/aethory/vulture/releases) page — ready-made installers land there once the first release is cut.

## Auto-cleaning

Vulture can run headlessly as a scheduled daily task:

```powershell
vulture.exe --auto-clean
```

Or enable it from the **Scheduler** tab inside the app — it registers a
`Vulture Auto Clean` Windows scheduled task that runs every day at 02:30, logging
to `%APPDATA%\Vulture\logs\`.

## Building for development

```powershell
cargo check        # fast type-checking
cargo run          # debug build + run
cargo clippy       # lints
cargo test         # unit tests
```

Elevated rights are only required for HKLM startup entries, `C:\Windows\Prefetch`
and `C:\Windows\SoftwareDistribution` — everything else works per-user, and the
app degrades gracefully with clear messaging when it needs admin.

## Project layout

```
src/
├── main.rs          # entry point (--auto-clean headless mode + eframe bootstrap)
├── icon.rs          # programmatically drawn app icon (no binary assets)
├── theme.rs         # palette, dark visuals, card/button styling helpers
├── config.rs        # persisted stats & settings (%APPDATA%\Vulture)
├── core/            # the engine — no UI code
│   ├── scanner.rs   # junk categories, scan, safe recycle-bin cleaning
│   ├── browsers.rs  # Chrome/Edge/Brave/Opera/Firefox detection & cleaning
│   ├── startup.rs   # registry Run keys + Startup folder management
│   ├── memory.rs    # sysinfo stats + EmptyWorkingSet RAM boost
│   ├── duplicates.rs# size-bucketed BLAKE3 duplicate detection
│   ├── disk.rs      # folder size breakdown + largest-files heap
│   ├── uninstall.rs # Uninstall registry hives enumeration & launch
│   ├── schedule.rs  # Windows Task Scheduler integration
│   └── auto_clean.rs# headless --auto-clean path
└── ui/              # one module per tab, egui front-end
```

## Roadmap

- [ ] MSI/MSIX installer + `winget install aethory.vulture`
- [ ] Real-time disk-space watcher with tray icon
- [ ] Large-file cleanup wizard ("clean 30 GB in 30 seconds")
- [ ] Optional Windows Defender "quick scan" shortcut
- [ ] Localization (i18n)

## Contributing

PRs are welcome. Please keep UI code in `src/ui/` and system logic in `src/core/`
so the engine stays testable and headless. Run `cargo clippy` and `cargo fmt`
before opening a PR.

## License

[MIT](LICENSE) © aethory
