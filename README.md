# rsdia

A **Rust TUI/CLI diagramming tool**: draw ASCII diagrams in your terminal and
export them as plain text — boxes, arrows, lines and labels, with undo, pan,
mouse and clipboard support.

I liked [mpospirit/lazydraw](https://github.com/mpospirit/lazydraw), so I forked
it, rewrote it in Rust (ratatui + crossterm) and improved it: one static binary,
no runtime, no dependencies to install. The drawing logic comes from that project
and from [ASCIIFlow](https://github.com/lewish/asciiflow), which it was modelled
on — credit for both ideas goes there.

## Install

```sh
cargo install --path .      # or: cargo build --release
```

## Use

```sh
rsdia                       # open the last drawing (or create "untitled")
rsdia architecture          # open or create a drawing by name
rsdia --import notes.txt    # start from a text file
rsdia list                  # what's saved

rsdia export architecture --basic --comment hashes --fenced -o diagram.txt
```

| | |
|---|---|
| `box select arrow line text eraser` | tools: `1`–`6` or `r v a l t e` |
| drag | draw; `f` flips a line or arrow elbow |
| `ctrl+z` / `ctrl+y` | undo / redo |
| `y` `x` `p` | copy / cut / paste the selection |
| `del`, arrows | erase / nudge the selection |
| wheel, middle-drag, space+drag | pan |
| `ctrl+o` `ctrl+e` `ctrl+s` | files / export / save |
| `ctrl+c` `ctrl+q` | quit (saves first) |

Drawings are `.rd.json` under `~/.local/share/rsdia`, settings under
`~/.config/rsdia`, and they autosave. `tmux` needs `set -g mouse on`; on macOS
use the bare digits instead of `Alt`+digit.

## Develop

```sh
cargo test    # golden fixtures from ASCIIFlow, its own spec files, end-to-end app tests
```

`src/core` is pure (no terminal, no I/O), `src/tui` is the ratatui front end,
`src/cli.rs` is the command surface.
