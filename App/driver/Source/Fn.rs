//! What this crate can do, one module per verb.

pub mod Cloudflare;
pub mod Decide;
pub mod Expressions;
pub mod Gate;
pub mod Send;

pub use crate::Fn::{Cloudflare::Advise, Decide::Decide as Policy, Expressions::*};
