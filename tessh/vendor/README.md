# Vendored crates

## reedline 0.53.0 (MIT, Nushell project)

The line editor of tessh's terminal REPL (`src/repl.rs`), from crates.io
with one change, in `src/enums.rs` (`TryFrom<Event> for ReedlineRawEvent`):
a key release of a modifier key (`KeyCode::Modifier`) is passed on instead
of dropped, so tessh's edit mode can see a lone tap of left or right Shift
or Ctrl when the terminal speaks the kitty keyboard protocol. The examples
and their manifest entries are left out.

To update: unpack the new crate over `reedline/` (src, Cargo.toml without
its `[[example]]` entries, LICENSE, README.md, KEYBINDINGS.md) and make the
change again.

A second change adds `EditHook` (`src/edit_hook.rs`, `Reedline::with_edit_hook`,
and a few lines at the top of `handle_editor_event` in `src/engine.rs`): a
hook that sees each editor event with the buffer, cursor and whether a menu
is open, and may replace it. tessh uses it for its Enter (run, or a new line
indented for the block), Tab and Backspace in indentation, and the rest of
its text-aware keys.

The optional `sqlite`, `sqlite-dynlib` and `system_clipboard` features and
their dependencies (rusqlite, arboard) are taken out of the manifest: tessh
uses the file history, and rusqlite's sqlite clashed with the workspace's.
