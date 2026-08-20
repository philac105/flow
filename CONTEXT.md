# Flow

Flow tracks a piece of work as it moves from idea to shipped code, and keeps that
tracking in files so it survives the end of an agent session. It does not do the
work — it records where the work is and what comes next.

## Language

### The definition

**Flow**:
An ordered, linear sequence of Stages, declared as data in `.flow/flow.toml`.
A repo has exactly one.
_Avoid_: pipeline, workflow, process

**Stage**:
One step in a Flow. Binds a name to a Command and an expected Artifact.
_Avoid_: step, phase, task

**Command**:
The literal string a Stage tells the agent to execute, such as `/to-spec`.
Flow stores it and prints it; Flow never executes it.
_Avoid_: action, script, hook

**Preset**:
A Flow definition shipped inside the binary, or a file someone wrote. `flow
init` writes a Preset out to `.flow/flow.toml`, after which the repo owns it and
the Preset is irrelevant.
_Avoid_: template, default, built-in

### The work

**Run**:
One traversal of the Flow by one piece of work. The unit that has a position.
_Avoid_: feature, ticket, issue, item, effort

**Kind**:
Free text on a Run describing what sort of work it is — `feature`, `bug`,
`task`, `project`. Flow never branches on it; it exists for the human reading
the board.
_Avoid_: type, category, label

**Current**:
The Run that bare commands act on, like a checked-out branch. Set by starting a
Run or by `flow switch`, and local to a checkout rather than committed.
_Avoid_: active, selected, focused

**Artifact**:
The file or tracker reference a Stage is expected to produce. Its existence on
disk is the evidence that the Stage really happened.
_Avoid_: output, deliverable, result

### Resuming

**Handoff**:
The always-current block at the top of a Run file describing where the work
stands right now. Rewritten on every Stage transition, never appended to. The
first thing a cold agent reads.
_Avoid_: summary, status note, context dump

**Log**:
The append-only history below the Handoff, one entry per Stage transition.
Never rewritten. The Handoff says where we are; the Log says how we got here.
_Avoid_: history, journal, changelog

**Drift**:
Disagreement between a Stage's recorded status and its Artifact's presence on
disk — a Stage marked pending whose Artifact exists, or marked done whose
Artifact is missing. Drift is how a session that died mid-Stage announces
itself.
_Avoid_: desync, inconsistency, staleness

**Launcher**:
The argv that starts an agent, with `{prompt}` substituted. Declared in the
user's config, because which agent someone drives is theirs and not the
project's. Flow spawns it without knowing what is on the other end.
_Avoid_: runner, executor, agent command

**Adapter**:
The per-agent file that teaches an agent the Flow protocol — a skill for Claude
Code, a section of `AGENTS.md` for others. Written by `flow init`. The only
part of Flow that knows which agent is running.
_Avoid_: integration, plugin, binding
