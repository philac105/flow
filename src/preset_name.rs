//! One rule, two severities: a preset's filename stem must equal the `name` it
//! declares.
//!
//! This file is the only place that rule is written. `build.rs` compiles it as
//! a module of its own and applies it fatally to the presets that ship in the
//! binary; the crate compiles it as a module too and applies it softly, as a
//! reason to skip a preset someone left on disk. It therefore depends on
//! nothing — not on this crate, not on any dependency.

/// `Ok` when a preset's declared `name` matches the filename stem it was found
/// under; otherwise the reason they disagree, written for whoever has to fix
/// the file.
// Two callers, one of them outside the crate: the build script, which treats a
// failure as fatal, and preset discovery, which treats it as a reason to skip.
pub fn check(stem: &str, declared: &str) -> Result<(), String> {
    if declared == stem {
        Ok(())
    } else if declared.is_empty() {
        Err(format!(
            "declares no `name` — it has to be `name = \"{stem}\"`, matching the filename"
        ))
    } else {
        Err(format!(
            "declares `name = \"{declared}\"` but its filename says `{stem}` — a preset is \
             named by its file, so the two have to agree"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn a_matching_stem_and_name_pass() {
        assert!(check("bugfix", "bugfix").is_ok());
    }

    #[test]
    fn a_mismatch_names_both_the_stem_and_the_declared_name() {
        let reason = check("bugfix", "hotfix").unwrap_err();
        assert!(reason.contains("bugfix"), "reason omits the stem: {reason}");
        assert!(reason.contains("hotfix"), "reason omits the name: {reason}");
    }

    #[test]
    fn a_missing_name_says_what_it_should_have_been() {
        let reason = check("bugfix", "").unwrap_err();
        assert!(reason.contains("bugfix"), "reason omits the stem: {reason}");
    }

    #[test]
    fn the_comparison_is_exact() {
        assert!(check("bugfix", "Bugfix").is_err());
        assert!(check("bugfix", " bugfix").is_err());
    }
}
