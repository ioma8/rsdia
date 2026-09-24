# rsdia
`rsdia` is a terminal ASCII editor for drawing quick diagrams and sketches, with options to export to text files or copy to the clipboard.

It is a Rust program: one static binary, no runtime, no package manager.
The drawing logic is a port of [ASCIIFlow](https://github.com/lewish/asciiflow)'s
`client/`, and the front end is [ratatui](https://ratatui.rs) over crossterm.

## Acknowledgements
- **[ASCIIFlow](https://github.com/lewish/asciiflow)** by Lewis Hemens: the original, and the thing rsdia is trying to be in a terminal. The glyph rules, snapping, line routing and entity moves in `src/core` are ported from its `client/`, MIT © Lewis Hemens, and its own spec files run in `tests/upstream.rs`.
- **[termDRAW](https://github.com/benvinegar/termdraw)** by Ben Vinegar: showed that this class of editor is possible in the terminal, and shaped how the rendering loop is approached.
- **[ratatui](https://ratatui.rs)**: the terminal UI engine the front end is built on.

## Install

```sh
cargo install --path .        # or: cargo build --release
```

You need a terminal with mouse support. `target/release/rsdia` is self-contained.

### Inside herdr

rsdia ships a [herdr](https://herdr.dev) plugin manifest, so it can live in your
multiplexer as a pane split to the right of whatever you're working on:

```sh
herdr plugin install mpospirit/rsdia
```

The install step runs `cargo build --release` in the clone, so Rust has to be on
your `PATH`.

Plugin manifests can't declare their own keybindings — herdr owns that surface — so add
this to your own `config.toml` (opened via herdr's settings, or
`~/.config/herdr/config.toml`) and run `herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+ctrl+d"
type = "plugin_action"
command = "mpospirit.rsdia.open"
description = "open rsdia"
```

`Ctrl+B` `Ctrl+D` then opens the canvas as a right-hand split. Rebind it by changing
`key` above.

## Run

```sh
rsdia                     # open the last drawing
rsdia architecture        # open or create a drawing by name
rsdia ./diagram.rd.json
rsdia --import notes.txt
```

Export a drawing without opening the UI:

```sh
rsdia export architecture --basic --comment hashes --fenced -o diagram.txt
rsdia list
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

- Drawings: `${XDG_DATA_HOME:-~/.local/share}/rsdia/drawings/<name>.rd.json`
- Config: `${XDG_CONFIG_HOME:-~/.config}/rsdia/config.json`

Drawings autosave 500 ms after each change, and again on quit.

Renamed from `lazydraw`? The directories and the file extension changed with the name.
An old file still opens when you name it (`rsdia ./diagram.ld.json`); to let
`rsdia list` find them again:

```sh
mkdir -p ~/.local/share/rsdia
mv ~/.local/share/lazydraw/drawings ~/.local/share/rsdia/
mv ~/.config/lazydraw ~/.config/rsdia        # fails if ~/.config/rsdia already exists
cd ~/.local/share/rsdia/drawings && for f in *.ld.json; do mv "$f" "${f%.ld.json}.rd.json"; done
```

## Terminal notes

- **tmux:** needs `set -g mouse on`. Copying via OSC 52 needs `set -g set-clipboard on`.
- **macOS:** Alt+digit needs "Option as Meta" (or use the bare digits).
- **Colors:** the default `terminal` theme reads your terminal's own palette (OSC 10/11/4) and wears it — background, text and ANSI colors — so rsdia is see-through and matches whatever your terminal looks like. `settings → theme` also offers ten IDE themes, which paint their own background: `dracula`, `nord`, `tokyo-night`, `catppuccin`, `one-dark`, `gruvbox`, `monokai`, `night-owl`, `solarized-dark`, `github-light`. Terminals that don't answer the color query fall back to `nord`.
- **Grid:** the default `lattice` style draws `🭼` (U+1FB7C) in each empty cell, tinted 7% from your background toward the text color so it stays faint. If your terminal doesn't render it as cell-edge hairlines, switch to `checker` in `settings`.

## Development

```sh
cargo run            # run from source
cargo test           # unit, fixture, and end-to-end tests
cargo build --release
```

- `src/core` is pure Rust with no terminal code: glyph connection, snapping, line routing, box/line/select/move tools and export wrappers.
- `src/tui` is the ratatui front end: one full-screen surface that paints the canvas, toolbar, popovers and dialogs into a buffer, so the end-to-end tests drive it without a terminal. Popovers only read from `Host` and describe their buttons as `Action` values, which the app interprets after the frame.
- `tests/fixtures/asciiflow` holds golden outputs captured by replaying `tests/scenarios/asciiflow.json` through ASCIIFlow's own code. `tests/fixtures.rs` replays the same scenarios through the Rust core and asserts both charsets byte for byte.
- `tests/upstream.rs` runs ASCIIFlow's own spec files against the Rust core, through no shim at all.

### One deliberate difference from the TypeScript original

The original asked the terminal for its palette from inside its renderer's event
loop, so a terminal slower than 300 ms still got its colors a frame or two later.
This port asks before the loop starts and keeps the stand-in scheme if the answer
misses the deadline; reading terminal replies while the event loop reads input
would race for the same file descriptor and can drop keystrokes.
