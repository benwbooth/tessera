# Tessera shell integration for bash: marks each prompt and command
# (OSC 133) and reports the working directory (OSC 7), so Tessera can jump
# between commands, select a command's output, and open new tabs in the
# same folder.
if [[ -n "${TESSERA_SHELL_INTEGRATION-}" || $- != *i* ]]; then return 0 2>/dev/null; fi
TESSERA_SHELL_INTEGRATION=1
__tessera_cwd() { printf '\e]7;file://%s%s\a' "${HOSTNAME:-localhost}" "$PWD"; }
__tessera_prompt() {
    local status=$?
    if [[ -n "${__tessera_running-}" ]]; then printf '\e]133;D;%s\a' "$status"; fi
    __tessera_running=
    __tessera_apply
    __tessera_cwd
    printf '\e]133;A;redraw=last\a'
}
# What a kernel in another shell changed (its folder, exports), handed over
# by Tessera: applied before the next command and prompt.
__tessera_apply() {
    local d="${XDG_RUNTIME_DIR:-/tmp}/tessera-env"
    [[ -f "$d/$$.apply" ]] || return 0
    source "$d/$$.apply"
    command rm -f "$d/$$.apply" "$d/$$.cwd"
}
__tessera_preexec() {
    [[ -n "${COMP_LINE-}" || "$BASH_COMMAND" == __tessera_* ]] && return
    if [[ -z "${__tessera_running-}" ]]; then
        __tessera_apply
        __tessera_running=1
        # The command line (OSC 1337 TesseraCommand, base64), then the mark.
        local line; line="$(HISTTIMEFORMAT= history 1 | sed 's/^ *[0-9]* *//')"
        # What it runs with (OSC 1337 TesseraExec, base64): the user, the
        # command bash is running, and the environment.
        printf '\e]1337;TesseraExec=%s\a' "$({ printf '%s\0%s\0' "${USER:-$LOGNAME}" "$BASH_COMMAND"; command env -0 2>/dev/null || command env; } | base64 | tr -d '\n')"
        printf '\e]1337;TesseraCommand=%s\a' "$(printf '%s' "$line" | base64 | tr -d '\n')"
        printf '\e]133;C\a'
    fi
}
PROMPT_COMMAND="__tessera_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
PS1="$PS1"'\[\e]133;B\a\]'
trap '__tessera_preexec' DEBUG
# The aliases (OSC 1337 TesseraAliases, base64), when they change: help and
# completion for an alias come from what it runs (ls -> lsd).
__tessera_aliases() {
    local a; a="$(alias -p)"
    [[ $a == "${__tessera_alias_seen-}" ]] && return
    __tessera_alias_seen=$a
    printf '\e]1337;TesseraAliases=%s\a' "$(printf '%s' "$a" | base64 | tr -d '\n')"
}
PROMPT_COMMAND="__tessera_aliases; $PROMPT_COMMAND"
# The exported environment, at each prompt, for Tessera's kernels and
# prompt (a file by the shell's pid: /proc only has what it started with).
__tessera_env() {
    local d="${XDG_RUNTIME_DIR:-/tmp}/tessera-env"
    [[ -d $d ]] || mkdir -p -m 700 "$d" 2>/dev/null
    command env -0 > "$d/$$" 2>/dev/null
}
PROMPT_COMMAND="__tessera_env; $PROMPT_COMMAND"
