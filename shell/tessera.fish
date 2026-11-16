# Tessera shell integration for fish: prompt and command marks (OSC 133)
# and the working directory (OSC 7).
status is-interactive; or exit
set -q TESSERA_SHELL_INTEGRATION; and exit
set -g TESSERA_SHELL_INTEGRATION 1
function __tessera_prompt --on-event fish_prompt
    set -l st $status
    if set -q __tessera_running
        printf '\e]133;D;%s\a' $st
        set -e __tessera_running
    end
    printf '\e]7;file://%s%s\a' (hostname) "$PWD"
    printf '\e]133;A;redraw=1\a'
end
function __tessera_preexec --on-event fish_preexec
    set -g __tessera_running 1
    printf '\e]133;C\a'
end
