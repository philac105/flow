//! The presets that ship in the binary, generated at build time from `presets/`
//! at the repo root. A preset's id is its filename stem and its description is
//! read from inside the file, so there is nowhere for the two to disagree.

include!(concat!(env!("OUT_DIR"), "/shipped.rs"));

/// What a bare `flow init` writes when nothing else is configured. A named
/// flow, deliberately, rather than whichever file happens to sort first.
pub const DEFAULT: &str = "main-flow";

/// The shipped preset with this name, if one ships.
pub fn shipped(name: &str) -> Option<&'static Shipped> {
    SHIPPED.iter().find(|preset| preset.name == name)
}

/// Every shipped preset's name, for telling someone what they could have asked
/// for instead.
pub fn shipped_names() -> Vec<&'static str> {
    SHIPPED.iter().map(|preset| preset.name).collect()
}
