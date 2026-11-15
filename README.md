# Tessera

A terminal in the spirit of iTerm2, for Linux first and cross-platform by design.

- **Engine:** [libghostty-vt](https://github.com/uzaaft/libghostty-rs) (Ghostty's terminal emulation: parsing, screen, scrollback, reflow, key/mouse encoding, selection).
- **UI:** Rust + [cxx-qt](https://github.com/KDAB/cxx-qt) + Qt Quick. Rows are painted with QPainter (native FreeType text, glyphs placed exactly on cells) only when they change, and composited as GPU textures.
- **Sessions survive the window:** a small daemon (`tessera daemon`, started on demand) owns the shells. Close the window, rebuild, or crash, and the next launch reattaches every shell with its recent output.
- **tmux control mode (`tmux -CC`) built in:** tmux windows become native tabs, panes become native splits, and splitting/closing/new-tab turn into tmux commands.
- Tabs, splits (iTerm2-style dimming of unfocused panes), copy-on-select, bracketed paste, mouse reporting, alternate-scroll, Kitty keyboard protocol (via libghostty).

## Like iTerm2

The ☰ menu (top left) is iTerm2's menu bar — Shell, Edit, View, Session,
Profiles, Window, Help — as native KDE/Breeze menus with icons and
shortcuts. Preferences mirror iTerm2's panels: General, Appearance,
**Profiles** (General, Colors, Text, Window, Terminal, Session, Keys),
Arrangements, Pointer. [docs/ITERM2-PARITY.md](docs/ITERM2-PARITY.md) lists
every iTerm2 menu item and setting with its status here.

Highlights: profiles, 14 color themes plus .itermcolors import (separate
light/dark themes following the desktop), broadcast input, maximize pane,
window arrangements, tab colors, badges, cursor guide, transparency, session
logging, confirm-close when a job is running, restore sessions with their
splits.

## Keys

iTerm2's Cmd shortcuts become Ctrl+Shift.

| | |
|---|---|
| New tab / window | Ctrl+Shift+T / Ctrl+Shift+N |
| Split right / down | Ctrl+Shift+D / Ctrl+Shift+E |
| Close pane | Ctrl+Shift+W |
| Maximize pane | Ctrl+Shift+Enter |
| Copy / paste | Ctrl+Shift+C / Ctrl+Shift+V (selecting also copies; Shift+Insert or middle click pastes the selection) |
| Select all / clear buffer | Ctrl+Shift+A / Ctrl+Shift+K |
| Next / previous tab | Ctrl+PgDn / Ctrl+PgUp, Ctrl+Tab, Alt+1…9 |
| Move tab | Ctrl+Shift+PgUp / Ctrl+Shift+PgDn |
| Move between panes | Ctrl+Alt+arrows, Ctrl+Shift+] / [ |
| Font size | Ctrl+= / Ctrl+- / Ctrl+0 |
| Full screen | F11 |
| Find cursor | Ctrl+Shift+/ |
| Preferences | Ctrl+, |
| Leave tmux control mode | Esc in the tmux gateway tab |

## Build

```
nix develop -c cargo build
target/debug/tessera
```

The dev shell provides Qt 6 and Zig 0.15 (libghostty-vt's build compiles Ghostty).

Testing: `--screenshot out.png` saves the window and quits, `--run CMD` types a command into the first terminal, `--then CMD` types another a few seconds later into whichever terminal has focus, `--prefs PAGE` opens Preferences (the screenshot is then of Preferences). Set `TESSERA_SOCKET`, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` to keep a test run apart from your own sessions and settings.

Development: `devloop start tessera` (from home-services) rebuilds and restarts the window on every change; shells survive, since they live in the daemon.
