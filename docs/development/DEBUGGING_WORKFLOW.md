<a id="debugging-con-terra-61-e-supporto-sol-61"></a>

# Debugging with Terra 6.1 and Sol 6.1 support

Terra leads debugging, applies changes and verifies the result.
Both models use **Extra High (`xhigh`)** reasoning.
When Terra cannot identify a cause or a useful check, it consults Sol.
Sol returns a diagnosis with evidence and a next action; Terra resumes
work in the same conversation.

The behavior is defined by the
[ffta-debugging](../../.cursor/skills/ffta-debugging/SKILL.md) skill and referenced
by [AGENTS.md](../../AGENTS.md). It applies to debugging in this repository;
development, validation and memory instructions still apply.

<a id="avvio"></a>

## Getting started

Open the FFTARecomp project in the app, select **GPT-6.1 Terra** as the primary
model and **Extra High** as the reasoning level. Then ask:

> Use ffta-debugging to understand and fix this bug: [symptom and expected result].

AGENTS requires the skill for debugging requests even when they do not name it.
Automatic catalog loading depends on the client; an agent reading AGENTS can
still find the local path to open.

With a compatible CLI, from the project directory:

```sh
codex -m gpt-6.1-terra \
  -c 'model_reasoning_effort="xhigh"' \
  -c 'agents.default_subagent_model="gpt-6.1-sol"' \
  -c 'agents.default_subagent_reasoning_effort="xhigh"'
```

The client selects Terra initially. Sol is consulted through a subagent;
the main conversation remains on Terra. No manual model change is needed
for each consultation. The command sets the model and reasoning level for
that session; the skill's Sol call still requires explicit `xhigh`. Returning
retains Terra's `xhigh` reasoning. These instructions do not by themselves
change the settings of an already open conversation.

`xhigh` is supported by the official model pages for
[Terra 6.1](https://developers.openai.com/api/docs/models/gpt-6.1-terra) and
[Sol 6.1](https://developers.openai.com/api/docs/models/gpt-6.1-sol).
Configuration parameters are described in the
[Codex reference](https://learn.chatgpt.com/docs/config-file/config-reference).

The [official model documentation](https://learn.chatgpt.com/docs/models)
specifies Codex CLI 0.159.0 or later for Terra. The local CLI inspected while
preparing this workflow was 0.110.0: that path requires a compatible version.
The workflow neither updates the CLI nor modifies global configuration.
Using it in the app requires both models and delegation tools in the session.
Model availability depends on the account/client.

<a id="avvio-del-gioco"></a>

## Starting the game

On Windows, extract `dist/docker-windows/FFTARecomp-windows-portable.zip` into
a writable directory and start `FFTARecomp.exe`. On first launch, select your
FFTA USA/Europe ROM; settings and saves stay in the `Data` directory.

On Linux, the checkout can use the already compiled runtime:

```sh
./target/release/recomp play "/path/to/FFTA.gba"
```

Replace the path with your own ROM's path. In an extracted portable Linux
edition, start `./FFTARecomp` from its directory instead. Local binaries may
predate the latest sources: documenting a command does not constitute a new
build or validation. Requirements, compilation and launch details are in the
[README](../../README.md) and [portable guide](../portable.md).

<a id="quando-interviene-sol"></a>

## When Sol is consulted

Terra calls Sol as soon as, after initial inspection, it cannot formulate a
plausible cause or choose a check that distinguishes the hypotheses. It also
calls Sol when two focused diagnostic attempts fail to reduce uncertainty,
or evidence contradicts the proposed fix. Two attempted patches are not required.

A running build or missing permission requires managing that dependency.
Neither alone is a reason to change models.

Terra sends Sol the objective, error, affected files, current diff, evidence
already collected, rejected attempts and a precise question. Sol reads the
source and responds with a cause/hypothesis, verifiable references, a minimal
fix or discriminating check, and validation to perform. Terra keeps the affected
files stable during consultation and remains responsible for edits and validation.

The required consultant model is `gpt-6.1-sol`, with `xhigh` reasoning.
The explicit payload and contract are in the skill. Subagent model overrides
are described in the
[official subagent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents).

<a id="ritorno-a-terra-e-continuità"></a>

## Returning to Terra and preserving continuity

When Sol finishes, Terra checks the evidence and resumes automatically.
If another observation is needed, it collects that before applying a fix.
A second consultation must contain new evidence. The same packet is not
repeated in a loop.

Record the task ID, consultation reason, consultant/model, conclusion, rejected
leads, Terra's next action and actual checks in the task handoff. Resume there
after compaction. Closed and paused tasks retain their state.

<a id="se-la-delega-non-è-disponibile"></a>

## If delegation is unavailable

The agent reports the limitation, including unsupported `xhigh` reasoning,
and does not claim unverifiable settings were applied. It preserves the
packet and next action in the handoff. If both models are accessible only as
the primary model, manually switch to Sol in the selector (or `/model` in the
CLI), also select **Extra High**, request a diagnosis using that packet, then
return to **Terra + Extra High** to apply it. If Sol is inaccessible, keep the
limitation explicit; Terra continues only with work supported by available evidence.

<a id="validazione-e-limiti"></a>

## Validation and limits

The skill passes structural validation. A synthetic test exercised delegation
and return: reproduction of three menu actions that all executed the last
action, one diagnostic consultation, a fix by the primary agent, and a passing
final check of all three actions. The fixture is separate from the game.
That earlier test requested Terra 6.1 with `medium` reasoning for the primary
agent and Sol 6.1 with `high` for the consultant; it remains historical evidence
of delegation and return. The later `xhigh` requirement for both models was
updated in the skill and launch commands without repeating that test. The test
API returned handles without actual-model metadata: it verifies the requested
routing and return without independent confirmation of the executed models.

Checks, links and memory details are in the
[change record](../changes/2026-10-31-add-terra-sol-debugging-workflow.json).
Diagnosis quality for a real bug depends on that case's evidence; the
instructions are not an automatic, deterministic stall detector.
