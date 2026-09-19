# lazydraw

[![npm](https://img.shields.io/npm/v/lazydraw)](https://www.npmjs.com/package/lazydraw)
[![license](https://img.shields.io/npm/l/lazydraw)](LICENSE)

A terminal ASCII diagram editor. It looks and behaves like [ASCIIFlow](https://asciiflow.com): the same grid, arrows, and toolbar, running in your terminal.

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ ≡ │ box select arrow line text eraser │ export │ ⟲ ⟳ │ ⚙ │ help │
  └─────────────────────────────────────────────────────────────────┘

  ┌──────────┐      ┌──────────┐       ┌─────────┐      ┌─────────────┐
  │    I     ├─────►│   love   ├───┬──►│   bun   ├─────►│   install   │
  └──────────┘      └──────────┘   │   └────┬────┘      └──────┬──────┘
                                   │        │                  │
                                   │        │                  │
  ┌──────────┐      ┌──────────┐   │   ┌────┴────┐      ┌──────▼──────┐
  │   draw   │◄─────┤   lazy   │◄──┘   │   npm   │      │ -g lazydraw │
  └──────────┘      └──────────┘       └─────────┘      └─────────────┘
```

## Install

lazydraw runs on the [Bun](https://bun.sh) runtime, so Bun 1.3 or later has to be
installed. You also want a terminal with mouse support.

```sh
bun install -g lazydraw
lazydraw
```

If the `lazydraw` command isn't found afterwards, Bun's global bin directory isn't on
your `PATH`. Add it to your shell profile:

```sh
export PATH="$HOME/.bun/bin:$PATH"
```

npm installs it just as well, and usually lands somewhere already on your `PATH`:

```sh
npm install -g lazydraw
```

Or skip installing altogether:

```sh
bunx lazydraw
```

## Run

```sh
lazydraw                     # open the last drawing
lazydraw architecture        # open or create a drawing by name
lazydraw ./diagram.ld.json
lazydraw --import notes.txt
```

Export a drawing without opening the UI:

```sh
lazydraw export architecture --basic --comment hashes --fenced -o diagram.txt
lazydraw list
```

## Toolbar

`≡ │ box select arrow line text eraser │ export │ ⟲ ⟳ │ ⚙ │ help`

| | |
|---|---|
| `≡` | switch drawings; new, rename, fork, import, clear, delete |
| `box` | drag corner to corner |
| `select` | drag a box's inside to move it with its arrows attached, drag a line to resize, drag a line end to reshape, drag empty space to select |
| `arrow` / `line` | drag start to end; hold Shift or press `f` mid-drag to flip the elbow |
| `text` | click and type; Enter starts a new line, Esc finishes |
| `eraser` | drag to erase; detaches from lines it touches |
| `export` | extended or basic characters, 11 comment styles, Markdown fence, preview, copy, save |
| `⟲` / `⟳` | undo / redo |
| `⚙` | grid style (lattice, checker, dots, off), theme (terminal + 10 IDE themes), copy on select, recenter |
| `help` | help for the active tool, plus the shortcut list |

## Keys

| Key | Action |
|---|---|
| `1`–`6`, `Alt+1`–`Alt+6` | switch tool |
| `r` `v` `a` `l` `t` `e` | switch tool: box, select, arrow, line, text, eraser |
| `Ctrl+Z` / `Ctrl+Y` | undo / redo (`Ctrl+Shift+Z` also redoes on terminals with the kitty keyboard protocol) |
| `Ctrl+C` `Ctrl+X` `Ctrl+V` | copy, cut, paste the selection (bracketed paste works too) |
| `Del` / arrow keys | erase or nudge the selection |
| wheel, `Shift`+wheel | pan vertically / horizontally |
| `Space`+drag, middle-drag | pan freely |
| `Ctrl+O` `Ctrl+E` `Ctrl+S` | files / export / save now |
| `?`, `Esc` | help; close a popover, cancel a drag, deselect |
| `Ctrl+Q` twice | quit; the first press prompts, a second within 3 s exits (drawings autosave) |

In text mode, printable keys (including digits, `q` and `?`) type text.

Selecting can also copy to the clipboard by itself — `settings → copy on select`, off by default.

## Files

- Drawings: `${XDG_DATA_HOME:-~/.local/share}/lazydraw/drawings/<name>.ld.json`
- Config: `${XDG_CONFIG_HOME:-~/.config}/lazydraw/config.json`

Drawings autosave 500 ms after each change, and again on quit.

## Terminal notes

- **tmux:** needs `set -g mouse on`. Copying via OSC 52 needs `set -g set-clipboard on`.
- **macOS:** Alt+digit needs "Option as Meta" (or use the bare digits).
- **Colors:** the default `terminal` theme reads your terminal's own palette (OSC 10/11/4) and wears it — background, text and ANSI colors — so lazydraw is see-through and matches whatever your terminal looks like. `settings → theme` also offers ten IDE themes, which paint their own background: `dracula`, `nord`, `tokyo-night`, `catppuccin`, `one-dark`, `gruvbox`, `monokai`, `night-owl`, `solarized-dark`, `github-light`. Terminals that don't answer the color query fall back to `nord`.
- **Grid:** the default `lattice` style draws `🭼` (U+1FB7C) in each empty cell, tinted 7% from your background toward the text color so it stays faint. If your terminal doesn't render it as cell-edge hairlines, switch to `checker` in `settings`.

## Development

```sh
bun install
bun start           # run from source
bun test            # unit, fixture, and end-to-end tests
bun run typecheck
bun run build       # standalone binary at dist/lazydraw
```

- `src/core` is pure TypeScript with no terminal code. Its drawing logic (glyph connection, snapping, line routing, box/line/select/move tools, export wrappers) is ported from ASCIIFlow's `client/`.
- `src/tui` is the OpenTUI front end: one full-screen surface that paints the canvas, toolbar, popovers and dialogs.
- `test/fixtures/asciiflow` holds golden outputs captured by replaying `test/scenarios/asciiflow.json` through ASCIIFlow's own code. To regenerate them:

  ```sh
  git clone https://github.com/lewish/asciiflow /tmp/asciiflow && (cd /tmp/asciiflow && bun install)
  bun run fixtures /tmp/asciiflow
  ```

- `test/upstream` runs ASCIIFlow's own spec files against lazydraw's core, through a small compatibility shim.

## Inspiration and credits

lazydraw exists because of the projects below.

- **[ASCIIFlow](https://github.com/lewish/asciiflow)** by Lewis Hemens: the original, and the thing lazydraw is trying to be in a terminal.
- **[termDRAW](https://github.com/benvinegar/termdraw)** by Ben Vinegar: showed that this class of editor is possible in the terminal, and shaped how the rendering loop is approached.
- **[OpenTUI](https://github.com/sst/opentui)** by SST: the terminal UI engine the whole front end is built on.
