//! Where the reader's compiled module sits.
//!
//! `App/reader` builds a `cdylib` and nothing else, so its artefact is a file
//! beside the crate rather than a library this crate links: the page
//! instantiates it next to the game's own instance. That makes its path a
//! runtime question, and it is answered by the same resolver as the corpus and
//! the dotenv file - see [`Location`].
//!
//! Nothing here loads the module. A run that reads state through the page's own
//! `Evaluate` never touches it; the path exists so that a composition root that
//! does instantiate the reader has one place to ask, and so that a run can say
//! which artefact it would have loaded.

use std::{env, path::PathBuf};

use crate::Fn::Location;

/// The artefact beside this crate: `App/reader`'s release build, relative to
/// `App/driver`.
pub const Relative:&str = "../reader/target/wasm32-unknown-unknown/release/NfsuReader.wasm";

/// The variable that names a reader module somewhere else.
pub const Variable:&str = "NFSU2_READER";

/// The module an installation keeps outside the tree, used only when
/// [`Relative`] is absent.
pub const Absolute:&str = "/usr/local/share/nfsu2/NfsuReader.wasm";

/// The reader module a run would instantiate.
///
/// Relative to this crate first, then the variable, then the absolute
/// location: the three candidates [`Location`] states, in its order.
pub fn Wasm() -> PathBuf {
	Location::Resolve(
		&Location::Crate(),
		env::var(Variable).ok().as_deref(),
		Relative,
		Absolute,
	)
}
