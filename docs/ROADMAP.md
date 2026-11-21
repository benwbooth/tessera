# Roadmap: the editor, AI and rich output

## 1. Input editor — done

A multi-line editor under the terminal while the shell waits at its prompt
(shell integration's OSC 133 marks) or a REPL (python, node) runs.

- Enter runs the command (typed into the shell, so it scrolls up with its
  output); a right Shift tap (or Shift+Enter) adds a line (multi-line commands go in as a paste).
- tree-sitter highlighting in the theme's colors (bash, Python, JavaScript)
  and syntax errors, marked once typing pauses.
- Keys: Emacs (readline), vi (normal/insert, operators, motions), VS Code.
- VS Code-style editing in every mode: auto-indent after `do`/`then`/`{`/`:`,
  outdent on `done`/`fi`/`}`, bracket and quote pairs, Tab/Shift+Tab indent.
- History (Up/Down, Ctrl+P/N), Ctrl+C clears (or interrupts when empty),
  Shift+PgUp/PgDn scroll the terminal, tapping Ctrl switches the keys between the editor and the terminal.

## 2. Completion and language servers — done

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

## 4. Rich output — done

- Inline images: iTerm2 (imgcat) and kitty graphics protocols.
- A command whose output is an image file (cat photo.png) shows the picture.
- Each command's output (between OSC 133 C and D) is recognised — JSON,
  CSV/TSV, HTML, Markdown, SVG — and gets a View chip: a folding JSON tree, a
  sortable table, formatted HTML/Markdown, the drawn SVG; Esc back to text.
- Still to do: audio, and an AI/Magika guess for output the rules can't place.

## 5. Built-in prompt — done

- A prompt drawn by Tessera above the editor: Starship's own (your
  starship.toml, every module) when it's installed, else built-in segments
  (folder, git, venv, nix-shell, exit status, duration, Kubernetes, AWS,
  gcloud, host, time) chosen and ordered in Preferences › General.

## 6. Notebooks — done

- Notebook tabs (Ctrl+Shift+J opens one; a tab is a terminal or a notebook for good, `qml/NotebookView.qml`): a card per command with resizable, expandable outputs and per-cell views (Markdown rich or as text); cells written there too, run by the pane's shell or a Jupyter kernel. Kept per session (terminal cells in `cells/`, the notebook's own cells and order in `notebooks/`).
- Jupyter `.ipynb`: the same notebook from a file (`src/jupyter.rs`: kernelspecs, ZMTP 3.0 and signed messages, no Python needed in Tessera itself). Kernels: installed ones, Python from Nix, or the pane's shell. A pane's notebook saves as `.ipynb`.
- Later: ipywidgets, a variables view, the stdin channel (`input()`), notebook diffs.
