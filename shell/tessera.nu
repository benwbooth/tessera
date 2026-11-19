# Tessera shell integration for nushell: prompt and command marks (OSC 133),
# the working directory (OSC 7), the command line (OSC 1337
# TesseraCommand), the exported environment at each prompt (for Tessera's
# kernels and prompt), and what another shell handed over (folder, exports).
# (Loaded with `nu -e 'source tessera.nu'`; nu's own marks are turned off so
# the command line comes before the command mark.)

$env.config = ($env.config | upsert shell_integration (
    try { $env.config.shell_integration | upsert osc133 false | upsert osc7 false } catch { {} }
))

$env.config = ($env.config | upsert hooks.pre_prompt (
    ($env.config.hooks.pre_prompt? | default []) | append {||
        let d = (($env.XDG_RUNTIME_DIR? | default "/tmp") | path join "tessera-env")
        let f = ($d | path join $"($nu.pid).apply")
        if ($f | path exists) {
            let a = (open --raw $f | from json)
            load-env ($a.set? | default {})
            for k in ($a.unset? | default []) { hide-env -i $k }
            if ($a.cd? | default "" | is-not-empty) { cd $a.cd }
            rm -f $f ($d | path join $"($nu.pid).cwd")
        }
        if ($env.__TESSERA_RUNNING? | default "" | is-not-empty) {
            print -n $"\e]133;D;($env.LAST_EXIT_CODE)\a"
            hide-env -i __TESSERA_RUNNING
        }
        mkdir $d
        ^env -0 | save -f ($d | path join $"($nu.pid)")
        print -n $"\e]7;file://($env.HOSTNAME? | default 'localhost')($env.PWD)\a\e]133;A\a"
    }
))

$env.config = ($env.config | upsert hooks.pre_execution (
    ($env.config.hooks.pre_execution? | default []) | append {||
        let line = (commandline)
        $env.__TESSERA_RUNNING = "1"
        let ex = ([($env.USER? | default ""), $line, (^env | str trim)] | str join (char nul) | encode base64 | str replace -a "\n" "")
        print -n $"\e]1337;TesseraExec=($ex)\a"
        print -n $"\e]1337;TesseraCommand=($line | encode base64 | str replace -a "\n" "")\a\e]133;C\a"
    }
))
