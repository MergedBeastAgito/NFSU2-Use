# Asking Perplexity from a task

A card that is blocked on a decision or on a fact it cannot read from the tree
must ask rather than guess. This is the procedure, and it is the same one a human
uses - except that a headless worker runs it end to end, with no person to open a
tab.

---

## The rule

**Ask before assuming.** A card that finds a gap - an unknown API shape, an
unclear standard, a trade-off with no data - writes the question down and asks.
The question and the answer are both artefacts; a card that guessed leaves no
trace of why.

---

## How to ask

The tab is opened by the task itself, in the browser that is already running -
the same one the extension lives in, on `127.0.0.1:9222`. There is nothing
headless about it and nothing for a person to do:

```sh
node Documentation/Workflow/ask.mjs --topic <Topic>
```

It reads `Documentation/Research/<Topic>.question.txt`, opens a tab, types the
question into the composer, waits for the answer to finish, files
`Documentation/Research/<Topic>.md`, and closes the tab again.

Nothing is focused and nothing is navigated away from: the tab is created with
`Target.createTarget` and `background: true`, so the person's own tabs stay where
they were, and the login is the profile's own because the profile is the one
already running.

What it needs:

- **A browser listening on CDP.** `http://127.0.0.1:9222` by default, or `--cdp`
  or `$CDP`. It must be the profile that is logged into Perplexity: a fresh
  profile is a browser with no session, and the run stops with a blocker rather
  than asking a stranger's page.
- **Node 22 or newer.** The driver uses the global `fetch` and `WebSocket` and
  has nothing to install.

Measured cost: a 22,000-character answer took about 134 seconds end to end. The
default `--timeout 300` covers a long answer; do not set it below 180.

A tab a task opened is a tab the task closes when the capture is written - the
driver does, unless `--keep` is passed. Leaving research tabs behind fills the
person's browser with work they did not ask to see.

---

## Pre-warm the question

A question with no context gets generic advice, so the question is a file, not a
paragraph, and it is sent byte for byte. Before asking, attach:

1. **The tree** -
   `find . -type f -not -path './.git/*' -not -path '*/target/*' | sort`.
2. **The conventions** - PascalCase identifiers, full-cased acronyms (URL, URI,
   SDK, ID, JSON, WASM, CDP), the `Source/` layout, the `module.json` vocabulary.
3. **The repositories** - what each one is and which is public.
4. **What already exists** - the schemas, the lanes, the measured numbers, so the
   answer builds on them instead of restating them.
5. **The questions**, numbered, last, and answerable one at a time.

A pre-warmed question is a file, not a paragraph: write it, review it, then send.
Two rules on the file:

- **The first line is the check.** It is one plain sentence that has to appear
  verbatim in the thread, because the run looks for it to prove the page received
  the question. Do not open with a heading, a code fence or a path.
- **Ask it to argue against itself.** A recommendation with no downside is a
  sales pitch: ask where the design is worse, what it costs, and what would make
  the model drop its own answer.

The question is a file beside the capture it will produce, so the two are read
together: `Documentation/Research/<Topic>.question.txt`.

---

## What the exit codes mean for the card

`ask.mjs` prints `result: ok` and the file it wrote, or one `blocker:` line. The
exit code says which kind of failure it was, and each has one response:

- **0 - filed.** The answer is a file. Link it from the decision it informed.
- **1 - the question is the problem.** Missing, empty, under 40 characters, or no
  first line long enough to find again. Fix the file; nothing was sent.
- **2 - no browser.** The endpoint did not answer, or refused a tab. This is
  infrastructure, not the page: comment the blocker on the card and block it as a
  dependency, naming the endpoint and the command that failed.
- **3 - the page refused the question.** No composer, no Search/Computer toggle, a
  mode that would not change, a submit that did not take, or a question that never
  appeared in the thread. Run `--probe` once to see what the driver sees before
  changing anything else; a Perplexity build change is the usual cause.
- **4 - partial, not an answer.** The answer did not settle inside the timeout, or
  it settled shorter than `--min-answer`. The capture is written to
  `<out>.partial` and **is not filed**: read it, then re-run with a longer
  `--timeout` or a narrower question. A partial answer filed as the answer is
  worse than no answer, because the next card builds on it.

---

## What the refusals look like

Each way the page can fail has one blocker line and one exit code, so a run that
stopped says which of them it hit rather than "it did not work". The modes the
driver refuses - no composer, no Search/Computer toggle, a toggle that will not
take a press, a question the page never accepts, an answer that never grows - are
the five exit `3` and exit `4` cases in the table above, and each is reported
before anything is filed.

---

## Search mode is mandatory

A research product answers differently in its search mode and its agent mode: one
reads sources and quotes them, the other browses and acts. For research, Search is
not a preference - it is the mode the question was written for - and an answer
from Computer mode looks exactly like a right one.

So the driver asserts it before a character is typed:

- Search pressed, Computer not pressed - type.
- Computer pressed - click Search with a real pointer press, re-read, and only
  then type.
- Still not pressed - exit `3` and send nothing.
- **No toggle at all - exit `3` as well.** A composer without one is a composer
  the driver does not understand, and an unknown mode is the thing the guard
  exists to refuse.

The mode and the model are written into the capture's header, because a reader
who cannot tell which one answered cannot weigh the answer.

---

## What counts as an answer

Settled, not merely non-empty. The driver resets its quiet clock while the page
reports itself busy (its stop control, its in-flight flag), requires the text to
have grown past the question echo, and then requires six seconds in which nothing
changes. Growth is measured on the page's own text rather than on one element, so
a short answer smaller than a long pre-warmed question still counts as growth.

Below `--min-answer` characters it is a partial answer and it is not filed.

The capture is the page's rendered text, which includes the question echo and the
chrome around the answer. That is deliberate: the echo is the proof the page
received the question, and the run's last line, `question-in-capture: true`, is
the same check a person would make with `grep`.

---

## Where answers go

`Documentation/Research/<Topic>.md`, one file per thread, named for the topic
rather than the date; the question stays beside it as `<Topic>.question.txt`. The
capture opens with `chars`, `mode`, `model`, `question` and `captured`, so the
answer carries its own provenance.

That directory is a working area and git ignores it: the captures stay on disk,
they are not repository content. What a capture settles is written into
`Documentation/` as prose, and an answer that changes a design decision is
summarised in the document that records the decision. A transcript is evidence
that a conversation happened; the document is the decision itself.

---

## Never run the game's own tools

`Stem/*` is reference source, not a tool to execute. In particular
`nfsu2-web/web/tools/drive.mjs` calls `osascript -l JavaScript` to activate a
macOS application by PID through AppKit:

```js
execFileSync('osascript', ['-l', 'JavaScript', '-e',
  `ObjC.import('AppKit'); $.NSRunningApplication.runningApplicationWithProcessIdentifier(${pid}).activateWithOptions(...)`])
```

With a stale or wrong PID that activates an arbitrary application on the person's
desktop - it has already surfaced an About This Mac window. It is a desktop tool
for the game's authors, not a browser tool, and it has no place in a task run.

Read `Stem/` freely. Never execute from it.

---

## When not to ask

- When the tree already answers it. Read the source first; the port's own C
  names every address we use.
- When a measurement would answer it faster. Two numbers beat a paragraph.
- When the question is a preference. Ask the person, not a model.

---

## Asking the person

Decisions that need a human - which repository is authoritative, whether to force
push, whether a privacy risk is acceptable - are asked directly and plainly, with
the evidence that makes the decision possible and the consequence of each choice.

A card asks a model for facts and a person for authority. Confusing the two is how
a card ends up waiting for an answer nobody was asked for.

---

## Formatting a capture

A raw capture is the accessibility tree: the sidebar, the account name, the mode
buttons and the follow-up chips are all in it, and none of it is the answer.
Every capture is formatted before it is kept, and the formatter is a verb of the
workspace's own binary rather than a script beside it:

```sh
agito chat --url <thread-url> --question "<question>" --format note
```

The note it prints is the shape a kept capture has: YAML frontmatter (title,
thread, project, kind, date, status, source, captured, segments, steps,
sources), a metadata table, and a `## Transcript` with the chrome stripped - the
same shape the Agito workspace's own research captures used, so the corpus reads
as one thing rather than thirteen captures. Reading the page as Markdown rather
than as rendered text is what keeps a formula as its TeX instead of as one glyph
per line.

---

## If the page changes

The driver reads the composer, the toggle group (`[data-command-menu-control]`),
the submit button and the busy signals from the live page. A Perplexity build can
rename any of them.

```sh
node Documentation/Workflow/ask.mjs --probe
```

prints what the driver sees on a fresh tab and sends nothing, which separates "the
page changed" from "the browser is not there". A build with no stop control and no
in-flight flag relies on the quiet window alone, so raise `--stable` and
`--min-answer` when a capture looks truncated.

The other knobs: `--url` (ask a different chat product), `--wait` (how long a
composer may take to appear), `--timeout`, `--stable`, `--min-answer`, `--cdp`,
`--keep` (leave the tab open, for a person to read the thread afterwards) and
`--trace <path>` (one JSON line per read, which is how the settle rule was tuned).

---

## Why the driver is in the repository

The earlier version of this page named the agent profile's own drivers
(`cdp-send` and `cdp-read`) and a tab a person had opened. A clone of this
repository inherits neither, a headless worker cannot wait for the person, and
the profile's `cdp-send` does not in fact read the mode toggle its documentation
says it refuses on. So `ask.mjs` carries the primitives - background target,
mode assertion, text typing, settlement - inside the repository, where a card
can run them and a clone can read them.
