# LOG.md

Dated, one line per item. Newest at the bottom. The latest entries say what is open and what is next, so a new session can start here.

## 2026-11-04

- Design drafted for regions, danger, survival, living NPCs, traversal, mounts and hazards (first in the removed `pokeclaude` repo, then moved into this fork).
- Fork of `rh-hideout/pokeemerald-expansion` created and became the codebase. `pokeclaude` was removed.
- `design/technical-feasibility-audit.md` written with the Gate A to D plan.

## 2026-11-05

- Audit of the design set: many items had been marked locked before any build or measurement. `design/STATUS.md` created as the status source of truth (Locked, Proposed, Aspirational, Open).
- 20 speculative specs in `design/root/` marked archived and off-limits through `AGENTS.md`.
- Pull request #1 (docs cleanup) merged into `master`. Two earlier pull requests were mistakenly opened against upstream.
- Gate A, partial: CI compile and tests passed. Measured EWRAM 226,420 B (86.37%), IWRAM 28,392 B (86.65%), ROM 26,752,416 B (79.73%). Recorded in `design/root/baseline-measurements.md`.
- Decision: docs go straight to `master`, code and workflow changes use a branch.
- `design/CORE.md` written: five core lines plus three design rules (no respawn, direct predecessor, version ladders).
- `features/` created: 11 version-ladder files, only version 1 of each marked Next.
- `PLACES.md` and `OPERATING.md` added.

## Open

- Owner to confirm the world clock policy (in-game time only, no offline catch-up).
- Emulator smoke test to close Gate A.
- Confirm upstream pull request #11032 is closed.
- Decide which regions have gyms.
- Ladder rows were drafted by the AI and need owner review.
- `design/trainers-and-gyms.md` still needs the "minority are persistent battlers" line.

## Next

- Review feature ladders with the owner, then promote the first row to Next to specify in detail.
