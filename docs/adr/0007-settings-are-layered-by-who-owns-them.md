# Settings are layered by who owns them

Configuration splits along ownership, not convenience:

- `<repo>/.flow/flow.toml` — **which stages exist**. The project's process.
  Committed, shared, reviewed like code.
- `$XDG_CONFIG_HOME/flow/config.toml` — **which agent you drive and how it
  starts**. Yours and your machine's. Never in a repo.

`flow go` briefly put the launcher in the repo file, which meant committing a
flow shipped its author's tooling to everyone who cloned it — a teammate on
Codex would inherit someone else's `claude` launcher. The two settings answer
questions with different owners, so they live in different files.

A repo may still declare an `[agents.<name>]` table to override, for the case
where a project genuinely needs a specific setup. The shipped preset does not,
so the usual case is that no repo mentions an agent at all.

Resolution order for the agent: `--agent` flag, then the repo, then the user
config, then the only launcher configured if there is exactly one.

## Consequences

`flow config` exists to answer "where do I set this up", so it prints real paths
and the source of every resolved setting rather than just the values. It works
outside a flow repo, because a machine gets set up before any repo does.
