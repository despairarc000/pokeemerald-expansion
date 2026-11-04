# Baseline measurements

**Source:** linker output from the `build-emerald` job, GitHub Actions run `41925848317` (2026-11-05), unchanged fork code, toolchain `arm-none-eabi-gcc 14.2.1`, `ld_script_modern.ld`. Numbers below are copied from the log; the free columns are simple subtraction.

| Region | Used | Region size | % used | Free (computed) |
|---|---|---|---|---|
| EWRAM | 226,420 B | 256 KB (262,144 B) | 86.37% | 35,724 B |
| IWRAM | 28,392 B | 32 KB (32,768 B) | 86.65% | 4,376 B |
| ROM | 26,752,416 B | 32 MB (33,554,432 B) | 79.73% | 6,802,016 B (about 6.5 MiB) |

## What this tells us

- **EWRAM headroom is about 35 KB.** This is the budget that matters most. Save blocks, object events, sprites and any simulation state all compete for it. Anything added at runtime must be small, and large simulation layers are unlikely to fit without removing other features.
- **IWRAM headroom is about 4 KB.** Effectively none for new per-frame logic data.
- **ROM headroom is about 6.5 MiB.** Room for new maps, graphics and code, but not unlimited. 14 regions of dense maps and new tilesets will need budgeting.
- These are for the **default Expansion configuration with all generations enabled**. Turning off unused features may free some EWRAM and ROM. That needs testing, not assumption.

## What this does NOT tell us

- Free **save** bytes: EWRAM holds the save blocks in RAM, but free space inside the flash save sectors is a separate measurement (Gate C).
- Object-event and sprite limits.
- Runtime behavior (no emulator test yet).

## Consequence for the design

The measured RAM headroom supports the decision to treat the heavy NPC and Pokemon simulation layers as Aspirational (see `design/STATUS.md`). Anything that keeps per-entity state in RAM must be budgeted against the 35 KB EWRAM figure.
