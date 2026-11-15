# Tessera

A terminal in the spirit of iTerm2, for Linux first and cross-platform by design.

- **Engine:** [libghostty-vt](https://github.com/uzaaft/libghostty-rs) (Ghostty's terminal emulation: parsing, screen, scrollback, reflow, key/mouse encoding, selection).
- **UI:** Rust + [cxx-qt](https://github.com/KDAB/cxx-qt) + Qt Quick. Rows are painted with QPainter (native FreeType text, glyphs placed exactly on cells) only when they change, and composited as GPU textures.
- **Sessions survive the window:** a small daemon (`tessera daemon`, started on demand) owns the shells. Close the window, rebuild, or crash, and the next launch reattaches every shell with its recent output.
- **tmux control mode (`tmux -CC`) built in:** tmux windows become native tabs, panes become native splits, and splitting/closing/new-tab turn into tmux commands.
- Tabs, splits (iTerm2-style dimming of unfocused panes), copy-on-select, bracketed paste, mouse reporting, alternate-scroll, Kitty keyboard protocol (via libghostty).

## Keys

| | |
|---|---|
| New tab | Ctrl+Shift+T |
| Split right / down | Ctrl+Shift+D / Ctrl+Shift+E |
| Close pane | Ctrl+Shift+W |
| Copy / paste | Ctrl+Shift+C / Ctrl+Shift+V (selecting also copies) |
| Next / previous tab | Ctrl+PgDn / Ctrl+PgUp, Ctrl+Tab, Alt+1…9 |
| Move between panes | Ctrl+Alt+arrows |
| Font size | Ctrl+= / Ctrl+- |
| Leave tmux control mode | Esc in the tmux gateway tab |

## Build

```
nix develop -c cargo build
target/debug/tessera
```

The dev shell provides Qt 6 and Zig 0.15 (libghostty-vt's build compiles Ghostty).

Testing: `--screenshot out.png` saves the window and quits, `--run CMD` types a command into the first terminal, `--then CMD` types another a few seconds later into whichever terminal has focus. Set `TESSERA_SOCKET` to use a separate daemon.
