//! Where this crate's data sits, and the three answers in one order.
//!
//! Every path a run reads - the corpus, the dotenv file, the reader's module -
//! is resolved by [`Resolve`], so the three of them cannot disagree about what
//! "beside the crate" means. The order is the same for all three: **a path the
//! caller named, the copy that travels with this crate, and one absolute
//! location as the last resort**.
//!
//! - A named path is read from the crate when it is relative, so
//!   `--corpus Templates` means this crate's `Templates` and not whatever
//!   directory the shell happened to be in. An absolute path is taken as it is.
//! - The crate's own copy is found by walking up from the running executable to
//!   the first ancestor holding a `Cargo.toml` and a `Source/`. Nothing is
//!   compiled in, so the binary does not carry the path of the machine that
//!   built it and a clone that moves still runs.
//! - The absolute location is read only when the crate's copy is absent. It is
//!   the one path in this crate that names a fixed place, and it is there for an
//!   installation that keeps its data outside the tree.
//!
//! A named path is never checked for existence: a caller who names a corpus
//! that is not there gets the loader's sentence about that path, rather than a
//! silent run against files nobody asked for.

use std::{
	env,
	path::{Path, PathBuf},
};

/// The directory holding this crate.
///
/// `target/<profile>/<binary>` is two directories below the crate, so the walk
/// stops at the first ancestor that holds both a `Cargo.toml` and a `Source/` -
/// the two things every crate here has and no build directory does. A binary
/// that was moved away from its tree falls back to its own directory, and every
/// relative candidate then misses, which leaves the absolute location.
pub fn Crate() -> PathBuf {
	let Start = env::current_exe()
		.ok()
		.and_then(|Exe| Exe.parent().map(Path::to_path_buf))
		.or_else(|| env::current_dir().ok())
		.unwrap_or_else(|| PathBuf::from("."));
	let Found = Start
		.ancestors()
		.find(|At| At.join("Cargo.toml").is_file() && At.join("Source").is_dir())
		.map(Path::to_path_buf);
	Found.unwrap_or(Start)
}

/// One path, from the three candidates in the order the module states.
///
/// `Relative` is joined to `Crate`, never to the working directory: the same
/// binary run from anywhere reads the same files. `Absolute` is returned
/// whether or not it exists, so the caller reports its own failure against the
/// path it actually tried.
pub fn Resolve(Crate:&Path, Given:Option<&str>, Relative:&str, Absolute:&str) -> PathBuf {
	if let Some(Named) = Given.map(str::trim).filter(|Named| !Named.is_empty()) {
		let Named = Path::new(Named);
		return if Named.is_absolute() {
			Named.to_path_buf()
		} else {
			Crate.join(Named)
		};
	}
	let Beside = Crate.join(Relative);
	if Beside.exists() {
		return Beside;
	}
	PathBuf::from(Absolute)
}
