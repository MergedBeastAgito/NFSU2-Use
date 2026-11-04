//! `agentic.nfsu.reader`: the game's car state, read from inside the game's own
//! WebAssembly memory.
//!
//! A tick used to be an `Evaluate` round trip: a string of JavaScript crossed
//! the debugger boundary, was parsed, and read the memory through a `DataView`.
//! This module is instantiated beside the game's instance and imports its
//! memory, so a tick is a handful of calls that return numbers.
//!
//! Five facts shaped the code. Each was measured against the running port, not
//! assumed:
//!
//! - The game's memory is **shared** (`web/src/host/memory.ts`, `allocMemory`:
//!   `new WebAssembly.Memory({ shared: true })`), so this module's import
//!   declares it shared and the build carries `+atomics`. An import without the
//!   flag is refused: the engine will not hand a shared memory to it.
//! - The imported memory **is the game's heap**. An allocator here would hand
//!   out the game's own blocks and corrupt it, so the crate is `#![no_std]`
//!   with no allocator, and the built artefact has no `memory.grow`, no
//!   `__rust_alloc`, no stack pointer and **no data segment**. The data segment
//!   matters as much as the allocator: wasm applies it at instantiation, at an
//!   address the linker chose, which would overwrite the game's own low memory
//!   before a single reading was taken.
//! - The result cannot be written to memory either, for the same reason. Every
//!   reading is returned, never stored - and a returned value is the whole
//!   transport budget.
//! - That budget is **one 64-bit value per call**, and it cannot be widened:
//!   Rust's wasm ABI returns a struct through a hidden pointer (measured: a
//!   `#[repr(C)]` struct and a `u128` both compile to `(i32) -> nil`), and the
//!   one 128-bit value the ABI does have, `v128`, is refused by the JavaScript
//!   API at the call site (`TypeError: type incompatibility when transforming
//!   from/to JS`). Seven numbers therefore take several calls, and
//!   [`read_state`] carries the two that move every frame. A caller comparing
//!   against the page's own reader must read the game twice and accept only the
//!   ticks where both reads agree - the physics runs on other threads.
//! - A panic must not take the game down. [`Panic`] calls `unreachable()`, so a
//!   panic is a trap: the page catches a `RuntimeError`, and the game's own
//!   instance never sees it. [`panic_probe`] exists to prove exactly that.
//!
//! The offsets are the port's own (`runtime/roam_game.c`, `local_state`, and
//! `runtime/career.c`, `G_PLAYER1`), so this is a read of a known layout rather
//! than a search through two gigabytes.

#![no_std]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core::{
	arch::wasm32::{memory_size, unreachable},
	panic::PanicInfo,
};

/// `PlayersByNumber[0]`: `Player*` (`runtime/career.c`, `G_PLAYER1`).
const Player1:u32 = 0x00D0_0000;
/// `Player + 4` is that player's `Car*`.
const PlayerCar:u32 = 4;
/// `Car + 0x60` is `float[3]` position, metres, world space.
const CarPosition:u32 = 0x60;
/// `Car + 0x40c` is the heading as a `u16`: 65536 to a turn.
const CarHeading:u32 = 0x40C;
/// `Car + 0x430` is the physics' speed; `+0x42c` is the one the dashboard
/// shows.
const CarSpeed:u32 = 0x430;
/// `Car + 0x434` is the gear (`i8`), `+0x440` the engine speed (`f32`).
const CarGear:u32 = 0x434;
const CarRpm:u32 = 0x440;
/// The car block `local_state` insists on before it reads anything
/// (`ok(car, 0x9e0)`); below that the pointer is stale and the answer is "no
/// car" rather than a read of somebody else's memory.
const CarBytes:u32 = 0x9E0;

/// The NaN a reading carries when there is no car. A car parked at the origin
/// and a world that has not given one are different answers, and only one of
/// them is a number.
const NoCar:f32 = f32::NAN;

/// Whether `Bytes` at `Address` lie inside the memory the game gave us.
///
/// This is what keeps a stale pointer from becoming a trap. A wasm load past
/// the end of the memory traps, and a trap here would be an exception thrown
/// into the page's tick loop - the game would survive, but the tick would not.
/// `memory_size` is the imported memory's current length in pages; it grows
/// with the game, so the check is against what the game has now, not what it
/// had at attach.
#[inline(always)]
fn Committed(Address:u32, Bytes:u32) -> bool { (Address as u64) + (Bytes as u64) <= (memory_size(0) as u64) << 16 }

// The reads are unaligned on purpose: the game's structs are packed by a 32-bit
// build, so a `u16` at `+0x40c` is not guaranteed to sit on its natural
// boundary, and an aligned load would be a trap rather than a slow read.

#[inline(always)]
fn Read32(Address:u32) -> u32 { unsafe { core::ptr::read_unaligned(Address as *const u32) } }

#[inline(always)]
fn Read16(Address:u32) -> u16 { unsafe { core::ptr::read_unaligned(Address as *const u16) } }

#[inline(always)]
fn Read8(Address:u32) -> u8 { unsafe { core::ptr::read_unaligned(Address as *const u8) } }

#[inline(always)]
fn ReadF32(Address:u32) -> f32 { unsafe { core::ptr::read_unaligned(Address as *const f32) } }

/// Two `f32` in one returned value: the low half is `A`, the high half `B`.
/// Their bit patterns travel unchanged, so a caller gets the game's own float
/// and not a rounded copy of it.
#[inline(always)]
fn Pair(A:f32, B:f32) -> i64 { (A.to_bits() as u64 | (B.to_bits() as u64) << 32) as i64 }

/// The player's car, or 0.
///
/// The same pointer chain the port walks, with every step bounded: a world that
/// has not given a player yet answers "no car", which is a different answer
/// from a car parked at the origin.
fn Car() -> u32 {
	if !Committed(Player1, 8) {
		return 0;
	}
	let Player = Read32(Player1);
	if Player == 0 || !Committed(Player, 8) {
		return 0;
	}
	let Car = Read32(Player + PlayerCar);
	if Car == 0 || !Committed(Car, CarBytes) {
		return 0;
	}
	Car
}

/// Position x and y (metres) as two `f32` in one `i64`: the two values that
/// change on every frame of a moving car.
///
/// Both halves are NaN when there is no car. Decode with a `Uint32Array` /
/// `Float32Array` pair over one buffer: the low half is x, the high half is y.
#[unsafe(no_mangle)]
pub extern fn read_state() -> i64 {
	let Car = Car();
	if Car == 0 {
		return Pair(NoCar, NoCar);
	}
	Pair(ReadF32(Car + CarPosition), ReadF32(Car + CarPosition + 4))
}

/// Position z, metres - height in world space, which a driver needs once a
/// corner rather than on every frame.
#[unsafe(no_mangle)]
pub extern fn read_position_z() -> f32 {
	let Car = Car();
	if Car == 0 {
		return NoCar;
	}
	ReadF32(Car + CarPosition + 8)
}

/// Heading in degrees, from the `u16` at `+0x40c`, or NaN when there is no car.
///
/// `360 / 65536` is exact in `f32` (65536 is a power of two), and the product
/// of the two is at most 22 bits, so this is the very number a `f64` page
/// computes for the same `u16` - not a rounded one.
#[unsafe(no_mangle)]
pub extern fn read_heading() -> f32 {
	let Car = Car();
	if Car == 0 {
		return NoCar;
	}
	(Read16(Car + CarHeading) as f32) * (360.0 / 65536.0)
}

/// The physics' speed, m/s (negative in reverse), or NaN.
#[unsafe(no_mangle)]
pub extern fn read_speed() -> f32 {
	let Car = Car();
	if Car == 0 {
		return NoCar;
	}
	ReadF32(Car + CarSpeed)
}

/// Engine speed, rpm, or NaN.
#[unsafe(no_mangle)]
pub extern fn read_rpm() -> f32 {
	let Car = Car();
	if Car == 0 {
		return NoCar;
	}
	ReadF32(Car + CarRpm)
}

/// The gear as the game stores it (`i8`, so -128..127), or 0 with no car.
#[unsafe(no_mangle)]
pub extern fn read_gear() -> i32 {
	let Car = Car();
	if Car == 0 {
		return 0;
	}
	Read8(Car + CarGear) as i8 as i32
}

/// The car's address, 0 when the world has not given one: the caller's validity
/// test, and the one number that says whether the rest mean anything.
#[unsafe(no_mangle)]
pub extern fn read_car() -> u32 { Car() }

/// A deliberate panic, for the containment test: the page must catch a
/// `RuntimeError` and the game must keep running afterwards.
#[unsafe(no_mangle)]
pub extern fn panic_probe() -> i32 { None::<i32>.unwrap() }

/// A panic is a trap, never an unwind: there is no allocator to unwind into and
/// no destructor to run, and the page sees an exception it can catch. The
/// game's own instance is a different module and is untouched.
#[panic_handler]
fn Panic(_:&PanicInfo) -> ! { unreachable() }
