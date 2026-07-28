//! Scripting & modding system — Phase 9.
//!
//! Rhai-based scripting with lifecycle hooks, full engine API,
//! hot-reload, and mod loading.

#![allow(dead_code)]

mod api;
mod bridge;
mod component;
mod hotreload;
mod lifecycle;
mod modloader;

pub use api::ScriptApi;
pub use bridge::ScriptEngine;
pub use component::{ScriptComponent, ScriptHandle};
pub use hotreload::ScriptWatcher;
pub use lifecycle::{ScriptContext, ScriptLifecycle};
pub use modloader::{ModError, ModLoader, ModMetadata};
