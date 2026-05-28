//! Scripting & modding system — Phase 9.
//!
//! Rhai-based scripting with lifecycle hooks, full engine API,
//! hot-reload, and mod loading.

#![allow(dead_code)]

mod bridge;
mod component;
mod lifecycle;
mod api;
mod hotreload;
mod modloader;

pub use bridge::ScriptEngine;
pub use component::{ScriptComponent, ScriptHandle};
pub use lifecycle::{ScriptLifecycle, ScriptContext};
pub use api::ScriptApi;
pub use hotreload::ScriptWatcher;
pub use modloader::{ModLoader, ModMetadata, ModError};
