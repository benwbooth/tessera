# Tessera shell integration for fish: prompt and command marks (OSC 133)
# and the working directory (OSC 7).
status is-interactive; or exit
set -q TESSERA_SHELL_INTEGRATION; and exit
set -g TESSERA_SHELL_INTEGRATION 1
function __tessera_prompt --on-event fish_prompt
    set -l st $status
    __tessera_apply
    if set -q __tessera_running
        printf '\e]133;D;%s\a' $st
        set -e __tessera_running
    end
    printf '\e]7;file://%s%s\a' (hostname) "$PWD"
    printf '\e]133;A;redraw=1\a'
end
# What a kernel in another shell changed (its folder, exports), handed over
# by Tessera: applied before the next command and prompt.
function __tessera_apply
    set -l d (set -q XDG_RUNTIME_DIR; and echo $XDG_RUNTIME_DIR; or echo /tmp)/tessera-env
    test -f $d/$fish_pid.apply; or return 0
    source $d/$fish_pid.apply
    command rm -f $d/$fish_pid.apply $d/$fish_pid.cwd
end
function __tessera_preexec --on-event fish_preexec
    __tessera_apply
    set -g __tessera_running 1
    # The command line first (OSC 1337 TesseraCommand, base64), then the mark.
    printf '\e]1337;TesseraCommand=%s\a' (printf '%s' "$argv[1]" | base64 | string join '')
    printf '\e]133;C\a'
end
# The aliases (OSC 1337 TesseraAliases, base64), when they change: help and
# completion for an alias come from what it runs (ls -> lsd).
function __tessera_aliases --on-event fish_prompt
    set -l a (alias | string collect)
    test "$a" = "$__tessera_alias_seen"; and return
    set -g __tessera_alias_seen $a
    printf '\e]1337;TesseraAliases=%s\a' (printf '%s' "$a" | base64 | string join '')
end
# The exported environment, at each prompt, for Tessera's kernels and
# prompt (a file by the shell's pid: /proc only has what it started with).
function __tessera_env --on-event fish_prompt
    set -l d (set -q XDG_RUNTIME_DIR; and echo $XDG_RUNTIME_DIR; or echo /tmp)/tessera-env
    test -d $d; or mkdir -p -m 700 $d 2>/dev/null
    command env -0 > $d/$fish_pid 2>/dev/null
end
