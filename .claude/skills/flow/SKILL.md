---
name: flow
description: Drive this repo's flow — find what stage a piece of work is on, do that stage, and record it. Use when the user asks what to work on, says "resume", "where were we", "what's next", or finishes a stage of work.
---

# Flow

This repo tracks work with `flow`, a CLI that keeps each run's position in files
under `.flow/`. The files are the source of truth, not this conversation — so a
session ending costs nothing.

**`flow next` never runs anything.** It tells you the command; you run it.

**Never run `flow go`.** That command launches a *new* agent session, and you
are already one — running it would fork sessions without end. It is for a human
at a terminal. `flow` guards against this, but do not lean on the guard.

## Resuming

Start every session on this repo by orienting:

```bash
flow status      # every run and the stage it's on; * marks the current one
flow next        # the current run's stage, and the command to run
```

Bare commands act on the **current** run. If flow says several are active and
none is current, ask the user which they mean rather than picking one.

`flow next` prints a `## Where we are` handoff written by whoever worked last.
Read it before touching the codebase — it is there so you do not have to
re-derive context by reading everything.

If `flow next` reports **drift**, stop and resolve it with the user first. Drift
means a stage's recorded status disagrees with whether its artifact exists on
disk — almost always a session that died mid-stage. Do not guess which is right.

## Working a stage

1. `flow next` — read the stage description and the command.
2. Run that command. It is an ordinary skill or slash command; nothing about it
   is special to flow.
3. When the stage is genuinely complete, record it.

## Recording

```bash
flow done -m "<handoff>" --artifact <path>
```

The `-m` message becomes two things: it **replaces** the `## Where we are` block
and it is **appended** to the `## Log`. Write it for someone who has never seen
this work — what got decided, what is next, and anything surprising. Two or
three sentences. Never write "done" or "completed stage".

`--artifact` records what the stage actually produced, when it produced a file.

Other moves:

```bash
flow skip -m "..."               # pass over an optional stage
flow back -m "..."               # review kicked the work back
flow back --stage implement -m "..."
flow finish -m "..."             # the run is complete
flow reopen -m "..."             # a finished run turned out not to be
flow start "<title>" --kind bug  # begin a new run
```

## Rules

- Record a stage as done only when it is actually done. The file outlives the
  session; a lie in it costs a future session real time.
- Never hand-edit `.flow/runs/*.md` when a command will do it. The log is
  append-only by construction, and editing breaks that.
- `.flow/flow.toml` is this repo's flow and is editable. If a stage is
  consistently wrong, say so rather than working around it.
- Commit `.flow/` along with the work, so position travels with the branch.
