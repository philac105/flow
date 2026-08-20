# flow

Track work from idea to shipped code, in files that outlive the session.

You already have skills for the hard parts — sharpening an idea, writing a spec,
breaking it into tickets, building it, reviewing it. What you don't have is
anywhere that remembers **which of those you were in the middle of**. That lives
in a chat session, and chat sessions end: you run out of tokens, you close the
terminal, it's Monday.

`flow` keeps that position in markdown files inside the repo. Come back in a
week, on another machine, with a different agent, and `flow next` tells you
where you were and what to run.

```
$ flow status
RUN          KIND     STAGE      DONE
auth-rework  feature  implement  3/5
billing-fix  bug      spec       1/5

$ flow next auth-rework
Auth rework — stage 4 of 5

  implement
  Build it, working the frontier of unblocked tickets. Test-first at the agreed seams.

  run: /implement

## Where we are

Nine tickets published, #2-#10, with blocking edges. Start at #2 — it carries
the test harness everything else uses. The tracker choice is still open.
```

## Hand a stage straight to an agent

```bash
flow go
```

opens your agent on the current stage, with the prompt already built — the
stage's command, who the run is, the handoff the last session left, and the
exact `flow done` line to record it with. No copy-paste, no re-explaining.

Set the agent up once per machine:

```bash
flow config --init      # writes ~/.config/flow/config.toml
flow config             # shows every path and where each setting came from
```

```toml
agent = "claude"

[agents.claude]
command = ["claude", "{prompt}"]
guard_env = ["CLAUDECODE"]
```

`flow` substitutes and spawns; it never learns what is on the other end, so
another agent is a table in a config file rather than a code path.

`flow go` will not record the stage for you. An agent exiting cleanly means the
session ended, not that the work is done — and a wrong entry in the one file a
later session trusts is worse than no entry. It tells you what changed and
leaves `flow done` to you.

**`flow next` still runs nothing.** It prints. That is what agents call, and why
launching is a separate command: the adapter tells an agent to run `flow next`,
so if that spawned a session, an agent following its instructions would fork
sessions forever. `guard_env` is the backstop — inside a session, `flow go`
prints the prompt instead of spawning.

## Install

```bash
cargo install --path .
```

The crate is `runflow`; the binary it installs is `flow`.

## Use

```bash
flow init                      # write the flow and the agent adapter into this repo
flow start "Auth rework" --kind feature
flow next                      # what to do now, and the command for it
flow go                        # hand the stage to your agent, prompt included
flow config                    # where settings live, and what resolved from where
flow done -m "<handoff>"       # record it and advance
flow status                    # the board
flow board                     # the board as a standalone HTML file
```

When a stage doesn't apply, or a review sends work back:

```bash
flow skip -m "one-liner, no spec needed"
flow back -m "spec was wrong"
flow back --stage implement -m "review found a hole"
flow finish -m "shipped"
flow reopen -m "it came back"
```

## Where settings live

Two files, split by who owns the answer:

| | |
|---|---|
| `<repo>/.flow/flow.toml` | **which stages exist** — the project's process. Committed and shared. |
| `~/.config/flow/config.toml` | **which agent you drive, and how it starts** — yours and this machine's. Never in a repo. |

That split is deliberate. A flow is worth committing so a team shares one
process; your choice of agent is not, or everyone who clones the repo inherits
your tooling. `flow config` prints both paths and the source of every resolved
setting, and works before any repo is set up.

A repo can override an agent by name when it genuinely needs to — the preset
never does.

## The flow is yours

`flow init` writes `.flow/flow.toml` into your repo and then forgets about it.
It is not a hidden default you have to fork the tool to change:

```toml
[[stage]]
name = "spec"
description = "Synthesise the conversation into a spec and publish it to the tracker."
command = "/to-spec"
artifact = "tracker:issue"
optional = false
repeatable = false
```

`command` is any string you want your agent to run. `artifact` is what the stage
should leave behind — a path (with `{slug}` standing in for the run) gets
checked on disk; a `tracker:` value is recorded and never checked.

The shipped preset mirrors [Matt Pocock's main flow](https://www.aihero.dev/)
— grill, spec, tickets, implement, review — because that is the flow this was
built and tested against. Delete stages, rename them, point them at your own
commands.

## Drift

A stage declaring an artifact is how a dead session announces itself. If the
file exists but the stage still reads pending, something happened that was never
recorded:

```
$ flow next
drift — resolve this before continuing:
  ! grill: .scratch/auth-rework/grill.md exists but the stage is in progress — did a session die here?
```

`flow` reports drift and never resolves it. Which of the two is lying is not
something a tool should guess.

## What's in a run

One file per run, `.flow/runs/<slug>.md`. Position in TOML frontmatter, then two
sections that are deliberately different:

- **`## Where we are`** — replaced on every transition. The twenty lines a cold
  agent reads first.
- **`## Log`** — appended to, never rewritten. How the run got here, including
  the reversals.

Commit `.flow/` with your work and the position travels with the branch.

## For agents

`flow init` also writes `.claude/skills/flow/SKILL.md` and a delimited block in
`AGENTS.md`, so an agent opening the repo finds the protocol without you
explaining it. Both writes are additive — re-running `init` updates its own
block and leaves the rest byte for byte.

## Not in v1

A registry spanning several repos, any execution of commands by the CLI,
branching or parallel stages, and syncing position to tracker labels. The
reasoning for each is in [`docs/adr/`](./docs/adr/).

## Development

```bash
cargo test    # 46 integration tests, all driving the real binary
```

There is one seam: the CLI. Tests run the compiled binary against a temp
directory and assert on exit codes, stdout, and the files left behind. State
crossing a process boundary is the property the tool rests on, so tests that
need state spend one invocation writing it and another reading it back.
