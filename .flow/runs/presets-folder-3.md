+++
slug = "presets-folder-3"
title = "presets-folder-3"
kind = "feature"
flow = "main-flow"
status = "active"
created = "2026-08-21T00:01:10.922Z"
updated = "2026-08-21T14:18:19.616Z"

[[stage]]
name = "grill"
status = "done"
artifact = ".scratch/presets-folder-3/grill.md"
started = "2026-08-21T00:01:10.922Z"
completed = "2026-08-21T00:13:14.264Z"

[[stage]]
name = "spec"
status = "done"
artifact = "tracker:https://github.com/philac105/flow/issues/11"
started = "2026-08-21T00:13:14.264Z"
completed = "2026-08-21T00:18:57.628Z"

[[stage]]
name = "tickets"
status = "done"
started = "2026-08-21T00:18:57.628Z"
completed = "2026-08-21T00:23:58.451Z"

[[stage]]
name = "implement"
status = "done"
started = "2026-08-21T00:23:58.451Z"
completed = "2026-08-21T14:18:19.616Z"

[[stage]]
name = "review"
status = "in_progress"
started = "2026-08-21T14:18:19.616Z"
+++

## Where we are

`implement` done → `review`

All six tickets for issue #11 are landed, closed on GitHub, and committed on build-flow. Presets are no longer a hardcoded Rust list: they are discovered on the Preset Path — project (.flow/presets/ in the repo and every ancestor of it, nearest first), then user ($XDG_CONFIG_HOME/flow/presets/), then shipped (embedded by build.rs from presets/ at the repo root).

The code: src/preset_path.rs is the new module that owns the Path and is deliberately NOT unit-tested — the CLI is the seam, and tests/cli.rs covers every behaviour through 'flow presets', 'flow init' and 'flow config'. The one unit-tested seam is src/preset_name.rs, the stem-equals-name rule, shared by build.rs (fatal) and discovery (skip with a reason). src/flow.rs is untouched: the ancestor walk is a second traversal, because find_root stops at the first .flow and would blind a package that has already run init to the repo root's menu. 'flow presets' lists every layer with a source column, keeps shadowed entries visible with the description of what was overridden, and has a Skipped section with four reasons. 'flow init' resolves a name (including the pinned default main-flow) through the Path, names the layer and for a project preset the ancestor, and hard-errors rather than falling back. 'flow config' prints the preset directories. README points at 'flow presets' instead of listing them.

Reviewed on both axes (/code-review, standards + spec). Four findings fixed in 871b014: the listing header claimed 'nearest owner first' while rows are alphabetical (that is the precedence rule, not the row order); a shadowed entry showed only its layer, not what you had overridden; a 4-tuple that was really a Preset; and Asked having three variants for two messages. Two findings were deliberately NOT actioned and a later session should not re-litigate them without a reason: 'flow config' prints ancestor preset dirs only when they exist (printing every directory up to / would be noise, and the nearest one IS marked when absent), and Layer has no CONTEXT.md entry (the Preset Path entry already names project/user/shipped).

State: 110 integration + 4 unit tests green, cargo fmt --check and cargo clippy --all-targets clean, release build clean. Parent spec #11 is still OPEN on purpose — closing it is the review stage's call. Note for whoever picks this up: .idea/ is untracked and not in .gitignore, so it is one 'git add -A' away from being committed.

## Log

### 2026-08-21T00:01:10Z — Started. First stage is `grill`.

l
l
I would like for it to be more explicit for the presets in flow. So instead of being hardcoded, i'd like it to be a folder presets/ with the different .toml presets of the app there so that it can evolve. we need to think about how to deal with user presets too to not override them and also about them overriding our intelligently.

Interview until the plan is sharp. Produces decisions, a glossary and ADRs.

Run `/grill-with-docs` next.

### 2026-08-21T00:13:14Z — `grill` done → `spec`

Grill is done: 16 decisions settled across three rounds, all confirmed. Presets stop being a hardcoded Rust list (PRESETS in src/commands/init.rs) and become a discovered set on a three-layer Preset Path — project (.flow/presets/, unioned up every ancestor), user (~/.config/flow/presets/), shipped (embedded from presets/ at the repo root via a build.rs that validates filename-stem == name). Load-bearing decision: flow NEVER writes to the user or project preset folders, which makes 'never overrides your presets' structural rather than careful. Override means whole-file shadowing, not extends/merge — that was considered and rejected in writing. A preset's id is its filename stem and its description lives inside the file, killing the Rust/TOML duplication that has already drifted. Decisions and rejected alternatives in .scratch/presets-folder-3/grill.md; ADR-0008 'Presets are discovered, never written' written; CONTEXT.md gains 'Preset Path' and redefines 'Preset' (it is no longer 'shipped inside the binary'). Next stage is spec, which should pick up the four known code consequences listed at the bottom of the grill: config::presets() takes no root and main.rs discards the one it computes, flow presets needs a source column, flow config should print the preset dirs, and README's hand-written preset table will drift again.

### 2026-08-21T00:18:57Z — `spec` done → `tickets`

Spec is published as GitHub issue #11, labelled ready-for-agent: 'Presets become a discovered set on a three-layer Preset Path'. It turns the 16 grill decisions into problem statement, 36 user stories, implementation decisions, testing decisions, and an explicit out-of-scope list. Read the issue first; ADR-0008 and CONTEXT.md's 'Preset Path' entry are the supporting docs, and .scratch/presets-folder-3/grill.md holds the rejected alternatives with reasons.

Two seams were agreed with the user and are pinned in the spec. (1) The CLI is the only seam for all discovery behaviour — precedence, ancestor unioning, shadowing, skip reasons, init's source line, the presets listing, the config output. The discovery module is deliberately NOT unit-tested; everything is observable through 'flow presets', 'flow init', 'flow config', and tests/cli.rs already isolates XDG_CONFIG_HOME per test. (2) One new tiny seam: the stem-equals-name validation function, shared by build.rs (fatal) and the runtime (skip loudly), unit-tested directly because a build failure cannot be reached through the CLI.

All four grill consequences are folded into the spec: commands::config::presets() must take a root (main.rs currently computes one and discards it), 'flow presets' gains a source column plus a shadow note plus a skipped-with-reasons section, 'flow config' prints the preset directories, and README's hand-written preset table becomes a pointer to 'flow presets'. The existing every_built_in_preset_actually_works test must iterate the build-generated set instead of three hardcoded names.

Nothing is blocked and no open questions remain. Next stage is tickets: break issue #11 into implementable pieces. The natural fault lines are the build.rs + shared validation + moving assets/*.toml to presets/, the discovery module itself, then the three command surfaces (init, presets, config), then the docs sweep.

### 2026-08-21T00:23:58Z — `tickets` done → `implement`

Issue #11 is broken into six tickets on GitHub, all labelled ready-for-agent and wired with native GitHub blocking dependencies (verified via issue_dependencies_summary). #11 itself was not modified; each ticket names it under a Parent heading.

The chain: #12 (no blockers) moves the three flow files from assets/ to presets/ at the repo root, adds the build.rs that generates the embedded set from that directory, deletes the name/description tuple array in init.rs, and lands the shared stem-equals-name function — unit-tested directly, fatal in the build script. It also converts every_built_in_preset_actually_works to iterate the generated set. #13 (blocked by #12) builds the discovery module and rewrites flow presets: three layers project -> user -> shipped, the project layer unioning every ancestor rather than reusing find_root's nearest-.flow search, a source column, and shadowed entries still listed and marked. presets() starts taking a root here. #14, #15 and #16 all hang off #13 and can run in parallel: #14 adds skip-with-reasons (four reasons, non-.toml silently ignored), #15 makes init resolve the name main-flow through the Path with a source line and a hard error for a default that resolves to nothing, #16 makes flow config print the preset directories. #17 (blocked by #14, #15, #16) replaces the README's hand-written preset table with a pointer to flow presets.

Granularity was confirmed with the user: #13 and #14 stay split rather than building the whole resolved/shadowed/skipped shape at once, and #17 stays its own ticket rather than folding into #15. The discovery module is deliberately not unit-tested — that decision is restated inside #13 so an implementer does not add a seam the spec rejected. Next stage is implement; start at #12, which is the only unblocked ticket.

### 2026-08-21T14:18:19Z — `implement` done → `review`

All six tickets for issue #11 are landed, closed on GitHub, and committed on build-flow. Presets are no longer a hardcoded Rust list: they are discovered on the Preset Path — project (.flow/presets/ in the repo and every ancestor of it, nearest first), then user ($XDG_CONFIG_HOME/flow/presets/), then shipped (embedded by build.rs from presets/ at the repo root).

The code: src/preset_path.rs is the new module that owns the Path and is deliberately NOT unit-tested — the CLI is the seam, and tests/cli.rs covers every behaviour through 'flow presets', 'flow init' and 'flow config'. The one unit-tested seam is src/preset_name.rs, the stem-equals-name rule, shared by build.rs (fatal) and discovery (skip with a reason). src/flow.rs is untouched: the ancestor walk is a second traversal, because find_root stops at the first .flow and would blind a package that has already run init to the repo root's menu. 'flow presets' lists every layer with a source column, keeps shadowed entries visible with the description of what was overridden, and has a Skipped section with four reasons. 'flow init' resolves a name (including the pinned default main-flow) through the Path, names the layer and for a project preset the ancestor, and hard-errors rather than falling back. 'flow config' prints the preset directories. README points at 'flow presets' instead of listing them.

Reviewed on both axes (/code-review, standards + spec). Four findings fixed in 871b014: the listing header claimed 'nearest owner first' while rows are alphabetical (that is the precedence rule, not the row order); a shadowed entry showed only its layer, not what you had overridden; a 4-tuple that was really a Preset; and Asked having three variants for two messages. Two findings were deliberately NOT actioned and a later session should not re-litigate them without a reason: 'flow config' prints ancestor preset dirs only when they exist (printing every directory up to / would be noise, and the nearest one IS marked when absent), and Layer has no CONTEXT.md entry (the Preset Path entry already names project/user/shipped).

State: 110 integration + 4 unit tests green, cargo fmt --check and cargo clippy --all-targets clean, release build clean. Parent spec #11 is still OPEN on purpose — closing it is the review stage's call. Note for whoever picks this up: .idea/ is untracked and not in .gitignore, so it is one 'git add -A' away from being committed.
