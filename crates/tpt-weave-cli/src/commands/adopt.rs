//! `tpt-weave adopt` — onboard: init + index --full + doctor.

use super::{Rendered, doctor, index, init};
use crate::args::{Cli, Command};
use crate::error::CliError;
use serde_json::json;

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let init_rendered = init::run(cli, false)?;
    let index_rendered = index::run(cli, true)?;
    let doctor_rendered = doctor::run(cli)?;

    let steps = json!({
        "init": init_rendered.json,
        "index": index_rendered.json,
        "doctor": doctor_rendered.json,
    });
    let human = format!(
        "adopted repository\n  {}\n  {}\n  {}",
        init_rendered.human.lines().next().unwrap_or_default(),
        index_rendered.human.lines().next().unwrap_or_default(),
        doctor_rendered.human.lines().next().unwrap_or_default(),
    );
    Ok(Rendered::new(human, json!({ "adopted": true, "steps": steps })).with_detail(
        format!(
            "{}{}{}",
            init_rendered.detail,
            index_rendered.detail,
            doctor_rendered.detail
        ),
    ))
}

/// Adopt is sugar over the subcommands it composes — this assertion keeps
/// the command list honest if `Command` grows without updating adopt.
#[allow(dead_code)]
fn _assert_adopt_composes_existing_commands() {
    let _ = Command::Adopt;
}
