//! Compile `Source/Script/**/*.ts` into the JavaScript this crate sends.
//!
//! The rule lives in Agentic's `Build/Script.rs` and is reused by path, so this
//! crate follows the same convention as `Crate/ChatDriver` and
//! `Crate/Workflow`: one expression per file under
//! `Source/Script/Expressions/`, the file is the thing an editor and a reader
//! check, and Rust never builds JavaScript with `format!`.
//!
//! What differs is the compiler. Agentic erases type syntax with OXC because
//! its scripts carry annotations; every script here is written in the
//! JavaScript subset, so the smallest transformation that makes TypeScript into
//! JavaScript is the identity, and this build copies the bytes and records the
//! declared parameters. A script that ever needs a type annotation fails the
//! build instead of silently shipping one.

// The house style is PascalCase everywhere, including the locals of this build
// script, so the lint that disagrees with it is off here as it is in the crate's
// own library.
#![allow(non_snake_case)]

use std::{env, fs, path::Path};

fn main() {
	let Out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("Script");
	let Root = Path::new("Source/Script");
	fs::create_dir_all(&Out).expect("OUT_DIR/Script");
	Walk(&Root.join("Expressions"), "Expressions", &Out);
	println!("cargo:rerun-if-changed=Source/Script");
}

fn Walk(Directory:&Path, Prefix:&str, Out:&Path) {
	let Ok(Entries) = fs::read_dir(Directory) else { return };
	for Entry in Entries.flatten() {
		let Path = Entry.path();
		let Name = Path.file_stem().unwrap_or_default().to_string_lossy().to_string();
		if Path.extension().is_some_and(|e| e == "ts") {
			let Source = fs::read_to_string(&Path).expect("read script");
			// A `declare const __PARAM_X__: string;` is both the declaration a
			// reader sees and the parameter the build records. The line itself
			// is removed, because it is not JavaScript.
			let mut Body = String::new();
			let mut Params = String::new();
			for Line in Source.lines() {
				let Trimmed = Line.trim();
				if let Some(Rest) = Trimmed.strip_prefix("declare const __PARAM_") {
					let (Name, Kind) = Rest.split_once("__:").expect("declare const __PARAM_Name__: type;");
					let Kind = match Kind.trim().trim_end_matches(';') {
						"string" => "text",
						Other => Other,
					};
					Params.push_str(&format!("{}:{}\n", Name.trim(), Kind));
					continue;
				}
				if Trimmed.starts_with("export ") {
					panic!("{Name}: a script is one expression, not a module");
				}
				Body.push_str(Line);
				Body.push('\n');
			}
			if Body.contains(": string") || Body.contains(": number") || Body.contains("interface ") {
				panic!("{Name}: this crate's scripts must be plain JavaScript; add a type eraser first");
			}
			let Directory = Out.join(Prefix);
			fs::create_dir_all(&Directory).expect("OUT_DIR/Script/<Prefix>");
			fs::write(Directory.join(format!("{Name}.js")), Body).expect("write js");
			if !Params.is_empty() {
				fs::write(Directory.join(format!("{Name}.params")), Params).expect("write params");
			}
		} else if Path.is_dir() {
			Walk(&Path, &format!("{Prefix}/{Name}"), Out);
		}
	}
}
