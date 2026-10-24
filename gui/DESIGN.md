# DESIGN.md — rxui

The desktop surface for `rx`. **Mode: Operate** — the visitor completes a task;
scanability, confidence and a calm hand outrank expression. Brand lives in the
details, not in decoration.

## World: instrument panel

A NixOS update is a sequence of irreversible-ish actions on a running machine.
The surface reads like a well-machined instrument: deep ink, hairlines, one
signal colour, everything else quiet. The existing README banner already owned
this world (#0D1117 ink, blue → violet → pink accent); the app extends it rather
than replacing it.

Composition: **rail / stage / status line**.

- Left rail (296 px): the pipeline, the newest generations, the machine's
  identity. Vertical beam connects the steps; it is the only element that
  animates.
- Stage: an eyebrow, one headline, one sentence, one primary action, then the
  live console well.
- Status line: state, elapsed, `rx <version>`, CLI presence.

Nothing is centred for its own sake. The console is a well, and emptiness in a
well reads as "ready", not as "unfinished".

## Tokens (`gui/src/theme.rs`)

| Token | Value | Use |
|---|---|---|
| `INK` | `#0A0E15` | canvas |
| `INK_RAISED` | `#0E141E` | sheet / plan card |
| `INK_SUNKEN` | `#070A10` | console well |
| `LINE` / `LINE_SOFT` | `#1C2735` / `#141C27` | 1 px rules |
| `TEXT` / `MUTED` / `FAINT` | `#E9F0F7` / `#93A2B5` / `#5F6E81` | three-step type scale |
| `BLUE` → `PINK` | `#58A6FF` → `#F778BA` | primary action, beam, section headers |
| `GREEN` / `RED` / `AMBER` | `#3FB950` / `#F85149` / `#E3B341` | success / failure / dry-run only |

Type: **Inter** (400 / 500 / 600) for human copy, **JetBrains Mono** for the
machine's voice (commands, durations, numbers, section headers, chips). Both
are bundled under the OFL. Scale: 30 px headline, 12.5 px body, 10.5 px console,
9–9.5 px eyebrows and meta.

## Rules

1. **One accent.** The aurora gradient appears on the primary action, the brand
   mark and the beam. If a third use appears, it is a bug.
2. **Colour is state, never decoration.** Green/red/amber only report success,
   failure and dry-run. Everything else is ink or the accent.
3. **The machine speaks in mono.** Every command, duration, generation number
   and section header is JetBrains Mono; every sentence is Inter.
4. **Hairlines, not shadows.** 1 px rules separate; the only shadow is the
   ambient lift on raised surfaces and the primary button.
5. **One decision per screen.** Idle → Confirm → Running → Done/Failed. The
   confirmation lists the exact commands and nothing else competes with it.
6. **Honest copy.** Never "up to date" when the run stopped; say what happened,
   name the step that failed, and say what is still running.
7. **Motion** is reserved for state change (active step, pulsing live dot).
   No decoration loops.
8. **No emoji, no exclamation marks, no "Oops".** Sentence case; imperative for
   actions ("Run update", "Start"), past tense for outcomes ("System updated").
9. **No client-side chrome imitations.** No fake window buttons, no traffic
   lights, no drop shadows on the window edge.

## Empty and edge states

- **Idle console** lists the plan as commands, then three notes: what runs, how
  sudo is handled, what staging does, what cleanup keeps.
- **Auth** replaces the primary action and explains `sudo -v` + Retry.
- **Failed** names the step ("Rebuild failed"), says the pipeline stopped before
  cleanup, and states the running generation is untouched.
- **CLI missing** is surfaced in the status line and blocks the primary action.
