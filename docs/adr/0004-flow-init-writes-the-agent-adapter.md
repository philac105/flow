# `flow init` writes the agent Adapter into the repo

`flow init` scaffolds `.claude/skills/flow/SKILL.md` and a block in `AGENTS.md`
alongside `.flow/`. Without this the protocol is something the human has to
re-explain to the agent every session — the precise failure Flow was built to
remove. Writing the Adapter into the repo means any agent opening the project
finds the protocol in files already in front of it.

## Consequences

Flow writes into `.claude/` and `AGENTS.md`, directories it does not own, so it
must be additive and idempotent: update its own block in place, never clobber
surrounding content.
