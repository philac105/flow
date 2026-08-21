# Rust, published as `runflow` with a `flow` binary

Flow is a Rust binary rather than an npx-installable Node package. Node would
have made distribution free; Rust makes it a single dependency-less binary that
does not care whether the target repo has a Node toolchain — which matters for a
tool meant to sit in *any* repo. The crate name `flow` is already taken on
crates.io by an unrelated 2017 log analyzer, so the crate is `runflow` and the
binary it installs is `flow`.

The two names are deliberately allowed to differ, and should not be unified.
This is ordinary in Rust — `fd-find` ships `fd`, `ripgrep` ships `rg` — and the
same forced rename is why `fd-find` is called that. A crate name is a
distribution detail; the binary name is the interface people and agents read.

## Consequences

Distribution needs prebuilt binaries per platform, not just `cargo install`, or
the tool is only usable by people who already have a Rust toolchain.

`flow-bin`, Facebook's JavaScript type checker, also installs a binary called
`flow`. In a repo that has it installed globally, PATH order decides which one
wins; the fix is a shell alias, not a rename here.
