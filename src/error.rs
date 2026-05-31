//! Error types for Chronos Engine.
//!
//! This module defines the error handling infrastructure used throughout the engine.
//! All production code should propagate errors using `anyhow::Result` (aliased as
//! [`ChronosResult`]) instead of calling `.unwrap()` or `.expect()`.
//!
//! # Migration guide
//!
//! Replace `.unwrap()` calls with the `?` operator + `.context()`:
//!
//! ```rust,ignore
//! // Before
//! let val = fallible_fn().unwrap();
//! let component = world.get_component::<Health>(entity).unwrap();
//!
//! // After
//! let val = fallible_fn().with_context(|| "description")?;
//! let component = ensure(world.get_component::<Health>(entity), "entity has no Health")?;
//! ```
//!
//! For truly infallible operations where a panic indicates a bug, use
//! `.expect("descriptive message")` instead of `.unwrap()`.

use std::fmt::Display;

/// A unified result type for Chronos Engine operations.
pub type ChronosResult<T> = anyhow::Result<T>;

/// Ensure an `Option` has a value, returning an error with the given message otherwise.
///
/// # Example
///
/// ```rust,ignore
/// let health = ensure(world.get_component::<Health>(entity), "Entity has no Health component")?;
/// ```
pub fn ensure<T, M: Into<String>>(option: Option<T>, msg: M) -> ChronosResult<T> {
    option.ok_or_else(|| anyhow::anyhow!("{}", msg.into()))
}

/// Ensure a condition is true, returning an error with the given message otherwise.
///
/// # Example
///
/// ```rust,ignore
/// ensure_or(world.entity_exists(entity), "Entity does not exist")?;
/// ```
pub fn ensure_or(condition: bool, msg: impl Into<String>) -> ChronosResult<()> {
    if condition {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}", msg.into()))
    }
}

/// Format an error context string from a displayable value.
///
/// Use with `anyhow::Context::with_context(|| ctx(...))`.
///
/// # Example
///
/// ```rust,ignore
/// use anyhow::Context;
/// load_file(path).with_context(|| ctx("loading file", path))?;
/// ```
pub fn ctx<T: Display>(action: &str, target: T) -> String {
    format!("failed to {}: {}", action, target)
}

/// Convert a `Result<T, E>` with only `E: Display` into a `ChronosResult<T>`.
pub fn from_display<T, E: Display>(result: Result<T, E>, msg: &str) -> ChronosResult<T> {
    result.map_err(|e| anyhow::anyhow!("{}: {}", msg, e))
}
