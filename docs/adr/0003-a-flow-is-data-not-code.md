# A Flow is data written into the repo, not code inside the binary

Stages are declared in `.flow/flow.toml`, which `flow init` writes into the repo
from a Preset. The alternative — hardcoding the idea → spec → tickets →
implement → review sequence — would have been considerably less work, and would
have made Flow a tool for exactly one flow. Writing the Preset out rather than
resolving it from inside the binary means the shipped default is editable on
day one without forking the tool, so "works with any flow" is a property of the
design rather than a promise about a future version.

## Consequences

Flow cannot assume any particular Stage exists. Nothing in the binary may
special-case a Stage called `spec` or `review`; a Stage is only a name, a
Command, and an Artifact.
