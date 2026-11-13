# Vulture vs. the field — honest comparison

Researched November 2026. How Vulture stacks up against CCleaner, BleachBit,
Wise Care 365, Glary Utilities, Microsoft PC Manager and Revo Uninstaller —
and where we deliberately refuse to go.

## What the others do

| Capability | CCleaner | BleachBit | Wise/Glary | Vulture today |
| --- | --- | --- | --- | --- |
| Junk/temp cleaning | ✅ | ✅ (strongest, auditable) | ✅ | ✅ Smart Clean + Auto Clean |
| Browser caches/history/cookies | ✅ | ✅ | ✅ | ✅ Privacy tab (opt-in cookies/history) |
| Startup manager | ✅ | ❌ | ✅ | ✅ Run keys + Startup folder, reversible, backed up |
| RAM booster (working-set trim) | partial | ❌ | ✅ | ✅ EmptyWorkingSet + protected-process list |
| Disk analyzer | ✅ | ❌ | ✅ | ✅ Folder breakdown + 40 largest files |
| Duplicate finder | Pro only | ❌ | ✅ | ✅ BLAKE3 content hashing |
| Uninstaller | ✅ | ❌ | ✅ | ✅ Interactive uninstall only; silent mode never auto-selected |
| Scheduled cleaning | Pro only | ❌ (CLI) | ✅ | ✅ Daily 02:30 via Task Scheduler, logs every run |
| Secure file shredder | Pro only | ✅ | partial | ❌ — by design; everything goes to the Recycle Bin first |
| Registry cleaner | ✅ | ❌ | ✅ | ❌ — most common cause of "cleaner broke my PC" worldwide |
| Driver/software updater | ✅ | ❌ | partial | ❌ — would require network calls; Vulture is offline forever |
| Restore points / backups | ✅ | ❌ | ✅ | ✅ Startup backups (raw registry type preserved) + Recycle Bin |
| Telemetry/ads/upsells | yes (haunted) | none | yes | ❌ — MIT, no network, ever |
| Open source | ❌ | ✅ GPLv3 | ❌ | ✅ MIT |

## The two features the leaders have that people actually love

1. **CCleaner "Health Check"** — one button: scans junk, privacy, startup,
   shows a score and a plain-English fix list. *Planned: on-demand Health Check
   on the Dashboard.*
2. **BleachBit exclusions** — a whitelist of paths the cleaner must never touch.
   *Planned: per-path exclusion list applied by Smart Clean and Auto Clean.*

What they charge for but adds nothing for a healthy machine: registry wiping,
driver updaters, "PC optimizer" snake oil. Vulture will never ship those.

## Anti-goals (written down so we don't drift)

- No network calls. Not for updates, not for telemetry, not for tips of the day.
- No registry deletion. Startup entries only, and always backed up first.
- No silent uninstalls ever auto-selected. `UninstallString` (interactive) wins.
- No force-deleting locked files. Skipped, reported, user decides to close the app.
- No shouting at non-experts. Every destructive action says what it touches first.
