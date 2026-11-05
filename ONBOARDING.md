# ONBOARDING.md: the routine for every session

If you are an AI (or a person) starting work on this project, follow this routine first. It takes a few minutes and keeps everyone on the same page without reading every file.

## 1. Start of a session

Read in this order, and stop reading a file once you have what you need:

1. `AGENTS.md`: the rules.
2. `design/CORE.md`: the five-line core game. Everything serves it.
3. `PLACES.md`: where things are, how the setup and CI work, known traps.
4. `OPERATING.md`: how the owner and AI work together.
5. **The latest diary page** in `diary/` (see below): what happened and where we are.
6. `features/README.md`: the feature index, then only the ladders you need.
7. `design/STATUS.md` and `design/technical-feasibility-audit.md`: only if the task touches older designs, status, or measurements.

Then look at the repository for changes since the last diary entry, and **tell the owner in a few lines** what you found, what is waiting on them, and what you plan to do.

## 2. The diary

The diary is the story of the project in order: a short summary of the important moments. It is context, not a change history. Git already records every edit.

### What gets a log entry
- A decision, or a change of direction.
- A problem we hit, a mistake we made, and how we recovered.
- A milestone (a gate passed, a feature version marked Current).
- Not every small edit or movement.

### Format
- One entry is a numbered **Log**, with a date, a title, and a few lines: what happened, and why.
- Be honest. Mistakes and reversals belong in the diary, because they explain why things are the way they are.
- Keep each entry short. If it needs a page, it belongs in a design doc and the diary links to it.

### Pages
- Each page holds **10 logs**. Files are `diary/diary01.md`, `diary/diary02.md`, and so on, two digits.
- Log numbers keep counting across pages (page 1 has logs 1 to 10, page 2 has 11 to 20).
- When the latest page has 10 logs, start the next page. Copy forward the **State** block (open items and next steps) into the top of the new page.
- Every page ends with a **State** block. Only the latest page's State block is current.

### How to read it
1. List `diary/` and open the page with the highest number. That is the latest.
2. Read the whole page, including its State block.
3. **Only if you need more context,** read the page before it, and keep going back one page at a time. Do not read the whole diary by default.

### How to write it
1. Open the latest page. If it already has 10 logs, create the next page first.
2. Add the next log at the end of the logs, above the State block.
3. Update the State block (open items, next steps).
4. Commit directly to `master`. Message style: `Docs: diary log N`.

## 3. End of a session

1. Update any ladders or docs the work touched.
2. Add a diary log if something important happened, and refresh the State block.
3. Give the owner a short summary: what changed, what is waiting on them, what is next.

## 4. When the routine itself is wrong

This routine describes how we actually work. If you notice that it does not match what we are really doing (a step nobody follows, a missing step, a rule that causes trouble), **do not keep following it blindly, and do not edit it silently.**

1. **Tell the owner first.** Say in a few lines what does not match, and what you would change.
2. **Change it once the owner agrees.** Then edit this file (or `OPERATING.md` or `PLACES.md`, if the process lives there).
3. **Record it** as a diary log: what was wrong, and what changed.

The point is that a wrong process gets fixed once, so future AIs do not inherit it. This applies to process files only. The core, the ladders and locked decisions still need the owner's explicit decision, as before.

## Quick checklist

- Read the rules, the core, the places, the operating guide, and the latest diary page.
- Summarize what you found before acting.
- Only touch what is marked Next. Check what it requires.
- Write decisions down in the right file in the same session.
- Add a diary log for anything important, and update State.
- If the routine does not match reality, tell the owner before changing it.
- Report only what you actually checked.
