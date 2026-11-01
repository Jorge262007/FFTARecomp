# FFTARecomp project guidance

## English-first project

Use English as the primary language for maintained documentation, source
comments, user-facing default UI, CLI messages, reports and development tools
in both the root workspace and `AndroidPort/`. Keep optional translations in
explicit locale catalogs; English is the default and fallback. New message
keys and identifiers should be English. Preserve compatibility for existing
machine tokens, paths and schemas when migrating older text.
Preserve upstream license wording, original game/imported data and immutable
historical evidence verbatim; explain them in English where needed. These
preservation exceptions do not exempt maintained guides or current UI from
translation. See [the project language policy](docs/development/ENGLISH_FIRST.md).

Start with the short [operational handoff](docs/development/TASK_HANDOFF.md),
including its final **Final handover**, and `git status --short`.
Read only task-relevant source, evidence and PROJECT_STATE sections before
changing code. Existing uncommitted work belongs to the user; preserve it and
inspect the diff before attributing changes. Do not automatically load the
historical archive, full CHANGELOG, full PROJECT_STATE or unrelated large TODOs.

## Resume work without repeating completed tasks

At session start, after context compaction, and on automatic goal continuation,
read [the task handoff](docs/development/TASK_HANDOFF.md) before selecting work.
It contains only active, suspended, partial, interrupted/failed and dependency-
waiting work, with accepted substeps and exact missing criteria. Closed/rejected
history belongs in [TASK_COMPLETED_ARCHIVE.md](docs/development/TASK_COMPLETED_ARCHIVE.md),
read only by specific ID/section to resolve a concrete doubt. An unchecked
parent does not reopen its children.
Check the linked evidence with cheap reads; do not rerun builds, benchmarks or
fixture creation merely to reconstruct context. Follow the repeat/reopen rules
in [the memory workflow](docs/development/MEMORY_WORKFLOW.md#resume-and-repeat-decisions).
Reconcile the latest user request before following an older queue or compacted
summary. A workflow-only request does not require resuming unrelated runtime work.
Before the first mutation or expensive command after resumption, record a
selection checkpoint in the handoff: latest request, task ID, existing outcome,
missing deliverable and next action. Completed/rejected work requires a concrete
reopening reason. Validate already implemented requests rather than redo them.
Check linked terminal results before trusting an old next-action line. Close a
completed/rejected selection immediately, before choosing the next unit.
Update the handoff before ending a work unit or a known context handoff.
Keep its five ordered sections and 10–20 KiB target; **Final handover
must always be last**, with only task, verified result, remaining work, errors,
next action and references. Never append chronology after it. On completion,
verify acceptance, record evidence/limits, append the closed outcome to the
archive and remove obsolete operational instructions. Partial tasks retain
their original IDs, accepted results and missing criteria.

Use the shared lock and both preimage hashes through
`scripts/task_handoff.py update` for coordinated memory writes; merge on conflict.
See [the publication protocol](docs/development/MEMORY_WORKFLOW.md#coordinated-memory-publication).
Do not overwrite another agent's newer update or remove its lock/process.
Pause/interruption never selects new work or authorizes ROM exploration.
Check associated handles, outputs and ownership before touching asynchronous
work; a lost handle/timeout is not failure. Sandbox-only process visibility
does not establish host inactivity. Keep other agents' jobs intact.

## Local skills

These skills are versioned in `.cursor/skills/`. Read the relevant `SKILL.md`
directly when the client has not listed it in its skill catalog:

- [ffta-development](.cursor/skills/ffta-development/SKILL.md): runtime, menu,
  input, rendering and recompilation changes.
- [ffta-windows-validation](.cursor/skills/ffta-windows-validation/SKILL.md):
  Windows builds, runtime checks, performance comparisons and packaging.
- [ffta-project-memory](.cursor/skills/ffta-project-memory/SKILL.md): recording
  implemented changes, validation and durable development knowledge.
- [ffta-debugging](.cursor/skills/ffta-debugging/SKILL.md): Terra-led debugging
  with a focused Sol consultation when diagnosis or progress stalls.

## Debugging model workflow

For debugging, read `ffta-debugging` and use GPT-6.1 Terra as the primary model
with Extra High thinking (`xhigh`), selected in the client or CLI. Require
`xhigh` for the Sol consultant too, explicitly setting its delegation effort;
do not silently lower either model's effort. If the cause or next discriminating
check is unclear, delegate a focused diagnosis to GPT-6.1 Sol, then resume implementation
and validation in Terra. This authorizes that diagnostic delegation without
asking again at each handoff. Follow the skill's triggers, evidence packet and
consultant contract; disclose unavailable model/delegation support. Workflow
setup alone does not resume paused runtime tasks. See the
[operating guide](docs/development/DEBUGGING_WORKFLOW.md).

## Finish a change

After a meaningful code, build or agent-workflow change, use
`ffta-project-memory` before the final response. Add or update a structured
record under `docs/changes/`, regenerate `CHANGELOG.md`, update the relevant
feature documentation, and revise `PROJECT_STATE.md` only when a lasting fact
or decision changed. Record the actual checks and remaining limitations.
Read-only questions do not require a new changelog entry.

Run `python3 scripts/project_memory.py check` before handing off changes.
Also run `python3 scripts/task_handoff.py check`; CI checks its size, final
handover, open states and lossless historical snapshot.
The project-memory checker identifies implementation files without a changed memory
record. Use `check --base <commit>` to cover a branch or commit range.
The CI workflow runs the same check; it does not invent change descriptions.

## Keep local validation output bounded

Follow [local artifact retention](docs/development/LOCAL_ARTIFACTS.md). After a
managed/native validation unit succeeds and its preservation checks/results are
recorded, run `python3 scripts/local_artifacts.py complete target/<unit> --apply`.
This retires only the disposable `interrupted` and `moved resources` copies;
retain the current dataset needed by active work, source preimages, logs/results
and current executables. Historical accepted datasets are not permanent: review
and retire superseded, reproducible copies as described in the retention guide.
Failed or live units must retain their diagnostic fixtures.
Retire old unit-local executables only after verifying they are superseded.
Future preservation maps should guard the selected unit's real inputs, rather
than require every historical mutable test copy to remain present forever.
Cleanup receipts record local availability; missing retired payloads do not
reopen completed acceptance or authorize rebuilding it merely to restore files.

Prefer Windows for the current development focus unless the task specifies
another platform. The root workspace and `AndroidPort/` are separate trees;
do not assume a root change also updates that port. Keep game data, personal
saves, input recordings and generated ROM code out of commits and packages.
