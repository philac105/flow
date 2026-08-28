//! Every test drives the compiled `flow` binary against a scratch directory and
//! asserts on what a user can see: the exit code, stdout, and the files left on
//! disk. Nothing reaches into the crate's internals — the CLI is the only seam.
//!
//! State crossing a *process boundary* is the property the whole tool rests on,
//! so tests that need state deliberately spend one invocation writing it and
//! another reading it back.

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn flow(dir: &Path) -> Command {
    flow_from(dir, dir)
}

/// `flow` run from `root`, with the user layer isolated under `owner`. The two
/// come apart only when a test runs from a nested package and still wants the
/// outer directory's `xdg` to be the machine's config.
fn flow_from(root: &Path, owner: &Path) -> Command {
    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.arg("--root").arg(root);
    // Isolate user config per test: nothing here may read the real ~/.config.
    cmd.env("XDG_CONFIG_HOME", owner.join("xdg"));
    // `owner` stands in for the machine's home directory too, which is what
    // bounds the project walk — without it the walk would stop at `root` and a
    // test that puts a preset in an ancestor would have nowhere to put it.
    cmd.env("HOME", owner);
    cmd
}

/// A preset on disk, in whichever presets directory the caller names.
fn write_preset(dir: &Path, name: &str, description: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "name = \"{name}\"\ndescription = \"{description}\"\n\n\
             [[stage]]\nname = \"do\"\ncommand = \"/do\"\n"
        ),
    )
    .unwrap();
}

/// An initialised repo with no runs.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    flow(dir.path()).arg("init").assert().success();
    dir
}

/// An initialised repo with one run on its first stage.
fn repo_with_run() -> TempDir {
    let dir = repo();
    flow(dir.path())
        .args([
            "start",
            "Auth rework",
            "--kind",
            "feature",
            "-m",
            "Sessions outlive their tokens; rework auth so they do not.",
        ])
        .assert()
        .success();
    dir
}

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

fn stdout(cmd: &mut Command) -> String {
    String::from_utf8(cmd.assert().success().get_output().stdout.clone()).unwrap()
}

// --- ticket 01: init writes the preset -------------------------------------

#[test]
fn init_writes_the_flow_and_runs_directory() {
    let dir = TempDir::new().unwrap();
    flow(dir.path()).arg("init").assert().success();

    assert!(dir.path().join(".flow/flow.toml").is_file());
    assert!(dir.path().join(".flow/runs").is_dir());
}

#[test]
fn the_default_preset_has_the_five_main_flow_stages() {
    let dir = repo();
    let toml = read(dir.path(), ".flow/flow.toml");
    for stage in ["grill", "spec", "tickets", "implement", "review"] {
        assert!(
            toml.contains(&format!("name = \"{stage}\"")),
            "missing {stage}"
        );
    }
    // Every stage must carry the fields `flow next` reports.
    assert!(toml.contains("description ="));
    assert!(toml.contains("command ="));
    assert!(toml.contains("artifact ="));
}

#[test]
fn init_is_idempotent_and_never_clobbers_an_edited_flow() {
    let dir = repo();
    let edited = "name = \"mine\"\n\n[[stage]]\nname = \"think\"\ncommand = \"/think\"\n";
    std::fs::write(dir.path().join(".flow/flow.toml"), edited).unwrap();

    flow(dir.path()).arg("init").assert().success();

    assert_eq!(read(dir.path(), ".flow/flow.toml"), edited);
}

#[test]
fn an_unknown_preset_is_refused() {
    let dir = TempDir::new().unwrap();
    flow(dir.path())
        .args(["init", "--preset", "nonsense"])
        .assert()
        .failure();
}

#[test]
fn commands_refuse_to_run_before_init() {
    let dir = TempDir::new().unwrap();
    flow(dir.path()).arg("status").assert().failure();
}

// --- ticket 02: a run round-trips across processes -------------------------

#[test]
fn a_run_round_trips_through_a_separate_process() {
    let dir = repo_with_run();
    assert!(dir.path().join(".flow/runs/auth-rework.md").is_file());

    // A second process, reading only what the first left on disk.
    let out = stdout(flow(dir.path()).args(["show", "auth-rework"]));
    assert!(out.contains("Auth rework"));
    assert!(out.contains("auth-rework"));
    assert!(out.contains("feature"));
}

#[test]
fn the_first_stage_is_in_flight_and_the_rest_are_pending() {
    let dir = repo_with_run();
    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    assert!(file.contains("status = \"in_progress\""));
    assert_eq!(file.matches("status = \"in_progress\"").count(), 1);
    assert_eq!(file.matches("status = \"pending\"").count(), 4);
}

#[test]
fn kind_is_free_text_and_optional() {
    let dir = repo();
    flow(dir.path())
        .args(["start", "Odd job", "--kind", "spike/experiment"])
        .assert()
        .success();
    flow(dir.path())
        .args(["start", "No kind"])
        .assert()
        .success();

    assert!(read(dir.path(), ".flow/runs/odd-job.md").contains("spike/experiment"));
    assert!(dir.path().join(".flow/runs/no-kind.md").is_file());
}

#[test]
fn starting_a_colliding_run_is_refused_and_changes_nothing() {
    let dir = repo_with_run();
    let before = read(dir.path(), ".flow/runs/auth-rework.md");

    flow(dir.path())
        .args(["start", "auth   rework"])
        .assert()
        .failure();

    assert_eq!(read(dir.path(), ".flow/runs/auth-rework.md"), before);
}

#[test]
fn showing_an_unknown_run_fails_clearly() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["show", "nope"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("nope"));
}

// --- ticket 03: next, done, handoff and log --------------------------------

#[test]
fn next_prints_the_stage_and_its_command_without_running_it() {
    let dir = repo_with_run();
    let out = stdout(flow(dir.path()).arg("next"));

    assert!(out.contains("grill"));
    assert!(out.contains("/grill-with-docs"));
    // The command is shown, never executed: nothing it would produce appears.
    assert!(!dir.path().join(".flow/artifacts").exists());
}

#[test]
fn done_advances_to_the_next_stage() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "Decisions settled."])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("spec"));
    assert!(out.contains("/to-spec"));
}

#[test]
fn the_handoff_is_replaced_while_the_log_only_grows() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "FIRST NOTE"])
        .assert()
        .success();
    flow(dir.path())
        .args(["done", "-m", "SECOND NOTE"])
        .assert()
        .success();

    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    let (_, body) = file.split_once("## Where we are").unwrap();
    let (handoff, log) = body.split_once("## Log").unwrap();

    // Exactly one handoff block, holding only the latest note.
    assert_eq!(file.matches("## Where we are").count(), 1);
    assert!(handoff.contains("SECOND NOTE"));
    assert!(!handoff.contains("FIRST NOTE"));

    // The log keeps everything, including the entry written at `start`.
    assert!(log.contains("FIRST NOTE"));
    assert!(log.contains("SECOND NOTE"));
    assert!(log.contains("Started."));
}

#[test]
fn log_entries_carry_the_stage_name() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "note"])
        .assert()
        .success();

    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    let log = file.split("## Log").nth(1).unwrap();
    assert!(log.contains("`grill` done"));
    assert!(log.contains("spec"));
}

#[test]
fn next_and_done_need_no_slug_when_one_run_is_active() {
    let dir = repo_with_run();
    flow(dir.path()).arg("next").assert().success();
    flow(dir.path()).arg("done").assert().success();
}

#[test]
fn several_active_runs_are_disambiguated_by_the_pointer_or_by_name() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["start", "Other thing"])
        .assert()
        .success();

    // Starting `other-thing` made it current, so bare commands mean that one.
    assert!(stdout(flow(dir.path()).arg("next")).contains("Other thing"));
    // Naming a run always wins over the pointer.
    assert!(stdout(flow(dir.path()).args(["next", "auth-rework"])).contains("Auth rework"));
}

#[test]
fn a_run_past_its_last_stage_says_so() {
    let dir = repo_with_run();
    for _ in 0..5 {
        flow(dir.path())
            .args(["done", "-m", "on"])
            .assert()
            .success();
    }
    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("flow finish"));

    flow(dir.path())
        .args(["done", "-m", "again"])
        .assert()
        .failure();
}

// --- ticket 04: the board --------------------------------------------------

#[test]
fn status_lists_every_run_with_its_stage_and_progress() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["start", "Billing fix", "--kind", "bug"])
        .assert()
        .success();
    flow(dir.path())
        .args(["done", "auth-rework", "-m", "note"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("status"));
    assert!(out.contains("RUN") && out.contains("KIND") && out.contains("STAGE"));
    assert!(out.contains("auth-rework"));
    assert!(out.contains("billing-fix"));
    assert!(out.contains("bug"));
    assert!(out.contains("1/5"));
    assert!(out.contains("0/5"));
}

#[test]
fn status_is_ordered_most_recently_updated_first() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["start", "Billing fix"])
        .assert()
        .success();
    flow(dir.path())
        .args(["done", "auth-rework", "-m", "touched last"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("status"));
    let auth = out.find("auth-rework").unwrap();
    let billing = out.find("billing-fix").unwrap();
    assert!(
        auth < billing,
        "most recently updated run should lead:\n{out}"
    );
}

#[test]
fn status_on_an_empty_repo_is_helpful_and_succeeds() {
    let dir = repo();
    let out = stdout(flow(dir.path()).arg("status"));
    assert!(out.contains("flow start"));
}

#[test]
fn finished_runs_are_hidden_until_asked_for() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["finish", "-m", "shipped"])
        .assert()
        .success();

    assert!(!stdout(flow(dir.path()).arg("status")).contains("auth-rework"));
    assert!(stdout(flow(dir.path()).args(["status", "--all"])).contains("auth-rework"));
}

// --- ticket 05: artifact evidence and drift --------------------------------

#[test]
fn a_stage_whose_artifact_exists_while_it_reads_pending_is_drift() {
    let dir = repo_with_run();
    // The grill stage declares `.flow/artifacts/{slug}/grill.md`. A session that died
    // after writing it but before recording leaves exactly this shape.
    std::fs::create_dir_all(dir.path().join(".flow/artifacts/auth-rework")).unwrap();
    std::fs::write(
        dir.path().join(".flow/artifacts/auth-rework/grill.md"),
        "notes",
    )
    .unwrap();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("drift"), "expected drift in:\n{out}");
    assert!(out.contains("grill"));

    assert!(stdout(flow(dir.path()).arg("status")).contains("drift"));
    assert!(stdout(flow(dir.path()).args(["show", "auth-rework"])).contains("drift"));
}

#[test]
fn a_done_stage_whose_artifact_vanished_is_drift() {
    let dir = repo_with_run();
    std::fs::write(dir.path().join("spec.md"), "the spec").unwrap();
    flow(dir.path())
        .args(["done", "-m", "note", "--artifact", "spec.md"])
        .assert()
        .success();
    assert!(!stdout(flow(dir.path()).arg("status")).contains("drift"));

    std::fs::remove_file(dir.path().join("spec.md")).unwrap();

    let out = stdout(flow(dir.path()).arg("status"));
    assert!(out.contains("drift"), "expected drift in:\n{out}");
    assert!(out.contains("missing"));
}

#[test]
fn drift_is_reported_and_never_silently_corrected() {
    let dir = repo_with_run();
    std::fs::create_dir_all(dir.path().join(".flow/artifacts/auth-rework")).unwrap();
    std::fs::write(
        dir.path().join(".flow/artifacts/auth-rework/grill.md"),
        "notes",
    )
    .unwrap();

    flow(dir.path()).arg("status").assert().success();
    flow(dir.path()).arg("next").assert().success();

    // The recorded position is left exactly as it was.
    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    assert_eq!(file.matches("status = \"pending\"").count(), 4);
    assert!(file.contains("status = \"in_progress\""));
}

#[test]
fn tracker_artifacts_are_never_checked_against_the_filesystem() {
    let dir = repo_with_run();
    // Satisfy grill's file artifact so the only artifact left in play is
    // spec's `tracker:issue`, which lives somewhere no filesystem can see.
    std::fs::create_dir_all(dir.path().join(".flow/artifacts/auth-rework")).unwrap();
    std::fs::write(
        dir.path().join(".flow/artifacts/auth-rework/grill.md"),
        "notes",
    )
    .unwrap();
    flow(dir.path())
        .args(["done", "-m", "note"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("status"));
    assert!(out.contains("spec"));
    assert!(
        !out.contains("drift"),
        "tracker artifacts must not drift:\n{out}"
    );
}

#[test]
fn a_stage_completed_without_its_declared_artifact_is_drift() {
    // The flow says grill leaves a file behind. Saying it is done when no such
    // file exists is exactly the kind of quiet lie drift is for.
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "note"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("status"));
    assert!(out.contains("drift"));
    assert!(out.contains("grill"));
}

#[test]
fn the_artifact_template_resolves_the_run_slug() {
    let dir = repo_with_run();
    let out = stdout(flow(dir.path()).arg("next"));
    assert!(
        out.contains(".flow/artifacts/auth-rework/grill.md"),
        "got:\n{out}"
    );
}

// --- ticket 06: skip, back and finish --------------------------------------

#[test]
fn an_optional_stage_can_be_skipped() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "n"])
        .assert()
        .success(); // grill
    flow(dir.path())
        .args(["done", "-m", "n"])
        .assert()
        .success(); // spec

    // `tickets` is the only optional stage in the preset.
    flow(dir.path())
        .args(["skip", "-m", "one-liner"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("implement"));
    assert!(read(dir.path(), ".flow/runs/auth-rework.md").contains("status = \"skipped\""));
}

#[test]
fn a_required_stage_cannot_be_skipped_and_nothing_changes() {
    let dir = repo_with_run();
    let before = read(dir.path(), ".flow/runs/auth-rework.md");

    flow(dir.path())
        .args(["skip", "-m", "nope"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("optional"));

    assert_eq!(read(dir.path(), ".flow/runs/auth-rework.md"), before);
}

#[test]
fn back_reopens_an_earlier_stage_and_keeps_the_log() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "GRILL NOTE"])
        .assert()
        .success();
    flow(dir.path())
        .args(["done", "-m", "SPEC NOTE"])
        .assert()
        .success();

    flow(dir.path())
        .args(["back", "-m", "spec was wrong"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("spec"));

    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    let log = file.split("## Log").nth(1).unwrap();
    assert!(log.contains("GRILL NOTE"));
    assert!(log.contains("SPEC NOTE"));
    assert!(log.contains("spec was wrong"));
}

#[test]
fn back_can_name_a_stage_and_unsettles_everything_after_it() {
    let dir = repo_with_run();
    for _ in 0..4 {
        flow(dir.path())
            .args(["done", "-m", "n"])
            .assert()
            .success();
    }
    flow(dir.path())
        .args(["back", "--stage", "spec", "-m", "rethink"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("spec"));

    // grill stays done; spec is in flight; tickets, implement, review reset.
    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    assert_eq!(file.matches("status = \"done\"").count(), 1);
    assert_eq!(file.matches("status = \"pending\"").count(), 3);
}

#[test]
fn back_refuses_to_move_forwards() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["back", "--stage", "review", "-m", "no"])
        .assert()
        .failure();
    flow(dir.path())
        .args(["back", "-m", "no"])
        .assert()
        .failure();
}

#[test]
fn every_move_rewrites_the_handoff() {
    let dir = repo_with_run();
    for args in [
        vec!["done", "-m", "AFTER DONE"],
        vec!["back", "-m", "AFTER BACK"],
    ] {
        flow(dir.path()).args(&args).assert().success();
        let file = read(dir.path(), ".flow/runs/auth-rework.md");
        let handoff = file
            .split("## Where we are")
            .nth(1)
            .unwrap()
            .split("## Log")
            .next()
            .unwrap();
        assert!(handoff.contains(args[2]), "handoff missing {}", args[2]);
        assert_eq!(file.matches("## Where we are").count(), 1);
    }
}

// --- ticket 07: the HTML board ---------------------------------------------

#[test]
fn board_writes_a_standalone_html_file() {
    let dir = repo_with_run();
    flow(dir.path()).arg("board").assert().success();

    let html = read(dir.path(), ".flow/board.html");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Auth rework"));
    for stage in ["grill", "spec", "tickets", "implement", "review"] {
        assert!(html.contains(stage), "board missing stage {stage}");
    }
    assert!(html.contains("class=\"current\""));
}

#[test]
fn the_board_reaches_for_nothing_outside_itself() {
    let dir = repo_with_run();
    flow(dir.path()).arg("board").assert().success();
    let html = read(dir.path(), ".flow/board.html");

    for forbidden in ["http://", "https://", "//cdn", "<script"] {
        assert!(!html.contains(forbidden), "board references {forbidden}");
    }
}

#[test]
fn the_board_carries_both_themes() {
    let dir = repo_with_run();
    flow(dir.path()).arg("board").assert().success();
    let html = read(dir.path(), ".flow/board.html");
    assert!(html.contains("prefers-color-scheme: dark"));
    assert!(html.contains(":root {"));
}

#[test]
fn board_output_can_be_redirected() {
    // `-o` resolves the way every CLI does — against the working directory —
    // so the test names the destination outright rather than assuming --root.
    let dir = repo_with_run();
    let dest = dir.path().join("elsewhere/b.html");
    flow(dir.path())
        .arg("board")
        .arg("-o")
        .arg(&dest)
        .assert()
        .success();
    assert!(dest.is_file());
}

#[test]
fn run_titles_are_html_escaped() {
    let dir = repo();
    flow(dir.path())
        .args(["start", "Fix <script>alert(1)</script> & co"])
        .assert()
        .success();
    flow(dir.path()).arg("board").assert().success();

    let html = read(dir.path(), ".flow/board.html");
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&amp; co"));
    assert!(!html.contains("<script>alert"));
}

#[test]
fn the_board_shows_drift() {
    let dir = repo_with_run();
    std::fs::create_dir_all(dir.path().join(".flow/artifacts/auth-rework")).unwrap();
    std::fs::write(
        dir.path().join(".flow/artifacts/auth-rework/grill.md"),
        "notes",
    )
    .unwrap();
    flow(dir.path()).arg("board").assert().success();

    assert!(read(dir.path(), ".flow/board.html").contains("class=\"drift\""));
}

// --- ticket 08: the adapter ------------------------------------------------

#[test]
fn init_writes_the_claude_adapter_with_frontmatter() {
    let dir = repo();
    let skill = read(dir.path(), ".claude/skills/flow/SKILL.md");
    assert!(skill.starts_with("---\n"));
    assert!(skill.contains("name: flow"));
    assert!(skill.contains("description:"));
}

#[test]
fn the_adapter_documents_the_real_command_surface() {
    let dir = repo();
    let skill = read(dir.path(), ".claude/skills/flow/SKILL.md");
    for command in [
        "flow status",
        "flow next",
        "flow done",
        "flow skip",
        "flow back",
        "flow finish",
        "flow reopen",
        "flow start",
    ] {
        assert!(
            skill.contains(command),
            "adapter never mentions `{command}`"
        );
    }
}

#[test]
fn init_creates_agents_md_when_absent() {
    let dir = repo();
    let agents = read(dir.path(), "AGENTS.md");
    assert!(agents.contains("<!-- flow:start -->"));
    assert!(agents.contains("<!-- flow:end -->"));
    assert!(agents.contains("flow next"));
}

#[test]
fn init_preserves_surrounding_agents_md_content_byte_for_byte() {
    let dir = TempDir::new().unwrap();
    let original = "# My repo\n\nHouse rules that matter.\n";
    std::fs::write(dir.path().join("AGENTS.md"), original).unwrap();

    flow(dir.path()).arg("init").assert().success();
    let after_first = read(dir.path(), "AGENTS.md");
    assert!(after_first.starts_with("# My repo\n\nHouse rules that matter."));

    flow(dir.path()).arg("init").assert().success();
    let after_second = read(dir.path(), "AGENTS.md");

    assert_eq!(after_first, after_second, "init must be idempotent");
    assert_eq!(after_second.matches("<!-- flow:start -->").count(), 1);
}

#[test]
fn init_updates_its_block_in_place_without_touching_the_rest() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("AGENTS.md"),
        "# Top\n\n<!-- flow:start -->\nstale text\n<!-- flow:end -->\n\n# Bottom\n",
    )
    .unwrap();

    flow(dir.path()).arg("init").assert().success();

    let agents = read(dir.path(), "AGENTS.md");
    assert!(agents.starts_with("# Top\n"));
    assert!(agents.trim_end().ends_with("# Bottom"));
    assert!(!agents.contains("stale text"));
    assert_eq!(agents.matches("<!-- flow:start -->").count(), 1);
}

// --- the premise ------------------------------------------------------------

#[test]
fn commands_work_from_a_subdirectory_of_the_repo() {
    let dir = repo_with_run();
    let nested = dir.path().join("src/deep/nested");
    std::fs::create_dir_all(&nested).unwrap();

    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.current_dir(&nested);
    let out = String::from_utf8(
        cmd.arg("next")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(out.contains("Auth rework"));
}

#[test]
fn a_run_survives_being_read_by_a_process_that_never_wrote_it() {
    // The whole premise: everything a later session needs is on disk.
    let dir = repo_with_run();
    flow(dir.path())
        .args([
            "done",
            "-m",
            "Grilled. Spec is next; the tracker choice is still open.",
        ])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("spec"));
    assert!(out.contains("/to-spec"));
    assert!(out.contains("the tracker choice is still open"));
}

// --- review: gaps found reviewing the diff ---------------------------------

#[test]
fn deliberately_reopening_a_stage_is_not_reported_as_a_dead_session() {
    let dir = repo_with_run();
    std::fs::create_dir_all(dir.path().join(".flow/artifacts/auth-rework")).unwrap();
    std::fs::write(
        dir.path().join(".flow/artifacts/auth-rework/grill.md"),
        "notes",
    )
    .unwrap();
    flow(dir.path())
        .args([
            "done",
            "-m",
            "grilled",
            "--artifact",
            ".flow/artifacts/auth-rework/grill.md",
        ])
        .assert()
        .success();

    flow(dir.path())
        .args(["back", "--stage", "grill", "-m", "grill was wrong, redoing"])
        .assert()
        .success();

    let out = stdout(flow(dir.path()).arg("status"));
    assert!(
        !out.contains("drift"),
        "a redo is not a dead session:\n{out}"
    );

    // Completing it again clears the reopened mark, so a genuinely dead session
    // in that stage is still caught afterwards.
    flow(dir.path())
        .args(["done", "-m", "regrilled"])
        .assert()
        .success();
    assert!(!read(dir.path(), ".flow/runs/auth-rework.md").contains("reopened"));
}

#[test]
fn a_finished_run_refuses_recording_until_it_is_reopened() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["finish", "-m", "shipped"])
        .assert()
        .success();

    for args in [
        vec!["done", "-m", "more"],
        vec!["skip", "-m", "no"],
        vec!["back", "-m", "no"],
    ] {
        flow(dir.path())
            .args(&args)
            .assert()
            .failure()
            .stderr(predicates::str::contains("flow reopen"));
    }

    flow(dir.path())
        .args(["reopen", "-m", "review found a hole"])
        .assert()
        .success();
    assert!(stdout(flow(dir.path()).arg("status")).contains("auth-rework"));
    flow(dir.path())
        .args(["done", "-m", "now it works"])
        .assert()
        .success();
}

#[test]
fn a_handoff_quoting_the_log_heading_does_not_swallow_the_log() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["done", "-m", "FIRST"])
        .assert()
        .success();
    flow(dir.path())
        .args(["done", "-m", "See the ## Log section below for history"])
        .assert()
        .success();

    // A later process must still find both entries.
    let out = stdout(flow(dir.path()).args(["show", "auth-rework"]));
    assert!(out.contains("FIRST"));
    assert!(out.contains("Started."));
}

#[test]
fn a_stage_can_override_its_command_per_agent() {
    let dir = TempDir::new().unwrap();
    flow(dir.path()).arg("init").assert().success();
    std::fs::write(
        dir.path().join(".flow/flow.toml"),
        "name = \"mine\"\n\n[[stage]]\nname = \"spec\"\ncommand = \"/to-spec\"\n\n\
         [stage.agents]\ncodex = \"read SPEC.md and write the spec\"\n",
    )
    .unwrap();
    flow(dir.path()).args(["start", "Thing"]).assert().success();

    assert!(stdout(flow(dir.path()).arg("next")).contains("/to-spec"));
    assert!(stdout(flow(dir.path()).args(["next", "--agent", "codex"])).contains("read SPEC.md"));
    // An agent with no override falls back to the one command.
    assert!(stdout(flow(dir.path()).args(["next", "--agent", "cursor"])).contains("/to-spec"));
}

// --- handing a stage to an agent -------------------------------------------

/// A stand-in for a real agent, configured where a real one goes — the user's
/// config. `sh -c '<script>' <arg>` binds the first argument to `$0`.
fn with_fake_agent(dir: &Path, script: &str) {
    let path = dir.join("xdg/flow/config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "agent = \"test\"\n\n[agents.test]\ncommand = [\"sh\", \"-c\", \"{script}\", \"{{prompt}}\"]\nguard_env = [\"CLAUDECODE\"]\n"
        ),
    )
    .unwrap();
}

/// A repo overriding the launcher by name. Scalars must precede the stage
/// tables, so this goes at the top of the file.
fn with_repo_agent(dir: &Path, script: &str) {
    let existing = read(dir, ".flow/flow.toml");
    let block = format!(
        "agent = \"test\"\n\n[agents.test]\ncommand = [\"sh\", \"-c\", \"{script}\", \"{{prompt}}\"]\n\n"
    );
    // Once a table opens, every later bare key belongs to it — so this goes
    // after the top-level scalars and before the first stage.
    let at = existing.find("[[stage]]").expect("preset has stages");
    let merged = format!("{}{block}{}", &existing[..at], &existing[at..]);
    std::fs::write(dir.join(".flow/flow.toml"), merged).unwrap();
}

#[test]
fn go_launches_the_configured_agent_with_the_assembled_prompt() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "printf %s \\\"$0\\\" > prompt.txt");

    flow(dir.path())
        .arg("go")
        .env_remove("CLAUDECODE")
        .assert()
        .success();

    let prompt = read(dir.path(), "prompt.txt");
    // The stage's command leads, so it reads as an instruction.
    assert!(prompt.starts_with("/grill-with-docs"));
    assert!(prompt.contains("auth-rework"));
    assert!(prompt.contains("Stage 1 of 5"));
    assert!(prompt.contains("grill"));
    // And it carries the handoff and how to record.
    assert!(prompt.contains("## Where we are"));
    assert!(prompt.contains("flow done auth-rework -m"));
    assert!(prompt.contains("--artifact .flow/artifacts/auth-rework/grill.md"));
}

#[test]
fn the_brief_given_at_start_becomes_the_first_handoff() {
    let dir = repo_with_run();

    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    let (_, body) = file.split_once("## Where we are").unwrap();
    let (handoff, log) = body.split_once("## Log").unwrap();

    assert!(handoff.contains("Sessions outlive their tokens"));
    assert!(log.contains("Sessions outlive their tokens"));

    // And it reaches the agent that picks the first stage up.
    let out = stdout(flow(dir.path()).arg("next"));
    assert!(out.contains("Sessions outlive their tokens"));
}

#[test]
fn nothing_is_asked_for_when_no_one_is_there_to_answer() {
    let dir = repo();
    // Not a terminal, so the missing title is an error rather than a question
    // no agent on the other end could answer.
    flow(dir.path())
        .arg("start")
        .assert()
        .failure()
        .stderr(predicates::str::contains("no title"));
    assert!(!dir.path().join(".flow/current").exists());
}

#[test]
fn a_colliding_run_is_refused_before_anything_is_asked_for() {
    let dir = repo_with_run();
    // The refusal has to come first: a brief typed at a prompt behind it would
    // be typed for nothing.
    flow(dir.path())
        .args(["start", "Auth rework"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[test]
fn starting_without_a_brief_says_so_instead_of_pretending() {
    let dir = repo();
    flow(dir.path())
        .args(["start", "Auth rework"])
        .assert()
        .success();

    let file = read(dir.path(), ".flow/runs/auth-rework.md");
    let (_, body) = file.split_once("## Where we are").unwrap();
    assert!(body.contains("No brief was given"));
    // The stage still has to be findable underneath it.
    assert!(body.contains("/grill-with-docs"));
}

#[test]
fn go_carries_the_handoff_the_last_session_left() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "printf %s \\\"$0\\\" > prompt.txt");
    flow(dir.path())
        .args(["done", "-m", "Tracker choice is still open."])
        .assert()
        .success();

    flow(dir.path())
        .arg("go")
        .env_remove("CLAUDECODE")
        .assert()
        .success();

    let prompt = read(dir.path(), "prompt.txt");
    assert!(prompt.starts_with("/to-spec"));
    assert!(prompt.contains("Tracker choice is still open."));
}

#[test]
fn go_never_records_the_stage_itself() {
    // An agent exiting cleanly means the session ended, not that the work is
    // done. Recording on exit code would write lies into the file.
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "true");
    let before = read(dir.path(), ".flow/runs/auth-rework.md");

    let out = stdout(flow(dir.path()).arg("go").env_remove("CLAUDECODE"));

    let after = read(dir.path(), ".flow/runs/auth-rework.md");
    assert_eq!(
        before.split("updated =").next(),
        after.split("updated =").next(),
        "go must not advance the run"
    );
    assert!(out.contains("still reads `grill`"));
    assert!(out.contains("flow done"));
}

#[test]
fn go_reports_an_artifact_that_appeared_while_the_agent_had_it() {
    let dir = repo_with_run();
    with_fake_agent(
        dir.path(),
        "mkdir -p .flow/artifacts/auth-rework; echo notes > .flow/artifacts/auth-rework/grill.md",
    );

    let out = stdout(flow(dir.path()).arg("go").env_remove("CLAUDECODE"));
    assert!(
        out.contains("grill.md"),
        "should notice the new artifact:\n{out}"
    );
}

#[test]
fn go_refuses_to_nest_a_session_inside_a_session() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "touch SHOULD_NOT_EXIST");

    let out = stdout(flow(dir.path()).arg("go").env("CLAUDECODE", "1"));

    assert!(
        !dir.path().join("SHOULD_NOT_EXIST").exists(),
        "must not launch"
    );
    assert!(out.contains("CLAUDECODE"));
    // It still hands over the prompt, so the agent already running can act.
    assert!(out.contains("/grill-with-docs"));
}

#[test]
fn go_print_shows_the_prompt_and_launches_nothing() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "touch SHOULD_NOT_EXIST");

    let out = stdout(
        flow(dir.path())
            .args(["go", "--print"])
            .env_remove("CLAUDECODE"),
    );

    assert!(!dir.path().join("SHOULD_NOT_EXIST").exists());
    assert!(out.contains("/grill-with-docs"));
    assert!(out.contains("flow done auth-rework"));
}

#[test]
fn an_unknown_agent_names_the_ones_that_exist() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "true");
    flow(dir.path())
        .args(["go", "--agent", "nonsense"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("test"));
}

#[test]
fn a_failing_agent_is_reported_rather_than_swallowed() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "exit 3");

    let out = stdout(flow(dir.path()).arg("go").env_remove("CLAUDECODE"));
    assert!(out.contains("status 3"), "got:\n{out}");
}

#[test]
fn go_picks_the_per_agent_command_override() {
    let dir = TempDir::new().unwrap();
    flow(dir.path()).arg("init").assert().success();
    std::fs::write(
        dir.path().join(".flow/flow.toml"),
        "name = \"mine\"\nagent = \"codex\"\n\n\
         [agents.codex]\ncommand = [\"sh\", \"-c\", \"printf %s \\\"$0\\\" > p.txt\", \"{prompt}\"]\n\n\
         [[stage]]\nname = \"spec\"\ncommand = \"/to-spec\"\n\n\
         [stage.agents]\ncodex = \"write the spec, codex-style\"\n",
    )
    .unwrap();
    flow(dir.path()).args(["start", "Thing"]).assert().success();

    flow(dir.path()).arg("go").assert().success();

    assert!(read(dir.path(), "p.txt").starts_with("write the spec, codex-style"));
}

// --- where settings live ---------------------------------------------------

#[test]
fn the_committed_flow_carries_no_agent_of_its_own() {
    // Committing a flow shares the process, not the author's tooling.
    let dir = repo();
    let toml = read(dir.path(), ".flow/flow.toml");
    assert!(
        !toml
            .lines()
            .any(|l| l.starts_with("[agents.") || l.starts_with("agent =")),
        "the preset must not ship a launcher:\n{toml}"
    );
}

#[test]
fn the_user_config_supplies_the_agent() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "printf %s \\\"$0\\\" > prompt.txt");

    flow(dir.path())
        .arg("go")
        .env_remove("CLAUDECODE")
        .assert()
        .success();

    assert!(read(dir.path(), "prompt.txt").starts_with("/grill-with-docs"));
}

#[test]
fn a_repo_may_override_the_users_agent_by_name() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "printf user > which.txt");
    with_repo_agent(dir.path(), "printf repo > which.txt");

    flow(dir.path())
        .arg("go")
        .env_remove("CLAUDECODE")
        .assert()
        .success();

    assert_eq!(read(dir.path(), "which.txt"), "repo");
}

#[test]
fn go_with_nothing_configured_points_at_the_setup_command() {
    let dir = repo_with_run();
    flow(dir.path())
        .arg("go")
        .assert()
        .failure()
        .stderr(predicates::str::contains("flow config --init"));
}

#[test]
fn config_names_both_files_and_where_each_setting_came_from() {
    let dir = repo_with_run();
    with_fake_agent(dir.path(), "true");

    let out = stdout(flow(dir.path()).arg("config"));

    // It must answer "where do I set this up" with actual paths.
    assert!(
        out.contains("xdg/flow/config.toml"),
        "no user path in:\n{out}"
    );
    assert!(out.contains(".flow/flow.toml"), "no repo path in:\n{out}");
    assert!(out.contains("test"));
    assert!(out.contains("user config"));
}

#[test]
fn config_says_so_when_the_user_config_is_missing() {
    let dir = repo();
    let out = stdout(flow(dir.path()).arg("config"));
    assert!(out.contains("does not exist"));
    assert!(out.contains("flow config --init"));
}

#[test]
fn config_init_writes_the_starter_and_never_clobbers_it() {
    let dir = repo();
    flow(dir.path())
        .args(["config", "--init"])
        .assert()
        .success();

    let path = dir.path().join("xdg/flow/config.toml");
    assert!(path.is_file());
    let starter = std::fs::read_to_string(&path).unwrap();
    assert!(starter.contains("[agents.claude]"));
    assert!(starter.contains("guard_env"));

    std::fs::write(&path, "agent = \"mine\"\n").unwrap();
    flow(dir.path())
        .args(["config", "--init"])
        .assert()
        .success();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "agent = \"mine\"\n"
    );
}

#[test]
fn config_works_outside_a_flow_repo() {
    // Answering "where do I set this up" must not require a repo to be set up.
    let dir = TempDir::new().unwrap();
    let out = stdout(flow(dir.path()).arg("config"));
    assert!(out.contains("flow init"));
}

#[test]
fn a_malformed_flow_says_so_instead_of_reading_as_absent() {
    let dir = repo();
    std::fs::write(
        dir.path().join(".flow/flow.toml"),
        "[agents.x]\nname = \"oops\"\n",
    )
    .unwrap();

    flow(dir.path())
        .arg("config")
        .assert()
        .failure()
        .stderr(predicates::str::contains("flow.toml"));
}

#[test]
fn init_points_a_new_user_at_the_setup_command() {
    let dir = TempDir::new().unwrap();
    let out = stdout(flow(dir.path()).arg("init"));
    assert!(out.contains("flow config --init"), "got:\n{out}");
}

// --- which run bare commands act on ----------------------------------------

#[test]
fn starting_a_run_makes_it_current() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();

    // Bare commands follow the pointer, not a guess about recency.
    assert!(stdout(flow(dir.path()).arg("next")).contains("Second"));
    assert!(stdout(flow(dir.path()).arg("status")).contains("* second"));
}

#[test]
fn switch_changes_what_bare_commands_act_on() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();

    flow(dir.path())
        .args(["switch", "first"])
        .assert()
        .success();

    assert!(stdout(flow(dir.path()).arg("next")).contains("First"));
    flow(dir.path())
        .args(["done", "-m", "note"])
        .assert()
        .success();
    // The one that was not current is untouched.
    assert!(read(dir.path(), ".flow/runs/second.md").contains("in_progress"));
    assert!(stdout(flow(dir.path()).args(["show", "first"])).contains("note"));
}

#[test]
fn an_explicit_slug_still_beats_the_pointer() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();

    assert!(stdout(flow(dir.path()).args(["next", "first"])).contains("First"));
}

#[test]
fn a_finished_current_run_stops_being_followed() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();
    flow(dir.path())
        .args(["finish", "-m", "shipped"])
        .assert()
        .success();

    // `second` was current and is now finished, so `first` is the only one left.
    assert!(stdout(flow(dir.path()).arg("next")).contains("First"));
}

#[test]
fn ambiguity_with_no_pointer_is_refused_rather_than_guessed() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();
    std::fs::remove_file(dir.path().join(".flow/current")).unwrap();

    flow(dir.path())
        .args(["done", "-m", "which one?"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("flow switch"));
}

#[test]
fn the_current_pointer_stays_out_of_the_repo() {
    let dir = repo();
    let ignore = read(dir.path(), ".flow/.gitignore");
    assert!(ignore.contains("/current"));
}

#[test]
fn the_generated_board_is_ignored_and_runs_are_offered_rather_than_ignored() {
    let dir = repo();
    let ignore = read(dir.path(), ".flow/.gitignore");

    // Generated from the run files, so committing it is churn with no reader.
    assert!(ignore.lines().any(|l| l.trim() == "/board.html"));
    // Runs and artifacts are the user's call, so they are presented commented
    // out rather than decided either way — ADR-0009.
    assert!(ignore.lines().any(|l| l.trim() == "# /runs/"));
    assert!(ignore.lines().any(|l| l.trim() == "# /artifacts/"));
}

#[test]
fn init_never_clobbers_an_answered_gitignore() {
    let dir = repo();
    // The answer someone gives by uncommenting a line must survive the next
    // `init`, or the invitation to edit it was not honest.
    let answered = read(dir.path(), ".flow/.gitignore").replace("# /runs/", "/runs/");
    std::fs::write(dir.path().join(".flow/.gitignore"), &answered).unwrap();

    let out = stdout(flow(dir.path()).arg("init"));

    assert_eq!(read(dir.path(), ".flow/.gitignore"), answered);
    assert!(out.contains("kept .flow/.gitignore"), "got:\n{out}");
}

#[test]
fn switching_to_an_unknown_run_fails() {
    let dir = repo_with_run();
    flow(dir.path()).args(["switch", "nope"]).assert().failure();
}

#[test]
fn switching_to_a_finished_run_is_refused() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();
    flow(dir.path())
        .args(["finish", "second", "-m", "shipped"])
        .assert()
        .success();

    // Pointing at a finished run would be a pointer nothing follows: `next`
    // and `done` step over it, so they would act on `first` regardless.
    flow(dir.path())
        .args(["switch", "second"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("flow reopen"));
    assert!(stdout(flow(dir.path()).arg("next")).contains("First"));
}

#[test]
fn reopening_a_run_makes_it_current() {
    let dir = repo();
    flow(dir.path()).args(["start", "First"]).assert().success();
    flow(dir.path())
        .args(["finish", "first", "-m", "shipped"])
        .assert()
        .success();
    // `second` is started after, so it is the one the pointer names.
    flow(dir.path())
        .args(["start", "Second"])
        .assert()
        .success();

    flow(dir.path())
        .args(["reopen", "first", "-m", "back on it"])
        .assert()
        .success();

    assert!(stdout(flow(dir.path()).arg("next")).contains("First"));
    flow(dir.path())
        .args(["done", "-m", "handoff for first"])
        .assert()
        .success();
    assert!(read(dir.path(), ".flow/runs/first.md").contains("handoff for first"));
    assert!(!read(dir.path(), ".flow/runs/second.md").contains("handoff for first"));
}

// --- choosing a flow -------------------------------------------------------

#[test]
fn presets_lists_the_built_in_flows() {
    let dir = repo();
    let out = stdout(flow(dir.path()).arg("presets"));
    for name in ["main-flow", "minimal", "bugfix"] {
        assert!(out.contains(name), "presets missing {name}:\n{out}");
    }
    assert!(
        out.contains("* main-flow"),
        "main-flow should be the default:\n{out}"
    );
}

#[test]
fn a_named_preset_is_written_out() {
    let dir = TempDir::new().unwrap();
    flow(dir.path())
        .args(["init", "--preset", "bugfix"])
        .assert()
        .success();

    let toml = read(dir.path(), ".flow/flow.toml");
    assert!(toml.contains("name = \"bugfix\""));
    for stage in ["reproduce", "diagnose", "fix", "verify"] {
        assert!(toml.contains(&format!("name = \"{stage}\"")));
    }
    // And it is a real flow, not just a file.
    flow(dir.path())
        .args(["start", "Crash on load"])
        .assert()
        .success();
    assert!(stdout(flow(dir.path()).arg("next")).contains("reproduce"));
}

/// The set the build script generated from `presets/`, so a flow file dropped
/// in there is covered by the test below without anyone editing a list.
mod shipped {
    // This test needs only the names; the binary is what reads the rest.
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/shipped.rs"));
}

#[test]
fn every_built_in_preset_actually_works() {
    assert!(!shipped::SHIPPED.is_empty(), "no presets were embedded");
    for preset in shipped::SHIPPED.iter().map(|preset| preset.name) {
        let dir = TempDir::new().unwrap();
        flow(dir.path())
            .args(["init", "--preset", preset])
            .assert()
            .success();
        flow(dir.path()).args(["start", "Thing"]).assert().success();
        flow(dir.path()).arg("next").assert().success();
        flow(dir.path())
            .args(["done", "-m", "n"])
            .assert()
            .success();
        flow(dir.path()).arg("board").assert().success();
    }
}

#[test]
fn a_preset_can_be_a_file_you_wrote() {
    let dir = TempDir::new().unwrap();
    let mine = dir.path().join("mine.toml");
    std::fs::write(
        &mine,
        "name = \"mine\"\n\n[[stage]]\nname = \"ponder\"\ncommand = \"/think\"\n",
    )
    .unwrap();

    flow(dir.path())
        .arg("init")
        .arg("--preset")
        .arg(&mine)
        .assert()
        .success();

    assert!(read(dir.path(), ".flow/flow.toml").contains("ponder"));
}

#[test]
fn the_user_config_can_change_which_flow_init_writes() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("xdg/flow/config.toml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, "preset = \"minimal\"\n").unwrap();

    flow(dir.path()).arg("init").assert().success();

    assert!(read(dir.path(), ".flow/flow.toml").contains("name = \"minimal\""));
    assert!(stdout(flow(dir.path()).arg("presets")).contains("* minimal"));
}

#[test]
fn an_unknown_preset_lists_the_ones_that_exist() {
    let dir = TempDir::new().unwrap();
    flow(dir.path())
        .args(["init", "--preset", "nonsense"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("minimal"));
}

// --- ticket 13: the Preset Path ---------------------------------------------

#[test]
fn a_user_preset_is_offered_and_says_it_came_from_the_user() {
    let dir = repo();
    write_preset(
        &dir.path().join("xdg/flow/presets"),
        "house",
        "The house style.",
    );

    let out = stdout(flow(dir.path()).arg("presets"));

    assert!(out.contains("house"), "user preset missing:\n{out}");
    assert!(
        out.contains("The house style."),
        "description missing:\n{out}"
    );
    let row = row_for(&out, "house");
    assert!(
        row.contains("user"),
        "row does not name the user layer: {row}"
    );
}

#[test]
fn a_project_preset_is_offered_and_says_it_came_from_the_project() {
    let dir = repo();
    write_preset(
        &dir.path().join(".flow/presets"),
        "ours",
        "What we do here.",
    );

    let out = stdout(flow(dir.path()).arg("presets"));

    assert!(
        out.contains("What we do here."),
        "description missing:\n{out}"
    );
    let row = row_for(&out, "ours");
    assert!(
        row.contains("project"),
        "row does not name the project layer: {row}"
    );
}

#[test]
fn a_preset_in_an_ancestor_reaches_a_package_that_has_its_own_flow() {
    // The case a nearest-`.flow` walk would break: the package has already run
    // `init`, so a search that stops at the first `.flow` never sees the repo
    // root's menu.
    let outer = TempDir::new().unwrap();
    let pkg = outer.path().join("packages/api");
    std::fs::create_dir_all(&pkg).unwrap();
    flow_from(&pkg, outer.path()).arg("init").assert().success();
    write_preset(
        &outer.path().join(".flow/presets"),
        "house",
        "Every package here uses this.",
    );

    let out = stdout(flow_from(&pkg, outer.path()).arg("presets"));

    assert!(
        out.contains("Every package here uses this."),
        "an ancestor's preset never reached the package:\n{out}"
    );
    assert!(row_for(&out, "house").contains("project"));
}

#[test]
fn the_project_beats_the_user_which_beats_what_ships() {
    let dir = repo();
    write_preset(
        &dir.path().join("xdg/flow/presets"),
        "main-flow",
        "The user's own.",
    );

    let out = stdout(flow(dir.path()).arg("presets"));
    assert!(
        row_for(&out, "main-flow").contains("The user's own."),
        "the user's preset did not beat the shipped one:\n{out}"
    );

    write_preset(
        &dir.path().join(".flow/presets"),
        "main-flow",
        "The project's own.",
    );
    let out = stdout(flow(dir.path()).arg("presets"));
    assert!(
        row_for(&out, "main-flow").contains("The project's own."),
        "the project's preset did not beat the user's:\n{out}"
    );
}

#[test]
fn a_nearer_ancestor_beats_a_farther_one() {
    let outer = TempDir::new().unwrap();
    let inner = outer.path().join("packages/api");
    std::fs::create_dir_all(&inner).unwrap();
    write_preset(&outer.path().join(".flow/presets"), "house", "The far one.");
    write_preset(&inner.join(".flow/presets"), "house", "The near one.");

    let out = stdout(flow_from(&inner, outer.path()).arg("presets"));

    assert!(
        row_for(&out, "house").contains("The near one."),
        "the farther ancestor won:\n{out}"
    );
}

#[test]
fn a_shadowed_preset_is_still_listed_and_says_what_beat_it() {
    let dir = repo();
    write_preset(
        &dir.path().join(".flow/presets"),
        "main-flow",
        "The project's own.",
    );

    let out = stdout(flow(dir.path()).arg("presets"));

    assert!(
        out.to_lowercase().contains("shadow"),
        "shadowing was silent, which is how someone loses an afternoon:\n{out}"
    );
    // The beaten entry stays visible, and its layer is named alongside the
    // layer that beat it.
    let shadow = out
        .lines()
        .find(|line| line.to_lowercase().contains("shadow"))
        .unwrap();
    assert!(
        shadow.contains("shipped"),
        "the beaten layer is unnamed: {shadow}"
    );
    assert!(
        shadow.contains("project"),
        "the winning layer is unnamed: {shadow}"
    );
}

#[test]
fn a_shadowed_preset_says_what_you_overrode_not_just_that_you_did() {
    let dir = repo();
    let shipped = stdout(flow(dir.path()).arg("presets"));
    let overridden = row_for(&shipped, "main-flow")
        .split_whitespace()
        .last()
        .unwrap()
        .to_string();
    write_preset(
        &dir.path().join(".flow/presets"),
        "main-flow",
        "The project's own.",
    );

    let out = stdout(flow(dir.path()).arg("presets"));

    // Knowing that you overrode something is only half of knowing what.
    let shadow = out
        .lines()
        .find(|line| line.to_lowercase().contains("shadow"))
        .unwrap();
    assert!(
        shadow.contains(&overridden),
        "the shadowed description is nowhere: {shadow}"
    );
}

#[test]
fn the_default_marker_and_the_footer_survive() {
    let dir = repo();
    let out = stdout(flow(dir.path()).arg("presets"));
    assert!(out.contains("* main-flow"), "default marker gone:\n{out}");
    assert!(out.contains("preset = "), "footer gone:\n{out}");
}

/// The line a preset is listed on, so assertions can be about that entry rather
/// than about anything else that happens to be on screen.
fn row_for<'a>(out: &'a str, name: &str) -> &'a str {
    out.lines()
        .find(|line| line.split_whitespace().any(|word| word == name))
        .unwrap_or_else(|| panic!("no row for `{name}`:\n{out}"))
}

// --- ticket 14: a bad preset is skipped, never fatal ------------------------

/// The five ways a file in a presets directory fails to be a preset, each in
/// its own file so one run of `flow presets` reports all of them.
fn write_the_bad_presets(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("garbled.toml"), "this is not = = toml\n").unwrap();
    std::fs::write(
        dir.join("notaflow.toml"),
        "name = \"notaflow\"\nstage = 5\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("stageless.toml"),
        "name = \"stageless\"\ndescription = \"Nothing to do.\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("misnamed.toml"),
        "name = \"something-else\"\n\n[[stage]]\nname = \"do\"\ncommand = \"/do\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("nameless.toml"),
        "description = \"Forgot the name.\"\n\n[[stage]]\nname = \"do\"\ncommand = \"/do\"\n",
    )
    .unwrap();
}

#[test]
fn a_malformed_preset_is_skipped_with_a_reason_and_presets_still_works() {
    let dir = repo();
    std::fs::create_dir_all(dir.path().join(".flow/presets")).unwrap();
    std::fs::write(
        dir.path().join(".flow/presets/garbled.toml"),
        "this is not = = toml\n",
    )
    .unwrap();

    let out = stdout(flow(dir.path()).arg("presets"));

    assert!(out.contains("garbled.toml"), "the file is unnamed:\n{out}");
    assert!(
        out.to_lowercase().contains("skip"),
        "nothing says it was skipped:\n{out}"
    );
    // The flows that are fine are still on offer.
    assert!(
        out.contains("main-flow"),
        "a bad file hid the good ones:\n{out}"
    );
}

#[test]
fn a_repo_still_initialises_with_a_broken_preset_sitting_in_the_tree() {
    let outer = TempDir::new().unwrap();
    write_the_bad_presets(&outer.path().join(".flow/presets"));
    let pkg = outer.path().join("packages/api");
    std::fs::create_dir_all(&pkg).unwrap();

    flow_from(&pkg, outer.path()).arg("init").assert().success();

    assert!(pkg.join(".flow/flow.toml").is_file());
}

#[test]
fn each_reason_a_preset_is_skipped_for_is_reported_distinguishably() {
    let dir = repo();
    write_the_bad_presets(&dir.path().join(".flow/presets"));

    let out = stdout(flow(dir.path()).arg("presets"));

    let reason = |file: &str| -> String {
        out.lines()
            .find(|line| line.contains(file))
            .unwrap_or_else(|| panic!("no line for {file}:\n{out}"))
            .to_string()
    };
    let reasons: Vec<String> = ["garbled", "notaflow", "stageless", "misnamed", "nameless"]
        .iter()
        .map(|f| reason(&format!("{f}.toml")))
        .collect();

    assert!(
        reasons[0].contains("TOML"),
        "not-TOML unexplained: {}",
        reasons[0]
    );
    assert!(
        reasons[1].contains("flow"),
        "not-a-flow unexplained: {}",
        reasons[1]
    );
    assert!(
        reasons[2].contains("stages"),
        "stageless unexplained: {}",
        reasons[2]
    );
    // The stem rule, at its softer severity: the same check the build script
    // treats as fatal, naming both values.
    assert!(
        reasons[3].contains("something-else") && reasons[3].contains("misnamed"),
        "the stem mismatch does not name both values: {}",
        reasons[3]
    );
    // A file that never declared a `name` is told what its name has to be,
    // rather than being handed serde's complaint about a missing field.
    assert!(
        reasons[4].contains("name") && reasons[4].contains("nameless"),
        "the missing name does not say what it should have been: {}",
        reasons[4]
    );
    for (i, a) in reasons.iter().enumerate() {
        for b in &reasons[i + 1..] {
            assert_ne!(a, b, "two reasons read the same");
        }
    }
}

#[test]
fn a_file_that_is_not_a_toml_is_ignored_in_silence() {
    let dir = repo();
    let presets = dir.path().join(".flow/presets");
    std::fs::create_dir_all(&presets).unwrap();
    std::fs::write(presets.join("README.md"), "How we use these.\n").unwrap();
    std::fs::write(presets.join(".main-flow.toml.swp"), "\0garbage").unwrap();

    let out = stdout(flow(dir.path()).arg("presets"));

    assert!(
        !out.contains("README"),
        "a README was reported as a problem:\n{out}"
    );
    assert!(
        !out.contains("swp"),
        "a swapfile was reported as a problem:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("skip"),
        "nothing was skipped, but the section appeared:\n{out}"
    );
}

#[test]
fn an_absent_presets_directory_is_not_an_error() {
    let dir = repo();
    assert!(!dir.path().join(".flow/presets").exists());
    flow(dir.path()).arg("presets").assert().success();
    flow(dir.path()).arg("init").assert().success();
}

// --- ticket 15: init resolves a name through the Preset Path ----------------

#[test]
fn a_preset_you_wrote_shadows_the_default_a_bare_init_writes() {
    let dir = TempDir::new().unwrap();
    write_preset(
        &dir.path().join("xdg/flow/presets"),
        "main-flow",
        "The house spine.",
    );

    flow(dir.path()).arg("init").assert().success();

    // Shadowing works for the default too, not only for an explicit choice.
    let toml = read(dir.path(), ".flow/flow.toml");
    assert!(
        toml.contains("The house spine."),
        "wrote the shipped one:\n{toml}"
    );
}

#[test]
fn init_writes_a_user_preset_named_on_the_command_line() {
    let dir = TempDir::new().unwrap();
    write_preset(&dir.path().join("xdg/flow/presets"), "spike", "Throwaway.");

    flow(dir.path())
        .args(["init", "--preset", "spike"])
        .assert()
        .success();

    assert!(read(dir.path(), ".flow/flow.toml").contains("Throwaway."));
}

#[test]
fn init_writes_a_preset_found_in_an_ancestor() {
    let outer = TempDir::new().unwrap();
    write_preset(
        &outer.path().join(".flow/presets"),
        "house",
        "Every package here uses this.",
    );
    let pkg = outer.path().join("packages/api");
    std::fs::create_dir_all(&pkg).unwrap();

    let out = stdout(flow_from(&pkg, outer.path()).args(["init", "--preset", "house"]));

    assert!(std::fs::read_to_string(pkg.join(".flow/flow.toml"))
        .unwrap()
        .contains("Every package here uses this."));
    // Inheritance from a parent directory must never be invisible.
    assert!(
        out.contains(&outer.path().display().to_string()),
        "init did not name the ancestor it drew from:\n{out}"
    );
}

#[test]
fn init_names_the_layer_the_flow_came_from() {
    let dir = TempDir::new().unwrap();
    let out = stdout(flow(dir.path()).arg("init"));
    assert!(
        out.contains("main-flow"),
        "init did not name the flow:\n{out}"
    );
    assert!(
        out.contains("shipped"),
        "init did not name the layer:\n{out}"
    );
}

#[test]
fn a_configured_default_that_resolves_to_nothing_is_a_hard_error() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("xdg/flow/config.toml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, "preset = \"gone\"\n").unwrap();

    flow(dir.path())
        .arg("init")
        .assert()
        .failure()
        // Never a silent fallback: writing a flow the user did not ask for
        // into a file they are then told they own is the worst outcome here.
        .stderr(predicates::str::contains("gone"))
        .stderr(predicates::str::contains("main-flow"));

    assert!(!dir.path().join(".flow/flow.toml").exists());
}

#[test]
fn an_unknown_preset_lists_presets_from_every_layer() {
    let dir = TempDir::new().unwrap();
    write_preset(&dir.path().join("xdg/flow/presets"), "spike", "Throwaway.");

    flow(dir.path())
        .args(["init", "--preset", "nonsense"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("spike"))
        .stderr(predicates::str::contains("minimal"));
}

// --- ticket 16: config answers "where do I put a flow of my own" ------------

#[test]
fn config_names_the_presets_directories_it_reads() {
    let dir = repo();
    write_preset(&dir.path().join("xdg/flow/presets"), "spike", "Throwaway.");
    write_preset(&dir.path().join(".flow/presets"), "house", "Ours.");

    let out = stdout(flow(dir.path()).arg("config"));

    assert!(
        out.contains("xdg/flow/presets"),
        "the user presets directory is unnamed:\n{out}"
    );
    assert!(
        out.contains(".flow/presets"),
        "the project presets directory is unnamed:\n{out}"
    );
}

#[test]
fn config_marks_a_presets_directory_that_does_not_exist_yet() {
    // "Where do I create it" is answered by the exact path on screen, not by
    // something the reader has to infer.
    let dir = repo();
    assert!(!dir.path().join(".flow/presets").exists());

    let out = stdout(flow(dir.path()).arg("config"));

    let row = out
        .lines()
        .find(|line| line.contains(".flow/presets"))
        .unwrap_or_else(|| panic!("the project presets directory is missing:\n{out}"));
    assert!(row.contains("does not exist"), "unmarked: {row}");
}

#[test]
fn config_names_an_ancestor_presets_directory_it_inherits_from() {
    let outer = TempDir::new().unwrap();
    write_preset(&outer.path().join(".flow/presets"), "house", "Ours.");
    let pkg = outer.path().join("packages/api");
    std::fs::create_dir_all(&pkg).unwrap();
    flow_from(&pkg, outer.path()).arg("init").assert().success();

    let out = stdout(flow_from(&pkg, outer.path()).arg("config"));

    assert!(
        out.contains(&outer.path().join(".flow/presets").display().to_string()),
        "an inherited presets directory is invisible:\n{out}"
    );
}

// --- review: the root a command acts on -------------------------------------

#[test]
fn a_relative_root_walks_the_roots_ancestry_not_the_working_directory() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("target-repo");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&target).unwrap();
    write_preset(
        &elsewhere.join(".flow/presets"),
        "elsewhere-only",
        "Belongs to another repo entirely.",
    );

    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.current_dir(&elsewhere);
    cmd.arg("--root").arg("../target-repo");
    cmd.env("XDG_CONFIG_HOME", dir.path().join("xdg"));
    let out = stdout(cmd.arg("presets"));

    assert!(
        !out.contains("elsewhere-only"),
        "the working directory's presets reached an unrelated root:\n{out}"
    );
}

#[test]
fn a_root_of_dot_does_not_shadow_itself() {
    let dir = TempDir::new().unwrap();
    write_preset(&dir.path().join(".flow/presets"), "house", "Ours.");

    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("--root").arg(".");
    cmd.env("XDG_CONFIG_HOME", dir.path().join("xdg"));
    let out = stdout(cmd.arg("presets"));

    assert!(
        !out.contains("shadows"),
        "the same directory was read twice, so a preset shadowed itself:\n{out}"
    );
}

#[test]
fn a_default_preset_that_resolves_to_nothing_is_said_so_in_the_listing() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("xdg/flow")).unwrap();
    std::fs::write(
        dir.path().join("xdg/flow/config.toml"),
        "preset = \"gone\"\n",
    )
    .unwrap();

    let out = stdout(flow(dir.path()).arg("presets"));

    // Every row is unmarked, and this is the screen someone reads to find out
    // why `flow init` refused — so it has to name the preset that is missing.
    assert!(
        out.contains("gone"),
        "the listing says nothing about a default that resolves to nothing:\n{out}"
    );
}

#[test]
fn the_project_walk_stops_at_your_home_directory() {
    let outer = TempDir::new().unwrap();
    let home = outer.path().join("home/someone");
    let repo = home.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    write_preset(&home.join(".flow/presets"), "inside-home", "Yours.");
    write_preset(
        &outer.path().join(".flow/presets"),
        "above-home",
        "In a directory nobody owns.",
    );

    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.arg("--root").arg(&repo);
    cmd.env("HOME", &home);
    cmd.env("XDG_CONFIG_HOME", home.join("xdg"));
    let out = stdout(cmd.arg("presets"));

    assert!(
        out.contains("inside-home"),
        "a preset inside your home was unreachable:\n{out}"
    );
    // A preset carries the launcher argv `flow go` spawns, so one sitting above
    // anything you own must not beat what ships.
    assert!(
        !out.contains("above-home"),
        "a preset above your home directory was read:\n{out}"
    );
}

#[test]
fn the_project_walk_reaches_the_repository_root_outside_your_home() {
    let outer = TempDir::new().unwrap();
    let repo_root = outer.path().join("srv/monorepo");
    let pkg = repo_root.join("packages/api");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::create_dir_all(repo_root.join(".git")).unwrap();
    write_preset(&repo_root.join(".flow/presets"), "house", "The monorepo's.");
    write_preset(
        &outer.path().join(".flow/presets"),
        "above-repo",
        "Outside the repository.",
    );

    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.arg("--root").arg(&pkg);
    // Home is somewhere else entirely, so the repository is the only bound.
    cmd.env("HOME", outer.path().join("elsewhere"));
    cmd.env("XDG_CONFIG_HOME", outer.path().join("xdg"));
    let out = stdout(cmd.arg("presets"));

    assert!(
        out.contains("house"),
        "the repository root's preset never reached the package:\n{out}"
    );
    assert!(
        !out.contains("above-repo"),
        "a preset above the repository root was read:\n{out}"
    );
}
