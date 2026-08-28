## Problem Statement

Adding a Preset means editing Rust. The Preset files already exist as TOML, but
the *set* of them is a hardcoded array in the init command — one entry per
Preset, each pairing a name and a description with an `include_str!`. Dropping a
new `.toml` next to the others does nothing until someone adds a fourth tuple
and recompiles.

Three things follow from that, and all three are already biting:

- **The description is stored twice and has already drifted.** Rust says
  main-flow is "The idea → ship spine, in order. Five stages, uses an issue
  tracker."; the file itself says "The idea → ship spine, in order." Two sources
  of truth for one sentence, and they no longer agree.
- **There is no way to have a Preset of your own.** You can pass a path to
  `flow init --preset ./mine.toml`, but you have to remember and retype that
  path every time, it cannot be the default a bare `flow init` writes, and it
  never appears in `flow presets`. A Preset you wrote is a second-class thing.
- **A team cannot standardise.** A monorepo that wants its packages reaching for
  the same three flows has nowhere to put them.

And the original ask — "user presets that we never override" — describes a fear
about a feature that does not exist yet. Nothing writes to a user's config
directory today except `flow config --init`. The question is not how to merge
carefully; it is whether `flow` should ever write there at all.

## Solution

Presets become a **discovered set on the Preset Path** rather than a compiled
list.

`flow` looks for Presets in three places, nearest owner first:

1. **project** — `.flow/presets/` in the working directory and in *every*
   ancestor of it, nearest ancestor first
2. **user** — `$XDG_CONFIG_HOME/flow/presets/` (falling back to `~/.config`)
3. **shipped** — embedded in the binary at build time from `presets/` at the
   repo root

A Preset's identity is its **filename stem**. Its description is read from
**inside the file**. Adding one is dropping a `.toml` into a directory: no Rust,
no manifest, no rebuild for the two layers that are on disk.

A nearer Preset **shadows** a farther one of the same name — whole-file
shadowing, not merging. `flow presets` shows every Preset with the layer it came
from, and says so when one shadows another, so inherited flows are never spooky.

The load-bearing part: **`flow` never writes to `.flow/presets/` or
`~/.config/flow/presets/`.** Both are read-only to the tool. The only file
`flow init` creates from a Preset is the repo's own `.flow/flow.toml`. That is
what makes "flow never overrides your presets" structural rather than careful —
there is no merge logic to get wrong, because there is no write. See ADR-0008.

## User Stories

**Authoring a Preset**

1. As someone with a process of my own, I want to drop a `.toml` into
   `~/.config/flow/presets/` and have it appear in `flow presets`, so that I do
   not have to fork and rebuild the tool to have my own flow.
2. As someone with a process of my own, I want my Preset to be selectable as
   `flow init --preset <stem>`, so that I never have to remember a file path.
3. As someone with a process of my own, I want to set my Preset as the default a
   bare `flow init` writes, so that the flow I actually use is the one I get.
4. As a Preset author, I want the description shown in `flow presets` to come
   from inside the file, so that editing the description is a one-file change
   that cannot drift.
5. As a Preset author, I want the Preset's name to be its filename, so that
   renaming it is renaming a file and there is no second place to update.
6. As a Preset author, I want a typo in my Preset to be reported rather than
   silently ignored, so that I am not left wondering why my flow does not show
   up.
7. As a Preset author, I want one bad file in my presets directory to not stop
   `flow init` working, so that a half-finished flow I am drafting does not
   break the tool for the repo I am actually initialising.

**Standardising across a team**

8. As a monorepo maintainer, I want to put Presets in `.flow/presets/` at the
   repo root and have every package below it see them, so that the whole
   repository reaches for the same set of flows.
9. As a monorepo maintainer, I want those Presets committed alongside the code,
   so that a new clone gets the team's flows with no setup step.
10. As a developer in a package that already has its own `.flow/`, I want to
    still see the Presets from the repo root, so that running `flow init` once
    does not cut me off from the menu the repo publishes.
11. As a developer in a package with its own `.flow/presets/`, I want a Preset
    there to win over one of the same name at the repo root, so that a package
    can deviate deliberately.
12. As a monorepo maintainer, I want a project Preset to beat a personal one of
    the same name, so that the repo's process is what its contributors get by
    default.

**Trusting the tool with my files**

13. As someone with my own Presets, I want `flow` to never write into my presets
    directories, so that no upgrade, first run, or `init` can ever clobber
    something I wrote.
14. As someone with my own Presets, I want the shipped ones to stay inside the
    binary, so that there is no installed copy on disk to go stale or be
    silently replaced.
15. As someone upgrading `flow`, I want my Presets to be untouched by the
    upgrade, so that upgrading is never a thing I have to think about.

**Seeing what is available**

16. As a user running `flow presets`, I want each Preset labelled with the layer
    it came from — project, user, or shipped — so that I know who owns it and
    where to go to change it.
17. As a user running `flow presets`, I want to see when one Preset shadows
    another of the same name, so that I do not lose an afternoon to a flow I do
    not recognise.
18. As a user running `flow presets`, I want the shadowed Preset to still be
    visible rather than hidden, so that I can tell what I am overriding.
19. As a user running `flow presets`, I want to see which Preset a bare
    `flow init` will write, so that I can predict what happens before it
    happens.
20. As a user running `flow presets`, I want files that could not be read listed
    with the reason, so that a malformed Preset tells me what is wrong instead
    of just being absent.
21. As a user with unrelated files in a presets directory, I want non-`.toml`
    files ignored without comment, so that a README or an editor swapfile is not
    reported as a problem.
22. As a user running `flow config`, I want the preset directories printed
    alongside the paths it already shows, so that "where do I put my own flow"
    is answered by the command whose job is answering "where do I set this up".
23. As a user running `flow config`, I want to be told when a preset directory
    does not exist yet, so that I know the exact path to create.

**Initialising a repo**

24. As a user running a bare `flow init`, I want `main-flow` written, so that the
    default is a named flow rather than whatever happens to sort first.
25. As a user who wrote my own `main-flow`, I want a bare `flow init` to write
    *mine*, so that shadowing works for the default too and not just for
    explicit choices.
26. As a user running `flow init --preset <name>`, I want the nearest Preset of
    that name on the Preset Path, so that precedence is one rule everywhere.
27. As a user running `flow init`, I want to be told which layer — and for a
    project Preset, which ancestor directory — the flow came from, so that
    inheritance from a parent directory is never invisible.
28. As a user running `flow init --preset ./some/file.toml`, I want a path to
    still work exactly as it does today, so that a one-off flow needs no
    installation.
29. As a user whose configured default Preset no longer exists, I want a hard
    error naming what *is* available, so that I am never silently handed a flow
    I did not ask for.
30. As a user who typo'd `--preset`, I want the error to list the Presets that
    exist across all three layers, so that the fix is in front of me.
31. As a user re-running `flow init` in a repo that already has a flow, I want my
    `.flow/flow.toml` left alone exactly as today, so that the repo keeps owning
    its copy.

**Maintaining `flow` itself**

32. As a flow maintainer, I want to add a shipped Preset by adding a file to
    `presets/`, so that the change is one file and not three.
33. As a flow maintainer, I want the build to fail if a shipped Preset's filename
    and declared name disagree, so that the rule is enforced before anything
    ships rather than discovered by a user.
34. As a flow maintainer, I want the end-to-end test that exercises every shipped
    Preset to iterate the discovered set, so that a new file in `presets/` is
    covered automatically and cannot ship untested.
35. As a flow maintainer, I want the README to point at `flow presets` instead of
    listing them by hand, so that the table cannot drift out of date again.
36. As a flow maintainer, I want no new crate dependency for embedding, so that
    the dependency count stays where ADR-0005 left it.

## Implementation Decisions

Every decision below was settled during the grill stage; the reasoning and the
rejected alternatives are in ADR-0008 and in the run's grill notes.

### The Preset Path

- **Three layers, in precedence order: project → user → shipped.** This mirrors
  the ordering `Settings::resolve` already uses to layer agents (ADR-0007:
  settings are layered by who owns them).
- **Project layer unions every ancestor**, nearest first — it does *not* reuse
  the existing nearest-`.flow` root search. That search stops at the first
  ancestor with a `.flow` directory, which would mean a package that has already
  run `init` never sees the repo root's menu — gutting the only case the project
  layer exists for. The walk starts at the working directory (or `--root`) and
  visits it and every ancestor, collecting each `.flow/presets/` it finds.
- **Nested `.flow` directories are supported and deliberate.** `init` writes to
  the working directory; the Preset walk reads from every ancestor.
- **User layer** resolves its base the same way the user config path already
  does: `$XDG_CONFIG_HOME`, falling back to `$HOME/.config`.
- **Shipped layer** is embedded at build time from `presets/` at the repo root.
  The three existing preset files move out of `assets/`, which keeps the adapter
  skill, board template, and starter user config.

### Identity, description, and shadowing

- **A Preset's id is its filename stem**; its description is the `description`
  field already declared inside the flow file. The name/description tuple array
  in the init command is deleted outright.
- **Filename stem must equal the flow's declared `name`.** Two mechanisms, one
  rule: for shipped Presets this is enforced at build time and fails the build;
  for on-disk Presets it is a skip reason at runtime.
- **Override means whole-file shadowing.** No `extends`, no stage-level merge,
  no patching. Considered and rejected in writing — see ADR-0008.
- **A shadowed Preset stays visible** in `flow presets`, with the listing saying
  what it shadowed.

### Discovery module

- A new module owns the Preset Path. Its job is to produce, from a starting
  directory, the resolved set of Presets and the list of files it declined —
  each with its layer, and each shadowing entry knowing what it shadowed. It
  also resolves a single name to its contents.
- The returned shape distinguishes three things a caller needs: the Presets that
  resolved, the entries that were shadowed, and the files that were skipped with
  a reason. `flow presets` renders all three; `flow init` needs only the first.
- **A Preset is skipped, never fatal**, when: the file is not valid TOML, does
  not parse as a flow, declares no stages, or its `name` does not match its
  filename stem. Each skip carries a human-readable reason. Files without a
  `.toml` extension are ignored silently. An unreadable or absent presets
  directory is not an error.

### Build-time embedding

- **A `build.rs` walks `presets/` and generates the embedded array.** No new
  dependency (the `include_dir` crate was considered and rejected: fewer lines,
  but a sixth dependency and no build-time validation).
- The generated set is what both the runtime and the end-to-end test iterate, so
  a new file in `presets/` is picked up by both without further edits.
- The build script re-runs when `presets/` changes.
- **The stem-equals-`name` rule lives in exactly one place** — a small source
  file that the build script textually includes and that the crate also compiles
  as a module. Same rule, two severities: fatal during the build, a skip reason
  at runtime. This is the one new seam (see Testing Decisions).

### Command surfaces

- **`flow init`**: a bare invocation resolves the pinned name `main-flow`
  through the Preset Path — never a positional first-entry lookup — so a
  project or user Preset called `main-flow` is what gets written. An explicit
  `--preset <name>` resolves through the Preset Path. An explicit
  `--preset <path>` that names an existing file is read verbatim, exactly as
  today; a path is not a discovered Preset and the stem rule does not apply to
  it. `init` reports the layer it drew from, naming the ancestor directory for a
  project Preset.
- **A configured default that resolves to nothing is a hard error** naming what
  is available. Never a silent fallback, never a warning-then-proceed: writing a
  flow the user did not ask for into a file they are then told they own is the
  worst available outcome.
- **`flow presets`**: takes a root. The command currently takes no arguments
  while the dispatcher computes a root for it and throws it away; that
  signature has to change before any project layer can be visible. Output gains
  a source column per row (project / user / shipped), a shadow note, and a
  trailing section listing skipped files with reasons when there are any. The
  `*` default marker and the "change it with `preset = ...`" footer stay.
- **`flow config`**: prints the preset directories alongside the paths it
  already prints, grouped under the same yours/project headings, marking any
  that do not exist — it is the "where do I set this up" command (ADR-0007).
- **No `--force` / re-init / overwrite path.** Considered and left out
  deliberately; reopen as its own piece of work if wanted.
- **A Preset stays exactly one flow file.** No bundles of flow + adapter + seed
  notes — that is different work.

### Domain and docs

- `CONTEXT.md` gains **Preset Path** and redefines **Preset**: it is no longer
  "shipped inside the binary", which is now true of only one of three sources.
  Both edits are already written.
- **ADR-0008, "Presets are discovered, never written"**, is already written and
  covers precedence as a paragraph rather than as a sibling document.
- The README's hand-maintained table of Presets is replaced by a pointer to
  `flow presets`, because it duplicates descriptions that now live in the files
  and will drift again otherwise.

## Testing Decisions

### What makes a good test here

The existing suite states its own rule at the top of the file and it holds for
this work: every test drives the compiled binary against a scratch directory and
asserts on what a user can see — exit code, stdout, and the files left on disk.
Nothing reaches into the crate's internals. State that has to cross a process
boundary is deliberately written by one invocation and read back by another.

Assert on observable behaviour, not on layout: that `flow presets` names the
layer a Preset came from, not the exact column width; that a shadowed Preset is
still listed and marked, not the precise wording of the marker.

### Seams

**Two seams, one of them new and deliberately tiny.**

1. **The CLI** — everything about discovery. Precedence, ancestor unioning,
   shadowing, skip reasons, `init`'s choice and its source line, the `presets`
   listing, the `config` output, and the hard error for a missing configured
   default are all visible through `flow presets`, `flow init`, and
   `flow config`. No new seam is needed for any of it, and none should be
   added: the discovery module is not tested directly.
2. **The stem-equals-`name` validation function** — unit-tested directly,
   because a build failure cannot be reached through the CLI. This is the shared
   rule the build script and the runtime both call, so testing it once covers
   both severities.

The test harness already isolates `XDG_CONFIG_HOME` per test, which gives the
user layer for free. The project layer is reachable by creating nested
directories inside a temp dir and pointing `--root` at the innermost.

### Coverage to add (CLI seam)

- A user Preset appears in `flow presets`, is selectable by stem, and is written
  by `flow init --preset <stem>`.
- A project Preset in `.flow/presets/` appears and is selectable.
- A project Preset in an *ancestor* directory appears from a nested package,
  including from a package that already has its own `.flow/` — the case the
  nearest-root walk would have broken.
- Precedence: project beats user beats shipped for the same name; nearest
  ancestor beats farther.
- A shadowed Preset is still listed, and the listing says what shadowed it.
- A bare `flow init` writes `main-flow`; with a user Preset named `main-flow`
  present, it writes that one instead.
- `flow init` names the layer, and for a project Preset the ancestor directory,
  it drew from.
- A malformed Preset is listed as skipped with a reason and does not fail the
  command; a repo in the same tree still initialises.
- A Preset whose stem and `name` disagree is skipped with a reason.
- A non-`.toml` file in a presets directory produces no output at all.
- A configured default that resolves to nothing is a hard error listing what
  exists. `an_unknown_preset_is_refused` already pins this shape for the argv
  path; the config path needs the same coverage.
- `flow config` prints the preset directories, including ones that do not exist.
- The existing "every built-in preset actually works" end-to-end test iterates
  the build-generated set instead of a hardcoded array of three names.

### Prior art

The whole `--- choosing a flow ---` block near the end of the CLI test file is
the direct model: `presets_lists_the_built_in_flows`, `a_named_preset_is_written_out`,
`every_built_in_preset_actually_works`, `a_preset_can_be_a_file_you_wrote`,
`the_user_config_can_change_which_flow_init_writes`, and
`an_unknown_preset_lists_the_ones_that_exist`. The last two already demonstrate
writing into the isolated `XDG_CONFIG_HOME` and asserting on stderr; the new
user-layer tests extend exactly that pattern. `a_preset_can_be_a_file_you_wrote`
must keep passing unchanged — the path escape hatch is not being altered.

## Out of Scope

- **`extends` / stage-level merging of Presets.** Considered and rejected in
  ADR-0008. Whole-file shadowing is the whole of override.
- **Preset bundles** — a Preset that carries an adapter, seed notes, or anything
  beyond one flow file. Different work.
- **A `--force` or re-init path** that swaps an existing `.flow/flow.toml`. Left
  out deliberately; `init` stays additive and idempotent (ADR-0004).
- **Installing shipped Presets to disk** on first run or upgrade. This is the
  design ADR-0008 exists to reject.
- **A `presets/index.toml` manifest.** It would reintroduce exactly the
  hardcoded list being removed.
- **Fetching Presets from a URL or registry**, or any network access.
- **Changing the flow file format**, stage semantics, or anything about how a
  flow behaves after `init` has written it. Only *where flow definitions come
  from* changes.
- **Deleting or migrating anyone's existing `.flow/flow.toml`.** Repos that have
  already run `init` are unaffected.

## Further Notes

- The four consequences flagged at the end of the grill are all folded in above:
  the `presets()` signature, the source column, `flow config` printing the
  directories, and the README table.
- The description drift between the Rust table and `assets/main-flow.toml` is
  the concrete evidence for the single-source-of-truth decision. Whichever
  wording survives, it survives in the file.
- Two mechanisms enforce one rule at two severities — build-time fatal for
  shipped Presets, runtime skip for on-disk ones. That asymmetry is intentional:
  we control what ships, so a mismatch there is a bug we should never publish;
  a user's directory is theirs, and one bad file in it must not stop them
  initialising an unrelated repo.
- `find_root` keeps its current behaviour and its current callers. The Preset
  walk is a second, different traversal of the same ancestry, and the difference
  between them is worth a comment where it is written.
