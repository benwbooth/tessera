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
    __tessera_cwd
    printf '\e]133;A\a'
}
__tessera_preexec() {
    [[ -n "${COMP_LINE-}" || "$BASH_COMMAND" == __tessera_* ]] && return
    if [[ -z "${__tessera_running-}" ]]; then __tessera_running=1; printf '\e]133;C\a'; fi
}
PROMPT_COMMAND="__tessera_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
PS1="$PS1"'\[\e]133;B\a\]'
trap '__tessera_preexec' DEBUG
