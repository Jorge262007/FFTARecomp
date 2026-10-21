---
name: ffta-project-memory
description: Maintain FFTARecomp's changelog, task handoff and durable memory after meaningful changes or when resuming development after context loss.
---

# Record project memory

Read `AGENTS.md`, `docs/development/PROJECT_STATE.md`, relevant existing
records and the current diff. Use files, Git and test output as evidence;
conversation recollection only helps locate it. Preserve unrelated work.

For resumption, read `docs/development/TASK_HANDOFF.md` first and follow
`docs/development/MEMORY_WORKFLOW.md#resume-and-repeat-decisions`. Treat the
handoff as the work queue, PROJECT_STATE as durable facts and change records as
evidence. Do not rerun completed validation to rebuild conversational memory.

Before resuming execution, reconcile the latest user request with the handoff
and current files. A compacted summary is a locator, not authority to restart
its queued work. If the current request only changes workflow or asks for
status, finish that request without executing an unrelated queued code change.
Keep any existing partial implementation recorded as partial.

Before the first mutation or expensive command after resumption, record the
selection checkpoint defined in MEMORY_WORKFLOW: latest request, selected ID,
existing status/evidence, missing deliverable and next action. A completed or
rejected ID needs a concrete reopening reason; otherwise choose unfinished
work within the request. If this request is already implemented, validate and
report the existing result instead of duplicating its code or change record.

When a unit reaches a terminal outcome, immediately close its handoff selection
and remove its stale next action before choosing further work. Cross-check the
selected ID against its linked outcome record; correct a stale queue instead of
rerunning work that the newer evidence already closes.

After a meaningful change:
1. Update the affected feature doc with behavior, configuration and validation.
2. Record what changed, why, platforms, actual validation and known limits in
   `docs/changes/`. Keep existing history intact; use one coherent record per
   change, not one per tool call. Do not create records for read-only answers.
3. Use `python3 scripts/project_memory.py record --help` for a new record.
   Supply the session's actual date and the implementation files it covers.
   Updating an existing record is appropriate while finishing that same
   uncommitted change; then run `python3 scripts/project_memory.py render`.
4. Update `PROJECT_STATE.md` only for durable decisions, entry points,
   constraints or verified outstanding work. Do not paste transient logs or
   keep a second chronological changelog there.
5. Run `python3 scripts/project_memory.py check`. For a branch or PR, also
   pass `--base <commit>`. Resolve missing coverage with an accurate record;
   do not change the checker just to accept undocumented implementation.
6. Update `TASK_HANDOFF.md` before the final response or a known compaction:
   stable subtask ID, terminal/partial status, evidence, remaining criterion,
   next action and any live process/artifact locator. Mark rejected experiments
   explicitly. Separate unavailable local artifacts from unfinished work.
   A broad open task must retain its completed children. Replace stale next
   actions rather than appending another competing queue. Record repeat reasons
   before an expensive rerun; do not invent successful validation.

`CHANGELOG.md` is generated from records; edit the JSON source rather than
the generated Markdown. See `docs/development/MEMORY_WORKFLOW.md` for schema,
commands, CI behavior and recovery from failed checks.

Separate completed behavior from planned work. Never infer passing tests,
native Windows performance, physical-controller coverage, releases or commits
from compatible-looking code. Reference detailed tracked docs for durable
evidence; ignored local logs may be mentioned but are not portable fixtures.
Do not store copyrighted game data, save contents, secrets or machine-specific
absolute paths in memory records. Recording a change does not authorize a
commit, push, release, deployment or global agent configuration change.
