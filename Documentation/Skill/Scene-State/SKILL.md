---
name: scene-state
description: Use when a decision model drives a game from the game's own memory.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [nfsu2, decision-model, clef, state, surroundings, escape]
---

# Scene State

A decision model is only as good as the sentence it is given. Numbers without a description are
answered as if the world were empty: a car pressed against a wall at full throttle still reports its
wheel speed, so `speed_kmh: 46` reads exactly like driving - and the model answers `throttle`.

---

## The dashboard is the wheels, not the car

The speed in memory is the wheels' speed. Against a wall the wheels turn and the car does not move,
so no single read of it can separate "driving" from "spinning". Measure progress instead: keep the
previous position and accumulate the distance over a window of ticks.

```
window        6 ticks
advancing     >= 1.5 m per completed window (about 11 m at 40 km/h, so only a scraping car falls under it)
insist        3 ticks before a verdict

moved_m       metres advanced in the last completed window
turned_deg    heading change over the same window, wrapped to a half turn
blocked_ticks consecutive ticks with the throttle held and the window under advancing
verdict       moving | creeping | blocked | stalled
```

Two of those verdicts carry the work: `blocked` (throttle held, no metres, three ticks running) and
`creeping` (the same before the third tick confirms it). A window that has not completed yet is
evidence of neither - presume moving, or the first tick of a run reverses out of an empty road.

---

## Send the sentence, not only the numbers

```json
{
	"verdict": "blocked",
	"situation": "the throttle is held and the car has advanced only 0.6 m over the last 6 ticks, turning 18° without getting free - it is pressed against something (a wall, a kerb or a pole) and cannot go forward",
	"speed_kmh": 5,
	"gear": 2,
	"moved_m": 0.6,
	"turned_degrees": 18,
	"blocked_ticks": 3
}
```

The sentence is what changes the answer. Given the numbers alone the model answered `throttle` at a
wall; given the sentence it has something to disagree with. The rules read the same verdict, because
a rule that escapes a wall from the speed alone never fires on the car that needs it most.

---

## The escape has to be an option

A question whose options are `steer_left | steer_right | throttle | brake | hold` cannot be answered
"reverse out", however well the model understands the wall. When the verdict is jammed, the question
changes: `reverse_left` and `reverse_right` are added, and the instructions say plainly that
reversing with the wheel turned is how a car leaves a wall.

- **An obeyed reverse is a manoeuvre, not a tick.** Hold it for several ticks (five here). A 150 ms
  tap of the reverse key moves nothing and the next tick drives back into the wall.
- **Hold the side that was chosen.** Recomputing the side from an attempt counter turns the
  manoeuvre the other way from the one the model picked.
- **Finish what is under way.** While a manoeuvre runs, an answer that is not an escape is gated:
  `throttle` would drive back into the wall, `steer` would abandon the turn carrying the car away.
- **Refuse a reverse on an open road.** `reverse_left` while `moving` is a misreading, not a
  decision.

---

## The escape is a reverse *and* a turn

Reversing alone leaves the car a metre further from the wall and still pointing at it, so the next
tick drives back in - and a rule that alternates sides without turning saws at the same corner
indefinitely. The manoeuvre therefore hands over to a turn on the side it reversed on, held while
the throttle comes back up:

```
tick  1-5   reverse <side>          (no throttle)
tick  6-15  steer <side> + throttle (the turn out)
tick 16+    the ordinary rules
```

Measured on a car wedged in a corner: five ticks of reverse, ten of turn, then 40 km/h in a new
direction - where the same car without the turn alternated reverses for a hundred and fifty ticks
without leaving.

---

## Claim → test

| claim | how it is checked |
| --- | --- |
| wheel speed cannot say "blocked" | a car at 46 km/h with a fixed position: the verdict becomes `blocked` at `window + insist` ticks |
| a moving car is never blocked | a car advancing 2.5 m a tick for 24 ticks stays `moving` |
| a reverse is held, not tapped | an obeyed reverse sets the manoeuvre counter; the held ticks keep the side |
| a manoeuvre ends in a turn | the last backing tick sets the turn, on the side the reverse used, with the throttle on |
| the model may pick the side | a jammed scene plus `reverse_right` above the gate obeys and holds the right |
