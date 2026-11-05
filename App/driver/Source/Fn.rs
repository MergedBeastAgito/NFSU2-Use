//! What this crate can do, one module per verb.

pub mod Cloudflare;
pub mod Decide;
pub mod Expressions;
pub mod Gate;

pub use crate::Fn::{Cloudflare::Post, Decide::Decide as Policy, Expressions::*};
