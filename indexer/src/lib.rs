// src/lib.rs
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod util;
pub mod helpers;
pub mod file_intent_entry;
pub mod snippet;
pub mod intent;
pub mod scan;
pub mod chunker;
pub mod types_view;
pub mod diff;
pub mod map_view;
pub mod commands;
pub mod functions_view;
pub mod custom_view;
pub mod index_v3;