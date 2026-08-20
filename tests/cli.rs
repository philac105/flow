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
    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.arg("--root").arg(dir);
    cmd
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
        .args(["start", "Auth rework", "--kind", "feature"])
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
    assert!(!dir.path().join(".scratch").exists());
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
fn next_and_done_demand_a_slug_when_several_runs_are_active() {
    let dir = repo_with_run();
    flow(dir.path())
        .args(["start", "Other thing"])
        .assert()
        .success();

    flow(dir.path())
        .arg("next")
        .assert()
        .failure()
        .stderr(predicates::str::contains("auth-rework"));
    flow(dir.path())
        .args(["next", "auth-rework"])
        .assert()
        .success();
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
    // The grill stage declares `.scratch/{slug}/grill.md`. A session that died
    // after writing it but before recording leaves exactly this shape.
    std::fs::create_dir_all(dir.path().join(".scratch/auth-rework")).unwrap();
    std::fs::write(dir.path().join(".scratch/auth-rework/grill.md"), "notes").unwrap();

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
    std::fs::create_dir_all(dir.path().join(".scratch/auth-rework")).unwrap();
    std::fs::write(dir.path().join(".scratch/auth-rework/grill.md"), "notes").unwrap();

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
    std::fs::create_dir_all(dir.path().join(".scratch/auth-rework")).unwrap();
    std::fs::write(dir.path().join(".scratch/auth-rework/grill.md"), "notes").unwrap();
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
    assert!(out.contains(".scratch/auth-rework/grill.md"), "got:\n{out}");
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
    std::fs::create_dir_all(dir.path().join(".scratch/auth-rework")).unwrap();
    std::fs::write(dir.path().join(".scratch/auth-rework/grill.md"), "notes").unwrap();
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
    std::fs::create_dir_all(dir.path().join(".scratch/auth-rework")).unwrap();
    std::fs::write(dir.path().join(".scratch/auth-rework/grill.md"), "notes").unwrap();
    flow(dir.path())
        .args([
            "done",
            "-m",
            "grilled",
            "--artifact",
            ".scratch/auth-rework/grill.md",
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

/// A stand-in for a real agent: it records the prompt it was given and exits.
/// `sh -c '<script>' <arg>` binds the first argument to `$0`.
fn with_fake_agent(dir: &Path, script: &str) {
    let flow_toml = read(dir, ".flow/flow.toml");
    let replaced = flow_toml.replace(
        "command = [\"claude\", \"{prompt}\"]",
        &format!("command = [\"sh\", \"-c\", \"{script}\", \"{{prompt}}\"]"),
    );
    assert_ne!(flow_toml, replaced, "launcher line not found in the preset");
    std::fs::write(dir.join(".flow/flow.toml"), replaced).unwrap();
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
    assert!(prompt.contains("--artifact .scratch/auth-rework/grill.md"));
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
        "mkdir -p .scratch/auth-rework; echo notes > .scratch/auth-rework/grill.md",
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
    flow(dir.path())
        .args(["go", "--agent", "nonsense"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("claude"));
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
