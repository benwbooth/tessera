# Tessera shell integration for zsh: prompt and command marks (OSC 133)
# and the working directory (OSC 7).
[[ -n "${TESSERA_SHELL_INTEGRATION-}" || ! -o interactive ]] && return
TESSERA_SHELL_INTEGRATION=1
autoload -Uz add-zsh-hook
__tessera_running=
__tessera_precmd() {
    local st=$?
    [[ -n $__tessera_running ]] && printf '\e]133;D;%s\a' $st
    __tessera_running=
    printf '\e]7;file://%s%s\a' "${HOST:-localhost}" "$PWD"
    # redraw=1: zsh redraws its prompt when the window is resized, so the
    # terminal clears the old one first (no leftover prompt lines).
    printf '\e]133;A;redraw=1\a'
}
__tessera_preexec() { __tessera_running=1; printf '\e]133;C\a'; }
add-zsh-hook precmd __tessera_precmd
add-zsh-hook preexec __tessera_preexec
# End of prompt / start of input, after whatever theme sets PS1.
__tessera_mark_input() { [[ $PS1 != *$'\e]133;B'* ]] && PS1="$PS1%{"$'\e]133;B\a'"%}"; }
add-zsh-hook precmd __tessera_mark_input
