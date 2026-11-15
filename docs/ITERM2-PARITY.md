# iTerm2 parity

Every iTerm2 menu item and setting, and where Tessera stands on it. Taken from
iTerm2's own [menu](https://iterm2.com/documentation-menu-items.html) and
[preferences](https://iterm2.com/documentation-preferences.html) documentation.

✅ done · 🔜 planned · ➖ not applicable (with why)

Shortcuts: iTerm2's Cmd becomes Ctrl+Shift (Ctrl alone belongs to the shell).

## Menus

The ☰ menu holds iTerm2's menu bar as submenus (Shell, Edit, View, Session,
Profiles, Window, Help), with Preferences, About and Quit at the bottom. Right
click in a terminal opens a context menu.

### iTerm2 (app menu)
| Item | | Notes |
|---|---|---|
| Preferences | ✅ | Ctrl+, |
| About | ✅ | |
| Quit | ✅ | Ctrl+Shift+Q; sessions survive unless "Restore sessions" is off |
| Show Tip of the Day | ➖ | |
| Check for Updates | ➖ | Nix / distribution packages update it |
| Toggle Debug Logging, Retrospective Logs, Performance Stats, Capture GPU Frame | 🔜 | a debug log to a file |
| Secure Keyboard Entry | ➖ | macOS-only API (Wayland already isolates input) |
| Make iTerm2 Default Term | 🔜 | as "Set as default terminal" (xdg-terminal-exec) |
| Install Shell Integration | 🔜 | with shell integration (below) |
| Companion Device, Claude Code Integration | ➖ | |

### Shell
| Item | | Notes |
|---|---|---|
| New Window | ✅ | Ctrl+Shift+N (its own process) |
| New Tab | ✅ | Ctrl+Shift+T, default profile |
| New Tab with Current Profile | ✅ | |
| Duplicate Tab | ✅ | same splits, profiles and folders |
| Split Vertically/Horizontally with Current Profile | ✅ | Ctrl+Shift+D / Ctrl+Shift+E ("Split Right/Down") |
| Split Vertically/Horizontally (default profile) | ✅ | |
| Save Contents / Save Selected Text | ✅ | |
| Close | ✅ | Ctrl+Shift+W, asks when a job is running (per profile) |
| Close All Panes in Tab | ✅ | |
| Close Terminal Window | ✅ | |
| Undo Close | 🔜 | keep the shell alive in the daemon for N seconds |
| Archives (save/restore session archive) | 🔜 | |
| Toggle Buffer Input | 🔜 | |
| Broadcast Input (current only / all panes in all tabs / all panes in tab) | ✅ | receiving panes get a red outline |
| Broadcast Input › toggle for current session, background pattern | 🔜 | |
| tmux › Detach, New tmux Window | ✅ | |
| tmux › Dashboard | 🔜 | |
| ssh › … (SSH integration) | 🔜 | |
| Workgroups | ➖ | |
| Print | 🔜 | |

### Edit
| Item | | Notes |
|---|---|---|
| Copy | ✅ | Ctrl+Shift+C; copy-on-select by default |
| Copy with Styles | ✅ | HTML on the clipboard |
| Copy with Control Sequences | ✅ | |
| Paste | ✅ | Ctrl+Shift+V |
| Paste Special › Paste Selection | ✅ | Shift+Insert, middle click |
| Paste Special › Paste Slowly | ✅ | 16 bytes / 125 ms |
| Paste Special › Warn Before Multi-Line Paste | ✅ | |
| Paste Special › Advanced Paste, Base64 file, faster/slower, tabs→spaces, newline-at-prompt warnings | 🔜 | |
| Select All | ✅ | Ctrl+Shift+A |
| Clear Buffer | ✅ | Ctrl+Shift+K |
| Clear Scrollback Buffer | ✅ | |
| Find › Find, Find Next / Previous | ✅ | Ctrl+Shift+F, F3 / Shift+F3 (Enter / Shift+Enter in the bar); smart case, regex; searches upward like iTerm2; all matches highlighted, kept current as output arrives |
| Find › Find URLs | ✅ | |
| Find › Find Globally, Select Matches, Filter, Pick Result to Open | 🔜 | |
| Copy Mode | 🔜 | |
| Open Selection, Jump to Selection | 🔜 | |
| Select Output of Last Command / Current Command | 🔜 | needs shell integration (OSC 133; libghostty already tracks it) |
| Marks and Annotations | 🔜 | shell-integration marks first |
| Fold lines, Clear Instant Replay, Clear to mark/selection | 🔜 | |
| Replace Selection (JSON, Base64, render Markdown) | 🔜 | |
| Snippets, Actions | 🔜 | |
| AI items | ➖ | |

### View
| Item | | Notes |
|---|---|---|
| Toggle Full Screen | ✅ | F11 |
| Make Text Bigger / Smaller / Normal Size | ✅ | Ctrl+= / Ctrl+- / Ctrl+0 |
| Maximize Active Pane | ✅ | Ctrl+Shift+Enter |
| Use Transparency | ✅ | |
| Find Cursor | ✅ | Ctrl+Shift+/ |
| Show Cursor Guide | ✅ | |
| Tab Color | ✅ | |
| Show Tabs in Fullscreen | ✅ | the tab bar always shows |
| Disable Transparency for Active Window | 🔜 | |
| Toolbelt (profiles, paste history, command history, recent dirs, notes, jobs, captured output) | 🔜 | |
| Zoom In on Selection / Zoom Out | 🔜 | |
| Show Timestamps | 🔜 | |
| Open Quickly | 🔜 | |
| Start Instant Replay | 🔜 | |
| Size Changes Update Profile | 🔜 | |
| Show Annotations, Composer, Auto Composer, Command Completion, Session Note, AI chat | 🔜 / ➖ | Composer and completion planned; AI not |

### Session
| Item | | Notes |
|---|---|---|
| Edit Session | ✅ | opens the pane's profile (per-session overrides 🔜) |
| Restart Session | ✅ | |
| Duplicate Session | ✅ | |
| Reset | ✅ | |
| Log › Log to File | ✅ | raw output |
| Log › Import/Export Recording | 🔜 | |
| Run/Stop Coprocess | 🔜 | |
| Open Autocomplete, Command History, Recent Directories, Paste History | 🔜 | |
| Triggers | 🔜 | |
| Make Screenshot | 🔜 | |
| Bury Session | 🔜 | |

### Profiles
| Item | | Notes |
|---|---|---|
| Profile list (opens a tab with it) | ✅ | |
| Open All | ✅ | |
| Open Profiles… | ✅ | "Edit Profiles…" opens Preferences › Profiles |

### Window
| Item | | Notes |
|---|---|---|
| Minimize, Zoom | ✅ | |
| Edit Window Title / Edit Tab Title | ✅ | |
| Select Next/Previous Tab | ✅ | Ctrl+PgDn / Ctrl+PgUp, Ctrl+Tab, Alt+1…9 |
| Move Tab Left/Right | ✅ | Ctrl+Shift+PgUp / PgDn; or drag the tab |
| Drag a tab out to a new window / onto another window | ✅ | its shells move with it (the daemon reattaches them) |
| Select Split Pane › Above/Below/Left/Right, Next/Previous | ✅ | Ctrl+Alt+arrows, Ctrl+Shift+] / [ |
| Arrangements › Save, Restore, Manage | ✅ | |
| Arrangements › Load from file | 🔜 | |
| Lock Split Pane Width, Lock Size, Lock Layout | 🔜 | |
| Password Manager | 🔜 | via the desktop keyring |
| Notify on Status Change, Cockpit, AI Chat | ➖ | |

### Help
| Item | | Notes |
|---|---|---|
| Help (opens the README) | ✅ | |

## Preferences

### General
| Setting | | Notes |
|---|---|---|
| Window restoration | ✅ | "Restore sessions": shells, tabs, splits and size come back (they never stopped: the daemon kept them) |
| Open profiles window / restore to Spaces / always open a window | ➖ | |
| Quit when all windows are closed | ✅ | always |
| Confirm closing multiple sessions | ✅ | |
| Confirm Quit | ✅ | "Confirm closing the window" |
| Copy to clipboard on selection | ✅ | |
| Copied text includes trailing newline | ✅ | |
| Applications may access clipboard (OSC 52) | ✅ | write only |
| Triple-click selects full wrapped lines | ✅ | always on (libghostty line selection) |
| Double-click smart selection | 🔜 | with smart selection rules |
| Characters considered part of a word | ✅ | |
| Shift+arrow enters copy mode, click selects command, quote dropped filenames | 🔜 | |
| Each tab may have its own title | ✅ | |
| Smart placement, remember window frames, adjust window on font change, native fullscreen | 🔜 / ➖ | Wayland places windows itself |
| Load settings from folder/URL, export/import all | 🔜 | settings are plain JSON in ~/.config/tessera |
| tmux: windows as tabs | ✅ | always tabs |
| tmux: bury client session, tmux status bar, pause panes, mirror paste buffer | 🔜 | |
| Magic: Bonjour hosts, Instant Replay memory, Python API, GPU options, throughput | ➖ / 🔜 | Python API → a scripting API later |
| AI, Software Update | ➖ | |

### Appearance
| Setting | | Notes |
|---|---|---|
| Theme (window chrome) | ✅ | follows the terminal theme, light or dark |
| Unified title bar (Minimal/Compact themes) | ✅ | the tab bar is the title bar: KDE Breeze buttons on Linux, Windows buttons on Windows, native traffic lights on macOS |
| Tab bar location | ✅ | top or bottom (left/right 🔜) |
| Show tab bar with one tab | ✅ | |
| Show tab numbers | ✅ | |
| Tabs have close buttons | ✅ | |
| Activity indicator / new-output indicator | ✅ | |
| Stretch tabs to fill bar | ✅ | |
| Tab bar scrolls, HTML in titles, flash in fullscreen, preserve window size | 🔜 | |
| Per-pane title bars | ✅ | |
| Separate status bars / background images per pane | 🔜 | with status bar and images |
| Margins | ✅ | per profile (Window tab) |
| Dim inactive split panes, dimming amount, dim background windows | ✅ | |
| Dimming affects only text | 🔜 | |
| Hide scrollbars | 🔜 | scrollbar itself 🔜 |
| Window number in title, proxy icon, Dock options | ➖ | |

### Profiles
Profiles: add, remove, duplicate, rename, set default ✅.

| Tab | Setting | | Notes |
|---|---|---|---|
| General | Name | ✅ | |
| | Command: login shell / custom command | ✅ | through the login shell |
| | Send text at start | ✅ | |
| | Initial directory: home / reuse previous / directory | ✅ | |
| | Title | ✅ | "Tab title" (interpolated components 🔜) |
| | Badge | ✅ | {title}, {cwd}, {profile} |
| | Shortcut key, tags, icon, subtitle, URL schemes, SSH | 🔜 | |
| Colors | Color presets, import .itermcolors | ✅ | 14 built in |
| | Separate light/dark colors | ✅ | follow the desktop |
| | Faint text opacity | ✅ | |
| | Cursor guide | ✅ | |
| | Selection colors | ✅ | from the theme |
| | Editing individual colors, cursor boost, minimum contrast, bold color, underline color, tab color | 🔜 | |
| Text | Cursor shape, blink | ✅ | |
| | Font, size, line and character spacing | ✅ | |
| | Draw bold in bold font, italic | ✅ | |
| | Blinking text, ligatures, non-ASCII font, Powerline glyphs, Unicode widths, ambiguous width | 🔜 | |
| Window | Transparency, keep backgrounds opaque | ✅ | |
| | Columns / rows | ✅ | new windows |
| | Background image, blur, style (fullscreen, edges), screen | 🔜 | |
| Terminal | Scrollback lines, unlimited | ✅ | |
| | Report terminal type | ✅ | |
| | Mouse reporting, alternate scroll | ✅ | |
| | Terminal may change title / enable paste bracketing | ✅ | |
| | Silence bell, visual bell, bell icon in tabs | ✅ | |
| | Notifications (desktop) | 🔜 | |
| | Encoding, ENQ answerback, locale, report-window-title, printing, resize-window | 🔜 | UTF-8 only today |
| | Shell integration marks, offscreen command line | 🔜 | |
| Session | After session ends: close / restart / keep | ✅ | |
| | Prompt before closing: never / always / if jobs (with ignore list) | ✅ | |
| | Undo-close window, automatic logging, timestamps, status bar, idle keepalive, archive | 🔜 | |
| Keys | Alt as +Esc / Normal | ✅ | |
| | Key mappings and presets | 🔜 | |
| | Application keypad, CSI u / Kitty | ✅ | programs switch them on (libghostty) |
| | Hotkey window | 🔜 | Wayland: global-shortcuts portal |
| Advanced | Triggers, smart selection, automatic profile switching, semantic history | 🔜 | |

### Keys (global)
| Setting | | Notes |
|---|---|---|
| Key bindings, leader, navigation shortcut modifiers | 🔜 | |
| Hotkey window / show-hide all windows | 🔜 | global-shortcuts portal |
| Touch Bar, remap modifiers, force keyboard | ➖ | |

### Arrangements
| Setting | | Notes |
|---|---|---|
| Saved arrangements list, delete, restore | ✅ | |
| Default arrangement at startup | 🔜 | (session restore covers the common case) |

### Pointer
| Setting | | Notes |
|---|---|---|
| Middle click pastes selection | ✅ | |
| Right click: menu or paste | ✅ | |
| Focus follows mouse | ✅ | |
| Ctrl/Alt-click actions, click opens URL/file, gesture bindings | 🔜 | |

### Advanced
Not mirrored: iTerm2's hundreds of hidden switches. Tessera adds settings as features need them.
