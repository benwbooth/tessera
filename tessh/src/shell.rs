//! The shell builtins a login shell needs beyond cd and jobs: trap, ulimit,
//! umask, read, getopts, shift, history, times, type, hash, exec, eval,
//! builtin and `command -v`, written in Python (the `__tessh_shell__`
//! module, below). And bash's builtins tessh does its own way (alias,
//! declare, local, readonly, shopt, …): each says what that way is, rather
//! than "command not found".

use crate::runtime::{output, set_status};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyString, PyTuple};
use std::os::unix::process::CommandExt;

/// Each builtin's usage, for `help NAME` and the completion panel.
pub fn usage(name: &str) -> Option<&'static str> {
    Some(match name {
        "wait" => "wait [-n] [%job | pid …]\n\nWait for background jobs: all of them without one (then the status is 0), or each one named (its exit status; a Python job's result). -n waits for whichever ends first. A job's value waits the same way: j = cmd & … then $j.wait on a line of its own, or r = ($j.wait). Ctrl+C stops the waiting, not the job.",
        "disown" => "disown [-ahr] [%job | pid …]\n\nTake a job off the list (the current one without a job), so it goes on running after tessh ends instead of getting SIGHUP. -h keeps it on the list but spares it the SIGHUP; -a does every job, -r every running one.",
        "kill" => "kill [-s SIG | -n NUM | -SIG] %job | pid …\nkill -l [NUM | SIG …]\n\nSend a signal (TERM without one) to jobs (%1, %%, %-, %name, $job: every program in it) and processes. A stopped job sent TERM or HUP is continued, so it can act on it. -l lists the signals, or names one by number (an exit status above 128 too). \\kill runs the program instead.",
        "suspend" => "suspend [-f]\n\nAt the prompt: stop tessh, as Ctrl+Z stops a program, until the shell that started it brings it back (fg). A login shell only with -f.",
        "logout" => "logout [status]\n\nEnd a login shell (exit, for one that isn't).",
        "trap" => "trap 'tessh text' SIGNAL …\ntrap - SIGNAL …   trap '' SIGNAL …   trap [-p] [SIGNAL …]   trap -l\n\nRun the text (tessh, in this shell: a function's name calls it) when a signal comes, or EXIT when tessh ends (a script, -c, the REPL). - puts a signal back as it was, '' ignores it; -p, or trap alone, lists the traps, -l the signals. Signals are named INT, SIGINT or 2. ERR runs when a command on a line of its own fails (as bash's: not one that if, ||, && or r = (…) checks), before strict mode stops the script. DEBUG and RETURN aren't in tessh: Python's sys.settrace does both.",
        "ulimit" => "ulimit [-SHa] [-cdefilmnqrstuvx] [limit]\n\nShow or set a resource limit for tessh and the programs it runs: -c core file size, -d data, -f file size (the default), -l locked memory, -m resident memory, -n open files, -s stack, -t CPU seconds, -u processes, -v virtual memory, -e nice, -i pending signals, -q message queues, -r real-time priority, -x file locks. Sizes are in KiB (-c and -f too). limit is a number, unlimited, soft or hard. -S the soft limit, -H the hard one (setting without either sets both, as bash does); -a shows them all. Python: import resource.",
        "umask" => "umask [-p] [-S] [mode]\n\nThe file mode mask: the permissions new files don't get. Alone, it prints it (0022); -S as u=rwx,g=rx,o=rx. mode is octal (077) or symbolic (u=rwx,g=,o=, g-w, a+r).",
        "read" => "read [-ers] [-i text] [-p prompt] [-n N | -N N] [-t secs] [-d delim] [-a list] [-u fd] [name …]\n\nRead a line from standard input (what <, <<< or a pipe gives this line) into names: split at $IFS (spaces, tabs and newlines without one), the last name getting the rest. No name: $REPLY, the whole line. -a: a list of all the words. -r keeps backslashes (without it, \\ joins lines and quotes the next character), -s doesn't echo (a password), -n N stops after N characters, -N after exactly N, -t gives up after secs (status 142), -d reads to delim instead of a newline, -p shows a prompt (on a terminal), -e reads the line with readline (editable, ↑ for earlier ones) and -i gives it text to start with. The status is 1 at the end of the input, so `while read line` loops over lines. It sets the module's names; in Python, input reads a line.",
        "getopts" => "getopts optstring name [arg …]\n\nbash's option parser, for a script's $argv (or the args): each call sets $name to the next option letter and $OPTARG to its argument (a letter followed by : in optstring takes one; None for one that doesn't), and $OPTIND to the index of the next argument; at the end it fails, so `while getopts \"ab:\" opt` loops. An unknown option gives ?, with a message (none, and $OPTARG the letter, when optstring starts with :). Then `shift ($OPTIND - 1)` leaves the rest. Python's argparse does more.",
        "shift" => "shift [n]\n\nDrop the first n (1) arguments after the script's name from $argv: $argv[1] goes, and the next takes its place. Fails, changing nothing, if there aren't that many.",
        "times" => "times\n\nThe CPU time tessh has used (user, then system) and, on the next line, its children's. Python: os.times.",
        "history" => "history [N]\nhistory -c   history -d N   history -s text\n\nThe commands typed at tessh's prompt, numbered, the last N of them with N. -c clears the history file, -d N deletes entry N, -s adds text. The file is $XDG_DATA_HOME/tessh/history (~/.local/share/tessh/history); the editor's ↑ and Ctrl+R search it with bash's and zsh's.",
        "fc" => "fc [-e editor] [-r] [first [last]]\nfc -l [-nr] [first [last]]   fc -s [old=new] [command]\n\nEdit past commands (the last without first: a number, -n back, or the start of one) in $FCEDIT, $EDITOR or vi, and run what's saved, shown first. -l lists them instead, as history does; -s runs one again at once, with old made new.",
        "caller" => "caller\n\nNot in tessh: a function's caller is Python's frame. traceback.print_stack shows the calls (import traceback), inspect.stack gives them as values.",
        "builtin" => "builtin name [arg …]\n\nRun tessh's builtin of that name, even when a function of yours has the name (def cd … that ends with builtin cd $dir).",
        "enable" => "enable\n\nNot in tessh: its builtins can't be turned off. A function of the same name takes a builtin's place, and \\name runs the program of that name.",
        "type" => "type [-afptP] name …\n\nWhat each name is as a command: an alias, a tessh keyword, a function (yours, or a command function), a variable, a tessh builtin, a Python builtin, or a program (and where). -t says only which (alias, keyword, function, variable, builtin or file), -p only a program's path, -P looks only for a program, -a shows everything the name is. Given a value instead of words (type $x), it's Python's type.",
        "hash" => "hash [-r] [-t name …] [name …]\n\ntessh doesn't remember where programs are: it looks each one up on $PATH as it runs it, so hash -r has nothing to forget. hash name checks it's there (and fails if not); -t prints where. Given a value (hash $x), it's Python's hash.",
        "exec" => "exec [-cl] [-a name] program [arg …]\n\nReplace tessh with the program (exec zsh, exec tmux): it never comes back. -c gives it an empty environment, -l starts it as a login shell (a - before its name), -a gives it another name. With only redirections (exec >log, exec 2>&1, exec 3<file) they stay in force, for tessh and every program after. Given code (exec $code, exec \"x = 1\"), it's Python's exec; builtins.exec is Python's whatever it's given.",
        "eval" => "eval text …\n\nRun the words joined as tessh text, here, as bash's eval does: eval \"echo hi\", eval $\"(ssh-agent -c)\" (tessh doesn't read sh: give it tessh, or run sh -c). Its value is the last form's, so x = (eval \"1 + 2\") is 3. Given a value (eval $code), it's Python's eval.",
        "set" => "set [-exvuCfn] [+exvCfn] [-o NAME | +o NAME] [-o] [-- arg …]\n\nbash's options: -e (errexit, strict mode), -C (noclobber: > won't overwrite a file, >| will), -f (noglob: globs stay as written), -n (noexec: a script is read and checked, not run), -o ignoreeof (Ctrl+D ten times at the prompt before it leaves; $IGNOREEOF sets how many), -x (xtrace: each command shown before it runs, after $PS4 or +), -v (verbose: each line shown as it's read), -u (nounset: always so in tessh), -o pipefail (a pipe fails if any of its programs does; $PIPESTATUS has each one's status). + turns one off, -o alone lists them. set -- a b (or set a b) makes $argv [script a b]. set with a value is Python's set.",
        "mapfile" | "readarray" => "mapfile [-t] [-d delim] [-n count] [-O origin] [-s count] [-u fd] [-C callback] [-c quantum] [name]\n\nThe lines of standard input (what <, <<< or a pipe gives: cat f | mapfile -t lines) into a list, $MAPFILE without a name. -t drops each line's newline (delim with -d), -n reads at most count, -s skips the first count, -O puts them from index origin on (keeping what's before), -u reads descriptor fd, -C calls callback (index, line) every quantum lines (-c, 5000).",
        "time" => "time [-p] command …\ntime [-p]\n    block\n\nRun the command (a program, a pipe, a call: tessh text), or the indented block under it, then say how long it took, real, user and sys, to stderr, as bash's time. -p in POSIX's form. Its value is the command's. \\time runs the time program (/usr/bin/time -v …).",
        "coproc" => "coproc [NAME --] command [arg …]\ncoproc [NAME]\n    block\n\nRun the command (or the indented block, in a fork of tessh that sees its names) in the background with its input and output piped to tessh: $COPROC is [read fd, write fd] (from its output, to its input), $COPROC_PID its process ID, and its value is its Job (jobs, wait and kill take it). NAME -- names them $NAME and $NAME_PID instead. Write with cmd >&$COPROC[1] or os.write, read with read -u $COPROC[0] or os.read; exec $COPROC[1]>&- closes its input.",
        "compopt" => "compopt [-o option] [+o option] [name …]\n\nTurn complete's -o options (default, bashdefault, dirnames, plusdirs, nosort, filenames, nospace) on (-o) or off (+o) for the named commands, or, inside a -F function, for the completion it's making. Alone, it lists them.",
        "readonly" => "readonly [-p] [NAME[=value] …]\n\nMake module names read-only: a later NAME = … (at the prompt, or further on in the script) is an error. NAME=value sets it first (to the text value). Alone, or with -p, it lists them. A def's own names, and globals() from Python, aren't checked.",
        "select" => "select item …\n\nbash's menu, as a loop: for choice in (select red green blue) prints the items numbered (to stderr), asks $PS3 (#? without it) and gives the item picked each time; a number that isn't on the menu gives None, Enter alone shows the menu again, and the end of the input ends the loop (break ends it too). $REPLY is what was typed.",
        "complete" => "complete [-fdcbkv] [-W words] [-F function] [-A action] name …\ncomplete -r [name …]   complete [-p] [name …]\n\nWhat Tab offers after a command: -W the words (a string split at spaces), -F a function given (word, words of the line so far) that returns a list, -f files, -d folders, -c commands, -v variables, -k keywords, -b builtins. -r forgets a command's, -p (or complete alone) lists them.",
        "compgen" => "compgen [-fdcbkv] [-W words] [-F function] [-A action] [--] [word]\n\nThe completions complete would offer for word, one a line: -W words, -f files, -d folders, -c commands (programs, builtins and functions), -v variables, -k keywords, -b builtins, -A function the functions.",
        "alias" => "alias [-p] [name=value …] [name …]\n\nbash's aliases: name runs value with its words after it (alias ll='ls -l', then ll /tmp is ls -l /tmp). The value is tessh text, run where the alias is used, so a pipe or a call works too; an alias that starts with its own name runs the program (alias ls='ls --color'). alias alone, or -p, lists them; alias name shows one. unalias name removes one, unalias -a all. A @command def does the same with its arguments anywhere.",
        "unalias" => "unalias [-a] name …\n\nRemove aliases (all of them with -a).",
        "declare" | "typeset" | "local" | "shopt" | "bind" => return idiom(name),
        _ => return None,
    })
}

/// bash's builtins tessh does another way: what to do instead.
fn idiom(name: &str) -> Option<&'static str> {
    Some(match name {
        "declare" | "typeset" => "tessh's variables are Python's: name = value sets one, of any type (bash's -i, -a and -A are an int, a list and a dict), export NAME=value sets an environment variable (declare -x), a name set in a def is the def's own (declare in a function), and print $name or pretty $name shows one (declare -p).",
        "local" => "a name set in a def is already the def's own (Python's scopes): no local is needed. global name or nonlocal name sets one outside instead.",
        "shopt" => "tessh has no shell options: globs, strings and folders work one way (help globs), strict on is bash's set -e, and help abs-options says what to do for the rest.",
        "bind" => "tessh's editor keys are its own, not readline's: help keys (or tessh --keys) lists them.",
        _ => return None,
    })
}

/// Whether the builtin is given the namespace it runs in (to set names, run text there).
pub fn wants_ns(name: &str) -> bool {
    matches!(name, "trap" | "read" | "getopts" | "shift" | "type" | "eval" | "builtin" | "hash" | "readonly" | "select" | "complete" | "compgen" | "compopt" | "fc" | "alias" | "unalias" | "set" | "mapfile" | "readarray" | "time" | "coproc")
}

/// Run builtin `name` with its words: the Python function `sh_name`, or the
/// message for one tessh does another way.
pub fn run(py: Python<'_>, name: &str, words: &[String], ns: Option<Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
    if let Some(msg) = idiom(name) {
        return Err(pyo3::exceptions::PyNotImplementedError::new_err(format!("{name}: {msg}")));
    }
    let m = py.import("__tessh_shell__")?;
    let f = m.getattr(format!("sh_{name}").as_str())?;
    let kw = PyDict::new(py);
    if wants_ns(name) {
        kw.set_item("_ns", ns)?;
    }
    let args = PyTuple::new(py, words.iter().map(|w| PyString::new(py, w)))?;
    let v = f.call(args, Some(&kw))?;
    // (An Output: its status is the shell's.)
    if let Ok(c) = v.getattr("code").and_then(|c| c.extract::<i32>()) {
        if v.hasattr("attached").unwrap_or(false) {
            set_status(c);
        }
    }
    Ok(v.unbind())
}

/// `command -v name`, `command -V name`, `command -p prog …`.
pub fn command_opts(py: Python<'_>, args: &[Bound<'_, PyAny>]) -> PyResult<Py<PyAny>> {
    let words: Vec<String> = args.iter().map(|a| a.str().map(|s| s.to_string())).collect::<PyResult<_>>()?;
    // (The namespace of whatever called command: the frame above.)
    let ns = py.import("sys")?.getattr("_getframe")?.call1((0,))?.getattr("f_globals")?;
    let f = py.import("__tessh_shell__")?.getattr("command_opts")?;
    Ok(f.call1((words, ns))?.unbind())
}

/// The words of a command line, as history expansion counts them (quotes
/// kept, spaces inside them not splitting).
fn hist_words(e: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut q: Option<char> = None;
    for c in e.chars() {
        match q {
            Some(x) => {
                cur.push(c);
                if c == x {
                    q = None;
                }
            }
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    q = Some(c);
                }
                cur.push(c);
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// One modifier's change: :h :t :r :e :q :x, s/old/new/ (g: every one).
fn hist_modify(v: &str, m: &str, last_sub: &mut Option<(String, String)>) -> Result<String, String> {
    let path_part = |v: &str, f: &dyn Fn(&str) -> String| v.to_string().split(' ').map(f).collect::<Vec<_>>().join(" ");
    Ok(match m {
        "h" => path_part(v, &|w| match w.rfind('/') { Some(0) => "/".into(), Some(i) => w[..i].into(), None => w.into() }),
        "t" => path_part(v, &|w| w.rsplit('/').next().unwrap_or(w).into()),
        "r" => path_part(v, &|w| match (w.rfind('.'), w.rfind('/')) { (Some(d), s) if s.is_none_or(|s| d > s) => w[..d].into(), _ => w.into() }),
        "e" => path_part(v, &|w| match (w.rfind('.'), w.rfind('/')) { (Some(d), s) if s.is_none_or(|s| d > s) => w[d + 1..].into(), _ => String::new() }),
        "q" => format!("'{}'", v.replace('\'', "'\\''")),
        "x" => hist_words(v).iter().map(|w| format!("'{}'", w.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" "),
        _ => {
            let (global, body) = if let Some(b) = m.strip_prefix("gs").or_else(|| m.strip_prefix("as")) { (true, Some(b)) } else if let Some(b) = m.strip_prefix('s') { (false, Some(b)) } else { (m.starts_with('g') || m.starts_with('a'), None) };
            let (old, new) = match body {
                Some(b) => {
                    let d = b.chars().next().ok_or_else(|| ":s: which delimiter?".to_string())?;
                    let parts: Vec<&str> = b[d.len_utf8()..].splitn(3, d).collect();
                    let old = parts.first().copied().unwrap_or("").to_string();
                    let old = if old.is_empty() { last_sub.as_ref().map(|s| s.0.clone()).ok_or_else(|| ":s: no previous substitution".to_string())? } else { old };
                    let new = parts.get(1).copied().unwrap_or("").replace('&', &old);
                    (old, new)
                }
                None if m.trim_start_matches(['g', 'a']) == "&" => last_sub.clone().ok_or_else(|| ":&: no previous substitution".to_string())?,
                None => return Err(format!(":{m}: unrecognized history modifier")),
            };
            *last_sub = Some((old.clone(), new.clone()));
            if !v.contains(&old) {
                return Err(format!(":s/{old}/: substitution failed"));
            }
            if global { v.replace(&old, &new) } else { v.replacen(&old, &new, 1) }
        }
    })
}

/// bash's history expansion, at the prompt. Events: `!!` the last command,
/// `!n` entry n (as history numbers them), `!-n` the nth back, `!prefix`
/// the latest starting with prefix, `!?text?` the latest with text in it,
/// `!#` the line so far. Words, after `:` (or `!$ !^ !*` alone): `0`, `n`,
/// `^`, `$`, `*`, `x-y`, `x-`, `%` (the `?text?` word). Modifiers, each
/// after `:`: `h t r e` (path parts), `p` (show, don't run), `q x` (quote),
/// `s/old/new/`, `gs/old/new/`, `&`, `g&`. `^old^new^` at the start is
/// `!!:s/old/new/`. Not inside '…', and `!` before a space, =, ( or the end
/// is left as it is (tessh's ! and !=). None: nothing to expand; else the
/// line, and whether only to show it (:p).
pub fn expand_history(text: &str, file: &std::path::Path) -> Result<Option<(String, bool)>, String> {
    let entries: Vec<String> = std::fs::read_to_string(file).unwrap_or_default().lines().filter(|l| !l.trim().is_empty()).map(|l| l.replace("<\\n>", "\n")).collect();
    let last = || entries.last().cloned().ok_or_else(|| "!!: no history yet".to_string());
    let mut last_sub: Option<(String, String)> = None;
    let mut print_only = false;
    // (^old^new^: the last command with old made new.)
    if let Some(rest) = text.strip_prefix('^') {
        let parts: Vec<&str> = rest.splitn(3, '^').collect();
        if parts.len() >= 2 && !parts[0].is_empty() {
            let e = last()?;
            let line = hist_modify(&e, &format!("s^{}^{}^", parts[0], parts[1]), &mut last_sub)?;
            let tail = parts.get(2).copied().unwrap_or("");
            return Ok(Some((line + tail, false)));
        }
    }
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut changed = false;
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if let Some(q) = quote {
            if c == '\\' && q == '"' && i + 1 < cs.len() {
                out.push(c);
                out.push(cs[i + 1]);
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            } else if q == '\'' || c != '!' {
                out.push(c);
                i += 1;
                continue;
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c == '\\' && i + 1 < cs.len() {
            out.push(c);
            out.push(cs[i + 1]);
            i += 2;
            continue;
        }
        if c != '!' || i + 1 >= cs.len() {
            out.push(c);
            i += 1;
            continue;
        }
        let at_start = i == 0 || cs[i - 1].is_whitespace() || "(;|&".contains(cs[i - 1]);
        let n = cs[i + 1];
        let mut j = i + 1;
        let take = |j: &mut usize, stop: &dyn Fn(char) -> bool| -> String {
            let mut s = String::new();
            while *j < cs.len() && !stop(cs[*j]) {
                s.push(cs[*j]);
                *j += 1;
            }
            s
        };
        let word_end = |c: char| c.is_whitespace() || "();|&'\":".contains(c);
        // (The event, and a word designator given with it: !$ !^ !*.)
        let mut matched_word: Option<String> = None;
        let (event, mut designator): (String, Option<String>) = match n {
            '!' => {
                j += 1;
                (last()?, None)
            }
            '$' | '^' | '*' => {
                j += 1;
                (last()?, Some(n.to_string()))
            }
            '#' => {
                j += 1;
                (out.clone(), None)
            }
            _ if !at_start && n != '?' => {
                out.push(c);
                i += 1;
                continue;
            }
            '?' => {
                j += 1;
                let t = take(&mut j, &|c| c == '?' || c == '\n');
                if j < cs.len() && cs[j] == '?' {
                    j += 1;
                }
                let e = entries.iter().rev().find(|e| e.contains(&t)).ok_or_else(|| format!("!?{t}: event not found"))?;
                matched_word = hist_words(e).into_iter().find(|w| w.contains(&t));
                (e.clone(), None)
            }
            '-' if cs.get(i + 2).is_some_and(|c| c.is_ascii_digit()) => {
                j += 1;
                let d = take(&mut j, &|c| !c.is_ascii_digit());
                let k: usize = d.parse().map_err(|_| format!("!-{d}: event not found"))?;
                let e = entries.len().checked_sub(k).and_then(|x| entries.get(x)).ok_or_else(|| format!("!-{d}: event not found"))?;
                (e.clone(), None)
            }
            _ if n.is_ascii_digit() => {
                let d = take(&mut j, &|c| !c.is_ascii_digit());
                let k: usize = d.parse().map_err(|_| format!("!{d}: event not found"))?;
                let e = k.checked_sub(1).and_then(|x| entries.get(x)).ok_or_else(|| format!("!{d}: event not found"))?;
                (e.clone(), None)
            }
            _ if n.is_alphabetic() || "_./~".contains(n) => {
                let p = take(&mut j, &word_end);
                let e = entries.iter().rev().find(|e| e.starts_with(&p)).ok_or_else(|| format!("!{p}: event not found"))?;
                (e.clone(), None)
            }
            _ => {
                out.push(c);
                i += 1;
                continue;
            }
        };
        // (A designator right after the event needs no colon: !?text?%, !cp$.)
        if designator.is_none() && j < cs.len() && "^$*%".contains(cs[j]) && !matches!(n, '$' | '^' | '*') {
            designator = Some(cs[j].to_string());
            j += 1;
        }
        // (:designator and :modifiers.)
        let mut mods: Vec<String> = vec![];
        while j < cs.len() && cs[j] == ':' && j + 1 < cs.len() {
            let k0 = j + 1;
            let first = cs[k0];
            if designator.is_none() && mods.is_empty() && (first.is_ascii_digit() || "^$*%-".contains(first)) {
                j = k0;
                designator = Some(take(&mut j, &|c| !(c.is_ascii_digit() || "^$*%-".contains(c))));
                continue;
            }
            j = k0;
            let m = if first == 's' || (first == 'g' || first == 'a') && cs.get(k0 + 1) == Some(&'s') {
                let start = j;
                let pre = if first == 's' { 1 } else { 2 };
                j += pre;
                let d = cs.get(j).copied().ok_or_else(|| ":s: which delimiter?".to_string())?;
                j += 1;
                let mut seen = 0;
                while j < cs.len() && seen < 2 {
                    if cs[j] == d {
                        seen += 1;
                    }
                    j += 1;
                    if seen < 2 && j >= cs.len() {
                        break;
                    }
                }
                cs[start..j].iter().collect::<String>()
            } else if first == 'g' || first == 'a' {
                j += 2;
                cs[k0..j.min(cs.len())].iter().collect()
            } else {
                j += 1;
                first.to_string()
            };
            mods.push(m);
        }
        let ws = hist_words(&event);
        let pick = |a: usize, b: usize| -> Result<String, String> {
            if a > b || b >= ws.len() {
                return Err(format!("{}: bad word specifier", designator.clone().unwrap_or_default()));
            }
            Ok(ws[a..=b].join(" "))
        };
        let lastw = ws.len().saturating_sub(1);
        let mut v = match designator.as_deref() {
            None => event.clone(),
            Some("$") => pick(lastw, lastw)?,
            Some("^") => pick(1, 1)?,
            Some("*") => if ws.len() > 1 { pick(1, lastw)? } else { String::new() },
            Some("%") => matched_word.clone().ok_or_else(|| ":%: no ?text? search".to_string())?,
            Some(d) => {
                let (a, b) = match d.split_once('-') {
                    Some((a, "")) => (a.parse().unwrap_or(0), lastw.saturating_sub(1)),
                    Some(("", b)) => (0, if b == "$" { lastw } else { b.parse().map_err(|_| format!("{d}: bad word specifier"))? }),
                    Some((a, b)) => (a.parse().map_err(|_| format!("{d}: bad word specifier"))?, if b == "$" { lastw } else { b.parse().map_err(|_| format!("{d}: bad word specifier"))? }),
                    None => {
                        let k: usize = d.parse().map_err(|_| format!("{d}: bad word specifier"))?;
                        (k, k)
                    }
                };
                pick(a, b)?
            }
        };
        for m in &mods {
            if m == "p" {
                print_only = true;
            } else {
                v = hist_modify(&v, m, &mut last_sub)?;
            }
        }
        out.push_str(&v);
        changed = true;
        i = j;
    }
    Ok(changed.then_some((out, print_only)))
}

/// A shell variable bash reads (HISTSIZE, IGNOREEOF…): the REPL's name
/// first, then the environment's.
fn shell_var(ns: &Bound<'_, PyDict>, name: &str) -> Option<String> {
    if let Ok(Some(v)) = ns.get_item(name) {
        if !v.is_none() {
            return v.str().ok().map(|s| s.to_string());
        }
    }
    std::env::var(name).ok()
}

/// What happens to a line typed at the prompt, for history.
pub struct HistPolicy {
    /// Kept in history ($HISTCONTROL, $HISTIGNORE and a leading space can say no).
    pub keep: bool,
    /// erasedups: earlier copies go.
    pub erase: bool,
    /// $HISTSIZE: how many the editor keeps.
    pub size: Option<usize>,
    /// $HISTFILESIZE: how many the file keeps.
    pub file_size: Option<usize>,
    text: String,
}

/// $HISTCONTROL (ignorespace, ignoredups, ignoreboth, erasedups),
/// $HISTIGNORE (patterns, : between them, & the last line), $HISTSIZE and
/// $HISTFILESIZE, as bash reads them.
pub fn history_policy(_py: Python<'_>, ns: &Bound<'_, PyDict>, text: &str, file: &std::path::Path) -> HistPolicy {
    let control = shell_var(ns, "HISTCONTROL").unwrap_or_default();
    let has = |w: &str| control.split(':').any(|c| c == w);
    let last = std::fs::read_to_string(file).ok().and_then(|t| t.lines().filter(|l| !l.trim().is_empty()).last().map(|l| l.replace("<\\n>", "\n")));
    let mut keep = !text.starts_with(' ');
    if (has("ignorespace") || has("ignoreboth")) && text.starts_with(' ') {
        keep = false;
    }
    if (has("ignoredups") || has("ignoreboth")) && last.as_deref() == Some(text) {
        keep = false;
    }
    if let Some(ign) = shell_var(ns, "HISTIGNORE") {
        for pat in ign.split(':').filter(|p| !p.is_empty()) {
            let pat = if pat == "&" { last.clone().unwrap_or_default() } else { pat.to_string() };
            if glob::Pattern::new(&pat).is_ok_and(|p| p.matches(text)) {
                keep = false;
            }
        }
    }
    let num = |n: &str| shell_var(ns, n).and_then(|v| v.trim().parse::<i64>().ok()).map(|v| v.max(0) as usize);
    HistPolicy { keep, erase: has("erasedups"), size: num("HISTSIZE"), file_size: num("HISTFILESIZE").or(num("HISTSIZE")), text: text.to_string() }
}

/// After a line went in: erasedups and $HISTFILESIZE, in the file.
pub fn history_trim(file: &std::path::Path, p: &HistPolicy) {
    if !p.erase && p.file_size.is_none() {
        return;
    }
    let Ok(t) = std::fs::read_to_string(file) else { return };
    let mut lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
    if p.erase {
        let me = p.text.replace('\n', "<\\n>");
        let n = lines.len();
        let mut k = 0;
        lines.retain(|l| {
            k += 1;
            k == n || *l != me
        });
    }
    if let Some(n) = p.file_size {
        if lines.len() > n {
            lines.drain(..lines.len() - n);
        }
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    let _ = std::fs::write(file, out);
}

static EOFS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Ctrl+D at the prompt: whether to stay ($IGNOREEOF, or 10 with set -o
/// ignoreeof, Ctrl+Ds in a row before it leaves).
pub fn ignore_eof(_py: Python<'_>, ns: &Bound<'_, PyDict>) -> bool {
    let limit = match shell_var(ns, "IGNOREEOF") {
        Some(v) => v.trim().parse::<usize>().unwrap_or(10),
        None if crate::runtime::IGNOREEOF.load(std::sync::atomic::Ordering::SeqCst) => 10,
        None => return false,
    };
    let n = EOFS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    n <= limit
}

/// A line was typed: Ctrl+Ds count from nothing again.
pub fn eof_reset() {
    EOFS.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// Tessh text, run in ns (a trap's, eval's): the last form's value.
#[pyfunction]
fn eval_tessh(py: Python<'_>, text: &str, ns: Bound<'_, PyDict>) -> PyResult<Py<PyAny>> {
    crate::runner::eval_source(py, text, &ns, "<eval>").map_err(|e| e.into_pyerr(py))
}

/// exec: this process becomes the program; on success it never returns.
#[pyfunction]
#[pyo3(signature = (prog, argv0, args, clear_env=false))]
fn exec_program(py: Python<'_>, prog: String, argv0: String, args: Vec<String>, clear_env: bool) -> PyResult<()> {
    let path = if prog.contains('/') { prog.clone() } else { crate::runtime::which(&prog).ok_or_else(|| pyo3::exceptions::PyFileNotFoundError::new_err(format!("exec: {prog}: not found")))? };
    if let Ok(sys) = py.import("sys") {
        for s in ["stdout", "stderr"] {
            if let Ok(f) = sys.getattr(s) {
                let _ = f.call_method0("flush");
            }
        }
    }
    let mut c = std::process::Command::new(&path);
    c.arg0(&argv0).args(&args);
    if clear_env {
        c.env_clear();
    }
    // (The program starts with every signal as the system has it: what tessh
    // and Python ignore or catch (Ctrl+Z, SIGPIPE, …) would carry over.
    // Kept, to put back if exec fails.)
    let mut old: Vec<(i32, libc::sigaction)> = vec![];
    unsafe {
        for s in 1..32 {
            if s == libc::SIGKILL || s == libc::SIGSTOP {
                continue;
            }
            let mut new: libc::sigaction = std::mem::zeroed();
            new.sa_sigaction = libc::SIG_DFL;
            let mut was: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(s, &new, &mut was) == 0 {
                old.push((s, was));
            }
        }
        let mut none: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut none);
        libc::sigprocmask(libc::SIG_SETMASK, &none, std::ptr::null_mut());
    }
    let e = c.exec();
    unsafe {
        for (s, was) in old {
            libc::sigaction(s, &was, std::ptr::null_mut());
        }
    }
    Err(pyo3::exceptions::PyOSError::new_err(format!("exec: {path}: {e}")))
}

/// A signal back to what tessh has it as: Ctrl+C's KeyboardInterrupt, the
/// REPL's ignored Ctrl+Z, SIGHUP passed on to the jobs, the rest the system's.
#[pyfunction]
fn restore_signal(py: Python<'_>, sig: i32) -> PyResult<()> {
    let signal = py.import("signal")?;
    if sig == libc::SIGINT {
        signal.getattr("signal")?.call1((sig, signal.getattr("default_int_handler")?))?;
        return Ok(());
    }
    // (Through Python, so it forgets its handler; then the real disposition.)
    signal.getattr("signal")?.call1((sig, signal.getattr("SIG_DFL")?))?;
    let interactive = crate::runtime::INTERACTIVE.load(std::sync::atomic::Ordering::SeqCst);
    unsafe {
        if sig == libc::SIGHUP {
            crate::jobs::on_hangup();
        } else if interactive && [libc::SIGTSTP, libc::SIGTTOU, libc::SIGTTIN].contains(&sig) {
            libc::signal(sig, libc::SIG_IGN);
        } else if sig == libc::SIGPIPE || sig == libc::SIGXFSZ {
            libc::signal(sig, libc::SIG_IGN);
        }
    }
    Ok(())
}

#[pyfunction]
fn builtin_named<'py>(py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    crate::runtime::builtin_named(py, name)
}

/// set -o NAME / +o NAME.
#[pyfunction]
fn set_option(name: &str, on: bool) -> PyResult<()> {
    use crate::runtime::*;
    let flag = match name {
        "errexit" => &STRICT,
        "xtrace" => &XTRACE,
        "verbose" => &VERBOSE,
        "pipefail" => &PIPEFAIL,
        "noclobber" => &NOCLOBBER,
        "noglob" => &NOGLOB,
        "noexec" => &NOEXEC,
        "ignoreeof" => &IGNOREEOF,
        _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("set: {name}: invalid option name"))),
    };
    flag.store(on, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[pyfunction]
fn get_option(name: &str) -> bool {
    use crate::runtime::*;
    let o = std::sync::atomic::Ordering::SeqCst;
    match name {
        "errexit" => STRICT.load(o),
        "xtrace" => XTRACE.load(o),
        "verbose" => VERBOSE.load(o),
        "pipefail" => PIPEFAIL.load(o),
        "noclobber" => NOCLOBBER.load(o),
        "noglob" => NOGLOB.load(o),
        "noexec" => NOEXEC.load(o),
        "ignoreeof" => IGNOREEOF.load(o),
        _ => false,
    }
}

#[pyfunction]
fn xtrace_on() -> bool {
    crate::runtime::XTRACE.load(std::sync::atomic::Ordering::SeqCst)
}

#[pyfunction]
fn set_debug_trap(on: bool) {
    crate::runtime::DEBUG_TRAP.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[pyfunction]
fn builtin_names() -> Vec<&'static str> {
    crate::runtime::COMMAND_BUILTINS.to_vec()
}

#[pyfunction]
fn set_err_trap(on: bool) {
    crate::runtime::ERR_TRAP.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[pyfunction]
fn is_login() -> bool {
    crate::LOGIN.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn add(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add_function(wrap_pyfunction!(eval_tessh, m)?)?;
    m.add_function(wrap_pyfunction!(exec_program, m)?)?;
    m.add_function(wrap_pyfunction!(restore_signal, m)?)?;
    m.add_function(wrap_pyfunction!(builtin_named, m)?)?;
    m.add_function(wrap_pyfunction!(is_login, m)?)?;
    m.add_function(wrap_pyfunction!(set_err_trap, m)?)?;
    m.add_function(wrap_pyfunction!(builtin_names, m)?)?;
    m.add_function(wrap_pyfunction!(set_option, m)?)?;
    m.add_function(wrap_pyfunction!(get_option, m)?)?;
    m.add_function(wrap_pyfunction!(xtrace_on, m)?)?;
    m.add_function(wrap_pyfunction!(set_debug_trap, m)?)?;
    let code = std::ffi::CString::new(SHELL_PY).unwrap();
    let sh = PyModule::from_code(py, &code, c"<tessh builtins>", c"__tessh_shell__")?;
    py.import("sys")?.getattr("modules")?.set_item("__tessh_shell__", &sh)?;
    // (What `time` and `coproc` with a block compile to.)
    m.add("timed", sh.getattr("timed")?)?;
    m.add("coproc_block", sh.getattr("coproc_block")?)?;
    // (An interactive tessh's jobs get SIGHUP when it ends, or its terminal does.)
    py.import("atexit")?.call_method1("register", (m.getattr("hup_jobs")?,))?;
    crate::jobs::on_hangup();
    // (SHLVL one deeper, as every shell has it.)
    let lvl = std::env::var("SHLVL").ok().and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0);
    crate::runtime::setenv(py, "SHLVL", Some(&(lvl.max(0) + 1).to_string()))?;
    // ($SHELL, when nothing set it: this tessh, as bash sets it.)
    if std::env::var_os("SHELL").is_none() {
        if let Ok(exe) = std::env::current_exe() {
            crate::runtime::setenv(py, "SHELL", Some(&exe.to_string_lossy()))?;
        }
    }
    let _ = output;
    Ok(())
}

const SHELL_PY: &str = r#####"
import os, sys

def _tessh():
    import __tessh__
    return __tessh__

def _status(code):
    return _tessh().Output("", code, "", True)

def _err(msg):
    sys.stdout.flush()
    print("tessh: " + msg, file=sys.stderr, flush=True)

def _split_opts(args, with_value=""):
    """bash-style options: (letters, {letter: value}, rest). with_value: the
    letters that take a value (glued, -p'> ', or the next word)."""
    flags, vals, i = [], {}, 0
    args = list(args)
    while i < len(args):
        a = args[i]
        if a == "--":
            i += 1
            break
        if not a.startswith("-") or a == "-":
            break
        j = 1
        while j < len(a):
            c = a[j]
            if c in with_value:
                v = a[j + 1:]
                if not v:
                    i += 1
                    if i >= len(args):
                        raise ValueError(f"-{c}: option requires an argument")
                    v = args[i]
                vals[c] = v
                break
            flags.append(c)
            j += 1
        i += 1
    return flags, vals, args[i:]

def staged(f, *rest, _last=False, **kw):
    """A builtin in a pipe: what came in (rest's last) is its input, and what
    it prints its output, as text for the next stage (unless it's last)."""
    import io, contextlib
    words, data = rest[:-1], rest[-1]
    old_in = sys.stdin
    if data is not None:
        text = data.decode("utf-8", "replace") if isinstance(data, bytes) else str(data)
        sys.stdin = io.StringIO(text + ("\n" if text and not text.endswith("\n") else ""))
    try:
        if _last:
            return f(*words, **kw)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            v = f(*words, **kw)
        # (Its last newline kept: the next program reads lines.)
        code = getattr(v, "code", 0) if v is not None else 0
        return _tessh().Output(buf.getvalue(), code, "", False)
    finally:
        sys.stdin = old_in

def captured(f, *words, **kw):
    """A builtin used as a value: what it prints, as text (with its status),
    unless it gives a value of its own (which's path, glob's list)."""
    import io, contextlib
    Output = _tessh().Output
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        v = f(*words, **kw)
    if v is None or (isinstance(v, Output) and getattr(v, "attached", False) and not str(v)):
        out = buf.getvalue()
        if out.endswith("\n"):
            out = out[:-1]
        return Output(out, getattr(v, "code", 0) if v is not None else 0, getattr(v, "err", "") if v is not None else "", False)
    return v

# ---- trap ------------------------------------------------------------------------------

_traps = {}
_trap_ns = {}
_exit_hooked = False

def _signum(s):
    u = s.upper()
    if u.startswith("SIG") and u[3:] in ("ERR", "DEBUG", "RETURN", "EXIT"):
        u = u[3:]
    if u in ("EXIT", "SIGEXIT", "0"):
        return "EXIT"
    if u in ("ERR", "DEBUG", "RETURN"):
        return u
    return _tessh().signal_number(s)

def _signame(k):
    if k in ("EXIT", "ERR", "DEBUG", "RETURN"):
        return k
    names = {v: n for n, v in _tessh().signal_names()}
    return "SIG" + names[k] if k in names else str(k)

def _run_trap(k):
    text = _traps.get(k)
    if not text:
        return
    try:
        _tessh().eval_tessh(text, _trap_ns[k])
    except (SystemExit, KeyboardInterrupt):
        raise
    except BaseException as e:
        _err(f"trap {_signame(k)}: {type(e).__name__}: {e}")

_in_err_trap = False
_in_trap = set()

def before_command(line):
    """Before each command: set -x prints it ($PS4, + without one), then the
    DEBUG trap runs, $BASH_COMMAND the command."""
    # (set's own line, as written: the compiler sends it through builtin.)
    if line.startswith("builtin set "):
        line = line[len("builtin "):]
    if _tessh().xtrace_on():
        ps4 = os.environ.get("PS4") or "+ "
        sys.stdout.flush()
        sys.stderr.write(ps4 + line + "\n")
        sys.stderr.flush()
    if "DEBUG" in _traps and "DEBUG" not in _in_trap:
        ns = _trap_ns.get("DEBUG")
        if ns is not None:
            ns["BASH_COMMAND"] = line
        _in_trap.add("DEBUG")
        try:
            _run_trap("DEBUG")
        finally:
            _in_trap.discard("DEBUG")

def _user_code(code):
    """Code of the user's tessh (a script, the prompt, a sourced file), not
    tessh's own or a library's."""
    f = code.co_filename
    return not f.startswith(("<tessh", "<frozen", "<eval>")) and not f.endswith(".py")

def _on_profile(frame, event, arg):
    if event == "return" and "RETURN" in _traps and "RETURN" not in _in_trap and frame.f_code.co_name != "<module>" and _user_code(frame.f_code):
        _in_trap.add("RETURN")
        try:
            _run_trap("RETURN")
        finally:
            _in_trap.discard("RETURN")

def _return_hook(on):
    """trap … RETURN: run when a function of the user's tessh returns (and
    when source finishes a file)."""
    import threading
    sys.setprofile(_on_profile if on else None)
    threading.setprofile(_on_profile if on else None)

def after_source():
    if "RETURN" in _traps and "RETURN" not in _in_trap:
        _in_trap.add("RETURN")
        try:
            _run_trap("RETURN")
        finally:
            _in_trap.discard("RETURN")

def run_err_trap():
    """A command failed on a line of its own: the ERR trap (not from inside it)."""
    global _in_err_trap
    if _in_err_trap:
        return
    _in_err_trap = True
    try:
        _run_trap("ERR")
    finally:
        _in_err_trap = False

def _on_exit():
    k = "EXIT"
    if k in _traps:
        try:
            _run_trap(k)
        except KeyboardInterrupt:
            pass
        except SystemExit:
            pass
        _traps.pop(k, None)

def _on_signal(signum, frame):
    _run_trap(signum)

def sh_trap(*args, _ns=None):
    import signal, atexit, shlex
    global _exit_hooked
    args = list(args)
    if args and args[0] == "--":
        args = args[1:]
    if not args or args[0] == "-p":
        keys = [_signum(a) for a in args[1:]] if args else list(_traps)
        for k in keys:
            if k in _traps:
                print(f"trap -- {shlex.quote(_traps[k])} {_signame(k)}")
        return None
    if args[0] == "-l":
        rows = [f"{v:2}) SIG{n}" for n, v in _tessh().signal_names()]
        for i in range(0, len(rows), 5):
            print("\t".join(rows[i:i + 5]))
        return None
    if len(args) == 1 or args[0].isdigit():
        action, sigs = "-", args
    else:
        action, sigs = args[0], args[1:]
    for s in sigs:
        k = _signum(s)
        if k is None:
            raise ValueError(f"trap: {s}: invalid signal specification")
        if k in ("DEBUG", "RETURN"):
            if action == "-" or action == "":
                _traps.pop(k, None)
            else:
                _traps[k] = action
                _trap_ns[k] = _ns
            if k == "DEBUG":
                _tessh().set_debug_trap(k in _traps)
            else:
                _return_hook(k in _traps)
            continue
        if k == "ERR":
            if action == "-" or action == "":
                _traps.pop(k, None)
            else:
                _traps[k] = action
                _trap_ns[k] = _ns
            _tessh().set_err_trap(k in _traps)
            continue
        if k == "EXIT":
            _tessh().set_exit_trap(action != "-")
            if action == "-":
                _traps.pop(k, None)
            else:
                _traps[k] = action
                _trap_ns[k] = _ns
                if not _exit_hooked:
                    atexit.register(_on_exit)
                    _exit_hooked = True
            continue
        if k in (signal.SIGKILL, signal.SIGSTOP):
            raise ValueError(f"trap: {_signame(k)}: can't be caught or ignored")
        if action == "-":
            _tessh().restore_signal(k)
            _traps.pop(k, None)
        elif action == "":
            signal.signal(k, signal.SIG_IGN)
            _traps[k] = ""
        else:
            signal.signal(k, _on_signal)
            _traps[k] = action
            _trap_ns[k] = _ns
    return None

# ---- ulimit, umask, times ------------------------------------------------------------

_LIMITS = [
    ("c", "RLIMIT_CORE", "core file size", "kbytes", 1024),
    ("d", "RLIMIT_DATA", "data seg size", "kbytes", 1024),
    ("e", "RLIMIT_NICE", "scheduling priority", "", 1),
    ("f", "RLIMIT_FSIZE", "file size", "kbytes", 1024),
    ("i", "RLIMIT_SIGPENDING", "pending signals", "", 1),
    ("l", "RLIMIT_MEMLOCK", "max locked memory", "kbytes", 1024),
    ("m", "RLIMIT_RSS", "max memory size", "kbytes", 1024),
    ("n", "RLIMIT_NOFILE", "open files", "", 1),
    ("q", "RLIMIT_MSGQUEUE", "POSIX message queues", "bytes", 1),
    ("r", "RLIMIT_RTPRIO", "real-time priority", "", 1),
    ("s", "RLIMIT_STACK", "stack size", "kbytes", 1024),
    ("t", "RLIMIT_CPU", "cpu time", "seconds", 1),
    ("u", "RLIMIT_NPROC", "max user processes", "", 1),
    ("v", "RLIMIT_AS", "virtual memory", "kbytes", 1024),
    ("x", "RLIMIT_LOCKS", "file locks", "", 1),
]

def sh_ulimit(*args):
    import resource
    table = {o: (getattr(resource, r), d, u, k) for o, r, d, u, k in _LIMITS if hasattr(resource, r)}
    letters = "".join(table)
    hard = soft = every = False
    opts, value = [], None
    for a in args:
        if a.startswith("-") and len(a) > 1 and value is None:
            for c in a[1:]:
                if c == "H":
                    hard = True
                elif c == "S":
                    soft = True
                elif c == "a":
                    every = True
                elif c in table:
                    opts.append(c)
                else:
                    raise ValueError(f"ulimit: -{c}: invalid option\nulimit: usage: ulimit [-SHa{letters}] [limit]")
        elif value is None:
            value = a
        else:
            raise ValueError(f"ulimit: {a}: too many arguments\nulimit: usage: ulimit [-SHa{letters}] [limit]")
    def shown(c):
        r, d, u, k = table[c]
        s, h = resource.getrlimit(r)
        v = h if hard and not soft else s
        return "unlimited" if v == resource.RLIM_INFINITY else str(v // k)
    def row(c):
        r, d, u, k = table[c]
        unit = f"({u}, -{c})" if u else f"(-{c})"
        return f"{d:<26}{unit:>16} {shown(c)}"
    if every:
        for c in table:
            print(row(c))
        return None
    if not opts:
        opts = ["f"]
    if value is None:
        for c in opts:
            print(row(c) if len(opts) > 1 else shown(c))
        return None
    if len(opts) > 1:
        raise ValueError("ulimit: one limit at a time when setting one")
    r, d, u, k = table[opts[0]]
    s, h = resource.getrlimit(r)
    if value == "unlimited":
        n = resource.RLIM_INFINITY
    elif value == "hard":
        n = h
    elif value == "soft":
        n = s
    else:
        try:
            n = int(value) * k
        except ValueError:
            raise ValueError(f"ulimit: {value}: invalid number (a number, unlimited, soft or hard)") from None
    new_s = n if soft or not hard else s
    new_h = n if hard or not soft else h
    try:
        resource.setrlimit(r, (new_s, new_h))
    except (ValueError, OSError) as e:
        raise OSError(f"ulimit: {d}: can't change the limit: {e}") from None
    return None

def _umask_now():
    m = os.umask(0)
    os.umask(m)
    return m

def _umask_symbolic(m):
    allowed = ~m & 0o777
    parts = []
    for who, shift in (("u", 6), ("g", 3), ("o", 0)):
        bits = (allowed >> shift) & 7
        parts.append(who + "=" + "".join(ch for ch, b in (("r", 4), ("w", 2), ("x", 1)) if bits & b))
    return ",".join(parts)

def sh_umask(*args):
    import re
    flags, _, rest = _split_opts(args)
    for f in flags:
        if f not in "pS":
            raise ValueError(f"umask: -{f}: invalid option\numask: usage: umask [-p] [-S] [mode]")
    m = _umask_now()
    if not rest:
        text = _umask_symbolic(m) if "S" in flags else f"{m:04o}"
        print(("umask -S " if "S" in flags else "umask ") + text if "p" in flags else text)
        return None
    mode = rest[0]
    if re.fullmatch(r"[0-7]{1,4}", mode):
        os.umask(int(mode, 8) & 0o777)
        return None
    allowed = ~m & 0o777
    for clause in mode.split(","):
        mm = re.fullmatch(r"([ugoa]*)([=+-])([rwx]*)", clause)
        if not mm:
            raise ValueError(f"umask: {mode}: invalid mode (octal, as 022, or u=rwx,g=rx,o=)")
        who, op, perms = mm.groups()
        who = who or "a"
        bits = sum(b for ch, b in (("r", 4), ("w", 2), ("x", 1)) if ch in perms)
        mask = 0
        for w, shift in (("u", 6), ("g", 3), ("o", 0)):
            if w in who or "a" in who:
                mask |= 7 << shift
                spread = bits << shift
                if op == "=":
                    allowed = (allowed & ~(7 << shift)) | spread
                elif op == "+":
                    allowed |= spread
                else:
                    allowed &= ~spread
    os.umask(~allowed & 0o777)
    return None

def sh_times():
    t = os.times()
    f = lambda s: f"{int(s // 60)}m{s % 60:.3f}s"
    print(f"{f(t.user)} {f(t.system)}\n{f(t.children_user)} {f(t.children_system)}")
    return None

# ---- read, getopts, shift --------------------------------------------------------------

def _ifs_split(s, ifs, n=None):
    import re
    ws = "".join(c for c in ifs if c in " \t\n")
    other = "".join(c for c in ifs if c not in " \t\n")
    if ws:
        s = s.strip(ws)
    if not s:
        return []
    alts = []
    if other:
        alts.append((f"[{re.escape(ws)}]*" if ws else "") + f"[{re.escape(other)}]" + (f"[{re.escape(ws)}]*" if ws else ""))
    if ws:
        alts.append(f"[{re.escape(ws)}]+")
    if not alts or n == 1:
        # (One name: the whole line, as bash gives it. re.split's maxsplit=0 would mean no limit.)
        return [s]
    return re.split("|".join(alts), s, maxsplit=0 if n is None else n - 1)

_read_pos = {}

def sh_read(*args, _ns=None):
    import select, time
    flags, vals, names = _split_opts(args, "pnNtdaui")
    for f in flags:
        if f not in "rse":
            raise ValueError(f"read: -{f}: invalid option\nread: usage: read [-ers] [-a array] [-d delim] [-n nchars] [-N nchars] [-p prompt] [-t timeout] [-u fd] [name …]")
    raw, silent = "r" in flags, "s" in flags
    delim = vals.get("d", "\n")
    delim = delim[0] if delim else "\0"
    nchars = exact = None
    try:
        if "n" in vals:
            nchars = int(vals["n"])
        if "N" in vals:
            nchars = exact = int(vals["N"])
        timeout = float(vals["t"]) if "t" in vals else None
        fd = int(vals["u"]) if "u" in vals else 0
    except ValueError as e:
        raise ValueError(f"read: {e}") from None
    if exact is not None:
        delim = None
    redirected = fd == 0 and sys.stdin is not None and sys.stdin is not sys.__stdin__
    stream = sys.stdin if redirected or (fd == 0 and sys.__stdin__ is None) else None
    tty = stream is None and os.isatty(fd)
    # (-e at a terminal: the line read with readline, editable, -i its text to start with.)
    readline_line = None
    if ("e" in flags or "i" in vals) and tty and fd == 0 and nchars is None and not silent and delim == "\n":
        try:
            import readline
            start = vals.get("i", "")
            readline.set_startup_hook(lambda: readline.insert_text(start))
            try:
                readline_line = input(vals.get("p", ""))
            except EOFError:
                sys.stderr.write("\n")
                return _status(1)
            finally:
                readline.set_startup_hook(None)
        except ImportError:
            pass
    if readline_line is not None:
        stream = __import__("io").StringIO(readline_line + "\n")
        tty = False
        raw = True
    elif "p" in vals and (tty or (stream is not None and getattr(stream, "isatty", lambda: False)())):
        sys.stderr.write(vals["p"])
        sys.stderr.flush()
    # (read … <file, in a loop: the file read once, line after line, as
    # bash's `while read line; do …; done <file`; the same call site goes on
    # where it left off, until the end of the file.)
    pos_key = None
    if stream is not None and isinstance(getattr(stream, "name", None), str) and getattr(stream, "mode", "") == "r" and os.path.isfile(stream.name):
        fr = sys._getframe(1)
        while fr is not None and not _user_code(fr.f_code):
            fr = fr.f_back
        if fr is not None:
            st = os.stat(stream.name)
            pos_key = (id(fr), fr.f_code, fr.f_lasti, os.path.realpath(stream.name), st.st_ino)
            off = _read_pos.get(pos_key)
            if off:
                stream.seek(off)
    old = None
    if tty and (silent or nchars is not None):
        import termios
        old = termios.tcgetattr(fd)
        new = termios.tcgetattr(fd)
        if silent:
            new[3] &= ~termios.ECHO
        if nchars is not None:
            new[3] &= ~termios.ICANON
            new[6][termios.VMIN] = 1
            new[6][termios.VTIME] = 0
        termios.tcsetattr(fd, termios.TCSANOW, new)
    import codecs
    dec = codecs.getincrementaldecoder("utf-8")("replace")
    deadline = None if timeout is None else time.monotonic() + timeout
    def getc():
        if stream is not None:
            return stream.read(1)
        while True:
            if deadline is not None:
                left = deadline - time.monotonic()
                if left <= 0 or not select.select([fd], [], [], left)[0]:
                    raise TimeoutError
            b = os.read(fd, 1)
            if not b:
                return dec.decode(b"", True)
            ch = dec.decode(b)
            if ch:
                return ch
    out, eof, timed = [], False, False
    try:
        if stream is not None and delim == "\n" and nchars is None and raw:
            line = stream.readline()
            eof = not line.endswith("\n")
            out = list(line[:-1] if not eof else line)
        else:
            while True:
                if nchars is not None and len(out) >= nchars:
                    break
                ch = getc()
                if ch == "":
                    eof = True
                    break
                if ch == delim:
                    break
                if ch == "\\" and not raw:
                    nxt = getc()
                    if nxt == "":
                        eof = True
                        break
                    if nxt == "\n":
                        continue
                    out.append(nxt)
                    continue
                out.append(ch)
    except TimeoutError:
        timed = True
    except KeyboardInterrupt:
        raise
    finally:
        if old is not None:
            import termios
            termios.tcsetattr(fd, termios.TCSANOW, old)
            if silent and tty:
                sys.stderr.write("\n")
                sys.stderr.flush()
    if pos_key is not None:
        if eof:
            _read_pos.pop(pos_key, None)
        else:
            _read_pos[pos_key] = stream.tell()
    text = "".join(out)
    ns = _ns if _ns is not None else {}
    ifs = ns.get("IFS")
    if not isinstance(ifs, str):
        ifs = os.environ.get("IFS", " \t\n")
    if "a" in vals:
        ns[vals["a"]] = _ifs_split(text, ifs)
    elif not names:
        ns["REPLY"] = text
    else:
        parts = _ifs_split(text, ifs, len(names))
        for k, name in enumerate(names):
            v = parts[k] if k < len(parts) else ""
            if k == len(names) - 1 and ifs:
                ws = "".join(c for c in ifs if c in " \t\n")
                v = v.rstrip(ws) if ws else v
            ns[name] = v
    if timed:
        return _status(142)
    return _status(1 if eof else 0)

def sh_getopts(optstring=None, name=None, *args, _ns=None):
    if optstring is None or name is None:
        raise ValueError("getopts: usage: getopts optstring name [arg …]")
    ns = _ns
    argv = list(args) if args else [str(a) for a in list(ns.get("argv") or [])[1:]]
    try:
        optind = int(ns.get("OPTIND", 1))
    except (TypeError, ValueError):
        optind = 1
    state = ns.get("__tessh_getopts__")
    pos = state[1] if isinstance(state, tuple) and state[0] == optind else 0
    silent = optstring.startswith(":")
    prog = os.path.basename(str((ns.get("argv") or ["tessh"])[0])) if ns.get("argv") else "tessh"
    def done():
        ns[name] = "?"
        ns["OPTARG"] = None
        ns["OPTIND"] = optind
        ns["__tessh_getopts__"] = (optind, 0)
        return _status(1)
    if optind - 1 >= len(argv):
        return done()
    a = argv[optind - 1]
    if pos == 0:
        if a == "--":
            optind += 1
            return done()
        if not a.startswith("-") or a == "-":
            return done()
        pos = 1
    c = a[pos]
    pos += 1
    i = optstring.find(c)
    ns["OPTARG"] = None
    if c == ":" or i < 0:
        if silent:
            ns["OPTARG"] = c
        else:
            _err(f"{prog}: illegal option -- {c}")
        ns[name] = "?"
    elif optstring[i + 1:i + 2] == ":":
        if pos < len(a):
            ns["OPTARG"] = a[pos:]
            optind += 1
            pos = 0
        elif optind < len(argv):
            ns["OPTARG"] = argv[optind]
            optind += 2
            pos = 0
        else:
            optind += 1
            pos = 0
            if silent:
                ns[name] = ":"
                ns["OPTARG"] = c
            else:
                _err(f"{prog}: option requires an argument -- {c}")
                ns[name] = "?"
            ns["OPTIND"] = optind
            ns["__tessh_getopts__"] = (optind, pos)
            return _status(0)
        ns[name] = c
    else:
        ns[name] = c
    if pos and pos >= len(a):
        optind += 1
        pos = 0
    ns["OPTIND"] = optind
    ns["__tessh_getopts__"] = (optind, pos)
    return _status(0)

def sh_shift(n="1", *rest, _ns=None):
    if rest:
        raise ValueError("shift: too many arguments\nshift: usage: shift [n]")
    try:
        n = int(n)
    except ValueError:
        raise ValueError(f"shift: {n}: numeric argument required") from None
    if n < 0:
        raise ValueError(f"shift: {n}: shift count out of range")
    a = _ns.get("argv")
    if not isinstance(a, list):
        a = list(a or [])
        _ns["argv"] = a
    if n > max(len(a) - 1, 0):
        return _status(1)
    del a[1:1 + n]
    return None

def sh_readonly(*args, _ns=None):
    flags, _, names = _split_opts(args)
    ro = _ns.setdefault("__tessh_readonly__", set())
    if not names:
        for n in sorted(ro):
            print(f"readonly {n}={_ns.get(n)!r}")
        return None
    for a in names:
        n, eq, v = a.partition("=")
        if not n.isidentifier():
            raise ValueError(f"readonly: {n}: not a valid name")
        if n in ro and eq:
            raise ValueError(f"readonly: {n}: read-only already")
        if eq:
            _ns[n] = v
        elif n not in _ns:
            _ns[n] = None
        ro.add(n)
    return None

def sh_select(*items, _ns=None):
    """The menu loop, as a generator: for x in (select a b c)."""
    items = list(items)
    # (Its input and output as they are now: select <<< … and 2>… hold for
    # the whole loop, which runs after this call.)
    inp, err = sys.stdin, sys.stderr
    def menu():
        w = len(str(len(items)))
        for k, it in enumerate(items, 1):
            err.write(f"{k:>{w}}) {it}\n")
    def gen():
        menu()
        while True:
            ps3 = (_ns or {}).get("PS3") or os.environ.get("PS3") or "#? "
            err.write(str(ps3))
            err.flush()
            line = inp.readline()
            if not line:
                err.write("\n")
                return
            reply = line.rstrip("\n")
            if _ns is not None:
                _ns["REPLY"] = reply
            if not reply.strip():
                menu()
                continue
            try:
                k = int(reply.strip())
            except ValueError:
                k = 0
            yield items[k - 1] if 1 <= k <= len(items) else None
    return gen()

_SET_LONG = {"errexit": "e", "xtrace": "x", "verbose": "v", "nounset": "u", "pipefail": "P", "noclobber": "C", "noglob": "f", "noexec": "n", "ignoreeof": "I"}

def sh_set(*args, _ns=None):
    """bash's set: -e/+e (strict), -x (xtrace), -v (verbose), -u (nounset:
    always so in tessh), -o NAME / +o NAME, -o alone lists, -- args sets $argv."""
    T = _tessh()
    args = list(args)
    def flip(letter, on):
        names = {"e": "errexit", "x": "xtrace", "v": "verbose", "P": "pipefail", "C": "noclobber", "f": "noglob", "n": "noexec", "I": "ignoreeof"}
        if letter in names:
            T.set_option(names[letter], on)
        elif letter == "u":
            if not on:
                raise NotImplementedError("set +u: in tessh an unset name is always an error ($env.get NAME, or (globals).get 'name', gives a default)")
        else:
            raise ValueError(f"set: -{letter}: invalid option (tessh's are -e, -x, -v, -u, -C, -f, -n, and -o errexit, xtrace, verbose, nounset, pipefail, noclobber, noglob, noexec, ignoreeof)")
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--":
            argv = _ns.get("argv")
            _ns["argv"] = (list(argv[:1]) if argv else ["tessh"]) + list(args[i + 1:])
            return None
        if a in ("-o", "+o"):
            if i + 1 >= len(args):
                for name in _SET_LONG:
                    on = True if name == "nounset" else T.get_option(name)
                    print(f"{name:<15}{'on' if on else 'off'}")
                return None
            name = args[i + 1]
            if name not in _SET_LONG:
                raise ValueError(f"set: {name}: invalid option name (tessh's are {', '.join(_SET_LONG)})")
            flip(_SET_LONG[name], a == "-o")
            i += 2
            continue
        if len(a) > 1 and a[0] in "-+":
            for c in a[1:]:
                if c not in "exvuCfn":
                    raise ValueError(f"set: -{c}: invalid option (tessh's are -e, -x, -v, -u, -C, -f and -n, and -o NAME)")
                flip(c, a[0] == "-")
            i += 1
            continue
        argv = _ns.get("argv")
        _ns["argv"] = (list(argv[:1]) if argv else ["tessh"]) + list(args[i:])
        return None
    return None

def _read_chunks(fd, delim):
    """mapfile's input: what <, <<< or a pipe gives (sys.stdin), or descriptor fd, split after delim."""
    redirected = fd == 0 and sys.stdin is not None and sys.stdin is not sys.__stdin__
    if redirected or (fd == 0 and sys.__stdin__ is None):
        data = sys.stdin.read()
    else:
        import codecs
        dec = codecs.getincrementaldecoder("utf-8")("replace")
        parts = []
        while True:
            b = os.read(fd, 65536)
            if not b:
                break
            parts.append(dec.decode(b))
        parts.append(dec.decode(b"", True))
        data = "".join(parts)
    out, start = [], 0
    while start < len(data):
        k = data.find(delim, start)
        if k < 0:
            out.append(data[start:])
            break
        out.append(data[start:k + 1])
        start = k + 1
    return out

def sh_mapfile(*args, _ns=None):
    """mapfile / readarray: lines of the input into a list."""
    flags, vals, names = _split_opts(args, "dnOsuCc")
    for f in flags:
        if f != "t":
            raise ValueError(f"mapfile: -{f}: invalid option\nmapfile: usage: mapfile [-d delim] [-n count] [-O origin] [-s count] [-t] [-u fd] [-C callback] [-c quantum] [array]")
    try:
        count = int(vals.get("n", 0))
        origin = int(vals["O"]) if "O" in vals else None
        skip = int(vals.get("s", 0))
        fd = int(vals.get("u", 0))
        quantum = int(vals.get("c", 5000))
    except ValueError as e:
        raise ValueError(f"mapfile: {e}") from None
    delim = vals.get("d", "\n")
    delim = delim[0] if delim else "\0"
    name = names[0] if names else "MAPFILE"
    chunks = _read_chunks(fd, delim)[skip:]
    if count:
        chunks = chunks[:count]
    if "t" in flags:
        chunks = [c[:-1] if c.endswith(delim) else c for c in chunks]
    if origin is None:
        arr = []
    else:
        arr = list(_ns.get(name) or [])
        del arr[origin:]
        arr += [""] * (origin - len(arr))
    cb = None
    if "C" in vals:
        cb = _ns.get(vals["C"])
        if not callable(cb):
            raise ValueError(f"mapfile: {vals['C']}: not a function")
    for k, c in enumerate(chunks):
        if cb is not None and quantum > 0 and (k + 1) % quantum == 0:
            cb(len(arr), c)
        arr.append(c)
    _ns[name] = arr
    return None

sh_readarray = sh_mapfile

def _quote_word(w):
    """A word back into tessh text: as it was when it needs no quotes."""
    import re
    if re.fullmatch(r"[A-Za-z0-9_./=:,+@%~|&<>*?!-]+", w):
        return w
    return "'" + w.replace("\\", "\\\\").replace("'", "\\'") + "'"

def sh_time(*args, _ns=None):
    """time [-p] command …: run it (a program, a pipe, a call), then real,
    user and sys time to stderr, as bash's keyword."""
    import time as _time
    args = list(args)
    posix = False
    while args and args[0] in ("-p", "--"):
        posix = posix or args[0] == "-p"
        args.pop(0)
    t0, c0 = _time.monotonic(), os.times()
    v = None
    try:
        if args:
            v = _tessh().eval_tessh(" ".join(_quote_word(w) for w in args), _ns)
    finally:
        t1, c1 = _time.monotonic(), os.times()
        _time_report(t1 - t0, c0, c1, posix)
    return v

def sh_coproc(*args, _ns=None):
    """coproc [NAME --] command …: the command in the background, its input
    and output piped to tessh: $COPROC (or $NAME) is [read fd, write fd],
    $COPROC_PID (or $NAME_PID) its process ID; its value is the Job."""
    args = list(args)
    name = "COPROC"
    if "--" in args:
        k = args.index("--")
        if k > 1:
            raise ValueError("coproc: usage: coproc [NAME --] command [arg …]")
        if k == 1:
            name = args[0]
            if not name.isidentifier():
                raise ValueError(f"coproc: {name}: not a valid name")
        args = args[k + 1:]
    if not args:
        raise ValueError("coproc: usage: coproc [NAME --] command [arg …]")
    job, rfd, wfd, pid = _tessh().start_coproc(args)
    _ns[name] = [rfd, wfd]
    _ns[name + "_PID"] = pid
    return job

class timed:
    """time with a block: the block's real, user and sys time, after it, to stderr."""
    def __init__(self, posix=False):
        self.posix = posix
    def __enter__(self):
        import time as _time
        self.t0, self.c0 = _time.monotonic(), os.times()
        return self
    def __exit__(self, *exc):
        import time as _time
        t1, c1 = _time.monotonic(), os.times()
        _time_report(t1 - self.t0, self.c0, c1, self.posix)
        return False

def _time_report(real, c0, c1, posix):
    user = (c1.user - c0.user) + (c1.children_user - c0.children_user)
    system = (c1.system - c0.system) + (c1.children_system - c0.children_system)
    sys.stdout.flush()
    if posix:
        sys.stderr.write(f"real {real:.2f}\nuser {user:.2f}\nsys {system:.2f}\n")
    else:
        f = lambda x: f"{int(x // 60)}m{x % 60:.3f}s"
        sys.stderr.write(f"\nreal\t{f(real)}\nuser\t{f(user)}\nsys\t{f(system)}\n")
    sys.stderr.flush()

def coproc_block(name, body, ns):
    """coproc NAME with a block: the block runs in a fork of tessh, its
    input and output piped back, as bash's `coproc NAME { …; }`."""
    import warnings
    r_in, w_in = os.pipe()     # tessh writes, the block reads (its stdin)
    r_out, w_out = os.pipe()   # the block writes (its stdout), tessh reads
    sys.stdout.flush()
    sys.stderr.flush()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        pid = os.fork()
    if pid == 0:
        code = 0
        try:
            os.setpgid(0, 0)
            # (Out of the way first: a pipe end can be 0 or 1 itself, when
            # tessh's own was closed.)
            hi_in, hi_out = os.dup(r_in), os.dup(w_out)
            for fd in {r_in, w_in, r_out, w_out}:
                os.close(fd)
            os.dup2(hi_in, 0)
            os.dup2(hi_out, 1)
            os.close(hi_in)
            os.close(hi_out)
            sys.stdin = open(0, "r", closefd=False)
            sys.stdout = open(1, "w", closefd=False, buffering=1)
            body()
        except SystemExit as e:
            code = e.code if isinstance(e.code, int) else (0 if e.code is None else 1)
        except BaseException:
            import traceback
            traceback.print_exc()
            code = 1
        finally:
            try:
                sys.stdout.flush()
            except Exception:
                pass
            os._exit(code)
    os.close(r_in)
    os.close(w_out)
    for fd in (w_in, r_out):
        os.set_inheritable(fd, False)
    job = _tessh().adopt_job(pid, f"coproc {name}")
    ns[name] = [r_out, w_in]
    ns[name + "_PID"] = pid
    return job

_compopt_current = None

def sh_compopt(*args, _ns=None):
    """compopt [-o opt] [+o opt] [name …]: change completion options, of
    the named commands, or of the completion running now."""
    args = list(args)
    on, off, names = set(), set(), []
    k = 0
    while k < len(args):
        a = args[k]
        if a in ("-o", "+o") and k + 1 < len(args):
            if args[k + 1] not in _OPTIONS:
                raise ValueError(f"compopt: {args[k + 1]}: invalid option name")
            (on if a == "-o" else off).add(args[k + 1])
            k += 2
        elif a in ("-D", "-E", "-I"):
            k += 1
        else:
            names.append(a)
            k += 1
    specs = [_complete[n] for n in names if n in _complete] if names else ([_compopt_current] if _compopt_current is not None else [])
    if not specs:
        raise ValueError("compopt: not completing anything now; name a command complete knows")
    if not on and not off:
        for sp in specs:
            print("compopt " + " ".join(("-o " if o in sp.get("opts", ()) else "+o ") + o for o in sorted(_OPTIONS)))
        return None
    for sp in specs:
        sp.setdefault("opts", set())
        sp["opts"] |= on
        sp["opts"] -= off
    return None

class Alias:
    """An alias: its text, with the words it's given after it, run as tessh
    in the namespace it's used in (bash's alias, expanded when it runs)."""
    _tessh_command = True
    _tessh_wants_ns = True
    def __init__(self, name, text):
        self.name = name
        self.text = text
    @property
    def _tessh_usage(self):
        return f"{self.name}: an alias for {self.text}"
    def __call__(self, *words, _ns=None):
        # (An alias that starts with its own name runs the program or
        # builtin of that name, not itself again: alias ls='ls --color'.)
        ns = _ns if _ns is not None else {}
        return _tessh().eval_tessh(self._line(words), ns)
    def _tessh_pipe(self, *rest, _ns=None, last=False):
        """In a pipe: what comes in is the alias's input (… | alias args),
        and its output goes on (or is shown, last)."""
        words, data = rest[:-1], rest[-1]
        if data is None:
            return self(*words, _ns=_ns) if last else _tessh().eval_tessh("(" + self._line(words) + ")", _ns)
        key = "__tessh_pipe_in__"
        # (A captured value lost its last newline: back, for the program's lines.)
        if isinstance(data, str) and data and not data.endswith("\n"):
            data = str(data) + "\n"
        _ns[key] = data
        try:
            expr = f"${key} | " + self._line(words)
            return _tessh().eval_tessh(expr if last else "(" + expr + ")", _ns)
        finally:
            _ns.pop(key, None)
    def _line(self, words):
        text = self.text
        first = text.split(None, 1)[0] if text.strip() else ""
        if first == self.name:
            text = "\\" + text.lstrip()
        return text + "".join(" " + _quote_word(w) for w in words)
    def __repr__(self):
        import shlex
        return f"alias {self.name}={shlex.quote(self.text)}"

def _aliases(ns):
    return {k: v for k, v in (ns or {}).items() if isinstance(v, Alias)}

def sh_alias(*args, _ns=None):
    """alias [-p] [name[=value] …]"""
    args = [a for a in args if a != "-p"]
    if not args:
        for k, v in sorted(_aliases(_ns).items()):
            print(repr(v))
        return None
    code = 0
    for a in args:
        name, eq, text = a.partition("=")
        if not eq:
            v = (_ns or {}).get(name)
            if isinstance(v, Alias):
                print(repr(v))
            else:
                _err(f"alias: {name}: not found")
                code = 1
            continue
        if not name or any(c in name for c in " \t\n/$`=\\'\"|&;()<>"):
            raise ValueError(f"alias: {name}: invalid alias name")
        _ns[name] = Alias(name, text)
    return _status(code) if code else None

def sh_unalias(*args, _ns=None):
    """unalias [-a] name …"""
    if "-a" in args:
        for k in list(_aliases(_ns)):
            del _ns[k]
        return None
    if not args:
        raise ValueError("unalias: usage: unalias [-a] name [name …]")
    code = 0
    for n in args:
        if isinstance((_ns or {}).get(n), Alias):
            del _ns[n]
        else:
            _err(f"unalias: {n}: not found")
            code = 1
    return _status(code) if code else None

# ---- complete, compgen ---------------------------------------------------------------

_complete = {}

_ACTIONS = {"file": "f", "directory": "d", "command": "c", "builtin": "b", "keyword": "k", "variable": "v", "function": "F*", "export": "e", "user": "u", "hostname": "h", "job": "j", "signal": "s", "alias": "a"}
_OPTIONS = {"default", "dirnames", "filenames", "nospace", "plusdirs", "bashdefault", "noquote", "nosort", "fullquote"}

def _parse_spec(args):
    args = list(args)
    opts = set()
    k = 0
    while k < len(args):
        if args[k] in ("-o", "+o") and k + 1 < len(args):
            if args[k + 1] not in _OPTIONS:
                raise ValueError(f"-o {args[k + 1]}: invalid option name")
            if args[k] == "-o":
                opts.add(args[k + 1])
            del args[k:k + 2]
        else:
            k += 1
    flags, vals, rest = _split_opts(args, "WFAGXPSC")
    spec = {"flags": set(f for f in flags if f in "fdcbkveuhjsa"), "words": vals.get("W"), "func": vals.get("F"),
            "command": vals.get("C"), "glob": vals.get("G"), "filter": vals.get("X"),
            "prefix": vals.get("P", ""), "suffix": vals.get("S", ""), "opts": opts, "functions": False}
    a = vals.get("A")
    if a:
        k = _ACTIONS.get(a)
        if k is None:
            raise ValueError(f"-A {a}: unknown action")
        if k == "F*":
            spec["functions"] = True
        elif k:
            spec["flags"].add(k)
    return flags, spec, rest

def _files(word, dirs_only):
    import glob as _glob
    out = []
    pat = os.path.expanduser(word) + "*"
    for p in sorted(_glob.glob(pat)):
        isdir = os.path.isdir(p)
        if not dirs_only or isdir:
            shown = word + p[len(os.path.expanduser(word)):] if word.startswith("~") else p
            out.append(shown + ("/" if isdir else ""))
    return out

def _call_completer(f, word, line_words, ns, line, point):
    """A -F function: tessh's (two parameters: the word and the line's words,
    giving a list back) or bash's (cmd word prev, filling $COMPREPLY from
    $COMP_WORDS, $COMP_CWORD, $COMP_LINE and $COMP_POINT)."""
    import inspect
    try:
        params = [p for p in inspect.signature(f).parameters.values() if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)]
        two = len(params) == 2 and not any(p.kind == p.VAR_POSITIONAL for p in inspect.signature(f).parameters.values())
    except (TypeError, ValueError):
        two = False
    if two:
        return [str(x) for x in (f(word, list(line_words)) or [])]
    words = list(line_words) + [word]
    cmd = words[0] if words else ""
    prev = words[-2] if len(words) > 1 else ""
    if ns is not None:
        ns["COMP_WORDS"] = words
        ns["COMP_CWORD"] = len(words) - 1
        ns["COMP_LINE"] = line
        ns["COMP_POINT"] = point
        ns["COMPREPLY"] = []
    try:
        got = f(cmd, word, prev)
    except TypeError:
        got = f()
    if isinstance(got, (list, tuple)):
        return [str(x) for x in got]
    reply = (ns or {}).get("COMPREPLY") or []
    return [str(x) for x in (reply if isinstance(reply, (list, tuple)) else [reply])]

def _gen(spec, word, line_words, ns, line=None, point=None):
    import fnmatch, keyword, builtins, signal, pwd
    global _compopt_current
    _compopt_current = spec
    out = []
    if line is None:
        line = " ".join(list(line_words) + [word])
    if point is None:
        point = len(line)
    if spec.get("words"):
        out += [w for w in str(spec["words"]).split() if w.startswith(word)]
    fl = spec.get("flags", ())
    if "f" in fl or "d" in fl:
        out += _files(word, "f" not in fl)
    if spec.get("glob"):
        import glob as _glob
        out += [g for g in sorted(_glob.glob(os.path.expanduser(spec["glob"]))) if g.startswith(word)]
    if "c" in fl or "b" in fl:
        names = set(_tessh().builtin_names()) | set(_DUALS)
        if "c" in fl:
            for d in os.environ.get("PATH", "").split(os.pathsep):
                try:
                    names |= {n for n in os.listdir(d or ".") if os.access(os.path.join(d or ".", n), os.X_OK)}
                except OSError:
                    pass
            names |= {n for n, v in (ns or {}).items() if callable(v) and not n.startswith("_")}
        out += sorted(n for n in names if n.startswith(word))
    if "k" in fl:
        out += sorted(k for k in _keywords() if k.startswith(word))
    if "v" in fl:
        out += sorted(n for n in (ns or {}) if n.startswith(word) and not n.startswith("_") and not callable(ns[n]))
    if "e" in fl:
        out += sorted(n for n in os.environ if n.startswith(word))
    if "a" in fl:
        out += sorted(n for n in _aliases(ns) if n.startswith(word))
    if "u" in fl:
        out += sorted({p.pw_name for p in pwd.getpwall() if p.pw_name.startswith(word)})
    if "h" in fl:
        hosts = set()
        try:
            for l in open("/etc/hosts"):
                hosts |= set(l.split("#")[0].split()[1:])
        except OSError:
            pass
        out += sorted(h for h in hosts if h.startswith(word))
    if "s" in fl:
        out += sorted("SIG" + n for n, _ in _tessh().signal_names() if ("SIG" + n).startswith(word))
    if "j" in fl:
        out += [w for w in _tessh().job_words() if w.startswith(word)]
    if spec.get("functions"):
        out += sorted(n for n, v in (ns or {}).items() if n.startswith(word) and callable(v) and getattr(v, "__module__", "") == (ns or {}).get("__name__"))
    if spec.get("func"):
        f = (ns or {}).get(spec["func"])
        if not callable(f):
            raise ValueError(f"{spec['func']}: not a function")
        out += _call_completer(f, word, line_words, ns, line, point)
    if spec.get("command"):
        words = list(line_words) + [word]
        prev = words[-2] if len(words) > 1 else ""
        env = dict(os.environ, COMP_LINE=line, COMP_POINT=str(point), COMP_TYPE="9", COMP_KEY="9")
        import subprocess
        r = subprocess.run(["sh", "-c", spec["command"] + ' "$@"', "complete", words[0] if words else "", word, prev], capture_output=True, text=True, env=env)
        out += [l for l in r.stdout.splitlines() if l]
    if spec.get("filter"):
        pat = spec["filter"]
        keep = pat.startswith("!")
        pat = pat[1:] if keep else pat
        pat = pat.replace("&", word)
        out = [x for x in out if fnmatch.fnmatchcase(x, pat) == keep]
    out = [spec.get("prefix", "") + x + spec.get("suffix", "") for x in out]
    opts = spec.get("opts", ())
    if "plusdirs" in opts:
        out += _files(word, True)
    if not out and "dirnames" in opts:
        out = _files(word, True)
    seen, res = set(), []
    for x in out:
        if x not in seen:
            seen.add(x)
            res.append(x)
    if "nosort" not in opts and spec.get("words") is None and not spec.get("func"):
        res.sort()
    return res

def sh_complete(*args, _ns=None):
    import shlex
    flags, spec, names = _parse_spec(args)
    if "r" in flags:
        for n in names or list(_complete):
            _complete.pop(n, None)
        return None
    if not names or "p" in flags:
        for n in (names or sorted(_complete)):
            sp = _complete.get(n)
            if sp is None:
                continue
            parts = ["-" + "".join(sorted(sp["flags"]))] if sp["flags"] else []
            for o in sorted(sp.get("opts", ())):
                parts += ["-o", o]
            for key, opt in (("words", "-W"), ("func", "-F"), ("command", "-C"), ("glob", "-G"), ("filter", "-X"), ("prefix", "-P"), ("suffix", "-S")):
                if sp.get(key):
                    parts += [opt, shlex.quote(sp[key]) if key != "func" else sp[key]]
            if sp["functions"]:
                parts += ["-A", "function"]
            print("complete " + " ".join(parts + [n]))
        return None
    for n in names:
        _complete[n] = spec
    return None

def sh_compgen(*args, _ns=None):
    flags, spec, rest = _parse_spec(args)
    word = rest[0] if rest else ""
    got = _gen(spec, word, [], _ns)
    for x in got:
        print(x)
    return None if got else _status(1)

def complete_for(before, word, ns):
    """Tab after a command complete was told about: its list (None: tessh's own)."""
    import re, shlex
    if not _complete:
        return None
    seg = re.split(r"\|\||&&|[|;&(]", before)[-1]
    try:
        ws = shlex.split(seg)
    except ValueError:
        ws = seg.split()
    if not ws:
        return None
    spec = _complete.get(os.path.basename(ws[0]))
    if spec is None:
        return None
    line = seg.lstrip() + word
    try:
        got = _gen(spec, word, ws, ns, line, len(line))
    except Exception:
        return None
    # (-o default, -o bashdefault: tessh's own when there's nothing.)
    if not got and spec.get("opts", set()) & {"default", "bashdefault"}:
        return None
    return got

# ---- bash's completions, borrowed -------------------------------------------------------

_BASH_QUERY = r"""
__tessh_q() {
  COMP_WORDS=("${__w[@]}"); COMP_CWORD=$__c; COMP_LINE=$__l; COMP_POINT=$__p; COMP_TYPE=9; COMP_KEY=9
  local cmd=${COMP_WORDS[0]} cur=${COMP_WORDS[COMP_CWORD]} prev=${COMP_WORDS[COMP_CWORD-1]} spec
  cmd=${cmd##*/}
  spec=$(complete -p -- "$cmd" 2>/dev/null)
  if [[ -z $spec ]]; then
    if declare -F _comp_load >/dev/null; then _comp_load -- "$cmd"
    elif declare -F __load_completion >/dev/null; then __load_completion "$cmd"
    elif declare -F _completion_loader >/dev/null; then _completion_loader "$cmd"
    fi
    spec=$(complete -p -- "$cmd" 2>/dev/null)
  fi
  [[ -z $spec ]] && return
  COMPREPLY=()
  if [[ $spec =~ \ -F\ ([^ ]+) ]]; then
    "${BASH_REMATCH[1]}" "$cmd" "$cur" "$prev"
  else
    local opts=${spec#complete }
    opts=${opts% *}
    mapfile -t COMPREPLY < <(eval "compgen $opts -- \"\$cur\"")
  fi
  printf '%s\n' "${COMPREPLY[@]}"
}
"""

_bash_state = {"proc": None, "script": None, "failed": False}

def _bash_completion_script():
    """bash-completion's main script: $TESSH_BASH_COMPLETION, the one
    /etc/bashrc loads (NixOS), or where distributions and Homebrew put it."""
    import re, glob
    env = os.environ.get("TESSH_BASH_COMPLETION")
    if env:
        return env if os.path.isfile(env) else None
    for rc in ("/etc/bashrc", "/etc/bash.bashrc", "/etc/bash/bashrc"):
        try:
            text = open(rc).read()
        except OSError:
            continue
        for m in re.finditer(r"[\"']?(/[^\s\"';]*bash[-_]completion[^\s\"';]*)", text):
            p = m.group(1)
            if p.endswith("profile.d/bash_completion.sh"):
                main = os.path.normpath(os.path.join(os.path.dirname(p), "..", "..", "share", "bash-completion", "bash_completion"))
                if os.path.isfile(main):
                    return main
            if os.path.isfile(p) and not p.endswith(".d"):
                return p
    home = os.path.expanduser("~")
    for p in ("/usr/share/bash-completion/bash_completion", "/etc/bash_completion",
              "/usr/local/share/bash-completion/bash_completion", home + "/.nix-profile/share/bash-completion/bash_completion",
              "/opt/homebrew/share/bash-completion/bash_completion", "/usr/local/etc/profile.d/bash_completion.sh",
              "/opt/homebrew/etc/profile.d/bash_completion.sh"):
        if os.path.isfile(p):
            return p
    return None

def _full_bash():
    """A bash with programmable completion (nixpkgs' non-interactive bash,
    a build's, has no `complete`): the system's first, then $PATH's."""
    import subprocess, shutil
    seen = []
    for b in ("/run/current-system/sw/bin/bash", "/bin/bash", "/usr/bin/bash", "/usr/local/bin/bash", "/opt/homebrew/bin/bash", shutil.which("bash")):
        if not b or b in seen or not os.access(b, os.X_OK):
            continue
        seen.append(b)
        try:
            r = subprocess.run([b, "--norc", "--noprofile", "-c", "type -t complete"], capture_output=True, text=True, timeout=3)
        except (OSError, subprocess.SubprocessError):
            continue
        if r.stdout.strip() == "builtin":
            return b
    return None

def _bash_proc():
    import subprocess, shutil
    st = _bash_state
    p = st["proc"]
    if p is not None and p.poll() is None:
        return p
    if st["failed"]:
        return None
    script = st["script"] or _bash_completion_script()
    bash = st.get("bash") or _full_bash()
    st["bash"] = bash
    if not script or not bash:
        st["failed"] = True
        return None
    st["script"] = script
    # (Where completions live: the system's, the user's profiles', as a login bash would see.)
    user = os.environ.get("USER", "")
    dirs = [d for d in os.environ.get("XDG_DATA_DIRS", "").split(":") if d]
    for d in ("/run/current-system/sw/share", f"/etc/profiles/per-user/{user}/share", os.path.expanduser("~/.nix-profile/share"),
              os.path.expanduser("~/.local/share"), "/usr/local/share", "/usr/share"):
        if d not in dirs:
            dirs.append(d)
    env = dict(os.environ, XDG_DATA_DIRS=":".join(dirs))
    p = subprocess.Popen([bash, "--norc", "--noprofile"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, env=env, text=True, bufsize=1, start_new_session=True)
    p.stdin.write(f"source {_shq(script)} >/dev/null 2>&1\n{_BASH_QUERY}\n")
    p.stdin.flush()
    st["proc"] = p
    return p

def _shq(s):
    import shlex
    return shlex.quote(s)

def bash_complete(before, word, ns, timeout=1.0):
    """What bash's own completion (git's, systemctl's, any command's that
    bash-completion knows) offers for the word: a list, or None for tessh's
    own. Off with TESSH_BASH_COMPLETION=0."""
    import re, select, time
    if os.environ.get("TESSH_BASH_COMPLETION") == "0":
        return None
    seg = re.split(r"\|\||&&|[|;&(]", before)[-1]
    ws = hist_split(seg)
    if not ws:
        return None
    cmd = ws[0]
    # (tessh's own names, and a NAME=value before it: not bash's to say.)
    if "=" in cmd or (ns is not None and cmd in ns) or cmd in _tessh().builtin_names() or cmd in _DUALS:
        return None
    p = _bash_proc()
    if p is None:
        return None
    words = ws + [word]
    line = seg.lstrip() + word
    marker = "\x1e__tessh_done__"
    q = "__w=(" + " ".join(_shq(w) for w in words) + f"); __c={len(words) - 1}; __l={_shq(line)}; __p={len(line)}; __tessh_q 2>/dev/null; printf '%s\\n' {_shq(marker)}\n"
    try:
        p.stdin.write(q)
        p.stdin.flush()
    except (BrokenPipeError, OSError):
        _bash_state["proc"] = None
        return None
    out, end = [], time.monotonic() + timeout
    fd = p.stdout.fileno()
    buf = ""
    while True:
        left = end - time.monotonic()
        if left <= 0 or not select.select([fd], [], [], left)[0]:
            # (Too slow, or stuck: given up on, a fresh bash next time.)
            try:
                p.kill()
            except OSError:
                pass
            _bash_state["proc"] = None
            return None
        chunk = os.read(fd, 65536).decode("utf-8", "replace")
        if not chunk:
            _bash_state["proc"] = None
            return None
        buf += chunk
        if marker + "\n" in buf:
            buf = buf[:buf.index(marker + "\n")]
            break
    seen, res = set(), []
    for l in buf.split("\n"):
        l = l.rstrip()
        if l and l not in seen:
            seen.add(l)
            res.append(l)
    return res or None

def hist_split(seg):
    """A command line's words, quotes kept out, as a shell splits them."""
    import shlex
    try:
        return shlex.split(seg)
    except ValueError:
        return seg.split()

# ---- history, fc -----------------------------------------------------------------------

def _history_file():
    d = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"), ".local", "share")
    return os.path.join(d, "tessh", "history")

def _history():
    try:
        with open(_history_file(), encoding="utf-8", errors="replace") as f:
            return [l.rstrip("\n").replace("<\\n>", "\n") for l in f if l.strip()]
    except FileNotFoundError:
        return []

def _write_history(entries):
    p = _history_file()
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        for e in entries:
            f.write(e.replace("\n", "<\\n>") + "\n")

def _show_history(entries, first=1, numbers=True):
    for k, e in enumerate(entries, first):
        lines = e.split("\n")
        print((f"{k:5}  " if numbers else "\t") + lines[0])
        for l in lines[1:]:
            print(("       " if numbers else "\t") + l)

def sh_history(*args):
    flags, vals, rest = _split_opts(args, "d")
    h = _history()
    if "c" in flags:
        _write_history([])
        return None
    if "d" in vals:
        try:
            k = int(vals["d"])
            k = k - 1 if k > 0 else len(h) + k
            del h[k]
        except (ValueError, IndexError):
            raise ValueError(f"history: {vals['d']}: history position out of range") from None
        _write_history(h)
        return None
    if "s" in flags:
        h.append(" ".join(rest))
        _write_history(h)
        return None
    for f in flags:
        if f in "wanr":
            return None
        raise ValueError(f"history: -{f}: invalid option\nhistory: usage: history [N] | -c | -d N | -s text")
    if rest:
        try:
            n = int(rest[0])
        except ValueError:
            raise ValueError(f"history: {rest[0]}: numeric argument required") from None
        _show_history(h[-n:] if n else [], len(h) - min(n, len(h)) + 1)
    else:
        _show_history(h)
    return None

def _append_history(text):
    p = _history_file()
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "a", encoding="utf-8") as f:
        f.write(text.replace("\n", "<\\n>") + "\n")

def sh_fc(*args, _ns=None):
    """fc -l lists; fc -s [old=new] [cmd] runs one again; fc [-e editor]
    [first [last]] edits them ($FCEDIT, $EDITOR, vi) and runs what's saved."""
    import tempfile, subprocess, shlex
    flags, vals, rest = _split_opts(args, "e")
    h = _history()
    # (This fc line itself, just put in history at the prompt: not one of them.)
    if h and h[-1].split()[:1] == ["fc"]:
        h = h[:-1]
    if not h:
        raise ValueError("fc: no history yet")
    def at(s):
        try:
            k = int(s)
            k = k - 1 if k > 0 else len(h) + k
            if not 0 <= k < len(h):
                raise IndexError
            return k
        except ValueError:
            for k in range(len(h) - 1, -1, -1):
                if h[k].startswith(s):
                    return k
            raise ValueError(f"fc: {s}: no command found") from None
        except IndexError:
            raise ValueError(f"fc: {s}: history position out of range") from None
    if "l" in flags:
        first = at(rest[0]) if rest else max(len(h) - 16, 0)
        last = at(rest[1]) if len(rest) > 1 else len(h) - 1
        lo, hi = sorted((first, last))
        sel = list(enumerate(h[lo:hi + 1], lo + 1))
        if "r" in flags:
            sel.reverse()
        for k, e in sel:
            print(("\t" if "n" in flags else f"{k}\t") + e.replace("\n", "\n\t"))
        return None
    def run(text):
        print(text)
        sys.stdout.flush()
        _append_history(text)
        return _tessh().eval_tessh(text, _ns)
    if "s" in flags or vals.get("e") == "-":
        subs = [r for r in rest if "=" in r and not r.startswith("=")]
        spec = [r for r in rest if r not in subs]
        line = h[at(spec[0])] if spec else h[-1]
        for sub in subs:
            old, new = sub.split("=", 1)
            line = line.replace(old, new)
        return run(line)
    first = at(rest[0]) if rest else len(h) - 1
    last = at(rest[1]) if len(rest) > 1 else first
    lo, hi = sorted((first, last))
    sel = h[lo:hi + 1]
    if "r" in flags:
        sel.reverse()
    editor = vals.get("e") or os.environ.get("FCEDIT") or os.environ.get("EDITOR") or "vi"
    with tempfile.NamedTemporaryFile("w", suffix=".tsh", delete=False) as f:
        f.write("\n".join(sel) + "\n")
        path = f.name
    try:
        code = subprocess.call(shlex.split(editor) + [path])
        with open(path) as f:
            text = f.read().strip("\n")
    finally:
        os.unlink(path)
    if code != 0:
        raise OSError(f"fc: {editor} failed (status {code}): nothing run")
    if not text.strip():
        return None
    return run(text)

def sh_caller(*args):
    raise NotImplementedError("caller: " + "tessh's functions are Python's: traceback.print_stack shows the calls (import traceback), and inspect.stack gives them as values.")

def sh_enable(*args):
    raise NotImplementedError("enable: tessh's builtins can't be turned off: a function of the same name takes a builtin's place (def cd …), and \\name runs the program of that name.")

def sh_builtin(name=None, *args, _ns=None):
    if name is None:
        return None
    b = _tessh().builtin_named(name)
    if b is None:
        raise ValueError(f"builtin: {name}: not a tessh builtin")
    kw = {"_ns": _ns} if getattr(b, "_tessh_wants_ns", False) else {}
    return b(*args, **kw)

# ---- type, command -v, hash ------------------------------------------------------------

_DUALS = ("type", "hash", "exec", "eval")

def _keywords():
    import keyword
    return set(keyword.kwlist) | {"let", "do", "defmacro", "defop", "match", "case"}

def _which_all(name):
    if "/" in name:
        return [name] if os.path.isfile(name) and os.access(name, os.X_OK) else []
    out = []
    for d in os.environ.get("PATH", "").split(os.pathsep):
        p = os.path.join(d or ".", name)
        if os.path.isfile(p) and os.access(p, os.X_OK) and p not in out:
            out.append(p)
    return out

def _what(name, ns, every=False, path_only=False):
    """[(kind, description, path or None)] for a command name, first first."""
    import builtins
    T = _tessh()
    out = []
    if not path_only:
        if name in _keywords():
            out.append(("keyword", f"{name} is a tessh keyword", None))
        v = ns.get(name, None) if ns is not None and name in ns else None
        if ns is not None and name in ns and isinstance(v, Alias):
            out.append(("alias", f"{name} is aliased to `{v.text}'", None))
        elif ns is not None and name in ns:
            if (type(v).__name__ == "Builtin" and type(v).__module__ == "__tessh__") or getattr(v, "__module__", None) in ("__tessh__", "__tessh_shell__", "__tessh_prelude__"):
                out.append(("builtin", f"{name} is a tessh builtin", None))
            elif callable(v):
                what = "command function" if getattr(v, "_tessh_command", False) else "function"
                where = ""
                try:
                    import inspect
                    f = inspect.getsourcefile(v)
                    line = inspect.getsourcelines(v)[1]
                    where = f" ({f}:{line})" if f and not f.startswith("<") else ""
                except (TypeError, OSError):
                    pass
                out.append(("function", f"{name} is a {what}{where}", None))
            else:
                r = repr(v)
                r = r if len(r) <= 60 else r[:57] + "…"
                out.append(("variable", f"{name} is a variable: {r}", None))
        elif name in _DUALS:
            out.append(("builtin", f"{name} is a tessh builtin (and, given a value, Python's {name})", None))
        elif hasattr(builtins, name) and not name.startswith("_"):
            out.append(("builtin", f"{name} is a Python builtin", None))
    for p in _which_all(name):
        out.append(("file", f"{name} is {p}", p))
        if not every:
            break
    return out if every else out[:1]

def sh_type(*args, _ns=None):
    flags, _, names = _split_opts(args)
    for f in flags:
        if f not in "aftpP":
            raise ValueError(f"type: -{f}: invalid option\ntype: usage: type [-afptP] name …")
    every, terse, path, force = "a" in flags, "t" in flags, "p" in flags, "P" in flags
    code = 0
    for n in names:
        found = _what(n, _ns, every, force)
        if not found:
            if not (terse or path):
                _err(f"type: {n}: not found")
            code = 1
            continue
        for kind, text, p in found:
            if path or force:
                if p:
                    print(p)
            elif terse:
                print(kind)
            else:
                print(text)
    return _status(code) if code else None

def command_opts(words, ns):
    flags, _, rest = _split_opts(words)
    if "p" in flags and not ("v" in flags or "V" in flags):
        return _tessh().command(*rest)
    code = 0
    for n in rest:
        found = _what(n, ns)
        if not found:
            if "V" in flags:
                _err(f"command: {n}: not found")
            code = 1
            continue
        kind, text, p = found[0]
        print(text if "V" in flags else (p or n))
    return _status(code)

def sh_hash(*args, _ns=None):
    flags, vals, names = _split_opts(args, "p")
    if not args:
        print("hash: tessh doesn't remember where programs are: it looks each one up on $PATH as it runs it")
        return None
    if "r" in flags or "d" in flags or "l" in flags or "p" in vals:
        return None
    code = 0
    for n in names:
        ps = _which_all(n)
        if not ps:
            _err(f"hash: {n}: not found")
            code = 1
        elif "t" in flags:
            print(ps[0] if len(names) == 1 else f"{n}\t{ps[0]}")
    return _status(code) if code else None

# ---- exec, eval ------------------------------------------------------------------------

def sh_exec(*args):
    flags, vals, rest = _split_opts(args, "a")
    for f in flags:
        if f not in "cl":
            raise ValueError(f"exec: -{f}: invalid option\nexec: usage: exec [-cl] [-a name] program [arg …]")
    if not rest:
        return None
    prog = rest[0]
    argv0 = vals.get("a", prog)
    if "l" in flags:
        argv0 = "-" + os.path.basename(argv0)
    _tessh().exec_program(prog, argv0, list(rest[1:]), "c" in flags)

def sh_eval(*args, _ns=None):
    text = " ".join(args)
    if not text.strip():
        return None
    return _tessh().eval_tessh(text, _ns)
"#####;
