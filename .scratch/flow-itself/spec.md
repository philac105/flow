## Problem Statement

I work on features, bugs, tasks and whole projects across a repo, and I move each
of them through the same rough sequence: sharpen the idea, write it down, break
it up, build it, review it. Two things go wrong.

First, the position is only ever in my head or in a chat session. When a session
ends — I run out of tokens mid-task, I close the terminal, I come back on
Monday — the answer to "where was I and what comes next" is gone. The artifacts
survive; the knowledge of which step produced them and which step is next does
not. I re-derive it by reading files and guessing, or I ask the agent to re-read
everything and burn the tokens I just ran out of.

Second, I have no view across work. I can see a list of GitHub issues, but not
"auth-rework is spec'd and not yet ticketed, billing-fix is mid-implementation,
three things are waiting on review". There is no board of where my work actually
stands.

## Solution

A CLI called `flow` that keeps that position in files inside the repo.

The sequence of steps is declared as data in `.flow/flow.toml`, so it is my
sequence, not one baked into a tool. Each Run — one piece of work traversing it —
is a single markdown file under `.flow/runs/`, with its position in frontmatter,
a Handoff block a cold agent reads first, and an append-only Log of how it got
there.

`flow next` answers "what do I run now" in one line. `flow status` prints the
board as a table. `flow board` writes a self-contained HTML card grid I can open
in a browser without a session running.

The CLI never executes anything. It prints the Command a Stage declares and
records that it happened. Running it is the agent's job, taught by an Adapter
file that `flow init` writes into the repo. That division is what lets the same
`.flow/` directory drive Claude Code, Codex, or me typing commands by hand.

## User Stories

1. As a developer returning to a repo after a week, I want `flow status` to show
   every Run and its current Stage, so that I can choose what to pick up without
   opening a single file.
2. As a developer whose session just died mid-task, I want `flow next` to print
   the exact command to run next, so that I can resume without re-deriving
   context.
3. As an agent starting cold in a repo, I want to read a Run's Handoff block and
   know where the work stands, so that I can continue without re-reading the
   whole codebase.
4. As a developer, I want each Stage transition appended to a Log, so that I can
   see how a Run reached its current state months later.
5. As a developer, I want the Flow declared in a file I can edit, so that I can
   use my own sequence of steps rather than the tool author's.
6. As a developer trying flow for the first time, I want `flow init` to write a
   working default Flow into my repo, so that I can start immediately and edit
   the sequence afterward.
7. As a developer, I want `flow init` to also write an Adapter file, so that my
   agent learns the protocol without me explaining it each session.
8. As a developer, I want to start a Run with a title and a Kind, so that a
   bug, a feature and a whole project can all be tracked by the same tool.
9. As a developer, I want `flow done` to advance a Run to the next Stage, so
   that recording progress is one command.
10. As a developer, I want to record the Artifact a Stage produced, so that the
    evidence of the work is attached to the position.
11. As a developer whose session died mid-Stage, I want flow to notice that a
    Stage's Artifact exists while the Stage reads pending, so that Drift is
    surfaced rather than silently believed.
12. As a developer whose review kicked work back, I want to move a Run
    backwards to an earlier Stage, so that the tool matches what actually
    happens.
13. As a developer, I want to skip a Stage that does not apply to a given Run,
    so that a one-line bugfix does not need a full spec.
14. As a developer, I want `flow show <run>` to print one Run's full position,
    Handoff and Log, so that I can read its whole story in one place.
15. As a developer, I want `flow board` to write a standalone HTML file, so that
    I can see the card grid without an agent session or a server.
16. As a developer, I want to mark a Run finished, so that the board shows
    active work rather than everything I have ever done.
17. As a developer, I want the Handoff rewritten and the Log appended on each
    transition, so that current state and history never blur together.
18. As a developer running flow in a repo with existing files, I want init to be
    additive and idempotent, so that re-running it never clobbers my content.
19. As a developer, I want the state files committed to git, so that position
    travels with the branch and across machines.
20. As a developer on a machine with no network, I want every command to work,
    so that resuming never depends on a tracker being reachable.

## Implementation Decisions

- **Crate `runflow`, binary `flow`.** `flow` is taken on crates.io by an
  unrelated 2017 log analyzer. See ADR-0005.
- **The CLI never invokes the agent.** `flow next` prints a Command; execution
  belongs to the Adapter. See ADR-0001.
- **Files are the source of truth for position**, never a tracker label. See
  ADR-0002.
- **A Flow is data written into the repo**, not a sequence hardcoded in the
  binary. Nothing in the code may special-case a Stage name. See ADR-0003.
- **`flow init` writes the Adapter** into `.claude/skills/flow/SKILL.md` and an
  `AGENTS.md` block, additively and idempotently. See ADR-0004.
- **One serialization format: TOML.** `.flow/flow.toml` for the Flow definition,
  and TOML frontmatter (`+++`) for Run files, so the tool carries one parser and
  one mental model rather than TOML plus YAML.
- **Run file layout**: `.flow/runs/<slug>.md`, a single file per Run —
  frontmatter holding slug, title, kind, flow name, status, timestamps and the
  per-Stage records; body holding a `## Where we are` Handoff block followed by
  a `## Log` section.
- **Stage fields**: `name`, `description`, `command`, `artifact` (a path
  template accepting `{slug}`, or a `tracker:` reference), `repeatable`,
  `optional`. `command` is one string, with an optional per-agent override
  table.
- **Flow is linear.** Stages have an order; a Run has an index into it. Skipping
  and moving backwards are supported; branching is not — it belongs inside a
  Stage.
- **Drift detection** compares each Stage's recorded status against the presence
  of its declared Artifact on disk, and reports disagreement. Flow reports Drift;
  it never silently resolves it.
- **Presets ship in the binary via `include_str!` and are written out by init**,
  never resolved at read time.
- **The HTML board is a single self-contained file** produced by substitution
  into an embedded template — no external assets, no template engine, no
  network. It must be readable in both light and dark browser themes.
- **v1 is single-repo.** The cross-repo registry at `~/.config/flow/repos.toml`
  is designed for but not built.

## Testing Decisions

A good test here exercises external behaviour only: it runs the real binary
against a scratch repo and asserts on what the user sees — the process exit
code, stdout, and the files left on disk. It never reaches into internal
functions, and it never asserts on the exact wording of prose, only on the
structural facts a user depends on.

- **One seam: the CLI surface.** Tests invoke the compiled binary with
  `assert_cmd` against a `tempfile::TempDir`. There are no unit tests on
  internal parsing or rendering; those are reached through the commands that
  use them. One seam is the ideal number and this codebase is small enough to
  hold it.
- **Modules under test**: all of them, through that seam — `init`, `start`,
  `status`, `next`, `done`, `show`, `skip`, `back`, `finish`, `board`.
- **Round-trip is the core property**: writing state and reading it back through
  a *separate process invocation* is what proves the file-first premise. Any
  test that keeps state in memory across commands proves nothing.
- **Drift gets explicit tests**: a Stage marked pending whose Artifact exists,
  and a Stage marked done whose Artifact is missing.
- **Prior art**: none in this repo — it is empty. `tests/cli.rs` establishes the
  pattern.

## Out of Scope

- The cross-repo registry and any board spanning multiple repos.
- Any execution of Commands by the CLI.
- Branching, parallel Stages, or a DAG of any kind.
- Syncing position to GitHub labels or any tracker.
- A server, a daemon, a database, or a TUI.
- Adapters for agents other than Claude Code, beyond the generic `AGENTS.md`
  block.
- Publishing to crates.io and prebuilt release binaries.
- Domains other than software work.

## Further Notes

The default Preset mirrors Matt Pocock's main flow — grill, spec, tickets,
implement, review — because that is the flow being tested against. It is written
into the repo as editable TOML precisely so that this is a starting point rather
than the tool's opinion.

This spec was produced by running that flow on itself: the grill transcript and
its settled decisions are in `.scratch/flow-itself/grill.md`.
