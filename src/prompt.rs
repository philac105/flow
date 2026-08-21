//! Asking the person at the keyboard.
//!
//! Half of `flow`'s callers are not people: the adapter skill has agents run
//! `flow start`, and `flow go` spawns more of them. So every prompt here is
//! gated on [`interactive`] — a question an agent cannot answer would hang the
//! session it is holding. Flags always win; prompting only fills what was left
//! out.

use anyhow::Result;
use std::io::{self, IsTerminal, Write};

/// Whether there is a human on the other end. Both directions matter: stdin
/// carries the answer, stdout carries the question.
pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// One line, trimmed. Empty means the question was passed over.
pub fn line(question: &str) -> Result<String> {
    print!("{question}> ");
    io::stdout().flush()?;
    let mut buf = String::new();
    // Zero bytes is end of input — ctrl-D. Treated as passing over the
    // question, the same as an empty line.
    if io::stdin().read_line(&mut buf)? == 0 {
        println!();
        return Ok(String::new());
    }
    Ok(buf.trim().to_string())
}

/// Lines until a blank one. A brief worth reading is a paragraph, and a single
/// `-m` at a shell prompt quietly discourages writing one.
pub fn paragraph(question: &str) -> Result<String> {
    println!("{question}");
    let mut lines: Vec<String> = Vec::new();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut buf = String::new();
        if io::stdin().read_line(&mut buf)? == 0 {
            println!();
            break;
        }
        if buf.trim().is_empty() {
            break;
        }
        lines.push(buf.trim_end().to_string());
    }
    Ok(lines.join("\n").trim().to_string())
}
