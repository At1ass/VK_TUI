//! Re-exports longpoll handler from vk-core.
//!
//! This module exists for backward compatibility during the transition
//! to the vk-core crate.

pub use vk_core::longpoll::handle_update;
