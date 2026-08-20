# Launching an agent is configured, not compiled

ADR-0001 said the CLI never invokes the agent, to keep the binary from learning
what Claude Code is. `flow go` now launches one — but the launcher is an argv
declared in `.flow/flow.toml`, substituted and spawned, so the property that
mattered survives: nothing in the Rust names an agent, and supporting another
one is a table in a config file rather than a code path.

`flow next` stays a pure read-model and is what the adapter tells agents to
call. Launching had to be a separate command: the adapter instructs an agent to
run `flow next`, so if that spawned a session, an agent following its own
instructions would fork sessions without end.

## Consequences

`flow go` is a command for a human at a terminal. Agents already running must
use `flow next`. That is a convention, so it is backed by a mechanism: a
launcher declares `guard_env`, and when any of those variables is set — the
preset uses `CLAUDECODE` — `flow go` prints the prompt instead of spawning.

`flow go` never records the stage. An agent exiting cleanly means the session
ended, not that the work was done, and inferring completion from an exit code
would write a lie into the one file a later session is supposed to trust. It
reports what changed and leaves recording to a deliberate `flow done`.
