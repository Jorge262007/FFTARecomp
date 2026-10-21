# Project skills and change memory

The repository contains three local skills, routed by [AGENTS.md](../../AGENTS.md):

- [ffta-development](../../.cursor/skills/ffta-development/SKILL.md) identifies
  code entry points and implementation checks.
- [ffta-windows-validation](../../.cursor/skills/ffta-windows-validation/SKILL.md)
  guides builds, realistic validation and packaging.
- [ffta-project-memory](../../.cursor/skills/ffta-project-memory/SKILL.md)
  records implemented behavior, evidence and lasting knowledge.

Clients supporting `.cursor/skills` can discover them locally. Other agents
should read the paths in AGENTS.md directly. No global skill installation or
background service is required. These files guide agents that read project
instructions; they cannot guarantee that every editor or agent follows them.

## Resume and repeat decisions

At session start, after compaction, and on automatic goal continuation:

0. Identify the latest user request and its scope. A compacted summary helps
   locate evidence; it does not supersede that request or authorize executing
   every historical next action. For workflow-only or status requests, finish
   that request and retain unrelated pending implementation as pending.
1. Read [TASK_HANDOFF.md](TASK_HANDOFF.md), current Git status and relevant
   PROJECT_STATE facts. Use the linked records and cheap file/hash/log reads to
   reconcile stale context; do not begin by rebuilding or replaying everything.
2. Check the selected ID in the linked change records and terminal results. If
   a newer recorded outcome closes it, correct the stale handoff/TODO selection
   before acting; a stale next-action line cannot reopen that outcome. Resolve
   ambiguous evidence with cheap inspection, not an automatic rerun.
   Choose one unfinished subtask with a stable ID, a missing acceptance criterion
   and a concrete deliverable. A parent TODO may remain open while several
   children are done. Rejected experiments are terminal decisions too.
3. Before an expensive repeat, record in the handoff the task ID, what changed
   or failed, which prior result is invalidated, and the smallest necessary check.
   Valid reasons include an affected implementation/input/toolchain change, an
   actual failed check, new evidence contradicting the result, an uncovered
   requirement, or an explicit user request. Context loss, missing conversation,
   an unchecked parent and a desire for another identical calibration are not
   reasons. Do not reopen an unrelated completed unit when checking new work.
4. Check any recorded live process handle before restarting. A lost handle or
   observation timeout alone does not establish process failure: inspect the
   current process/artifact state. Reuse a completed successful output; resume
   only genuinely unfinished work after establishing its state.

Before the first mutation or expensive command after resumption, write a brief
selection checkpoint in the handoff: latest request, selected task ID, existing
status/evidence, missing deliverable and next action. If the selected ID is
already completed or rejected, stop that selection unless a concrete reopening
reason above is recorded. If the request is already implemented, validate the
existing change and report it instead of producing a duplicate implementation
or change record. Ordinary read-only inspection needs no new checkpoint.

As soon as a unit completes or is rejected, close its selection and remove its
obsolete next action before starting another unit. Record the evidence and exact
remaining scope at that point; do not defer closure until the end of a long
benchmark/build sequence. Keep each stable ID in one current outcome entry,
linking its detailed record rather than copying a second competing status.

Maintain the handoff at work-unit boundaries, before the final response and
before known compaction. Replace stale next actions; keep it short enough to
read in full. Each closed unit needs its scope, decision and an evidence link.
Each partial unit needs the exact remaining criterion. Each running unit needs
its process/artifact locator and last observed state, without claiming that an
old observation is still live. Keep commands/logs and fixture manifests in
ignored local storage; tracked records must still describe the verified result
when those files are unavailable. Missing local artifacts are an availability
issue, not automatic reversal of completed work.

The handoff is agent-maintained. There is no compaction hook or background
process writing it automatically; AGENTS and all three local skills require its
use. The memory checker validates records and coverage, not whether an agent
actually read the handoff or chose useful work. Revalidate only the scope
affected by new evidence, and update the handoff and underlying TODO together
when their current-state descriptions disagree.

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

The [CI workflow](../../.github/workflows/project-memory.yml) runs tests and
checks PR changes against their base commit, or push changes against the
previous commit. An initial push without an available base validates metadata
only. CI does not write files or publish changes. Enable the workflow as a
required status check in repository settings if merges must be blocked by it;
this work does not change remote branch protections.

Covered areas are `crates/`, `AndroidPort/`, `scripts/`, `TinyCC/`, `android/`,
`assets/`, `Installer/`, `tools/`, `.github/workflows/`, `.cursor/skills/`, plus root AGENTS.md and Cargo
manifests/build entry points listed in the script. Ordinary documentation-only
edits do not require their own record. Extend the path policy when adding a new
implementation area. The checker detects missing file coverage and stale output;
humans and agents must still assess whether descriptions and evidence are true.

If coverage fails, add the missing paths to an accurate record of that change
(or create one), render and check again. Do not list unrelated paths to silence
it. If a reference moved, update the record's reference or restore the document.
Never store game data, save contents, secrets or machine-specific absolute paths.
