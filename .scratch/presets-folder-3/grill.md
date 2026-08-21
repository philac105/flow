# Grill: presets as a folder

Started 2026-08-20. Stage: grill (main-flow, 1/5). Run `presets-folder-3`.

## Premise

The brief was "presets should be a folder `presets/` instead of hardcoded, plus
user presets that we never override and that override ours intelligently".

Reading the code first moved the target. **Presets are already files** —
`assets/main-flow.toml`, `minimal.toml`, `bugfix.toml`. What is hardcoded is the
*list*: `PRESETS: &[(&str, &str, &str)]` in `src/commands/init.rs:9`, one
`include_str!` per entry. Adding a preset means editing Rust. So the real ask is
**make the set discoverable**, not "move them into files".

Two more things the code said:

- The description is duplicated and has already drifted. Rust says main-flow is
  "The idea → ship spine, in order. Five stages, uses an issue tracker.";
  `assets/main-flow.toml:16` says "The idea → ship spine, in order."
- Nothing in the codebase can currently override a user preset, because nothing
  writes to a user's config dir except `flow config --init`. "Don't override my
  presets" is therefore a constraint on a thing not yet built — which made
  "does flow ever write presets to disk?" the root of the whole tree.

## Decisions

| # | Decision | |
|---|---|---|
| D1 | Shipped presets are **embedded at build time** from `presets/` at the repo root. Files move out of `assets/`. | Q1 |
| D2 | A preset's id is its **filename stem**; its description is read from **inside the file**. The Rust name/description table is deleted. Filename stem must equal the flow's `name`. | Q2 |
| D3 | **Three layers**, the Preset Path: project (`.flow/presets/`), user (`~/.config/flow/presets/`), shipped (embedded). | Q3 |
| D4 | Override means **whole-file shadowing**. No `extends`, no stage-level merge. | Q4 |
| D5 | A shadowed preset is **visible** in `flow presets` — user wins over shipped, and the listing says what it shadowed. | Q5 |
| D6 | A bare `flow init` writes a **pinned name** (`main-flow`), never a positional `PRESETS[0]`. If a user or project preset shadows that name, theirs is what gets written. | Q6 |
| D7 | A preset stays **exactly one flow file**. No bundles (flow + adapter + seed notes) — that is different work. | Q7 |
| D8 | The project layer is reached by **walking upward**, so a monorepo can standardise the flows its packages reach for. No `--force`, no re-init/overwrite path. | Q8 |
| D9 | Precedence: **project > user > shipped**, matching how `Settings::resolve` already layers agents (`src/config.rs:81-85`). | Q9 |
| D10 | A malformed or misnamed on-disk preset is **skipped loudly** — listed as skipped with a reason, never fatal, never silent. Non-`.toml` files are ignored without comment. | Q10 |
| D11 | Embedding is a **`build.rs`** that walks `presets/` and generates the array. No new dependency, and the D2 stem-equals-`name` rule is enforced **at build time** for shipped presets. | Q11 |
| D12 | The upward walk **unions every ancestor** `.flow/presets/`, nearest shadowing farther — not `find_root`'s first-match. | Q12 |
| D13 | **Nested `.flow` directories are supported and deliberate.** `flow init` prints which ancestor it drew a preset from, so inheritance is never spooky. | Q13 |
| D14 | A configured default that resolves nowhere is a **hard error** naming what is available. Never a silent or warned fallback. | Q14 |
| D15 | New glossary term **Preset Path**, sources named **project / user / shipped**. **Preset** is redefined — "shipped inside the binary" is now true of only one source. | Q15 |
| D16 | **One ADR**: *Presets are discovered, never written*. Precedence goes in it as a paragraph, not a sibling document. | Q16 |

## The load-bearing one

**`flow` never writes to `~/.config/flow/presets/` or `.flow/presets/`.** Both
are read-only to the tool. That is what makes "never overrides your presets"
structural rather than careful — there is no merge logic to get wrong, because
there is no write. It is also why D1 chose embedding over an installed data dir:
installing is the only design where clobbering is possible at all.

## Rejected, and why

- **`extends = "main-flow"` with stage-level patching.** The "intelligent
  override" reading. Costs merge semantics for an *ordered* list (where does an
  appended stage go?), a story for when the base renames a stage a patch
  targets, and a failure mode. Buys nothing after `init`, because
  `.flow/flow.toml` is already a full copy the repo owns (ADR-0003). A flow is
  ~9 lines of TOML per stage; copy-and-edit is legible.
- **`presets/index.toml` manifest.** Reintroduces exactly the hardcoded list
  being removed.
- **A repo-layer `--force` re-init that swaps an existing flow.** Builds an
  overwrite path into a tool whose posture is "the repo owns its copy, we forget
  about it". Left out; reopen deliberately if wanted.
- **`include_dir` crate.** Fewer lines, but a sixth dependency and no build-time
  validation.
- **Nearest-ancestor-only walk (reusing `find_root`).** One rule instead of two,
  but a package that has already run `init` stops the walk and never sees the
  repo root's menu — gutting the only case the project layer exists to serve.
- **Silent fallback when a configured default is missing.** Writes a flow you
  did not ask for into a file you are then told you own.

## Known consequences for the spec stage

- `commands::config::presets()` takes no `root`; `main.rs:148` computes one for
  `Command::Presets` and discards it at `main.rs:154`. That signature has to
  change before any project layer is visible.
- `flow presets` grows a source marker per row (project / user / shipped) and a
  shadow note.
- `flow config` should print the preset directories alongside the paths it
  already prints — it is the "where do I set this up" command (ADR-0007).
- The existing end-to-end test that runs every shipped preset should iterate the
  build-generated list, so a new file in `presets/` is covered automatically.
- `tests/cli.rs:92` (`an_unknown_preset_is_refused`) already pins D14's shape for
  the argv path; the config path needs the same coverage.
- README's preset table (lines 167-174) is a hand-maintained duplicate of the
  descriptions and will drift again unless it points at `flow presets`.

## Open

Nothing. Every branch of the tree was visited and confirmed.
