# Appendix M. Sample .bashrc and .bash_profile Files
<!-- help: abs-sample-bashrc abs-m bashrc rc.tsh startup-file aliases prompt -->

`~/.bashrc` sets up interactive bash: aliases, functions, the prompt,
options, completion. Emmanuel Rouat contributed a very full one, written
for Linux, to study and borrow from. tessh's counterpart is
`~/.config/tessh/rc.tsh`, which the REPL runs before its first prompt
(scripts don't read it). Below is the bashrc, then a port of it as an
`rc.tsh`: everything tessh has a way to do, and a note at each part it
hasn't.

### Example M-1. Sample `.bashrc` file (`bashrc`)

```bash no-run
# =============================================================== #
#
# PERSONAL $HOME/.bashrc FILE for bash-3.0 (or later)
# By Emmanuel Rouat [no-email]
#
# Last modified: Tue Nov 20 22:04:47 CET 2012

#  This file is normally read by interactive shells only.
#+ Here is the place to define your aliases, functions and
#+ other interactive features like your prompt.
#
#  The majority of the code here assumes you are on a GNU
#+ system (most likely a Linux box) and is often based on code
#+ found on Usenet or Internet.
#
#  See for instance:
#  http://tldp.org/LDP/abs/html/index.html
#  http://www.caliban.org/bash
#  http://www.shelldorado.com/scripts/categories.html
#  http://www.dotfiles.org
#
#  The choice of colors was done for a shell with a dark background
#+ (white on black), and this is usually also suited for pure text-mode
#+ consoles (no X server available). If you use a white background,
#+ you'll have to do some other choices for readability.
#
#  This bashrc file is a bit overcrowded.
#  Remember, it is just just an example.
#  Tailor it to your needs.
#
# =============================================================== #

# --> Comments added by HOWTO author.

# If not running interactively, don't do anything
[ -z "$PS1" ] && return


#-------------------------------------------------------------
# Source global definitions (if any)
#-------------------------------------------------------------


if [ -f /etc/bashrc ]; then
      . /etc/bashrc   # --> Read /etc/bashrc, if present.
fi


#--------------------------------------------------------------
#  Automatic setting of $DISPLAY (if not set already).
#  This works for me - your mileage may vary. . . .
#  The problem is that different types of terminals give
#+ different answers to 'who am i' (rxvt in particular can be
#+ troublesome) - however this code seems to work in a majority
#+ of cases.
#--------------------------------------------------------------

function get_xserver ()
{
    case $TERM in
        xterm )
            XSERVER=$(who am i | awk '{print $NF}' | tr -d ')''(' )
            # Ane-Pieter Wieringa suggests the following alternative:
            #  I_AM=$(who am i)
            #  SERVER=${I_AM#*(}
            #  SERVER=${SERVER%*)}
            XSERVER=${XSERVER%%:*}
            ;;
            aterm | rxvt)
            # Find some code that works here. ...
            ;;
    esac
}

if [ -z ${DISPLAY:=""} ]; then
    get_xserver
    if [[ -z ${XSERVER}  || ${XSERVER} == $(hostname) ||
       ${XSERVER} == "unix" ]]; then
          DISPLAY=":0.0"          # Display on local host.
    else
       DISPLAY=${XSERVER}:0.0     # Display on remote host.
    fi
fi

export DISPLAY

#-------------------------------------------------------------
# Some settings
#-------------------------------------------------------------

#set -o nounset     # These  two options are useful for debugging.
#set -o xtrace
alias debug="set -o nounset; set -o xtrace"

ulimit -S -c 0      # Don't want coredumps.
set -o notify
set -o noclobber
set -o ignoreeof


# Enable options:
shopt -s cdspell
shopt -s cdable_vars
shopt -s checkhash
shopt -s checkwinsize
shopt -s sourcepath
shopt -s no_empty_cmd_completion
shopt -s cmdhist
shopt -s histappend histreedit histverify
shopt -s extglob       # Necessary for programmable completion.

# Disable options:
shopt -u mailwarn
unset MAILCHECK        # Don't want my shell to warn me of incoming mail.


#-------------------------------------------------------------
# Greeting, motd etc. ...
#-------------------------------------------------------------

# Color definitions (taken from Color Bash Prompt HowTo).
# Some colors might look different of some terminals.
# For example, I see 'Bold Red' as 'orange' on my screen,
# hence the 'Green' 'BRed' 'Red' sequence I often use in my prompt.


# Normal Colors
Black='\e[0;30m'        # Black
Red='\e[0;31m'          # Red
Green='\e[0;32m'        # Green
Yellow='\e[0;33m'       # Yellow
Blue='\e[0;34m'         # Blue
Purple='\e[0;35m'       # Purple
Cyan='\e[0;36m'         # Cyan
White='\e[0;37m'        # White

# Bold
BBlack='\e[1;30m'       # Black
BRed='\e[1;31m'         # Red
BGreen='\e[1;32m'       # Green
BYellow='\e[1;33m'      # Yellow
BBlue='\e[1;34m'        # Blue
BPurple='\e[1;35m'      # Purple
BCyan='\e[1;36m'        # Cyan
BWhite='\e[1;37m'       # White

# Background
On_Black='\e[40m'       # Black
On_Red='\e[41m'         # Red
On_Green='\e[42m'       # Green
On_Yellow='\e[43m'      # Yellow
On_Blue='\e[44m'        # Blue
On_Purple='\e[45m'      # Purple
On_Cyan='\e[46m'        # Cyan
On_White='\e[47m'       # White

NC="\e[m"               # Color Reset


ALERT=${BWhite}${On_Red} # Bold White on red background



echo -e "${BCyan}This is BASH ${BRed}${BASH_VERSION%.*}${BCyan}\
- DISPLAY on ${BRed}$DISPLAY${NC}\n"
date
if [ -x /usr/games/fortune ]; then
    /usr/games/fortune -s     # Makes our day a bit more fun.... :-)
fi

function _exit()              # Function to run upon exit of shell.
{
    echo -e "${BRed}Hasta la vista, baby${NC}"
}
trap _exit EXIT

#-------------------------------------------------------------
# Shell Prompt - for many examples, see:
#       http://www.debian-administration.org/articles/205
#       http://www.askapache.com/linux/bash-power-prompt.html
#       http://tldp.org/HOWTO/Bash-Prompt-HOWTO
#       https://github.com/nojhan/liquidprompt
#-------------------------------------------------------------
# Current Format: [TIME USER@HOST PWD] >
# TIME:
#    Green     == machine load is low
#    Orange    == machine load is medium
#    Red       == machine load is high
#    ALERT     == machine load is very high
# USER:
#    Cyan      == normal user
#    Orange    == SU to user
#    Red       == root
# HOST:
#    Cyan      == local session
#    Green     == secured remote connection (via ssh)
#    Red       == unsecured remote connection
# PWD:
#    Green     == more than 10% free disk space
#    Orange    == less than 10% free disk space
#    ALERT     == less than 5% free disk space
#    Red       == current user does not have write privileges
#    Cyan      == current filesystem is size zero (like /proc)
# >:
#    White     == no background or suspended jobs in this shell
#    Cyan      == at least one background job in this shell
#    Orange    == at least one suspended job in this shell
#
#    Command is added to the history file each time you hit enter,
#    so it's available to all shells (using 'history -a').


# Test connection type:
if [ -n "${SSH_CONNECTION}" ]; then
    CNX=${Green}        # Connected on remote machine, via ssh (good).
elif [[ "${DISPLAY%%:0*}" != "" ]]; then
    CNX=${ALERT}        # Connected on remote machine, not via ssh (bad).
else
    CNX=${BCyan}        # Connected on local machine.
fi

# Test user type:
if [[ ${USER} == "root" ]]; then
    SU=${Red}           # User is root.
elif [[ ${USER} != $(logname) ]]; then
    SU=${BRed}          # User is not login user.
else
    SU=${BCyan}         # User is normal (well ... most of us are).
fi



NCPU=$(grep -c 'processor' /proc/cpuinfo)    # Number of CPUs
SLOAD=$(( 100*${NCPU} ))        # Small load
MLOAD=$(( 200*${NCPU} ))        # Medium load
XLOAD=$(( 400*${NCPU} ))        # Xlarge load

# Returns system load as percentage, i.e., '40' rather than '0.40)'.
function load()
{
    local SYSLOAD=$(cut -d " " -f1 /proc/loadavg | tr -d '.')
    # System load of the current host.
    echo $((10#$SYSLOAD))       # Convert to decimal.
}

# Returns a color indicating system load.
function load_color()
{
    local SYSLOAD=$(load)
    if [ ${SYSLOAD} -gt ${XLOAD} ]; then
        echo -en ${ALERT}
    elif [ ${SYSLOAD} -gt ${MLOAD} ]; then
        echo -en ${Red}
    elif [ ${SYSLOAD} -gt ${SLOAD} ]; then
        echo -en ${BRed}
    else
        echo -en ${Green}
    fi
}

# Returns a color according to free disk space in $PWD.
function disk_color()
{
    if [ ! -w "${PWD}" ] ; then
        echo -en ${Red}
        # No 'write' privilege in the current directory.
    elif [ -s "${PWD}" ] ; then
        local used=$(command df -P "$PWD" |
                   awk 'END {print $5} {sub(/%/,"")}')
        if [ ${used} -gt 95 ]; then
            echo -en ${ALERT}           # Disk almost full (>95%).
        elif [ ${used} -gt 90 ]; then
            echo -en ${BRed}            # Free disk space almost gone.
        else
            echo -en ${Green}           # Free disk space is ok.
        fi
    else
        echo -en ${Cyan}
        # Current directory is size '0' (like /proc, /sys etc).
    fi
}

# Returns a color according to running/suspended jobs.
function job_color()
{
    if [ $(jobs -s | wc -l) -gt "0" ]; then
        echo -en ${BRed}
    elif [ $(jobs -r | wc -l) -gt "0" ] ; then
        echo -en ${BCyan}
    fi
}

# Adds some text in the terminal frame (if applicable).


# Now we construct the prompt.
PROMPT_COMMAND="history -a"
case ${TERM} in
  *term | rxvt | linux)
        PS1="\[\$(load_color)\][\A\[${NC}\] "
        # Time of day (with load info):
        PS1="\[\$(load_color)\][\A\[${NC}\] "
        # User@Host (with connection type info):
        PS1=${PS1}"\[${SU}\]\u\[${NC}\]@\[${CNX}\]\h\[${NC}\] "
        # PWD (with 'disk space' info):
        PS1=${PS1}"\[\$(disk_color)\]\W]\[${NC}\] "
        # Prompt (with 'job' info):
        PS1=${PS1}"\[\$(job_color)\]>\[${NC}\] "
        # Set title of current xterm:
        PS1=${PS1}"\[\e]0;[\u@\h] \w\a\]"
        ;;
    *)
        PS1="(\A \u@\h \W) > " # --> PS1="(\A \u@\h \w) > "
                               # --> Shows full pathname of current dir.
        ;;
esac



export TIMEFORMAT=$'\nreal %3R\tuser %3U\tsys %3S\tpcpu %P\n'
export HISTIGNORE="&:bg:fg:ll:h"
export HISTTIMEFORMAT="$(echo -e ${BCyan})[%d/%m %H:%M:%S]$(echo -e ${NC}) "
export HISTCONTROL=ignoredups
export HOSTFILE=$HOME/.hosts    # Put a list of remote hosts in ~/.hosts


#============================================================
#
#  ALIASES AND FUNCTIONS
#
#  Arguably, some functions defined here are quite big.
#  If you want to make this file smaller, these functions can
#+ be converted into scripts and removed from here.
#
#============================================================

#-------------------
# Personnal Aliases
#-------------------

alias rm='rm -i'
alias cp='cp -i'
alias mv='mv -i'
# -> Prevents accidentally clobbering files.
alias mkdir='mkdir -p'

alias h='history'
alias j='jobs -l'
alias which='type -a'
alias ..='cd ..'

# Pretty-print of some PATH variables:
alias path='echo -e ${PATH//:/\\n}'
alias libpath='echo -e ${LD_LIBRARY_PATH//:/\\n}'


alias du='du -kh'    # Makes a more readable output.
alias df='df -kTh'

#-------------------------------------------------------------
# The 'ls' family (this assumes you use a recent GNU ls).
#-------------------------------------------------------------
# Add colors for filetype and  human-readable sizes by default on 'ls':
alias ls='ls -h --color'
alias lx='ls -lXB'         #  Sort by extension.
alias lk='ls -lSr'         #  Sort by size, biggest last.
alias lt='ls -ltr'         #  Sort by date, most recent last.
alias lc='ls -ltcr'        #  Sort by/show change time,most recent last.
alias lu='ls -ltur'        #  Sort by/show access time,most recent last.

# The ubiquitous 'll': directories first, with alphanumeric sorting:
alias ll="ls -lv --group-directories-first"
alias lm='ll |more'        #  Pipe through 'more'
alias lr='ll -R'           #  Recursive ls.
alias la='ll -A'           #  Show hidden files.
alias tree='tree -Csuh'    #  Nice alternative to 'recursive ls' ...


#-------------------------------------------------------------
# Tailoring 'less'
#-------------------------------------------------------------

alias more='less'
export PAGER=less
export LESSCHARSET='latin1'
export LESSOPEN='|/usr/bin/lesspipe.sh %s 2>&-'
                # Use this if lesspipe.sh exists.
export LESS='-i -N -w  -z-4 -g -e -M -X -F -R -P%t?f%f \
:stdin .?pb%pb\%:?lbLine %lb:?bbByte %bb:-...'

# LESS man page colors (makes Man pages more readable).
export LESS_TERMCAP_mb=$'\E[01;31m'
export LESS_TERMCAP_md=$'\E[01;31m'
export LESS_TERMCAP_me=$'\E[0m'
export LESS_TERMCAP_se=$'\E[0m'
export LESS_TERMCAP_so=$'\E[01;44;33m'
export LESS_TERMCAP_ue=$'\E[0m'
export LESS_TERMCAP_us=$'\E[01;32m'


#-------------------------------------------------------------
# Spelling typos - highly personnal and keyboard-dependent :-)
#-------------------------------------------------------------

alias xs='cd'
alias vf='cd'
alias moer='more'
alias moew='more'
alias kk='ll'


#-------------------------------------------------------------
# A few fun ones
#-------------------------------------------------------------

# Adds some text in the terminal frame (if applicable).

function xtitle()
{
    case "$TERM" in
    *term* | rxvt)
        echo -en  "\e]0;$*\a" ;;
    *)  ;;
    esac
}


# Aliases that use xtitle
alias top='xtitle Processes on $HOST && top'
alias make='xtitle Making $(basename $PWD) ; make'

# .. and functions
function man()
{
    for i ; do
        xtitle The $(basename $1|tr -d .[:digit:]) manual
        command man -a "$i"
    done
}


#-------------------------------------------------------------
# Make the following commands run in background automatically:
#-------------------------------------------------------------

function te()  # wrapper around xemacs/gnuserv
{
    if [ "$(gnuclient -batch -eval t 2>&-)" == "t" ]; then
       gnuclient -q "$@";
    else
       ( xemacs "$@" &);
    fi
}

function soffice() { command soffice "$@" & }
function firefox() { command firefox "$@" & }
function xpdf() { command xpdf "$@" & }


#-------------------------------------------------------------
# File & strings related functions:
#-------------------------------------------------------------


# Find a file with a pattern in name:
function ff() { find . -type f -iname '*'"$*"'*' -ls ; }

# Find a file with pattern $1 in name and Execute $2 on it:
function fe() { find . -type f -iname '*'"${1:-}"'*' \
-exec ${2:-file} {} \;  ; }

#  Find a pattern in a set of files and highlight them:
#+ (needs a recent version of egrep).
function fstr()
{
    OPTIND=1
    local mycase=""
    local usage="fstr: find string in files.
Usage: fstr [-i] \"pattern\" [\"filename pattern\"] "
    while getopts :it opt
    do
        case "$opt" in
           i) mycase="-i " ;;
           *) echo "$usage"; return ;;
        esac
    done
    shift $(( $OPTIND - 1 ))
    if [ "$#" -lt 1 ]; then
        echo "$usage"
        return;
    fi
    find . -type f -name "${2:-*}" -print0 | \
xargs -0 egrep --color=always -sn ${case} "$1" 2>&- | more

}


function swap()
{ # Swap 2 filenames around, if they exist (from Uzi's bashrc).
    local TMPFILE=tmp.$$

    [ $# -ne 2 ] && echo "swap: 2 arguments needed" && return 1
    [ ! -e $1 ] && echo "swap: $1 does not exist" && return 1
    [ ! -e $2 ] && echo "swap: $2 does not exist" && return 1

    mv "$1" $TMPFILE
    mv "$2" "$1"
    mv $TMPFILE "$2"
}

function extract()      # Handy Extract Program
{
    if [ -f $1 ] ; then
        case $1 in
            *.tar.bz2)   tar xvjf $1     ;;
            *.tar.gz)    tar xvzf $1     ;;
            *.bz2)       bunzip2 $1      ;;
            *.rar)       unrar x $1      ;;
            *.gz)        gunzip $1       ;;
            *.tar)       tar xvf $1      ;;
            *.tbz2)      tar xvjf $1     ;;
            *.tgz)       tar xvzf $1     ;;
            *.zip)       unzip $1        ;;
            *.Z)         uncompress $1   ;;
            *.7z)        7z x $1         ;;
            *)           echo "'$1' cannot be extracted via >extract<" ;;
        esac
    else
        echo "'$1' is not a valid file!"
    fi
}


# Creates an archive (*.tar.gz) from given directory.
function maketar() { tar cvzf "${1%%/}.tar.gz"  "${1%%/}/"; }

# Create a ZIP archive of a file or folder.
function makezip() { zip -r "${1%%/}.zip" "$1" ; }

# Make your directories and files access rights sane.
function sanitize() { chmod -R u=rwX,g=rX,o= "$@" ;}

#-------------------------------------------------------------
# Process/system related functions:
#-------------------------------------------------------------


function my_ps() { ps $@ -u $USER -o pid,%cpu,%mem,bsdtime,command ; }
function pp() { my_ps f | awk '!/awk/ && $0~var' var=${1:-".*"} ; }


function killps()   # kill by process name
{
    local pid pname sig="-TERM"   # default signal
    if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
        echo "Usage: killps [-SIGNAL] pattern"
        return;
    fi
    if [ $# = 2 ]; then sig=$1 ; fi
    for pid in $(my_ps| awk '!/awk/ && $0~pat { print $1 }' pat=${!#} )
    do
        pname=$(my_ps | awk '$1~var { print $5 }' var=$pid )
        if ask "Kill process $pid <$pname> with signal $sig?"
            then kill $sig $pid
        fi
    done
}

function mydf()         # Pretty-print of 'df' output.
{                       # Inspired by 'dfc' utility.
    for fs ; do

        if [ ! -d $fs ]
        then
          echo -e $fs" :No such file or directory" ; continue
        fi

        local info=( $(command df -P $fs | awk 'END{ print $2,$3,$5 }') )
        local free=( $(command df -Pkh $fs | awk 'END{ print $4 }') )
        local nbstars=$(( 20 * ${info[1]} / ${info[0]} ))
        local out="["
        for ((j=0;j<20;j++)); do
            if [ ${j} -lt ${nbstars} ]; then
               out=$out"*"
            else
               out=$out"-"
            fi
        done
        out=${info[2]}" "$out"] ("$free" free on "$fs")"
        echo -e $out
    done
}


function my_ip() # Get IP adress on ethernet.
{
    MY_IP=$(/sbin/ifconfig eth0 | awk '/inet/ { print $2 } ' |
      sed -e s/addr://)
    echo ${MY_IP:-"Not connected"}
}

function ii()   # Get current host related info.
{
    echo -e "\nYou are logged on ${BRed}$HOST"
    echo -e "\n${BRed}Additionnal information:$NC " ; uname -a
    echo -e "\n${BRed}Users logged on:$NC " ; w -hs |
             cut -d " " -f1 | sort | uniq
    echo -e "\n${BRed}Current date :$NC " ; date
    echo -e "\n${BRed}Machine stats :$NC " ; uptime
    echo -e "\n${BRed}Memory stats :$NC " ; free
    echo -e "\n${BRed}Diskspace :$NC " ; mydf / $HOME
    echo -e "\n${BRed}Local IP Address :$NC" ; my_ip
    echo -e "\n${BRed}Open connections :$NC "; netstat -pan --inet;
    echo
}

#-------------------------------------------------------------
# Misc utilities:
#-------------------------------------------------------------

function repeat()       # Repeat n times command.
{
    local i max
    max=$1; shift;
    for ((i=1; i <= max ; i++)); do  # --> C-like syntax
        eval "$@";
    done
}


function ask()          # See 'killps' for example of use.
{
    echo -n "$@" '[y/n] ' ; read ans
    case "$ans" in
        y*|Y*) return 0 ;;
        *) return 1 ;;
    esac
}

function corename()   # Get name of app that created a corefile.
{
    for file ; do
        echo -n $file : ; gdb --core=$file --batch | head -1
    done
}



#=========================================================================
#
#  PROGRAMMABLE COMPLETION SECTION
#  Most are taken from the bash 2.05 documentation and from Ian McDonald's
# 'Bash completion' package (http://www.caliban.org/bash/#completion)
#  You will in fact need bash more recent then 3.0 for some features.
#
#  Note that most linux distributions now provide many completions
# 'out of the box' - however, you might need to make your own one day,
#  so I kept those here as examples.
#=========================================================================

if [ "${BASH_VERSION%.*}" \< "3.0" ]; then
    echo "You will need to upgrade to version 3.0 for full \
          programmable completion features"
    return
fi

shopt -s extglob        # Necessary.

complete -A hostname   rsh rcp telnet rlogin ftp ping disk
complete -A export     printenv
complete -A variable   export local readonly unset
complete -A enabled    builtin
complete -A alias      alias unalias
complete -A function   function
complete -A user       su mail finger

complete -A helptopic  help     # Currently same as builtins.
complete -A shopt      shopt
complete -A stopped -P '%' bg
complete -A job -P '%'     fg jobs disown

complete -A directory  mkdir rmdir
complete -A directory   -o default cd

# Compression
complete -f -o default -X '*.+(zip|ZIP)'  zip
complete -f -o default -X '!*.+(zip|ZIP)' unzip
complete -f -o default -X '*.+(z|Z)'      compress
complete -f -o default -X '!*.+(z|Z)'     uncompress
complete -f -o default -X '*.+(gz|GZ)'    gzip
complete -f -o default -X '!*.+(gz|GZ)'   gunzip
complete -f -o default -X '*.+(bz2|BZ2)'  bzip2
complete -f -o default -X '!*.+(bz2|BZ2)' bunzip2
complete -f -o default -X '!*.+(zip|ZIP|z|Z|gz|GZ|bz2|BZ2)' extract


# Documents - Postscript,pdf,dvi.....
complete -f -o default -X '!*.+(ps|PS)'  gs ghostview ps2pdf ps2ascii
complete -f -o default -X \
'!*.+(dvi|DVI)' dvips dvipdf xdvi dviselect dvitype
complete -f -o default -X '!*.+(pdf|PDF)' acroread pdf2ps
complete -f -o default -X '!*.@(@(?(e)ps|?(E)PS|pdf|PDF)?\
(.gz|.GZ|.bz2|.BZ2|.Z))' gv ggv
complete -f -o default -X '!*.texi*' makeinfo texi2dvi texi2html texi2pdf
complete -f -o default -X '!*.tex' tex latex slitex
complete -f -o default -X '!*.lyx' lyx
complete -f -o default -X '!*.+(htm*|HTM*)' lynx html2ps
complete -f -o default -X \
'!*.+(doc|DOC|xls|XLS|ppt|PPT|sx?|SX?|csv|CSV|od?|OD?|ott|OTT)' soffice

# Multimedia
complete -f -o default -X \
'!*.+(gif|GIF|jp*g|JP*G|bmp|BMP|xpm|XPM|png|PNG)' xv gimp ee gqview
complete -f -o default -X '!*.+(mp3|MP3)' mpg123 mpg321
complete -f -o default -X '!*.+(ogg|OGG)' ogg123
complete -f -o default -X \
'!*.@(mp[23]|MP[23]|ogg|OGG|wav|WAV|pls|\
m3u|xm|mod|s[3t]m|it|mtm|ult|flac)' xmms
complete -f -o default -X '!*.@(mp?(e)g|MP?(E)G|wma|avi|AVI|\
asf|vob|VOB|bin|dat|vcd|ps|pes|fli|viv|rm|ram|yuv|mov|MOV|qt|\
QT|wmv|mp3|MP3|ogg|OGG|ogm|OGM|mp4|MP4|wav|WAV|asx|ASX)' xine



complete -f -o default -X '!*.pl'  perl perl5


#  This is a 'universal' completion function - it works when commands have
#+ a so-called 'long options' mode , ie: 'ls --all' instead of 'ls -a'
#  Needs the '-o' option of grep
#+ (try the commented-out version if not available).

#  First, remove '=' from completion word separators
#+ (this will allow completions like 'ls --color=auto' to work correctly).

COMP_WORDBREAKS=${COMP_WORDBREAKS/=/}


_get_longopts()
{
  #$1 --help | sed  -e '/--/!d' -e 's/.*--\([^[:space:].,]*\).*/--\1/'| \
  #grep ^"$2" |sort -u ;
    $1 --help | grep -o -e "--[^[:space:].,]*" | grep -e "$2" |sort -u
}

_longopts()
{
    local cur
    cur=${COMP_WORDS[COMP_CWORD]}

    case "${cur:-*}" in
       -*)      ;;
        *)      return ;;
    esac

    case "$1" in
       \~*)     eval cmd="$1" ;;
         *)     cmd="$1" ;;
    esac
    COMPREPLY=( $(_get_longopts ${1} ${cur} ) )
}
complete  -o default -F _longopts configure bash
complete  -o default -F _longopts wget id info a2ps ls recode

_tar()
{
    local cur ext regex tar untar

    COMPREPLY=()
    cur=${COMP_WORDS[COMP_CWORD]}

    # If we want an option, return the possible long options.
    case "$cur" in
        -*)     COMPREPLY=( $(_get_longopts $1 $cur ) ); return 0;;
    esac

    if [ $COMP_CWORD -eq 1 ]; then
        COMPREPLY=( $( compgen -W 'c t x u r d A' -- $cur ) )
        return 0
    fi

    case "${COMP_WORDS[1]}" in
        ?(-)c*f)
            COMPREPLY=( $( compgen -f $cur ) )
            return 0
            ;;
        +([^Izjy])f)
            ext='tar'
            regex=$ext
            ;;
        *z*f)
            ext='tar.gz'
            regex='t\(ar\.\)\(gz\|Z\)'
            ;;
        *[Ijy]*f)
            ext='t?(ar.)bz?(2)'
            regex='t\(ar\.\)bz2\?'
            ;;
        *)
            COMPREPLY=( $( compgen -f $cur ) )
            return 0
            ;;

    esac

    if [[ "$COMP_LINE" == tar*.$ext' '* ]]; then
        # Complete on files in tar file.
        #
        # Get name of tar file from command line.
        tar=$( echo "$COMP_LINE" | \
                        sed -e 's|^.* \([^ ]*'$regex'\) .*$|\1|' )
        # Devise how to untar and list it.
        untar=t${COMP_WORDS[1]//[^Izjyf]/}

        COMPREPLY=( $( compgen -W "$( echo $( tar $untar $tar \
                                2>/dev/null ) )" -- "$cur" ) )
        return 0

    else
        # File completion on relevant files.
        COMPREPLY=( $( compgen -G $cur\*.$ext ) )

    fi

    return 0

}

complete -F _tar -o default tar

_make()
{
    local mdef makef makef_dir="." makef_inc gcmd cur prev i;
    COMPREPLY=();
    cur=${COMP_WORDS[COMP_CWORD]};
    prev=${COMP_WORDS[COMP_CWORD-1]};
    case "$prev" in
        -*f)
            COMPREPLY=($(compgen -f $cur ));
            return 0
            ;;
    esac;
    case "$cur" in
        -*)
            COMPREPLY=($(_get_longopts $1 $cur ));
            return 0
            ;;
    esac;

    # ... make reads
    #          GNUmakefile,
    #     then makefile
    #     then Makefile ...
    if [ -f ${makef_dir}/GNUmakefile ]; then
        makef=${makef_dir}/GNUmakefile
    elif [ -f ${makef_dir}/makefile ]; then
        makef=${makef_dir}/makefile
    elif [ -f ${makef_dir}/Makefile ]; then
        makef=${makef_dir}/Makefile
    else
       makef=${makef_dir}/*.mk         # Local convention.
    fi


    #  Before we scan for targets, see if a Makefile name was
    #+ specified with -f.
    for (( i=0; i < ${#COMP_WORDS[@]}; i++ )); do
        if [[ ${COMP_WORDS[i]} == -f ]]; then
            # eval for tilde expansion
            eval makef=${COMP_WORDS[i+1]}
            break
        fi
    done
    [ ! -f $makef ] && return 0

    # Deal with included Makefiles.
    makef_inc=$( grep -E '^-?include' $makef |
                 sed -e "s,^.* ,"$makef_dir"/," )
    for file in $makef_inc; do
        [ -f $file ] && makef="$makef $file"
    done


    #  If we have a partial word to complete, restrict completions
    #+ to matches of that word.
    if [ -n "$cur" ]; then gcmd='grep "^$cur"' ; else gcmd=cat ; fi

    COMPREPLY=( $( awk -F':' '/^[a-zA-Z0-9][^$#\/\t=]*:([^=]|$)/ \
                               {split($1,A,/ /);for(i in A)print A[i]}' \
                                $makef 2>/dev/null | eval $gcmd  ))

}

complete -F _make -X '+($*|*.[cho])' make gmake pmake




_killall()
{
    local cur prev
    COMPREPLY=()
    cur=${COMP_WORDS[COMP_CWORD]}

    #  Get a list of processes
    #+ (the first sed evaluation
    #+ takes care of swapped out processes, the second
    #+ takes care of getting the basename of the process).
    COMPREPLY=( $( ps -u $USER -o comm  | \
        sed -e '1,1d' -e 's#[]\[]##g' -e 's#^.*/##'| \
        awk '{if ($0 ~ /^'$cur'/) print $0}' ))

    return 0
}

complete -F _killall killall killps



# Local Variables:
# mode:shell-script
# sh-shell:bash
# End:
```

What changes, part by part:

- **Interactive only.** `rc.tsh` is read by the REPL alone, so the `[ -z
  "$PS1" ] && return` guard isn't needed. There's no system-wide file like
  `/etc/bashrc` to source.
- **Options.** `set -o nounset` is always on in tessh (an unset variable is
  an error). **Gap:** there's no `xtrace`, `noclobber`, `ignoreeof`,
  `notify`, or `shopt` (`cdspell`, `cdable_vars`, `checkwinsize`,
  `histappend`…). `ulimit -S -c 0` is Python's `resource.setrlimit`.
- **Colors** are a dict of escape strings; `"\x1b"` is bash's `\e`.
- **The exit trap** is `atexit.register`.
- **Aliases.** tessh has no aliases; a function does the job. With
  `@command` it reads its arguments as words (globs included), as a
  command does, and passes them on with `$args`. A function named after a
  program runs the program itself as `\name`.
- **The prompt.** **Gap:** tessh's prompt is fixed: the folder, then `❯`,
  green after success and red after a failure. There's no `PS1` or
  `PROMPT_COMMAND`, so the load, disk and job colors go into a `status`
  function that prints the same line on demand.
- **History and timing variables** (`HISTIGNORE`, `HISTCONTROL`,
  `HISTTIMEFORMAT`, `TIMEFORMAT`) have no counterpart (Appendix L).
- **Programmable completion** (the long last section) has none either
  (Appendix J).
- `getopts` (in `fstr`) is a loop over the arguments; `${!#}`, the last
  argument, is `$args[-1]`; `local` is unnecessary, since a function's
  names are its own.

```tessh no-run
# =============================================================== #
#
# PERSONAL ~/.config/tessh/rc.tsh FILE, ported from
# Emmanuel Rouat's $HOME/.bashrc (bash-3.0 or later).
#
#  The REPL reads this file before its first prompt; scripts don't.
#  Here is the place to define your functions and other
#+ interactive features.
#
#  The choice of colors was done for a shell with a dark background.
#
# =============================================================== #

import os sys re shutil socket resource atexit datetime contextlib io

#--------------------------------------------------------------
#  Automatic setting of $DISPLAY (if not set already).
#--------------------------------------------------------------

def get_xserver
  match ($env.get TERM "")
    case xterm
      # The host in "who am i"'s last field: (host:0) or (host).
      xserver = ((who am i).split or [""])[-1].strip "()"
      ($xserver.split ":")[0]
    case _
      ""                       # aterm, rxvt: find some code that works here.

if not ($env.get DISPLAY)
  XSERVER = (get_xserver)
  if $XSERVER in ["" (socket.gethostname) "unix"]
    $env.DISPLAY = ":0.0"            # Display on local host.
  else
    $env.DISPLAY = "${XSERVER}:0.0"  # Display on remote host.

#-------------------------------------------------------------
# Some settings
#-------------------------------------------------------------

# Don't want coredumps.
resource.setrlimit $resource.RLIMIT_CORE (0, (resource.getrlimit $resource.RLIMIT_CORE)[1])

# (set -o nounset is always on; xtrace, notify, noclobber, ignoreeof and
#  the shopt options have no tessh counterpart. MAILCHECK isn't read.)
unset MAILCHECK

#-------------------------------------------------------------
# Greeting, motd etc. ...
#-------------------------------------------------------------

# Color definitions (taken from Color Bash Prompt HowTo).
def color code = "\x1b[$(code)m"
[Black Red Green Yellow Blue Purple Cyan White] = [(color "0;$n") for n in (range 30 38)]
[BBlack BRed BGreen BYellow BBlue BPurple BCyan BWhite] = [(color "1;$n") for n in (range 30 38)]
[On_Black On_Red On_Green On_Yellow On_Blue On_Purple On_Cyan On_White] = [(color $n) for n in (range 40 48)]
NC = color ""                  # Color Reset

ALERT = $BWhite + $On_Red      # Bold White on red background

pyver = (sys.version.split)[0]
print "${BCyan}This is tessh, on Python ${BRed}${pyver}${BCyan} - DISPLAY on ${BRed}${env.DISPLAY}${NC}\n"
date
if (os.access /usr/games/fortune $os.X_OK)
  /usr/games/fortune -s        # Makes our day a bit more fun.... :-)

def _exit                      # Function to run upon exit of shell.
  print "${BRed}Hasta la vista, baby${NC}"
atexit.register $_exit

#-------------------------------------------------------------
# Status line: what the bash prompt showed, [TIME USER@HOST PWD] >
# (tessh's prompt can't be changed: run `status` to see it).
# TIME:  Green, Orange, Red, ALERT == machine load low to very high
# USER:  Cyan == normal user, Orange == SU to user, Red == root
# HOST:  Cyan == local, Green == ssh, Red == unsecured remote connection
# PWD:   Green == more than 10% free disk space, Orange == less than 10%,
#        ALERT == less than 5%, Red == no write permission,
#        Cyan == filesystem of size zero (like /proc)
# >:     White == no jobs, Cyan == a background job, Orange == a stopped one
#-------------------------------------------------------------

# Test connection type:
CNX = if ($env.get SSH_CONNECTION)
  $Green                       # Connected on remote machine, via ssh (good).
elif not ($env.get DISPLAY "").startswith ":"
  $ALERT                       # Connected on remote machine, not via ssh (bad).
else
  $BCyan                       # Connected on local machine.

# Test user type:
login = try
  os.getlogin
except OSError
  $env.get USER
SU = if (os.geteuid) == 0
  $Red                         # User is root.
elif ($env.get USER) != $login
  $BRed                        # User is not login user.
else
  $BCyan                       # User is normal (well ... most of us are).

NCPU = (os.cpu_count)           # Number of CPUs
SLOAD = 100 * $NCPU            # Small load
MLOAD = 200 * $NCPU            # Medium load
XLOAD = 400 * $NCPU            # Xlarge load

# Returns system load as percentage, i.e., 40 rather than 0.40.
def load = int ((os.getloadavg)[0] * 100)

# Returns a color indicating system load.
def load_color
  sysload = (load)
  if $sysload > $XLOAD
    $ALERT
  elif $sysload > $MLOAD
    $Red
  elif $sysload > $SLOAD
    $BRed
  else
    $Green

# Returns a color according to free disk space in the current folder.
def disk_color
  here = (pwd)
  if not (os.access $here $os.W_OK)
    return $Red                # No 'write' privilege in the current folder.
  usage = shutil.disk_usage $here
  if $usage.total == 0
    return $Cyan               # Current folder is size '0' (like /proc, /sys etc).
  used = $usage.used * 100 // $usage.total
  if $used > 95
    $ALERT                     # Disk almost full (>95%).
  elif $used > 90
    $BRed                      # Free disk space almost gone.
  else
    $Green                     # Free disk space is ok.

# Returns a color according to running/stopped jobs.
def job_color
  buf = (io.StringIO)
  with (contextlib.redirect_stdout $buf)
    jobs
  listed = ($buf.getvalue)
  if " stopped " in $listed
    $BRed
  elif " running " in $listed
    $BCyan
  else
    ""

def status
  time = (datetime.datetime.now).strftime "%H:%M"
  user = $env.get USER "?"
  host = ((socket.gethostname).split ".")[0]
  [load disk job] = [(load_color) (disk_color) (job_color)]
  folder = (Path (pwd)).name
  print "${load}[${time}${NC} ${SU}${user}${NC}@${CNX}${host}${NC} ${disk}${folder}]${NC} ${job}>${NC}"

# (TIMEFORMAT, HISTIGNORE, HISTTIMEFORMAT and HISTCONTROL have no tessh
#  counterpart.)
export HOSTFILE=~/.hosts       # Put a list of remote hosts in ~/.hosts

#============================================================
#
#  FUNCTIONS (bash's aliases, too)
#
#============================================================

#-------------------
# Personal aliases, as functions
#-------------------

@command
def rm *args
  \rm -i *$args
@command
def cp *args
  \cp -i *$args
@command
def mv *args
  \mv -i *$args
# -> Prevents accidentally clobbering files.
@command
def mkdir *args
  \mkdir -p *$args

@command
def j *args
  jobs
# (h, history: tessh has no history command. which: tessh's own which
#  says where a program is. ..: a line that's only a folder goes there.)

# Pretty-print of some PATH variables:
def path
  print ($env.PATH.replace ":" "\n")
def libpath
  print (($env.get LD_LIBRARY_PATH "").replace ":" "\n")

@command
def du *args
  \du -kh *$args                # Makes a more readable output.
@command
def df *args
  \df -kTh *$args

#-------------------------------------------------------------
# The 'ls' family (this assumes you use a recent GNU ls).
#-------------------------------------------------------------
# Add colors for filetype and human-readable sizes by default on 'ls':
@command
def ls *args
  \ls -h --color *$args
@command
def lx *args
  ls -lXB *$args                # Sort by extension.
@command
def lk *args
  ls -lSr *$args                # Sort by size, biggest last.
@command
def lt *args
  ls -ltr *$args                # Sort by date, most recent last.
@command
def lc *args
  ls -ltcr *$args               # Sort by/show change time, most recent last.
@command
def lu *args
  ls -ltur *$args               # Sort by/show access time, most recent last.

# The ubiquitous 'll': directories first, with alphanumeric sorting:
@command
def ll *args
  ls -lv --group-directories-first *$args
@command
def lm *args
  ls -lv --group-directories-first *$args | less   # Pipe through 'more'
@command
def lr *args
  ll -R *$args                  # Recursive ls.
@command
def la *args
  ll -A *$args                  # Show hidden files.
@command
def tree *args
  \tree -Csuh *$args            # Nice alternative to 'recursive ls' ...

#-------------------------------------------------------------
# Tailoring 'less'
#-------------------------------------------------------------

@command
def more *args
  less *$args
export PAGER=less
export LESSCHARSET=latin1
export LESSOPEN='|/usr/bin/lesspipe.sh %s 2>&-'
                # Use this if lesspipe.sh exists.
export LESS='-i -N -w  -z-4 -g -e -M -X -F -R -P%t?f%f :stdin .?pb%pb\%:?lbLine %lb:?bbByte %bb:-...'

# LESS man page colors (makes Man pages more readable).
$env.LESS_TERMCAP_mb = "\x1b[01;31m"
$env.LESS_TERMCAP_md = "\x1b[01;31m"
$env.LESS_TERMCAP_me = "\x1b[0m"
$env.LESS_TERMCAP_se = "\x1b[0m"
$env.LESS_TERMCAP_so = "\x1b[01;44;33m"
$env.LESS_TERMCAP_ue = "\x1b[0m"
$env.LESS_TERMCAP_us = "\x1b[01;32m"

#-------------------------------------------------------------
# Spelling typos - highly personal and keyboard-dependent :-)
#-------------------------------------------------------------

@command
def xs *args
  cd *$args
@command
def vf *args
  cd *$args
@command
def moer *args
  less *$args
@command
def moew *args
  less *$args
@command
def kk *args
  ll *$args

#-------------------------------------------------------------
# A few fun ones
#-------------------------------------------------------------

# Adds some text in the terminal frame (if applicable).
def xtitle *words
  if ($env.get TERM "") =~ re'term|rxvt'
    print "\x1b]0;$(' '.join (map $str $words))\a" end=""

# Functions that use xtitle
@command
def top *args
  xtitle Processes on (socket.gethostname)
  \top *$args
@command
def make *args
  xtitle Making (Path (pwd)).name
  \make *$args

@command
def man *pages
  for i in $pages
    xtitle The ($i.rstrip ".0123456789") manual
    \man -a $i

#-------------------------------------------------------------
# Make the following commands run in background automatically:
#-------------------------------------------------------------

@command
def te *args                   # wrapper around xemacs/gnuserv
  if (gnuclient -batch -eval t 2>&-) == "t"
    gnuclient -q *$args
  else
    xemacs *$args &

@command
def soffice *args
  \soffice *$args &
@command
def firefox *args
  \firefox *$args &
@command
def xpdf *args
  \xpdf *$args &

#-------------------------------------------------------------
# File & strings related functions:
#-------------------------------------------------------------

# Find a file with a pattern in name:
@command
def ff *pattern
  find . -type f -iname "*$(' '.join $pattern)*" -ls

# Find a file with pattern $1 in name and Execute $2 on it:
@command
def fe pattern="" cmd="file"
  find . -type f -iname "*${pattern}*" -exec $cmd "{}" ";"

#  Find a pattern in a set of files and highlight them:
@command
def fstr *args
  args = list $args
  usage = "fstr: find string in files.\nUsage: fstr [-i] \"pattern\" [\"filename pattern\"] "
  mycase = []
  while $args and ($args[0].startswith "-")
    match ($args.pop 0)
      case "-i"
        mycase = ["-i"]
      case _
        print $usage
        return
  if not $args
    print $usage
    return
  names = if (len $args) > 1 then $args[1] else "*"
  find . -type f -name $names -print0 | xargs -0 grep -E --color=always -sn $mycase $args[0] 2>&- | less

def swap a b
  # Swap 2 filenames around, if they exist (from Uzi's bashrc).
  for f in [$a $b]
    if not (os.path.lexists $f)
      print "swap: $f does not exist"
      return 1
  tmp = "tmp.$(os.getpid)"
  os.rename $a $tmp
  os.rename $b $a
  os.rename $tmp $b

def extract file               # Handy Extract Program
  if not (os.path.isfile $file)
    print "'$file' is not a valid file!"
    return
  if ($file =~ *.tar.bz2) or ($file =~ *.tbz2)
    tar xvjf $file
  elif ($file =~ *.tar.gz) or ($file =~ *.tgz)
    tar xvzf $file
  elif $file =~ *.bz2
    bunzip2 $file
  elif $file =~ *.rar
    unrar x $file
  elif $file =~ *.gz
    gunzip $file
  elif $file =~ *.tar
    tar xvf $file
  elif $file =~ *.zip
    unzip $file
  elif $file =~ *.Z
    uncompress $file
  elif $file =~ *.7z
    7z x $file
  else
    print "'$file' cannot be extracted via >extract<"

# Creates an archive (*.tar.gz) from given directory.
def maketar dir
  dir = $dir.rstrip "/"
  tar cvzf "${dir}.tar.gz" "$dir/"

# Create a ZIP archive of a file or folder.
def makezip path
  name = $path.rstrip "/"
  zip -r "${name}.zip" $path

# Make your directories and files access rights sane.
@command
def sanitize *paths
  chmod -R u=rwX,g=rX,o= $paths

#-------------------------------------------------------------
# Process/system related functions:
#-------------------------------------------------------------

def ps_list *args
  # Your processes (pid, %cpu, %mem, time, command), as a list of lines.
  (ps *$args -u $env.USER -o pid,%cpu,%mem,bsdtime,command).lines

@command
def my_ps *args
  print ("\n".join (ps_list *$args))

def pp pattern=".*"
  for line in (ps_list f)
    if $line =~ (re.compile $pattern)
      print $line

def ask question
  answer = input "$question [y/n] "
  (.lower $answer).startswith "y"

@command
def killps *args                # kill by process name
  if not (1 <= (len $args) <= 2)
    print "Usage: killps [-SIGNAL] pattern"
    return
  sig = if (len $args) == 2 then $args[0] else "-TERM"   # default signal
  pattern = re.compile $args[-1]
  for line in (ps_list)[1:]
    [pid *_ pname] = $line.split
    if ($line =~ $pattern) and (ask "Kill process $pid <$pname> with signal $sig?")
      kill $sig $pid

@command
def mydf *filesystems           # Pretty-print of 'df' output.
  for fs in $filesystems        # Inspired by 'dfc' utility.
    if not (os.path.isdir $fs)
      print "$fs :No such file or directory"
      continue
    usage = shutil.disk_usage $fs
    nbstars = 20 * $usage.used // $usage.total
    bar = "*" * $nbstars + "-" * (20 - $nbstars)
    free = ((\df -Pkh $fs).lines[-1].split)[3]
    used = $usage.used * 100 // $usage.total
    print "$used% [$bar] ($free free on $fs)"

def my_ip                       # Get IP address on ethernet.
  out = ip -4 -o addr show scope global
  m = $out =~ re'inet (\S+)/'
  print (if $m then ($m.group 1) else "Not connected")

def ii                          # Get current host related info.
  print "\nYou are logged on ${BRed}$(socket.gethostname)"
  print "\n${BRed}Additional information:$NC " ; uname -a
  print "\n${BRed}Users logged on:$NC " ; print ("\n".join (sorted (set [($l.split)[0] for l in (w -hs).lines])))
  print "\n${BRed}Current date :$NC " ; date
  print "\n${BRed}Machine stats :$NC " ; uptime
  print "\n${BRed}Memory stats :$NC " ; free
  print "\n${BRed}Diskspace :$NC " ; mydf "/" ~            # ("/" quoted: a spaced / is division)
  print "\n${BRed}Local IP Address :$NC" ; my_ip
  print "\n${BRed}Open connections :$NC " ; ss -tuap
  print

#-------------------------------------------------------------
# Misc utilities:
#-------------------------------------------------------------

def repeat n f *args            # Repeat n times a function or command.
  for i in (range $n)
    f *$args

def corename *files             # Get name of app that created a corefile.
  for file in $files
    print "$file :" end=""
    gdb "--core=$file" --batch | head -1

# (PROGRAMMABLE COMPLETION SECTION: tessh has no programmable completion.
#  Tab completes names, programs, paths, variables and attributes.)
```

Some of the functions, tried on made-up files (with `ask` given a canned
answer, since it would read the terminal):

```tessh
import os
def swap a b
  # Swap 2 filenames around, if they exist (from Uzi's bashrc).
  for f in [$a $b]
    if not (os.path.lexists $f)
      print "swap: $f does not exist"
      return 1
  tmp = "tmp.$(os.getpid)"
  os.rename $a $tmp
  os.rename $b $a
  os.rename $tmp $b

def extract file               # Handy Extract Program (part of it)
  if not (os.path.isfile $file)
    print "'$file' is not a valid file!"
    return
  if ($file =~ *.tar.gz) or ($file =~ *.tgz)
    tar xvzf $file
  elif $file =~ *.gz
    gunzip $file
  else
    print "'$file' cannot be extracted via >extract<"

def maketar dir
  dir = $dir.rstrip "/"
  tar cvzf "${dir}.tar.gz" "$dir/"

def repeat n f *args
  for i in (range $n)
    f *$args

def ask question answer="y"    # (canned: really input "$question [y/n] ")
  print "$question [y/n] $answer"
  (.lower $answer).startswith "y"

# (Made-up files to try them on.)
print one >first ; print two >second ; mkdir -p notes/ ; print hi >notes/a.txt

swap first second
cat first second
swap first nowhere
maketar notes/ 2>&1
rm -r notes
extract notes.tar.gz
cat notes/a.txt
extract first
repeat 3 $print hello
if (ask "Kill process 1234 <sleep> with signal -TERM?")
  print "(kill -TERM 1234)"
```

```output
two
one
swap: nowhere does not exist
notes/
notes/a.txt
notes/
notes/a.txt
hi
'first' cannot be extracted via >extract<
hello
hello
hello
Kill process 1234 <sleep> with signal -TERM? [y/n] y
(kill -TERM 1234)
```

### Example M-2. `.bash_profile` file (`bash-profile.snippet`)

Andrzej Szelachowski's `.bash_profile` builds a colored prompt whose color
changes for root, and shows how to quote it so the test runs at each prompt
rather than once.

```bash no-run
# From Andrzej Szelachowski's ~/.bash_profile:


#  Note that a variable may require special treatment
#+ if it will be exported.

DARKGRAY='\e[1;30m'
LIGHTRED='\e[1;31m'
GREEN='\e[32m'
YELLOW='\e[1;33m'
LIGHTBLUE='\e[1;34m'
NC='\e[m'

PCT="\`if [[ \$EUID -eq 0 ]]; then T='$LIGHTRED' ; else T='$LIGHTBLUE'; fi; 
echo \$T \`"

#  For "literal" command substitution to be assigned to a variable,
#+ use escapes and double quotes:
#+       PCT="\` ... \`" . . .
#  Otherwise, the value of PCT variable is assigned only once,
#+ when the variable is exported/read from .bash_profile,
#+ and it will not change afterwards even if the user ID changes.


PS1="\n$GREEN[\w] \n$DARKGRAY($PCT\t$DARKGRAY)-($PCT\u$DARKGRAY)-($PCT\!
$DARKGRAY)$YELLOW-> $NC"

#  Escape a variables whose value changes:
#        if [[ \$EUID -eq 0 ]],
#  Otherwise the value of the EUID variable will be assigned only once,
#+ as above.

#  When a variable is assigned, it should be called escaped:
#+       echo \$T,
#  Otherwise the value of the T variable is taken from the moment the PCT 
#+ variable is exported/read from .bash_profile.
#  So, in this example it would be null.

#  When a variable's value contains a semicolon it should be strong quoted:
#        T='$LIGHTRED',
#  Otherwise, the semicolon will be interpreted as a command separator.


#  Variables PCT and PS1 can be merged into a new PS1 variable:

PS1="\`if [[ \$EUID -eq 0 ]]; then PCT='$LIGHTRED';
else PCT='$LIGHTBLUE'; fi; 
echo '\n$GREEN[\w] \n$DARKGRAY('\$PCT'\t$DARKGRAY)-\
('\$PCT'\u$DARKGRAY)-('\$PCT'\!$DARKGRAY)$YELLOW-> $NC'\`"

# The trick is to use strong quoting for parts of old PS1 variable.
```

All of it is about `PS1`, and tessh's prompt is fixed (**Gap:** no custom
prompt). The quoting problem it solves doesn't arise in tessh: code to run
later is a function, called when it's needed, so "evaluated once or every
time" is the difference between a value and a call. The prompt's content
as a function (one run, since it shows the time, user and history number):

```tessh
import os datetime getpass
DARKGRAY = "\x1b[1;30m" ; LIGHTRED = "\x1b[1;31m" ; GREEN = "\x1b[32m"
YELLOW = "\x1b[1;33m" ; LIGHTBLUE = "\x1b[1;34m" ; NC = "\x1b[m"

def prompt n
  # Evaluated each time it's called, so the root test is always current.
  pct = if (os.geteuid) == 0 then $LIGHTRED else $LIGHTBLUE
  folder = (pwd).replace ($env.get HOME "~") "~"
  time = (datetime.datetime.now).strftime "%H:%M:%S"
  user = (getpass.getuser)
  "\n${GREEN}[$folder] \n${DARKGRAY}(${pct}${time}${DARKGRAY})-(${pct}${user}${DARKGRAY})-(${pct}${n}${DARKGRAY})${YELLOW}-> ${NC}"

print ((prompt 42) | cat -v)
```

```text
(one run, in ~/projects as bozo)

^[[32m[~/projects] 
^[[1;30m(^[[1;34m11:00:33^[[1;30m)-(^[[1;34mbozo^[[1;30m)-(^[[1;34m42^[[1;30m)^[[1;33m-> ^[[m
```
