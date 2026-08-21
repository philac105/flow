# Presets are discovered, never written

`flow` reads presets from three places — `.flow/presets/` in the repo and its
ancestors up to the ceiling below, `$XDG_CONFIG_HOME/flow/presets/`, and a set
embedded in the binary at build time from `presets/` — and **writes to none of
them**. The only file `flow init` creates from a preset is the repo's own
`.flow/flow.toml`.

The alternative was installing our presets to `~/.config/flow/presets/` on first
run or upgrade, and reading everything back from disk. That is the only design
in which the tool can clobber a preset someone wrote, which then needs merge
rules, a backup story, and an upgrade path that gets them wrong quietly. Keeping
ours in the binary makes "flow never overrides your presets" structural rather
than careful: there is no write to get wrong.

Precedence follows ADR-0007's rule — layered by who owns it. Project beats user
beats shipped, nearest ancestor first, the same order in which
`Settings::resolve` already layers agent launchers. A preset that shadows
another is shown as shadowing it in `flow presets`, because silent shadowing is
how someone loses an afternoon to a flow they do not recognise.

## Considered options

**`extends = "main-flow"` with stage-level patching**, so a user preset could
append a stage or override one command and ride along as ours evolved. Rejected:
it needs merge semantics for an *ordered* list, a story for when the base renames
a stage a patch targets, and a failure mode — and it buys nothing after `init`,
since `.flow/flow.toml` is already a full copy the repo owns (ADR-0003). A flow
is a few lines of TOML per stage. Copying and editing one is legible; merging two
is not.

## Consequences

A preset's identity is its filename stem and its description lives inside the
file, so adding one is dropping a `.toml` in a directory — no Rust, no manifest.
A `build.rs` generates the embedded set and fails the build if a shipped
preset's filename and `name` disagree; the same disagreement in an on-disk
preset is skipped with a reason rather than being fatal, because one bad file in
a directory you are not using must not stop you initialising a repo.

Nested `.flow` directories are deliberate: `flow init` writes to the working
directory while the preset walk unions every ancestor, which is what lets a
monorepo standardise the flows its packages reach for. `init` names the ancestor
it took a preset from.

**The walk has a ceiling: your repository root, or your home directory,
whichever is farther — and the starting directory alone when neither is on the
ancestry.** It first said "any ancestor", which reached `/`. A preset is not
inert data: it carries the launcher argv `flow go` spawns, so on a shared
machine anyone able to write `/tmp/.flow/presets/main-flow.toml` would change
what a bare `flow init` writes for every repo beneath it, beating what ships and
looking exactly like the repo's own. The ceiling keeps the monorepo case whole —
that root is the outermost `.git`, above any submodule's — and stops the reach
at the last directory you can be said to own. Taking the farther of the two is
what lets a `~/work/.flow/presets` still cover the repos underneath it.
