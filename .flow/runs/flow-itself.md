+++
slug = "flow-itself"
title = "Flow itself"
kind = "project"
flow = "main-flow"
status = "finished"
created = "2026-08-20T22:35:18.891Z"
updated = "2026-08-20T23:58:16.845Z"

[[stage]]
name = "grill"
status = "done"
artifact = ".flow/artifacts/flow-itself/grill.md"
started = "2026-08-20T22:35:18.891Z"
completed = "2026-08-20T22:35:18.894Z"

[[stage]]
name = "spec"
status = "done"
artifact = "tracker:https://github.com/philac105/flow/issues/1"
started = "2026-08-20T22:35:18.894Z"
completed = "2026-08-20T22:35:18.896Z"

[[stage]]
name = "tickets"
status = "done"
artifact = "tracker:https://github.com/philac105/flow/issues/2-10"
started = "2026-08-20T22:35:18.896Z"
completed = "2026-08-20T22:35:18.900Z"

[[stage]]
name = "implement"
status = "done"
artifact = "tracker:https://github.com/philac105/flow/issues/2-10"
started = "2026-08-20T23:34:35.860Z"
completed = "2026-08-20T23:34:35.862Z"

[[stage]]
name = "review"
status = "done"
started = "2026-08-20T22:39:08.121Z"
completed = "2026-08-20T23:58:07.726Z"
+++

## Where we are

Run finished.

## Log

### 2026-08-20T22:35:18Z — Started. First stage is `grill`.

Interview until the plan is sharp. Produces decisions, a glossary and ADRs.

Run `/grill-with-docs` next.

### 2026-08-20T22:35:18Z — `grill` done → `spec`

Grilled over three rounds. Settled: files-first with a read-only CLI, agnostic on three axes (agent, tracker, and the flow itself), state in the target repo, wrap other people's skills rather than fork them, Rust. Glossary in CONTEXT.md, five ADRs in docs/adr/.

### 2026-08-20T22:35:18Z — `spec` done → `tickets`

Spec published as GitHub issue #1. One testing seam: the CLI surface, driven by assert_cmd against a tempdir. Deliberately out of scope for v1 — the cross-repo registry, any execution of commands by the CLI, and syncing position to tracker labels.

### 2026-08-20T22:35:18Z — `tickets` done → `implement`

Nine vertical slices published as issues #2-#10, with native GitHub blocking edges. Ticket 01 is the walking skeleton; 09 is this dogfood. Frontier order is 02 -> 03 -> (04, 05, 06) -> 07 -> 08 -> 09.

### 2026-08-20T22:39:08Z — `implement` done → `review`

Built and installed. 50 integration tests through one seam (the CLI), clippy and fmt clean. Reviewing the diff found four real defects, all fixed: flow back fabricated a dead session on every deliberate redo; a finished run was unreachable without naming it; second-resolution timestamps made status ordering arbitrary; and a handoff quoting '## Log' could swallow the log. Added flow reopen — back only moves backwards, so a run finished at its first stage had no way home.

### 2026-08-20T23:24:38Z — sent back to `implement`

Reopened: adding `flow go`, which hands a stage to an agent with the prompt assembled.

### 2026-08-20T23:24:38Z — `implement` done → `review`

Added `flow go`: builds a prompt from the stage command, the run, the handoff and the exact `flow done` line, then launches the agent declared in flow.toml. ADR-0001 amended by ADR-0006 — the binary still names no agent; the launcher is config. Launching is a separate command from `flow next` because the adapter tells agents to call next, and a next that spawned sessions would fork forever. guard_env is the backstop. 59 tests.

### 2026-08-20T23:34:35Z — sent back to `implement`

Reopened: adding a current-run pointer and more than one preset.

### 2026-08-20T23:34:35Z — `implement` done → `review`

Added `flow switch` and a current-run pointer at .flow/current (gitignored, local to a checkout) — bare commands follow it rather than guessing, and refuse when several runs are active with none current. Shipped three presets instead of one: main-flow, minimal, bugfix; --preset also accepts a path to your own .toml, and `preset =` in the user config changes what a bare `flow init` writes. 82 tests.

### 2026-08-20T23:58:07Z — `review` done — every stage settled

### 2026-08-20T23:58:16Z — Run finished.
