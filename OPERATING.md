# OPERATING.md: how we work together

The owner and the AI work as a pair. This file says who does what, how ideas become plans, how plans become documentation, and how documentation becomes code. The planning and documentation parts describe what we actually did. The **implementation part is a proposal**, because nothing has been implemented yet. Revise it after the first real implementation.

The session routine (what to read, the diary) is in `ONBOARDING.md`.

## Roles

| Owner | AI |
|---|---|
| Sets the core and decides what is locked, Next or Current | Proposes, drafts, and writes the documentation |
| Plays and tests on the phone and emulator | Reads and edits the repo through the GitHub connector |
| Relays build logs and screenshots the AI cannot see | Reports state honestly and flags what it cannot verify |
| Approves anything that changes engine code or the core | Never promotes its own work, never marks anything Current |

## 1. Planning: from idea to plan

1. **Talk at a high level first.** No detailed specs while the idea is still forming. One question at a time, short answers.
2. **Name the core line.** Every idea must serve one of the five lines in `design/CORE.md`. If it cannot, it waits.
3. **Put it on a ladder.** Add it to the right file in `features/` as a **Parked** row, or start a new ladder. Each row says what it adds and what it requires.
4. **Check the predecessor.** Every outcome needs a cause that already exists in the world. If a row needs an invented safety net, it waits for the system that would cause it.
5. **The owner promotes a row to Next.** Only then does the AI write a detailed spec for it, kept short.
6. **Check against measurements.** A Next row must fit the measured budget (about 35 KB EWRAM, 4 KB IWRAM, 6.5 MiB ROM free). Never write sizes or caps that were not measured.
7. **Conflicts with the core stop the work.** The AI flags it and asks. It does not quietly pick a side.

## 2. Documentation: who updates what

| Question | Where it lives | Rule |
|---|---|---|
| What is the game? | `design/CORE.md` | Owner decision only |
| What are we building, in what order? | `features/*.md` ladders | AI drafts, owner promotes |
| How did an older design stand? | `design/STATUS.md` | Update when status changes |
| What did we measure? | `design/root/baseline-measurements.md` and the audit | Only numbers actually seen in a log |
| Where are things, how does setup work? | `PLACES.md` | Update when setup changes |
| How do we work, what is the routine? | `OPERATING.md`, `ONBOARDING.md` | Update when the process changes |
| What happened, in order, and what is open? | `diary/` (latest page) | Add a log for important moments, update State |

How documentation gets updated:

- **Write it down in the same session.** A decision that lives only in chat does not count.
- **One home per fact.** Do not copy status lists or numbers into several files. Link instead.
- **Never delete a version.** Mark it Superseded.
- **Docs go straight to `master`.** Commit message style: `Docs: <what changed>`.
- **Say what changed in plain words** at the end of each message, in a few lines.
- **Mark the unverified.** If something was not actually checked, say so in the file.

## 3. Implementation (PROPOSED, not yet used)

Nothing is built yet. Until Gate A is fully closed (emulator smoke test pending), **do not change engine code**.

Once it is, a version moves from Next to Current like this:

1. **Pick a Next version whose requirements are built.** If a requirement is missing, build that first or stop.
2. **Make a branch** named `feature/<feature>-v<n>`. Engine code, build files and workflows never go straight to `master`.
3. **Make the smallest change that delivers the version.** No bonus features.
4. **Let CI run** and read the result. A failure is reported as a failure.
5. **Measure again** and compare with the baseline: ROM, EWRAM and IWRAM before and after.
6. **Owner verifies in play** on emulator or device. The AI cannot do this.
7. **Definition of done:** it compiles, CI is green, the size change is recorded, the owner confirmed the behavior, the ladder is updated, and the core line is still served.
8. **Owner marks it Current.** The AI then updates the ladder and adds a diary log.
9. **If it breaks the budget or the core,** revert the branch and park the version with a note about why.

**Merging a code branch** needs a GitHub pull request, because the connector can only merge through one. To keep overhead low, use one pull request per version, with the base repository set to `despairarc000/pokeemerald-expansion` and never upstream.

## 4. Decision authority

| Decision | Who |
|---|---|
| Core lines, locking anything | Owner |
| Promote a row to Next or Current | Owner |
| Draft rows, specs, and docs | AI proposes, owner accepts |
| Engine code changes | Owner approves before merge |
| Recording a measurement | AI, from real output only |

## 5. Honest limits

- The AI cannot run builds, emulators, or shell commands on the repo, and cannot read Actions job logs.
- The owner relays logs and tests in play.
- Anything not seen in real output is marked unmeasured, never assumed.
