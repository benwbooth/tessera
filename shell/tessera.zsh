# Tessera shell integration for zsh: prompt and command marks (OSC 133)
# and the working directory (OSC 7).
[[ -n "${TESSERA_SHELL_INTEGRATION-}" || ! -o interactive ]] && return
TESSERA_SHELL_INTEGRATION=1
autoload -Uz add-zsh-hook
__tessera_running=
# What a kernel in another shell changed (its folder, exports), handed over
# by Tessera: applied before the next command and prompt.
__tessera_apply() {
    local d="${XDG_RUNTIME_DIR:-/tmp}/tessera-env"
    [[ -f "$d/$$.apply" ]] || return 0
    source "$d/$$.apply"
    command rm -f "$d/$$.apply" "$d/$$.cwd"
}
__tessera_precmd() {
    local st=$?
    __tessera_apply
    [[ -n $__tessera_running ]] && printf '\e]133;D;%s\a' $st
    __tessera_running=
    printf '\e]7;file://%s%s\a' "${HOST:-localhost}" "$PWD"
    # redraw=1: zsh redraws its prompt when the window is resized, so the
    # terminal clears the old one first (no leftover prompt lines).
    printf '\e]133;A;redraw=1\a'
}
__tessera_preexec() {
    __tessera_apply
    __tessera_running=1
    # The command line (OSC 1337 TesseraCommand, base64), then the mark: in
    # that order, so the notebook pairs each command with its output.
    printf '\e]1337;TesseraCommand=%s\a' "$(print -rn -- "$1" | base64 | tr -d '\n')"
    printf '\e]133;C\a'
}
add-zsh-hook precmd __tessera_precmd
add-zsh-hook preexec __tessera_preexec
# End of prompt / start of input, after whatever theme sets PS1.
__tessera_mark_input() { [[ $PS1 != *$'\e]133;B'* ]] && PS1="$PS1%{"$'\e]133;B\a'"%}"; }
add-zsh-hook precmd __tessera_mark_input
# The aliases (OSC 1337 TesseraAliases, base64), when they change: help and
# completion for an alias come from what it runs (ls -> lsd).
__tessera_aliases() {
    local a="$(alias)"
    [[ $a == "${__tessera_alias_seen-}" ]] && return
    __tessera_alias_seen=$a
    printf '\e]1337;TesseraAliases=%s\a' "$(print -rn -- "$a" | base64 | tr -d '\n')"
}
add-zsh-hook precmd __tessera_aliases
# The exported environment, at each prompt, for Tessera's kernels and
# prompt (a file by the shell's pid: /proc only has what it started with).
__tessera_env() {
    local d="${XDG_RUNTIME_DIR:-/tmp}/tessera-env"
    [[ -d $d ]] || mkdir -p -m 700 "$d" 2>/dev/null
    command env -0 >| "$d/$$" 2>/dev/null
}
add-zsh-hook precmd __tessera_env
