# Flow's state is one folder, and Flow never moves or deletes what is in it

Everything Flow writes lives under `.flow/`: the flow definition, the run files,
and the stage artifacts at `.flow/artifacts/{slug}/<stage>.md`. The shipped
presets previously disagreed about that last one — `main-flow` wrote to
`.scratch/{slug}/`, `minimal` and `bugfix` to `.flow/notes/{slug}.md` — which is
how a second top-level dot-directory came to exist for one tool. One folder
means ignoring, backing up or wiping Flow's state is one path rather than a list
somebody has to keep current.

Within that folder Flow only ever adds. There is no archive directory, no
`flow delete`, no `flow prune`, and `flow finish` leaves the run file exactly
where it was.

## Considered options

**Archiving finished runs to `.flow/runs/archive/`.** Rejected because the
artifacts would stay behind, and filing the run while its output sits in the
live directory is half a move. Taking the artifacts along is worse: a run file
records the artifact path as it was when the stage completed, and `Run::drift`
trusts that record over the stage's declaration in `flow.toml`. A `finish` that
moved artifacts would have to rewrite those paths, turning the record from what
happened into where things ended up — the tool quietly editing the evidence it
checks itself against.

**A `flow delete` or `flow prune` subcommand.** Rejected: `rm .flow/runs/x.md`
already does it. A destructive command in a tool whose entire pitch is that
state survives the session has to do something `rm` cannot, and this one does
not.

**Gitignoring `runs/` by default**, on the grounds that a run's position is
personal like `.git/HEAD`. Rejected because every noun in the glossary points
the other way: a Handoff is defined as the first thing a *cold* agent reads and
a Brief as the only context the first Stage has, and both of those readers are
usually on another machine. A fresh clone that printed "no runs yet" would make
Flow a single-person tool.

## Consequences

The clutter that prompted all this gets two answers rather than a feature.
`flow status` already hides finished runs unless you pass `--all`; and the
`.flow/.gitignore` that `init` writes now carries commented-out `/runs/` and
`/artifacts/` blocks under a comment explaining the trade-off. If run files are
noise to you, uncomment one and they become local files you can delete without
the tool's blessing. ADR-0002 puts position in files; it never required those
files to be committed, and which of them git tracks stays yours.

That gitignore is now a file someone is invited to edit, so `init` writes it
only when absent and reports `kept .flow/.gitignore (already yours)` otherwise —
the treatment `flow.toml` already gets, and what ADR-0004 meant by idempotent.
`.claude/skills/flow/SKILL.md` keeps being overwritten, because the adapter is
ours and has to track the protocol.

The rule constrains the tool, not the person. A human doing a one-off migration
may edit a recorded artifact path — that is how this repo's `.scratch/` files
move under `.flow/artifacts/` without leaving permanent drift behind. What is
forbidden is Flow doing it on its own.

`.flow/artifacts/` accumulates one directory per run and is never pruned. That
is accepted rather than regretted: a spec is something you go back and reread,
unlike a finished run's log. And because a repo owns its `flow.toml` from the
moment `init` writes it (ADR-0003), the new artifact path reaches new repos
only — every existing one keeps whatever it already has.
