+++
slug = "flow-itself"
title = "Flow itself"
kind = "project"
flow = "main-flow"
status = "active"
created = "2026-08-20T22:35:18.891Z"
updated = "2026-08-20T22:35:18.900Z"

[[stage]]
name = "grill"
status = "done"
artifact = ".scratch/flow-itself/grill.md"
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
status = "in_progress"
started = "2026-08-20T22:35:18.900Z"

[[stage]]
name = "review"
status = "pending"
+++

## Where we are

`tickets` done → `implement`

Nine vertical slices published as issues #2-#10, with native GitHub blocking edges. Ticket 01 is the walking skeleton; 09 is this dogfood. Frontier order is 02 -> 03 -> (04, 05, 06) -> 07 -> 08 -> 09.

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
