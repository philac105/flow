+++
slug = "flow-folder"
title = "flow-folder"
kind = "feature"
flow = "main-flow"
status = "finished"
created = "2026-08-21T18:07:25.494Z"
updated = "2026-08-21T18:38:11.282Z"

[[stage]]
name = "grill"
status = "done"
artifact = ".flow/artifacts/flow-folder/grill.md"
started = "2026-08-21T18:07:25.494Z"
completed = "2026-08-21T18:34:28.347Z"

[[stage]]
name = "spec"
status = "in_progress"
started = "2026-08-21T18:34:28.347Z"

[[stage]]
name = "tickets"
status = "pending"

[[stage]]
name = "implement"
status = "pending"

[[stage]]
name = "review"
status = "pending"
+++

## Where we are

Run finished.

Built in full during the grill session rather than walking the remaining stages — spec, tickets, implement and review were skipped by decision, not left undone.

Shipped: all three presets now write artifacts to .flow/artifacts/{slug}/<stage>.md, so everything Flow writes lives under one folder (main-flow used .scratch/, minimal and bugfix used .flow/notes/). The .flow/.gitignore that init writes drops the generated /board.html and presents /runs/ and /artifacts/ commented out, so each user decides; init now keeps an existing .flow/.gitignore instead of overwriting it, since two of its lines are an invitation to edit. This repo migrated: 13 files git mv'd out of .scratch/, the recorded artifact paths in three run files hand-corrected to match, board.html untracked. flow status --all reports no drift.

Not built, by decision: no archive directory, no flow delete, no flow prune, and no way to pull preset improvements into an existing repo — flow update and flow presets --show were both proposed and declined. ADR-0009 records why, and .flow/artifacts/flow-folder/grill.md has the full decision trail including the declined gap.

117 tests pass, fmt and clippy clean.

## Log

### 2026-08-21T18:07:25Z — Started. First stage is `grill`.

finished runs should maybe get deleted. let's talk about that
flow.toml maybe shouldnt be a full file, but a link to either one of our presets OR a user preset instead of a full .toml.
i feel like we should ignore the runs/ folder, it's user based and not app based

Interview until the plan is sharp. Produces decisions, a glossary and ADRs.

Run `/grill-with-docs` next.

### 2026-08-21T18:34:28Z — `grill` done → `spec`

Grill is done: 18 questions over five rounds, all answered, decisions in .scratch/flow-folder/grill.md and ADR-0009.

The shape of it: stage artifacts move to .flow/artifacts/{slug}/<stage>.md in all three shipped presets (they disagreed — main-flow used .scratch/, minimal and bugfix used .flow/notes/), so everything Flow writes is under one folder. Runs stay committed by default and .flow/.gitignore gains commented-out /runs/ and /artifacts/ blocks so each user decides; /board.html is ignored outright as generated output. No archiving, no flow delete, no flow prune — archiving was recommended and then withdrawn once it became clear that moving artifacts means rewriting the recorded artifact path, which is the evidence drift checks against. That rule constrains the tool, not the person: this repo's own .scratch/ files get git mv'd and their recorded paths hand-edited as part of the work.

flow.toml stays a full self-contained copy — no link to a preset, no provenance, no update or diff command. Adds no new command and no new type: init.rs gets the gitignore template plus a write-if-absent guard, the three presets get an artifact path, and this repo gets a one-time migration.

Known gap, seen and declined rather than missed: there is still no way to pull preset improvements into an existing repo, or even to read a shipped preset (they live in the binary). flow update and flow presets --show were both proposed and both turned down as not needed yet. The grill artifact records why and what the cheap fix would be.

Next stage is spec.

### 2026-08-21T18:38:11Z — Run finished.

Built in full during the grill session rather than walking the remaining stages — spec, tickets, implement and review were skipped by decision, not left undone.

Shipped: all three presets now write artifacts to .flow/artifacts/{slug}/<stage>.md, so everything Flow writes lives under one folder (main-flow used .scratch/, minimal and bugfix used .flow/notes/). The .flow/.gitignore that init writes drops the generated /board.html and presents /runs/ and /artifacts/ commented out, so each user decides; init now keeps an existing .flow/.gitignore instead of overwriting it, since two of its lines are an invitation to edit. This repo migrated: 13 files git mv'd out of .scratch/, the recorded artifact paths in three run files hand-corrected to match, board.html untracked. flow status --all reports no drift.

Not built, by decision: no archive directory, no flow delete, no flow prune, and no way to pull preset improvements into an existing repo — flow update and flow presets --show were both proposed and declined. ADR-0009 records why, and .flow/artifacts/flow-folder/grill.md has the full decision trail including the declined gap.

117 tests pass, fmt and clippy clean.
