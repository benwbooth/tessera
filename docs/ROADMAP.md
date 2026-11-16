# Roadmap: the editor, AI and rich output

## 1. Input editor — done

A multi-line editor under the terminal while the shell waits at its prompt
(shell integration's OSC 133 marks) or a REPL (python, node) runs.

- Enter runs the command (typed into the shell, so it scrolls up with its
  output); Shift+Enter adds a line (multi-line commands go in as a paste).
- tree-sitter highlighting in the theme's colors (bash, Python, JavaScript)
  and syntax errors, marked once typing pauses.
- Keys: Emacs (readline), vi (normal/insert, operators, motions), VS Code.
- VS Code-style editing in every mode: auto-indent after `do`/`then`/`{`/`:`,
  outdent on `done`/`fi`/`}`, bracket and quote pairs, Tab/Shift+Tab indent.
- History (Up/Down, Ctrl+P/N), Ctrl+C clears (or interrupts when empty),
  Shift+PgUp/PgDn scroll the terminal, Esc hands the keys to the terminal.

## 2. Completion and language servers — done (language servers to come)

- A dropdown for commands, options and arguments with their descriptions:
  fish's completion files (thousands of commands), parsed `--help` and man
  pages (cached), paths, git refs, ssh hosts, processes.
- LSP: bash-language-server for shells (completions, hover help, shellcheck
  diagnostics), pyright/pylsp in Python REPLs, when installed.
- Hover and signature help: the option's help text as a tooltip.
- Unknown commands marked (like fish); Ctrl+R history search over every
  shell's history.

## 3. AI — done

- Providers: any OpenAI-compatible server (Ollama, llama.cpp, LM Studio,
  vLLM, LocalAI, OpenAI/Codex, Gemini, OpenRouter, Groq, Mistral, DeepSeek,
  xAI) and Anthropic (Claude); keys in secrets.json (owner-only) or env vars.
- Ghost-text completion (Tab accepts) from a small local model — on this
  machine `qwen3.5:4b` in Ollama; larger models for chat.
- AI checking as you type: risky or wrong commands flagged inline with a
  reason (rm -rf with a variable, a flag the command doesn't have, ...).
- Plain English to a command; explain a command; fix the last failed one
  from its output and exit code.
- A chat sidebar that sees the folder and recent output, suggests commands,
  and runs them in the terminal with your approval.

## 4. Rich output

- Inline images: iTerm2 (imgcat) and kitty graphics protocols.
- Each command's output (between OSC 133 C and D) is sniffed — Magika (a
  small local content-type model) plus magic bytes — and images, SVG, HTML,
  JSON, CSV, Markdown and audio get a rich view in place, with a toggle back
  to text.

## 5. Built-in prompt

- A prompt drawn by Tessera above the editor, like Starship but built in:
  segments (folder, git, language versions, exit status, duration, time,
  host, k8s, ...) each configurable — what shows, colors, icons, order,
  separators — in Preferences, with a live preview.
