+++
slug = "presets-folder-3"
title = "presets-folder-3"
kind = "feature"
flow = "main-flow"
status = "active"
created = "2026-08-21T00:01:10.922Z"
updated = "2026-08-21T00:23:58.451Z"

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
status = "in_progress"
started = "2026-08-21T00:23:58.451Z"

[[stage]]
name = "review"
status = "pending"
+++

## Where we are

`tickets` done → `implement`

Issue #11 is broken into six tickets on GitHub, all labelled ready-for-agent and wired with native GitHub blocking dependencies (verified via issue_dependencies_summary). #11 itself was not modified; each ticket names it under a Parent heading.

The chain: #12 (no blockers) moves the three flow files from assets/ to presets/ at the repo root, adds the build.rs that generates the embedded set from that directory, deletes the name/description tuple array in init.rs, and lands the shared stem-equals-name function — unit-tested directly, fatal in the build script. It also converts every_built_in_preset_actually_works to iterate the generated set. #13 (blocked by #12) builds the discovery module and rewrites flow presets: three layers project -> user -> shipped, the project layer unioning every ancestor rather than reusing find_root's nearest-.flow search, a source column, and shadowed entries still listed and marked. presets() starts taking a root here. #14, #15 and #16 all hang off #13 and can run in parallel: #14 adds skip-with-reasons (four reasons, non-.toml silently ignored), #15 makes init resolve the name main-flow through the Path with a source line and a hard error for a default that resolves to nothing, #16 makes flow config print the preset directories. #17 (blocked by #14, #15, #16) replaces the README's hand-written preset table with a pointer to flow presets.

Granularity was confirmed with the user: #13 and #14 stay split rather than building the whole resolved/shadowed/skipped shape at once, and #17 stays its own ticket rather than folding into #15. The discovery module is deliberately not unit-tested — that decision is restated inside #13 so an implementer does not add a seam the spec rejected. Next stage is implement; start at #12, which is the only unblocked ticket.

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
