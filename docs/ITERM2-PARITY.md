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
| Install Shell Integration | ✅ | ☰ › Install Shell Integration › bash / zsh / fish (OSC 133 marks + OSC 7 folder); says what to add when the startup file is managed by Nix |
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
| Close | ✅ | Ctrl+Shift+W closes the tab, Ctrl+Alt+W one pane; asks when a job is running (per profile) |
| Close All Panes in Tab | ✅ | |
| Close Terminal Window | ✅ | |
| Undo Close | ✅ | Ctrl+Shift+Z; closed shells stay alive for N seconds (General) |
| Archives (save/restore session archive) | 🔜 | |
| Toggle Buffer Input | 🔜 | |
| Broadcast Input (current only / all panes in all tabs / all panes in tab) | ✅ | receiving panes get a red outline |
| Broadcast Input › toggle for current session, background pattern | 🔜 | |
| tmux › Detach, New tmux Window, Dashboard | ➖ | tesserad does what tmux integration was for: sessions outlive the window, here and on SSH hosts |
| ssh › … (SSH integration) | ✅ | + menu SSH hosts run in that host's tesserad (copied there when missing) |
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
| Paste History | ✅ | Edit › Paste History (last 20 copied/pasted) |
| Select All | ✅ | Ctrl+Shift+A |
| Clear Buffer | ✅ | Ctrl+Shift+K |
| Clear Scrollback Buffer | ✅ | |
| Find › Find, Find Next / Previous | ✅ | Ctrl+Shift+F, F3 / Shift+F3 (Enter / Shift+Enter in the bar); smart case, regex; searches upward like iTerm2; all matches highlighted, kept current as output arrives |
| Find › Find URLs | ✅ | |
| Find › Find Globally, Select Matches, Filter, Pick Result to Open | 🔜 | |
| Copy Mode | 🔜 | |
| Open Selection, Jump to Selection | 🔜 | Ctrl+click opens links and files already (Pointer) |
| Select Output of Last Command | ✅ | Ctrl+Shift+L (shell integration) |
| Select Current Command | 🔜 | |
| Marks › Previous / Next Prompt | ✅ | Ctrl+Shift+Up / Down (shell integration); mark indicators in the margin |
| Set Mark / Jump to Mark, Annotations, mark alerts | 🔜 | |
| Fold lines, Clear Instant Replay, Clear to mark/selection | 🔜 | |
| Replace Selection (JSON, Base64, render Markdown) | 🔜 | |
| Snippets | ✅ | Edit › Snippets; Preferences › Snippets |
| Actions | 🔜 | (key mappings can run named actions) |
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
| Toolbelt (profiles, paste history, command history, recent dirs, notes, jobs, captured output) | 🔜 | paste history is in the Edit menu |
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
| Open Autocomplete, Command History, Recent Directories | 🔜 | |
| Triggers | ✅ | Profiles › Advanced: highlight, notify, bell, send text (more actions 🔜) |
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
| Tab context menu (right-click a tab) | ✅ | new, duplicate, rename, color, move left/right, move to new window, close, close others, close to the right |
| Select Next/Previous Tab | ✅ | Ctrl+PgDn / Ctrl+PgUp, Ctrl+Tab, Alt+1…9 |
| Move Tab Left/Right | ✅ | Ctrl+Shift+PgUp / PgDn; or drag the tab |
| Drag a tab out to a new window / onto another window | ✅ | its shells move with it (the daemon reattaches them) |
| Select Split Pane › Above/Below/Left/Right, Next/Previous | ✅ | Ctrl+Alt+arrows, Ctrl+Shift+] / [ |
| Arrangements › Save, Restore, Manage | ✅ | |
| Arrangements › Load from file | 🔜 | |
| Lock Split Pane Width, Lock Size, Lock Layout | 🔜 | |
| Password Manager | 🔜 | via the desktop keyring |
| Notify on Status Change, Cockpit, AI Chat | ➖ | |

### Tab bar (Tessera)
| Item | | Notes |
|---|---|---|
| Right-click a tab | ✅ | new, duplicate, rename, color, move, move to new window, close, close others, close to the right |
| + menu | ✅ | profiles (Ctrl+Shift+1…9), installed shells, ssh hosts from ~/.ssh/config, new ssh connection |
| Drag tabs | ✅ | reorder live; out to a new window; onto another window |

### Help
| Item | | Notes |
|---|---|---|
| Help (opens the README) | ✅ | |

## Preferences

### General
| Setting | | Notes |
|---|---|---|
| Window restoration | ✅ | "Restore sessions": shells, tabs, splits, size and position come back exactly (the daemon kept the shells and their screens); after a reboot, panes come back as new shells under their saved screens |
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
| tmux integration (windows as tabs, bury client, status bar, pause panes, paste buffer) | ➖ | removed: tesserad keeps sessions alive instead |
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
| Scrollbars | ✅ | overlay: shows while scrolling or near the edge; drag the thumb, click the track to page; momentum that builds (wheel and touchpad); selecting past an edge scrolls, faster over time |
| Hide scrollbars | 🔜 | |
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
| | Use separate colors for light and dark mode | ✅ | off by default, as in iTerm2; on, it shows a gallery for each mode |
| | Faint text opacity | ✅ | |
| | Cursor guide | ✅ | |
| | Selection colors | ✅ | from the theme |
| | Minimum contrast | ✅ | default 0.1: only near-invisible text changes |
| | Editing individual colors, cursor boost, bold color, underline color, tab color | 🔜 | |
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
| | Notifications (desktop): on bell, when a session ends | ✅ | trigger notifications too |
| | Encoding, ENQ answerback, locale, report-window-title, printing, resize-window | 🔜 | UTF-8 only today |
| | Show mark indicators | ✅ | |
| | Offscreen command line | 🔜 | |
| Session | After session ends: close / restart / keep | ✅ | |
| | Prompt before closing: never / always / if jobs (with ignore list) | ✅ | |
| | Automatic session logging (folder) | ✅ | raw format (plain text / HTML / asciinema 🔜) |
| | Undo-close time | ✅ | General › Undo Close |
| | Timestamps, status bar, idle keepalive, archive | 🔜 | |
| Keys | Alt as +Esc / Normal | ✅ | |
| | Key mappings | ✅ | send text / hex / escape, ignore, run an action; key recorder |
| | Key-mapping presets (xterm, Terminal.app), import/export | 🔜 | |
| | Application keypad, CSI u / Kitty | ✅ | programs switch them on (libghostty) |
| | Hotkey window | 🔜 | Wayland: global-shortcuts portal |
| Advanced | Triggers | ✅ | |
| | Smart selection, automatic profile switching, semantic history (open with editor) | 🔜 | |

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
| Ctrl+click opens URL / file | ✅ | OSC 8 hyperlinks, URLs, existing files (file:line) |
| Alt-click moves cursor, other click and gesture bindings | 🔜 | |

### Advanced
Not mirrored: iTerm2's hundreds of hidden switches. Tessera adds settings as features need them.
