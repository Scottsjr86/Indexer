// src/main.rs

pub mod chunker;
pub mod commands;
pub mod diff;
pub mod intent;
pub mod map_view;
pub mod scan;
pub mod snippet;
pub mod tree_view;
pub mod file_intent_entry;
pub mod util;
pub mod helpers;

use anyhow::Result;

fn main() -> Result<()> {
    commands::run_cli()
}
