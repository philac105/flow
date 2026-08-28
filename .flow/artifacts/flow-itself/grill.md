# Grill: what is `flow`?

Started 2026-08-20. Step: grill-with-docs (main flow, step 1/5).

## Premise

Matt Pocock's skills already persist to disk. The three gaps flow fills:

1. No status layer - nothing records which step a run is on.
2. No resume protocol - a fresh session can't answer "where were we, what's next".
3. No cross-run view.

## A. Design decisions ABOUT THE TOOL

- D1 Shape: file convention + a read-only CLI over the files. No server, no DB.
- D2 Agnostic on THREE axes: agent-agnostic, tracker-agnostic, flow-agnostic.
     The flow itself is DATA. Matt's main flow is the shipped default preset,
     not the only one. NOT domain-agnostic (v1 targets software work).
- D3 State lives in the TARGET repo (.flow/), git-versioned.
- D4 Wrap, never fork. Flow owns the state machine + resume, never the thinking.
     Stages bind to ANY command/skill.
- D6 Flow is LINEAR, with skippable and repeatable stages (review -> implement
     kickback is real). Branching lives inside a stage, not in the flow.
- D7 The CLI NEVER invokes the agent. Pure read-model: it prints what to run
     next. A thin per-agent adapter does the running. This is what buys
     agent-agnosticism for free.
- D8 Completion = artifact evidence AND explicit mark. Each stage declares an
     expected artifact, so a crashed session is recoverable from disk alone.
     Explicit-only state lies after a crash; artifact-only can't express
     "started".
- D9 Stage position lives in FILES, never behind a network call. This is the
     "pick up where I left off, always" requirement - it belongs to the TOOL's
     design. (Not a statement about this repo's own tracker.)
- D10 The unit moving through a flow is a RUN, with a free-text `kind`
     (feature/bug/task/project). Not "feature".
- D11 Stack: RUST. Note: distribution is not free the way npx was - needs
     cargo install + prebuilt releases. `flow` is likely taken on crates.io;
     verify at implement time, budget for a different crate name w/ `flow`
     binary alias.
- D12 Render: BOTH. `flow status` terminal table (the 90% case) + `flow board`
     writing a self-contained HTML card grid. Not a Claude Artifact - that
     needs a session, and the point is outliving sessions.

## B. Config decisions about THIS repo

- Issue tracker: GitHub Issues (philac105/flow, gh authed). User: "don't care".

## Round 3 - asked, awaiting answers

- Q13 Run file: one file vs a directory. Rec: ONE file (frontmatter + log +
      handoff). Greppable, diffable, one keystroke to open.
- Q14 Resume payload: state / +append-only stage log / +always-current
      "Where we are" block. Rec: all three. Log is history and never lies;
      block is the 20 lines a cold agent reads first. Wrap Matt's /handoff
      for the prose rather than inventing a format.
- Q15 Flow def format. Rec: TOML at .flow/flow.toml. Presets ship in the binary
      but `flow init --preset` WRITES THE TOML OUT, so the default is editable
      without forking the tool. This is what makes "any flow" real.
- Q16 Stage fields: name, description, command, artifact, repeatable, optional.
      Open part: command as one string vs per-agent map. Rec: one string +
      optional per-agent override table.
- Q17 v1 scope: one repo vs cross-repo registry (~/.config/flow/repos.toml).
      Rec: one repo. Registry designed-for, not built. Build what you'll use
      daily, not what demos well.
- Q18 Does `flow init` scaffold the agent adapter (.claude/skills/flow/SKILL.md
      + AGENTS.md block)? Rec: YES. Without it, "the agent updates state" is
      something you must remember to say every session - the exact failure mode
      being escaped.

## Next after grill

- Write CONTEXT.md (glossary: Flow, Stage, Run, Artifact, Handoff, Adapter)
  + any ADRs. Then /to-spec.
