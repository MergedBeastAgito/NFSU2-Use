# Schemas

Two files, one pipeline: `Observation.json` is what the game gives us,
`Questions.json` is what we ask the model about it. Together they compose into
the request the driver sends.

---

## The request

```
POST https://api.cloudflare.com/client/v4/accounts/<account>/ai/run/@cf/cloudflare/<model>
{ "model": "clef-flash", "state": <fields from Observation.json>, "questions": <one template> }
```

The answer comes back per question: a `choice` yields the chosen option and a
probability for every option, a `score` yields a weighted value and its legend.
Clef does not bill output tokens, so a question costs only what its state costs
to read - measured at 343 input tokens for the cruise template, about $0.000013
on `clef-flash`.

---

## Why typed questions

A decision model that returns free text has to be parsed, and a parse failure at
40 km/h is a crash. A typed question cannot answer outside its options: the
model returns a probability per option, so "steer left" at 0.11 and "throttle"
at 0.53 is a decision the driver can threshold rather than interpret.

That is also why `score` questions exist beside `choice` ones: `confidence` says
how much to trust the answer, which is the difference between an advisor worth
obeying and one worth recording.

---

## The templates

| template             | when it applies                             |
| -------------------- | ------------------------------------------- |
| `freeroam.cruise`    | no cue on screen, under target pace         |
| `freeroam.track_cue` | the route arrow is visible                  |
| `corner.entry`       | approaching a turn above the cornering pace |
| `recovery.stuck`     | speed under 2 km/h for three ticks          |
| `race.start`         | `racetype` is 1 or 7                        |
| `surface.loss`       | slip above 5 on a road, or a wheel off-road |
| `traffic.ghost`      | other cars near while ghost mode is on      |

Each names the condition it belongs to in `when`, so a driver selects by state
rather than by guessing which question is current.

---

## What the model actually does with these

Measured on `clef-flash` with the `freeroam.track_cue` template and a 38 km/h
state, twice, with the direction convention spelled out in the instructions:

- cue at 0.42 (road turns left): `steer_left 0.32`, `steer_right 0.27`,
  `throttle 0.36` - it chose **throttle**, and its left/right ordering was
  right.
- cue at 0.58 (road turns right): `steer_left 0.27`, `steer_right 0.23`,
  `throttle 0.44` - again **throttle**, and its left/right ordering was wrong.

Two things follow, and both are load-bearing:

- **The templates are structurally sound.** Every call returned a choice and a
  probability per option, 342 input tokens, no output tokens.
- **The steering judgement is not good enough to drive from.** Near-uniform
  probabilities on a single 9B model over a text state is a coin flip with a
  margin. That is why the model advises and the rules decide: the driver obeys a
  probability above a threshold and records everything else.

The honest next step is not a better prompt but a better observation: an image
of the frame beside the text state, since Clef is multimodal and the cue is
visual. Until that is measured, this template is a template, not a controller.

---

## Lanes

A lane is a request _shape_: the fields a question actually reads, not all
sixteen. `Questions.json` declares five - `drive`, `recover`, `corner`, `race`,
`context` - each naming its fields, its templates and how often it is worth
sending.

Measured with the same question (`freeroam.track_cue`) against `clef-flash`:

| state sent            | input tokens | latency |
| --------------------- | ------------ | ------- |
| drive lane, 3 fields  | 375          | 451 ms  |
| full state, 16 fields | 499          | 268 ms  |

Two conclusions, one of which is not the obvious one:

- **Tokens fall by about a quarter** (124 of 499), which is money and context
  budget: on `clef-flash` that is $0.0000047 saved per call.
- **Latency does not fall.** The three-field call was the _slower_ of the two,
  because at this size the round trip is network variance rather than payload. A
  faster lane comes from calling less often (`--every`) and from answering on
  the page, not from shrinking the body.

So the lanes exist to save tokens and to keep each question honest about what it
reads; the speed comes from cadence.

---

## Adding one

Add a key to `Questions.json` with `when` and `questions`. Every question is
`noul` (a probability), `choice` (options with criteria) or `score` (ordered
criteria). If a template needs a field the state does not carry, add it to
`Observation.json` with its address first - a question about a field the
ingestion does not send is a question the model cannot answer.
