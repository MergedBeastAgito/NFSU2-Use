/**
 * Install the page-side driver: one object that reads the car and holds keys.
 *
 * Holding a key here means re-asserting it every 16 ms, because the game's input
 * ring takes one frame per call - a single `input.key(code, true)` reads as a
 * tap. Measured: a 16 ms interval moved the car 11 m in 4 s, where one call
 * moved it 0.2 m.
 *
 * The addresses are the port's own (`runtime/roam_game.c`): the wasm build puts
 * `PlayersByNumber[0]` at 0x00d00000, the car is its `+4`, position `+0x60`,
 * heading `+0x40c` (u16, 65536 a turn), signed speed `+0x430`, rpm `+0x440`,
 * gear `+0x434`. Reading them is the whole state feed.
 */
(() => {
	if (window.__drv) return "already";
	const dv = new DataView(window.__nfsu2.session.proc.memory.buffer);
	const u32 = (a) => dv.getUint32(a, true);
	const f32 = (a) => dv.getFloat32(a, true);
	const u16 = (a) => dv.getUint16(a, true);
	const Held = {};
	window.__drv = {
		state() {
			const Player = u32(0x00d00000);
			const Car = Player ? u32(Player + 4) : 0;
			if (!Car) return { car: 0 };
			const Speed = f32(Car + 0x430);
			return {
				car: Car,
				pos: [+f32(Car + 0x60).toFixed(1), +f32(Car + 0x64).toFixed(1), +f32(Car + 0x68).toFixed(1)],
				heading: +((u16(Car + 0x40c) / 65536) * 360).toFixed(1),
				speed: +Speed.toFixed(2),
				kmh: Math.round(Math.abs(Speed) * 3.6),
				rpm: Math.round(f32(Car + 0x440)),
				gear: dv.getInt8(Car + 0x434),
			};
		},
		hold(Key, On) {
			if (On && !Held[Key]) Held[Key] = setInterval(() => window.__nfsu2.session.input.key(Key, true), 16);
			if (!On && Held[Key]) {
				clearInterval(Held[Key]);
				delete Held[Key];
				window.__nfsu2.session.input.key(Key, false);
			}
		},
		releaseAll() {
			for (const Key of Object.keys(Held)) {
				clearInterval(Held[Key]);
				delete Held[Key];
				window.__nfsu2.session.input.key(Key, false);
			}
		},
		publish(Payload) {
			const Debug = window.__hermesDebug;
			if (!Debug) return 0;
			Object.assign(Debug, Payload);
			if (Debug.paint) Debug.paint();
			return 1;
		},
	};
	return "installed";
})()
