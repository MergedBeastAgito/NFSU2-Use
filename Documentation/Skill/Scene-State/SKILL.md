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

## Read the port's own source before scanning its memory

The port ships its probes: `runtime/roam.c` walks the world's cars, names the
globals, and prints the rows this skill describes (`roam cars`). A pointer
table found by pattern-matching floats is a guess that survives until the game
changes; a global named in the source is a fact. When an address here disagrees
with the source, the source is right.

The map that matters, and what it buys:

```
G_WORLD   0x00890080  World*: +0 the car list head, +0x10 count
G_FLOW    0x008654a4  GameFlowManager: 3 front end, 6 in the world
G_TRACK   0x0089e7a0  the track: 4000 is the free-roam city
car +0x24            driver type: 1 the player, 3 an AI racer, 4 traffic
car +0x0c            the car type info; +0 of it is the name
```

`Documentation/Memory.md` holds the full table and the walk. Two consequences
for a question:

- **Traffic is measured, not inferred.** Every car in the world is in one list,
  and each entry is the same struct the player drives, so "a car 23 m ahead,
  closing at 14 km/h" is a read rather than a guess from a frame.
- **The game says what a car is.** `+0x24` separates traffic from an AI racer
  from a friend's car; a model told which one it is has something to reason
  with that speed and position cannot give it.

---

## The instructions do the work, not the numbers

A measured finding worth keeping: with the traffic question and its numbers in
place, `clef-flash` answered `throttle` on 37 of 44 calls made with a car under
60 m ahead - and it was right to. The question said ghost mode was on, so the
car ahead could not be hit. The instruction told the model the traffic was not
a hazard, and the model believed it.

So a following question has to ask about **pace and road position**, not about
safety, or it is asking a question whose answer is already in the instructions.
What a question should carry:

- the gap, the other car's speed, and the rate the gap is changing;
- whether it is in our lane (`|right| <= 1 + ahead * 0.25` is roughly a lane);
- what the car is (traffic, an AI racer, a friend) and what type;
- and a stated consequence that is true - ghost mode means contact cannot
  happen, so the decision is about the line and the pace.

---

## Rule → what it rests on

| rule | the observation behind it |
| --- | --- |
| wheel speed cannot say "blocked" | a car at 46 km/h against a wall reports the same wheel speed as a car at 46 km/h on a road, while its position stops changing; six ticks of position separate the two |
| a moving car is never blocked | 2.5 m a tick is 15 m over the window, ten times its 1.5 m threshold |
| a reverse is held, not tapped | a 150 ms tap of the reverse key moved nothing; five ticks of it moved the car clear |
| a manoeuvre ends in a turn | five ticks of reverse then ten of turn put a wedged car at 40 km/h in a new direction, where reverse alone alternated for 150 ticks without leaving |
| the model may pick the side | the answer names the side, and the manoeuvre holds the side it was given rather than recomputing one |
| the car list is the game's, not a scan | the walk starts at `World (0x00890080) +0` and ends when the node is the world again; `runtime/roam.c` `log_cars` is the reference |
| traffic is a read, not a guess | a car 20 m ahead appears with its own speed and closing rate in the same tick, without a frame |
| the question changes with the scene | the trace's `asked` field reads `blocked` at a wall, `traffic` behind a car in lane, `open` otherwise |
