//! What this crate can do, one module per verb.

pub mod Cloudflare;
pub mod Consult;
pub mod Corpus;
pub mod Decide;
pub mod Expressions;
pub mod Gate;
pub mod Observe;
pub mod Poster;

pub use crate::Fn::{Cloudflare::Post, Decide::Decide as Policy, Expressions::*};
