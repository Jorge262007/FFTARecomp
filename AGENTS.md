# FFTARecomp project guidance

Read [the current project state](docs/development/PROJECT_STATE.md) and inspect
`git status --short` before changing code. Existing uncommitted work belongs
to the user; preserve it and inspect the diff before attributing changes.

## Resume work without repeating completed tasks

At session start, after context compaction, and on automatic goal continuation,
read [the task handoff](docs/development/TASK_HANDOFF.md) before selecting work.
It distinguishes completed substeps and rejected experiments from the remaining
criteria in broad TODO items. An unchecked parent does not reopen its children.
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

## Local skills

These skills are versioned in `.cursor/skills/`. Read the relevant `SKILL.md`
directly when the client has not listed it in its skill catalog:

- [ffta-development](.cursor/skills/ffta-development/SKILL.md): runtime, menu,
  input, rendering and recompilation changes.
- [ffta-windows-validation](.cursor/skills/ffta-windows-validation/SKILL.md):
  Windows builds, runtime checks, performance comparisons and packaging.
- [ffta-project-memory](.cursor/skills/ffta-project-memory/SKILL.md): recording
  implemented changes, validation and durable development knowledge.

## Finish a change

After a meaningful code, build or agent-workflow change, use
`ffta-project-memory` before the final response. Add or update a structured
record under `docs/changes/`, regenerate `CHANGELOG.md`, update the relevant
feature documentation, and revise `PROJECT_STATE.md` only when a lasting fact
or decision changed. Record the actual checks and remaining limitations.
Read-only questions do not require a new changelog entry.

Run `python3 scripts/project_memory.py check` before handing off changes.
The checker identifies changed implementation files without a changed memory
record. Use `check --base <commit>` to cover a branch or commit range.
The CI workflow runs the same check; it does not invent change descriptions.

Prefer Windows for the current development focus unless the task specifies
another platform. The root workspace and `AndroidPort/` are separate trees;
do not assume a root change also updates that port. Keep game data, personal
saves, input recordings and generated ROM code out of commits and packages.
