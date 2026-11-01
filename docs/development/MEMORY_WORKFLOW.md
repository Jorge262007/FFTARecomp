# Project skills and change memory

The repository contains local skills, routed by [AGENTS.md](../../AGENTS.md):

- [ffta-development](../../.cursor/skills/ffta-development/SKILL.md) identifies
  code entry points and implementation checks.
- [ffta-windows-validation](../../.cursor/skills/ffta-windows-validation/SKILL.md)
  guides builds, realistic validation and packaging.
- [ffta-project-memory](../../.cursor/skills/ffta-project-memory/SKILL.md)
  records implemented behavior, evidence and lasting knowledge.
- [ffta-debugging](../../.cursor/skills/ffta-debugging/SKILL.md) keeps Terra
  responsible for debugging and consults Sol when diagnosis stalls; see the
  [operating guide](DEBUGGING_WORKFLOW.md).

Clients supporting `.cursor/skills` can discover them locally. Other agents
should read the paths in AGENTS.md directly. No global skill installation or
background service is required. These files guide agents that read project
instructions; they cannot guarantee that every editor or agent follows them.

## Resume and repeat decisions

At session start, after compaction, and on automatic goal continuation:

1. Reconcile the latest user request first. A summary is an evidence locator,
   not authorization to execute its older queue. Workflow-only work does not
   resume unrelated implementation. An explicit pause/interruption selects no
   replacement task and no speculative ROM exploration.
2. Read only the short [TASK_HANDOFF.md](TASK_HANDOFF.md), including the final
   **Final handover**, and `git status --short`. Inspect the relevant diff
   before attributing changes. Then read only the selected task's source,
   relevant PROJECT_STATE/feature/TODO sections and linked terminal results.
   Never automatically load TASK_COMPLETED_ARCHIVE, full CHANGELOG, full
   PROJECT_STATE or large unrelated TODOs. Name the ID/section needed before
   historical inspection; search and read a bounded interval.
3. Verify the selected ID's actual status with cheap file/hash/log reads. A
   newer terminal result overrides a stale next-action line. Choose a genuinely
   missing acceptance criterion and concrete deliverable within the request.
   Partial tasks retain original IDs, accepted results and exact missing scope.
   An unchecked parent never reopens accepted children or rejected experiments.
   Reopening a closed ID also requires a concrete `Reopening` field; the checker
   rejects a closed ID returning to the queue without that reason.
4. Check recorded async handles, owner, source/input pins and current artifacts
   before restarting anything. A timeout, lost handle or no immediate result
   does not prove failure. Sandbox ps does not prove host inactivity. Record
   the observation time and visibility limit; leave other agents' processes,
   prefixes, displays, locks and canonical binaries intact. Do not signal or
   terminate others' jobs without authorization.
5. Before the first mutation/expensive command, update the selected task/final
   handover with a checkpoint: latest request, stable ID, existing outcome and
   evidence, missing deliverable and next action. Use the publication protocol
   below. If already implemented, validate its existing evidence and close the
   selection instead of duplicating code or records.
6. Before an expensive repeat, record what changed/failed, the invalidated
   prior result and the smallest affected check. Valid reasons: implementation,
   input or toolchain change, actual failure, contradictory new evidence,
   uncovered requirement or explicit request. Context loss, unavailable retired
   payloads, an open parent or desire for identical calibration are not reasons.

### Operational structure and task states

The operational handoff targets 10–20 KiB (checker maximum 20 KiB), in this order:
`Operational status`; `Running or suspended tasks`; `Incomplete tasks and open criteria`;
`Blockers, dependencies and shared resources`; **`Final handover` last**.
Nothing may append a new section or chronology after the handover. Its six fields
are only task, last verified result, remaining work, errors/blockers, concrete
next action and required references, one Markdown source line per field (the
renderer wraps them). A completed final selection may remain as
this brief stopping instruction; its detailed outcome belongs in the archive.

Use `### ORIGINAL-TASK-ID` with `State`, `Outcome`, `Verified`, `Missing`, `Resume`:

- `active`: selected work with owner and current process/output locators if any.
- `suspended`: preserved but outside current selection or explicitly paused;
  record the reason and what authorizes resumption. This does not change another
  thread's goal state or authorize stopping its already running jobs.
- `interrupted` / `failed`: interrupted execution or observed failure; retain
  diagnostics and exact incomplete criterion. Neither state implies completion.
- `waiting`: dependency/input/ownership prerequisite missing; name it.
- `Outcome: partial`, `integrated_unverified` or `not_started` records acceptance
  separately from execution. Do not infer it from a symbol's existence in code.

Completed, rejected and definitively closed units leave the operational queue.
Verify their acceptance criteria, record actual results and remaining limits in
`docs/changes/`/feature docs, append a concise terminal entry to
[TASK_COMPLETED_ARCHIVE.md](TASK_COMPLETED_ARCHIVE.md), then remove stale
operational directions. Link existing detailed records instead of copying logs.
A failed experiment can be rejected without claiming the parent completed.
Update the final handover immediately at each boundary and before final response
or known compaction; do not leave an obsolete next action pointing at closed work.

The archive's 2026-11-01 legacy snapshot is immutable provenance, including old
pending descriptions needed for lossless preservation. It does **not** close
those tasks: current missing criteria remain operational. Original section byte
ranges/hashes and explicitly terminal scoped ID locators are in
`TASK_HANDOFF_MIGRATION.json`; the index does not recertify whole parent features.
Source snapshot line numbers count from its begin marker. Do not load the
manifest or archive as routine startup context. Retired local artifacts affect
availability, not previously recorded acceptance; tracked records retain results.

### Coordinated memory publication

One cooperating writer at a time updates handoff/archive. Before editing, read
both preimages only when publication is needed, retain their SHA256 values, and
prepare proposals in temporary files. Another writer's update must be merged,
never overwritten. Publish through:

```sh
python3 scripts/task_handoff.py update \
  --expected-handoff HANDOFF_SHA256 --expected-archive ARCHIVE_SHA256 \
  --handoff /tmp/handoff-proposal.md --archive /tmp/archive-proposal.md
python3 scripts/task_handoff.py check
python3 scripts/project_memory.py check
```

The helper takes `docs/development/.task-handoff.lock` exclusively and checks
both exact preimages. A held lock or stale hash refuses without changing memory;
re-read and merge. Do not delete another writer's lock. A leftover lock after an
interruption requires owner/process coordination, not age alone. Editors that
ignore this protocol are not protected by its cooperative lock; stop and reconcile
when their updates are detected. Apply the same coordination to shared change
records/CHANGELOG and re-render after merging another record.

Archive updates must append to its exact existing bytes. Removing an operational
ID requires a matching appended `### ID` entry with `Outcome: completed`, `rejected`
or `closed`, and `Verified criteria`, `Evidence`, `Limits`. The writer assesses the
criteria; the helper cannot certify runtime acceptance. New publications use the English
headings and fields above. The checker also reads legacy Italian headings,
fields and state/outcome tokens, allowing lossless migration and historical
compatibility. Mixed duplicate aliases are rejected; snapshot bytes stay unchanged. Partial histories stay
linked from their original open ID. Publication writes archive first, then
atomically replaces handoff. A crash between them may duplicate context, but
cannot discard it from both. Re-read both before recovery; never blindly retry
an old proposal. No background service or compaction hook writes memory or starts
work automatically; agents perform these updates at boundaries, and CI checks
structure/preservation. The checker cannot prove that an agent read the handoff,
that a claimed acceptance is true, or that a host process is idle.

## After a meaningful change

Update the relevant feature documentation, add a JSON record under
`docs/changes/`, then regenerate [CHANGELOG.md](../../CHANGELOG.md). Revise
[PROJECT_STATE.md](PROJECT_STATE.md) only for durable facts or decisions.
Use the real session date and only validation actually performed.

```sh
python3 scripts/project_memory.py record \
  --date 2026-10-20 --slug example-fix --title "Describe the resulting behavior" \
  --kind fixed --platform Windows \
  --change "Describe the implemented change and its purpose." \
  --validation "Name the checks actually performed and their result." \
  --limitation "Name relevant behavior or platforms still unverified." \
  --file crates/recomp/src/play.rs --reference docs/menu_shortcuts.md
python3 scripts/project_memory.py check
```

The example is a template, not a change to record verbatim. All repeatable flags
can be supplied multiple times. `--kind` accepts added, changed, fixed,
performance, or docs. The script uses Python 3.9+ and the standard library.
On Windows use `py -3` in place of `python3` where appropriate.

The record command never overwrites an existing entry. While finishing the same
uncommitted change, edit its JSON explicitly and run:

```sh
python3 scripts/project_memory.py render
python3 scripts/project_memory.py check
```

Preserve historical entries. For a subsequent change, create a new record.
Never edit the generated changelog directly. Records use schema 1 with fields
`date`, `slug`, `title`, `kind`, `platforms`, `changes`, `validation`,
`limitations`, `files`, and `references`, plus `schema`. Platforms and the five fields following it
are arrays of single-line strings; changes and files must be
nonempty. References are existing repository files. Files identify implementation
paths covered by the change, including deleted files. Record filenames are
`YYYY-MM-DD-slug.json`; output order is descending filename, not commit order.

## Automated checks

`check` validates the schema, references, generated changelog and documentation
coverage of changed implementation files. It compares the working tree,
including staged changes and untracked files, with HEAD by default. Only a new
or changed record can cover a changed implementation file; an old record alone
cannot satisfy a new change.

```sh
python3 scripts/project_memory.py check --base <base-commit>
python3 scripts/project_memory.py check --metadata-only
python3 -m unittest discover -s scripts/tests -p test_project_memory.py
```

Use `--base` for a branch/PR range. A clean working tree checked against HEAD
has no pending implementation coverage to assess. `--metadata-only` is for
validating a snapshot without Git history; it does not check change coverage.

After aligning branches, also check against the destination branch's previous
remote tip, retained before updating it. That is the push event's comparison
base and can span many imported commits. A check against HEAD or the source's
latest parent does not cover that range. Include any omitted implementation
paths in an accurate correction record and validate against the original base
before publishing; keep the existing checker and coverage requirements.

The [CI workflow](../../.github/workflows/project-memory.yml) runs tests and
checks PR changes against their base commit, or push changes against the
previous commit. An initial push without an available base validates metadata
only. Checkout is pinned to v7.0.1 (Node 24) with full history so both comparison
bases remain available. CI does not write files or publish changes. Enable the
workflow as a required status check in repository settings if merges must be
blocked by it; this work does not change remote branch protections.

Covered areas are `crates/`, `AndroidPort/`, `scripts/`, `TinyCC/`, `android/`,
`assets/`, `Installer/`, `tools/`, `.github/workflows/`, `.cursor/skills/`, plus root AGENTS.md and Cargo
manifests/build entry points listed in the script. Ordinary documentation-only
edits do not require their own record. Extend the path policy when adding a new
implementation area. The checker detects missing file coverage and stale output;
humans and agents must still assess whether descriptions and evidence are true.

If coverage fails, add the missing paths to an accurate record of that change
(or create one), render and check again. Do not list unrelated paths to silence
it. Include source-tracing helpers and partial implementations even when runtime
wiring will follow in another commit. After a missing record is discovered in
an earlier push, validate the repair against the commit before that push as
well as HEAD; a green later push only checks its own range and can leave earlier
paths undocumented. Rerunning the old failed commit uses its original files;
the corrected record must be included in a new commit to change the result.
If a reference moved, update the record's reference or restore the document.
Never store game data, save contents, secrets or machine-specific absolute paths.
