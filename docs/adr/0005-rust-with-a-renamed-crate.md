# Rust, published as `runflow` with a `flow` binary

Flow is a Rust binary rather than an npx-installable Node package. Node would
have made distribution free; Rust makes it a single dependency-less binary that
does not care whether the target repo has a Node toolchain — which matters for a
tool meant to sit in *any* repo. The crate name `flow` is already taken on
crates.io by an unrelated 2017 log analyzer, so the crate is `runflow` and the
binary it installs is `flow`.

## Consequences

Distribution needs prebuilt binaries per platform, not just `cargo install`, or
the tool is only usable by people who already have a Rust toolchain.
