---
name: ffta-debugging
description: Debug FFTARecomp with GPT-6.1 Terra leading implementation and validation, consulting GPT-6.1 Sol when the cause or next useful check is unclear. Use for bugs, regressions, unexplained behavior and failing checks.
---

# Debug with Terra and consult Sol

Keep GPT-6.1 Terra (`gpt-6.1-terra`) as the primary agent with Extra High
thinking (`xhigh`). Use GPT-6.1 Sol (`gpt-6.1-sol`), also with `xhigh`, for a
focused diagnostic consultation, then resume execution in the same Terra
conversation at `xhigh`. Require that effort throughout this workflow; do not
silently fall back to `high`, `medium` or a model default.

A skill cannot change the primary model or its runtime reasoning setting:
the user selects Terra and Extra High in the client, or starts Codex with
`-m gpt-6.1-terra -c 'model_reasoning_effort="xhigh"'`.
If the primary model/effort is different or unknown, disclose that rather than
claiming a settings change. See the [operating guide](../../../docs/development/DEBUGGING_WORKFLOW.md)
for setup, unavailable tools and the manual handoff.

If assigned the Sol consultant role, follow the consultation contract below
and return findings to Terra without spawning more agents.

## Terra: start with existing evidence

Follow AGENTS and the short task handoff, including its final **Final handover** section, before choosing work. Read only the selected task's source/evidence
and relevant PROJECT_STATE sections; never preload historical archive,
CHANGELOG or unrelated TODOs. Read
[ffta-development](../ffta-development/SKILL.md) for implementation and
[ffta-windows-validation](../ffta-windows-validation/SKILL.md) when validating
Windows behavior. Preserve existing user changes and accepted results.
Record the selected task, expected/observed behavior, reproduction or existing
failure evidence, working hypothesis and cheapest useful discriminator.
Keep task execution state explicit (active/suspended/interrupted/failed/waiting)
and separate from partial acceptance. Pause/interruption authorizes no new
task or ROM exploration. Check async ownership/handles/output without killing
others' jobs; sandbox visibility does not establish host inactivity. Use the
shared memory lock/preimage protocol; keep handover last and move only verified
terminal outcomes to the archive. Retain partial IDs and accepted substeps.

Continue directly while the cause and next useful action are supported by
evidence. An ordinary compile error with a clear fix needs no consultation.

## Terra: escalate when diagnosis stalls

Consult Sol as soon as one condition holds:

- After initial source/log inspection, Terra cannot explain the likely cause
  or choose a check that distinguishes the remaining hypotheses.
- Two targeted diagnostic attempts fail to reduce uncertainty or lead back to
  the same failure. Prefer observation over two speculative patches.
- The available evidence contradicts the proposed fix, or the fix depends on
  an uncertain interaction between input, menu, rendering or guest state.

Do not wait for two attempts when the first condition already holds. A running
build, missing permission, unavailable tool or missing user input is not by
itself a model-reasoning failure; handle that concrete dependency directly.

Record the trigger and evidence in the selected task's handoff entry. Tell the
user briefly that Sol is examining the blocker, then delegate one consultation
at a time. With the current collaboration API, use this call shape, replacing
`message` with the actual packet:

```json
{
  "task_name": "sol_debug_diagnosis",
  "fork_turns": "none",
  "model": "gpt-6.1-sol",
  "reasoning_effort": "xhigh",
  "message": "Focused diagnostic packet and consultant contract"
}
```

Use `collaboration.spawn_agent`; the independent context is intentional for
explicit model overrides. On other clients, use their equivalent delegation
tool with the same model, `xhigh` effort and contract. Record both explicit
requested settings and the successful agent handle. If the API reports the
resolved model/effort, check them; if it returns only a handle, record that
the effective settings are not independently confirmed. Do not silently replace
Sol with an inherited/default agent or reduce effort. On an unavailable-model,
unsupported-effort or delegation error, follow the guide's fallback.

The packet must be self-contained: task ID and user objective; symptom and
expected behavior; relevant source paths/symbols and current diff; exact
error or short log excerpts with full evidence locators; checks already run
and hypotheses rejected; preserved inputs, paused/closed scopes and platform
constraints; one precise question. Include the consultant contract below.
Send paths and focused excerpts rather than the whole conversation. Both
agents share the workspace; Terra must leave the inspected files stable while
Sol examines them. Independent reads can continue.

## Sol: consultation contract

Inspect the supplied evidence and relevant sources read-only. Do not edit
project files, launch builds/replays/GUI sessions, reconstruct fixtures, resume
paused work or delegate further. Request a specific missing observation from
Terra when needed. Return:

1. Supported cause, or ranked hypotheses with uncertainty clearly stated.
2. Evidence with file/symbol/log locations and rejected explanations.
3. Smallest proposed correction, or the next discriminating check if the
   cause is still unknown.
4. Focused validation, expected result and material regression risks.
5. A concrete next action Terra can execute immediately.

## Terra: take back execution

Wait for the consultant's result, inspect its evidence, then resume as Terra
without asking for a routine model-return confirmation. Implement the supported
fix and perform the smallest meaningful validation. A suggestion alone does
not establish correctness. If evidence is insufficient, obtain the proposed
discriminator first. Do not repeat accepted builds or rejected experiments.

If still blocked, send Sol only new evidence and one revised question, reusing
the existing consultant with `collaboration.followup_task` where available.
Reuse only a consultant originally requested as `gpt-6.1-sol` with `xhigh`;
otherwise create a fresh consultant with both settings explicit.
Never repeat an identical consultation packet without new evidence. After two
consultations on the same unchanged blocker, record the unresolved dependency
and pursue a concrete observation or necessary clarification instead of looping.

Keep the task ID, consultation trigger, agent handle/model, diagnostic conclusion,
rejected leads, Terra's next action and actual validation in the handoff, so
compaction resumes at the right stage. Close the selection and use
[ffta-project-memory](../ffta-project-memory/SKILL.md) when the unit finishes.
