# flow

A Rust CLI that tracks work moving through a declared flow, keeping state in
files so it survives the end of an agent session. Read `CONTEXT.md` for the
domain language before writing anything — use those terms, avoid the ones
listed under `_Avoid_`.

## Agent skills

### Issue tracker

Issues live in GitHub Issues on `philac105/flow`, via the `gh` CLI. See
`docs/agents/issue-tracker.md`.

### Triage labels

The five canonical roles, each label string equal to its name. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See
`docs/agents/domain.md`.

<!-- flow:start -->
## Flow

This repo tracks work with `flow`. Run `flow status` to see every run and the
stage it is on, and `flow next` for the current run's stage and the command to
run for it. `flow` never runs commands itself — it prints them and records that
they happened.

When a stage is complete, record it with `flow done -m "<handoff>"`. The message
replaces the run's `## Where we are` block and is appended to its `## Log`, so
write it for someone who has never seen the work.

State lives in `.flow/` and is the source of truth, not the conversation. Commit
it alongside the work.
<!-- flow:end -->
