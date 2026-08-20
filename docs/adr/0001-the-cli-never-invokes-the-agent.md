---
status: amended by ADR-0006
---

# The CLI never invokes the agent

Flow could shell out to `claude -p "/to-spec"` and drive each Stage itself. It
does not. `flow next` prints the Command; something else runs it and reports
back with `flow done`. Keeping execution out of the binary is what makes Flow
agent-agnostic for free — the core never learns what Claude Code is, and
support for a new agent is a ~30-line Adapter file rather than a new code path.

**Amended.** `flow go` does launch an agent — but from an argv declared in
`.flow/flow.toml`, never a name compiled into the binary. The reasoning below
still holds for `flow next`, which stays a pure read-model. See ADR-0006.

## Consequences

Flow cannot enforce that a Stage actually ran, so it leans on Artifact evidence
(see ADR-0002) rather than on having watched the work happen. The cost of a
missing Adapter is that the agent silently never updates state, which is why
`flow init` writes one (see ADR-0004).
