## Spammer identification
<!-- help: abs-is-spammer is_spammer spammer spam blacklist dnsbl dig -->

Michael S. Zick's `is_spammer.bash`, the longest script in the ABS Guide,
starts from a domain name or an IP address, follows the DNS (with `dig`)
to the names and addresses around it, and checks every address it found
against DNS blacklists. It's built as a library of general bash functions:
a stack of pending operations that drives the search, array editors that
take array *names* and `eval` them, a columnized printer, and wrappers
around `dig`. It needs the network, so neither version runs here; its
parts that don't are tried on made-up data after it.

### Example A-28. Spammer Identification (`is_spammer.bash`)

```bash no-run
#!/bin/bash

# $Id: is_spammer.bash,v 1.12.2.11 2004/10/01 21:42:33 mszick Exp $
# Above line is RCS info.

# The latest version of this script is available from http://www.morethan.org.
#
# Spammer-identification
# by Michael S. Zick
# Used in the ABS Guide with permission.



#######################################################
# Documentation
# See also "Quickstart" at end of script.
#######################################################

:<<-'__is_spammer_Doc_'

    Copyright (c) Michael S. Zick, 2004
    License: Unrestricted reuse in any form, for any purpose.
    Warranty: None -{Its a script; the user is on their own.}-

Impatient?
    Application code: goto "# # # Hunt the Spammer' program code # # #"
    Example output: ":<<-'_is_spammer_outputs_'"
    How to use: Enter script name without arguments.
                Or goto "Quickstart" at end of script.

Provides
    Given a domain name or IP(v4) address as input:

    Does an exhaustive set of queries to find the associated
    network resources (short of recursing into TLDs).

    Checks the IP(v4) addresses found against Blacklist
    nameservers.

    If found to be a blacklisted IP(v4) address,
    reports the blacklist text records.
    (Usually hyper-links to the specific report.)

Requires
    A working Internet connection.
    (Exercise: Add check and/or abort if not on-line when running script.)
    Bash with arrays (2.05b+).

    The external program 'dig' --
    a utility program provided with the 'bind' set of programs.
    Specifically, the version which is part of Bind series 9.x
    See: http://www.isc.org

    All usages of 'dig' are limited to wrapper functions,
    which may be rewritten as required.
    See: dig_wrappers.bash for details.
         ("Additional documentation" -- below)

Usage
    Script requires a single argument, which may be:
    1) A domain name;
    2) An IP(v4) address;
    3) A filename, with one name or address per line.

    Script accepts an optional second argument, which may be:
    1) A Blacklist server name;
    2) A filename, with one Blacklist server name per line.

    If the second argument is not provided, the script uses
    a built-in set of (free) Blacklist servers.

    See also, the Quickstart at the end of this script (after 'exit').

Return Codes
    0 - All OK
    1 - Script failure
    2 - Something is Blacklisted

Optional environment variables
    SPAMMER_TRACE
        If set to a writable file,
        script will log an execution flow trace.

    SPAMMER_DATA
        If set to a writable file, script will dump its
        discovered data in the form of GraphViz file.
        See: http://www.research.att.com/sw/tools/graphviz

    SPAMMER_LIMIT
        Limits the depth of resource tracing.

        Default is 2 levels.

        A setting of 0 (zero) means 'unlimited' . . .
          Caution: script might recurse the whole Internet!

        A limit of 1 or 2 is most useful when processing
        a file of domain names and addresses.
        A higher limit can be useful when hunting spam gangs.


Additional documentation
    Download the archived set of scripts
    explaining and illustrating the function contained within this script.
    http://bash.deta.in/mszick_clf.tar.bz2


Study notes
    This script uses a large number of functions.
    Nearly all general functions have their own example script.
    Each of the example scripts have tutorial level comments.

Scripting project
    Add support for IP(v6) addresses.
    IP(v6) addresses are recognized but not processed.

Advanced project
    Add the reverse lookup detail to the discovered information.

    Report the delegation chain and abuse contacts.

    Modify the GraphViz file output to include the
    newly discovered information.

__is_spammer_Doc_

#######################################################




#### Special IFS settings used for string parsing. ####

# Whitespace == :Space:Tab:Line Feed:Carriage Return:
WSP_IFS=$'\x20'$'\x09'$'\x0A'$'\x0D'

# No Whitespace == Line Feed:Carriage Return
NO_WSP=$'\x0A'$'\x0D'

# Field separator for dotted decimal IP addresses
ADR_IFS=${NO_WSP}'.'

# Array to dotted string conversions
DOT_IFS='.'${WSP_IFS}

# # # Pending operations stack machine # # #
# This set of functions described in func_stack.bash.
# (See "Additional documentation" above.)
# # #

# Global stack of pending operations.
declare -f -a _pending_
# Global sentinel for stack runners
declare -i _p_ctrl_
# Global holder for currently executing function
declare -f _pend_current_

# # # Debug version only - remove for regular use # # #
#
# The function stored in _pend_hook_ is called
# immediately before each pending function is
# evaluated.  Stack clean, _pend_current_ set.
#
# This thingy demonstrated in pend_hook.bash.
declare -f _pend_hook_
# # #

# The do nothing function
pend_dummy() { : ; }

# Clear and initialize the function stack.
pend_init() {
    unset _pending_[@]
    pend_func pend_stop_mark
    _pend_hook_='pend_dummy'  # Debug only.
}

# Discard the top function on the stack.
pend_pop() {
    if [ ${#_pending_[@]} -gt 0 ]
    then
        local -i _top_
        _top_=${#_pending_[@]}-1
        unset _pending_[$_top_]
    fi
}

# pend_func function_name [$(printf '%q\n' arguments)]
pend_func() {
    local IFS=${NO_WSP}
    set -f
    _pending_[${#_pending_[@]}]=$@
    set +f
}

# The function which stops the release:
pend_stop_mark() {
    _p_ctrl_=0
}

pend_mark() {
    pend_func pend_stop_mark
}

# Execute functions until 'pend_stop_mark' . . .
pend_release() {
    local -i _top_             # Declare _top_ as integer.
    _p_ctrl_=${#_pending_[@]}
    while [ ${_p_ctrl_} -gt 0 ]
    do
       _top_=${#_pending_[@]}-1
       _pend_current_=${_pending_[$_top_]}
       unset _pending_[$_top_]
       $_pend_hook_            # Debug only.
       eval $_pend_current_
    done
}

# Drop functions until 'pend_stop_mark' . . .
pend_drop() {
    local -i _top_
    local _pd_ctrl_=${#_pending_[@]}
    while [ ${_pd_ctrl_} -gt 0 ]
    do
       _top_=$_pd_ctrl_-1
       if [ "${_pending_[$_top_]}" == 'pend_stop_mark' ]
       then
           unset _pending_[$_top_]
           break
       else
           unset _pending_[$_top_]
           _pd_ctrl_=$_top_
       fi
    done
    if [ ${#_pending_[@]} -eq 0 ]
    then
        pend_func pend_stop_mark
    fi
}

#### Array editors ####

# This function described in edit_exact.bash.
# (See "Additional documentation," above.)
# edit_exact <excludes_array_name> <target_array_name>
edit_exact() {
    [ $# -eq 2 ] ||
    [ $# -eq 3 ] || return 1
    local -a _ee_Excludes
    local -a _ee_Target
    local _ee_x
    local _ee_t
    local IFS=${NO_WSP}
    set -f
    eval _ee_Excludes=\( \$\{$1\[@\]\} \)
    eval _ee_Target=\( \$\{$2\[@\]\} \)
    local _ee_len=${#_ee_Target[@]}     # Original length.
    local _ee_cnt=${#_ee_Excludes[@]}   # Exclude list length.
    [ ${_ee_len} -ne 0 ] || return 0    # Can't edit zero length.
    [ ${_ee_cnt} -ne 0 ] || return 0    # Can't edit zero length.
    for (( x = 0; x < ${_ee_cnt} ; x++ ))
    do
        _ee_x=${_ee_Excludes[$x]}
        for (( n = 0 ; n < ${_ee_len} ; n++ ))
        do
            _ee_t=${_ee_Target[$n]}
            if [ x"${_ee_t}" == x"${_ee_x}" ]
            then
                unset _ee_Target[$n]     # Discard match.
                [ $# -eq 2 ] && break    # If 2 arguments, then done.
            fi
        done
    done
    eval $2=\( \$\{_ee_Target\[@\]\} \)
    set +f
    return 0
}

# This function described in edit_by_glob.bash.
# edit_by_glob <excludes_array_name> <target_array_name>
edit_by_glob() {
    [ $# -eq 2 ] ||
    [ $# -eq 3 ] || return 1
    local -a _ebg_Excludes
    local -a _ebg_Target
    local _ebg_x
    local _ebg_t
    local IFS=${NO_WSP}
    set -f
    eval _ebg_Excludes=\( \$\{$1\[@\]\} \)
    eval _ebg_Target=\( \$\{$2\[@\]\} \)
    local _ebg_len=${#_ebg_Target[@]}
    local _ebg_cnt=${#_ebg_Excludes[@]}
    [ ${_ebg_len} -ne 0 ] || return 0
    [ ${_ebg_cnt} -ne 0 ] || return 0
    for (( x = 0; x < ${_ebg_cnt} ; x++ ))
    do
        _ebg_x=${_ebg_Excludes[$x]}
        for (( n = 0 ; n < ${_ebg_len} ; n++ ))
        do
            [ $# -eq 3 ] && _ebg_x=${_ebg_x}'*'  #  Do prefix edit
            if [ ${_ebg_Target[$n]:=} ]          #+ if defined & set.
            then
                _ebg_t=${_ebg_Target[$n]/#${_ebg_x}/}
                [ ${#_ebg_t} -eq 0 ] && unset _ebg_Target[$n]
            fi
        done
    done
    eval $2=\( \$\{_ebg_Target\[@\]\} \)
    set +f
    return 0
}

# This function described in unique_lines.bash.
# unique_lines <in_name> <out_name>
unique_lines() {
    [ $# -eq 2 ] || return 1
    local -a _ul_in
    local -a _ul_out
    local -i _ul_cnt
    local -i _ul_pos
    local _ul_tmp
    local IFS=${NO_WSP}
    set -f
    eval _ul_in=\( \$\{$1\[@\]\} \)
    _ul_cnt=${#_ul_in[@]}
    for (( _ul_pos = 0 ; _ul_pos < ${_ul_cnt} ; _ul_pos++ ))
    do
        if [ ${_ul_in[${_ul_pos}]:=} ]      # If defined & not empty
        then
            _ul_tmp=${_ul_in[${_ul_pos}]}
            _ul_out[${#_ul_out[@]}]=${_ul_tmp}
            for (( zap = _ul_pos ; zap < ${_ul_cnt} ; zap++ ))
            do
                [ ${_ul_in[${zap}]:=} ] &&
                [ 'x'${_ul_in[${zap}]} == 'x'${_ul_tmp} ] &&
                    unset _ul_in[${zap}]
            done
        fi
    done
    eval $2=\( \$\{_ul_out\[@\]\} \)
    set +f
    return 0
}

# This function described in char_convert.bash.
# to_lower <string>
to_lower() {
    [ $# -eq 1 ] || return 1
    local _tl_out
    _tl_out=${1//A/a}
    _tl_out=${_tl_out//B/b}
    _tl_out=${_tl_out//C/c}
    _tl_out=${_tl_out//D/d}
    _tl_out=${_tl_out//E/e}
    _tl_out=${_tl_out//F/f}
    _tl_out=${_tl_out//G/g}
    _tl_out=${_tl_out//H/h}
    _tl_out=${_tl_out//I/i}
    _tl_out=${_tl_out//J/j}
    _tl_out=${_tl_out//K/k}
    _tl_out=${_tl_out//L/l}
    _tl_out=${_tl_out//M/m}
    _tl_out=${_tl_out//N/n}
    _tl_out=${_tl_out//O/o}
    _tl_out=${_tl_out//P/p}
    _tl_out=${_tl_out//Q/q}
    _tl_out=${_tl_out//R/r}
    _tl_out=${_tl_out//S/s}
    _tl_out=${_tl_out//T/t}
    _tl_out=${_tl_out//U/u}
    _tl_out=${_tl_out//V/v}
    _tl_out=${_tl_out//W/w}
    _tl_out=${_tl_out//X/x}
    _tl_out=${_tl_out//Y/y}
    _tl_out=${_tl_out//Z/z}
    echo ${_tl_out}
    return 0
}

#### Application helper functions ####

# Not everybody uses dots as separators (APNIC, for example).
# This function described in to_dot.bash
# to_dot <string>
to_dot() {
    [ $# -eq 1 ] || return 1
    echo ${1//[#|@|%]/.}
    return 0
}

# This function described in is_number.bash.
# is_number <input>
is_number() {
    [ "$#" -eq 1 ]    || return 1  # is blank?
    [ x"$1" == 'x0' ] && return 0  # is zero?
    local -i tst
    let tst=$1 2>/dev/null         # else is numeric!
    return $?
}

# This function described in is_address.bash.
# is_address <input>
is_address() {
    [ $# -eq 1 ] || return 1    # Blank ==> false
    local -a _ia_input
    local IFS=${ADR_IFS}
    _ia_input=( $1 )
    if  [ ${#_ia_input[@]} -eq 4 ]  &&
        is_number ${_ia_input[0]}   &&
        is_number ${_ia_input[1]}   &&
        is_number ${_ia_input[2]}   &&
        is_number ${_ia_input[3]}   &&
        [ ${_ia_input[0]} -lt 256 ] &&
        [ ${_ia_input[1]} -lt 256 ] &&
        [ ${_ia_input[2]} -lt 256 ] &&
        [ ${_ia_input[3]} -lt 256 ]
    then
        return 0
    else
        return 1
    fi
}

#  This function described in split_ip.bash.
#  split_ip <IP_address>
#+ <array_name_norm> [<array_name_rev>]
split_ip() {
    [ $# -eq 3 ] ||              #  Either three
    [ $# -eq 2 ] || return 1     #+ or two arguments
    local -a _si_input
    local IFS=${ADR_IFS}
    _si_input=( $1 )
    IFS=${WSP_IFS}
    eval $2=\(\ \$\{_si_input\[@\]\}\ \)
    if [ $# -eq 3 ]
    then
        # Build query order array.
        local -a _dns_ip
        _dns_ip[0]=${_si_input[3]}
        _dns_ip[1]=${_si_input[2]}
        _dns_ip[2]=${_si_input[1]}
        _dns_ip[3]=${_si_input[0]}
        eval $3=\(\ \$\{_dns_ip\[@\]\}\ \)
    fi
    return 0
}

# This function described in dot_array.bash.
# dot_array <array_name>
dot_array() {
    [ $# -eq 1 ] || return 1     # Single argument required.
    local -a _da_input
    eval _da_input=\(\ \$\{$1\[@\]\}\ \)
    local IFS=${DOT_IFS}
    local _da_output=${_da_input[@]}
    IFS=${WSP_IFS}
    echo ${_da_output}
    return 0
}

# This function described in file_to_array.bash
# file_to_array <file_name> <line_array_name>
file_to_array() {
    [ $# -eq 2 ] || return 1  # Two arguments required.
    local IFS=${NO_WSP}
    local -a _fta_tmp_
    _fta_tmp_=( $(cat $1) )
    eval $2=\( \$\{_fta_tmp_\[@\]\} \)
    return 0
}

#  Columnized print of an array of multi-field strings.
#  col_print <array_name> <min_space> <
#+ tab_stop [tab_stops]>
col_print() {
    [ $# -gt 2 ] || return 0
    local -a _cp_inp
    local -a _cp_spc
    local -a _cp_line
    local _cp_min
    local _cp_mcnt
    local _cp_pos
    local _cp_cnt
    local _cp_tab
    local -i _cp
    local -i _cpf
    local _cp_fld
    # WARNING: FOLLOWING LINE NOT BLANK -- IT IS QUOTED SPACES.
    local _cp_max='                                                            '
    set -f
    local IFS=${NO_WSP}
    eval _cp_inp=\(\ \$\{$1\[@\]\}\ \)
    [ ${#_cp_inp[@]} -gt 0 ] || return 0 # Empty is easy.
    _cp_mcnt=$2
    _cp_min=${_cp_max:1:${_cp_mcnt}}
    shift
    shift
    _cp_cnt=$#
    for (( _cp = 0 ; _cp < _cp_cnt ; _cp++ ))
    do
        _cp_spc[${#_cp_spc[@]}]="${_cp_max:2:$1}" #"
        shift
    done
    _cp_cnt=${#_cp_inp[@]}
    for (( _cp = 0 ; _cp < _cp_cnt ; _cp++ ))
    do
        _cp_pos=1
        IFS=${NO_WSP}$'\x20'
        _cp_line=( ${_cp_inp[${_cp}]} )
        IFS=${NO_WSP}
        for (( _cpf = 0 ; _cpf < ${#_cp_line[@]} ; _cpf++ ))
        do
            _cp_tab=${_cp_spc[${_cpf}]:${_cp_pos}}
            if [ ${#_cp_tab} -lt ${_cp_mcnt} ]
            then
                _cp_tab="${_cp_min}"
            fi
            echo -n "${_cp_tab}"
            (( _cp_pos = ${_cp_pos} + ${#_cp_tab} ))
            _cp_fld="${_cp_line[${_cpf}]}"
            echo -n ${_cp_fld}
            (( _cp_pos = ${_cp_pos} + ${#_cp_fld} ))
        done
        echo
    done
    set +f
    return 0
}

# # # # 'Hunt the Spammer' data flow # # # #

# Application return code
declare -i _hs_RC

# Original input, from which IP addresses are removed
# After which, domain names to check
declare -a uc_name

# Original input IP addresses are moved here
# After which, IP addresses to check
declare -a uc_address

# Names against which address expansion run
# Ready for name detail lookup
declare -a chk_name

# Addresses against which name expansion run
# Ready for address detail lookup
declare -a chk_address

#  Recursion is depth-first-by-name.
#  The expand_input_address maintains this list
#+ to prohibit looking up addresses twice during
#+ domain name recursion.
declare -a been_there_addr
been_there_addr=( '127.0.0.1' ) # Whitelist localhost

# Names which we have checked (or given up on)
declare -a known_name

# Addresses which we have checked (or given up on)
declare -a known_address

#  List of zero or more Blacklist servers to check.
#  Each 'known_address' will be checked against each server,
#+ with negative replies and failures suppressed.
declare -a list_server

# Indirection limit - set to zero == no limit
indirect=${SPAMMER_LIMIT:=2}

# # # # 'Hunt the Spammer' information output data # # # #

# Any domain name may have multiple IP addresses.
# Any IP address may have multiple domain names.
# Therefore, track unique address-name pairs.
declare -a known_pair
declare -a reverse_pair

#  In addition to the data flow variables; known_address
#+ known_name and list_server, the following are output to the
#+ external graphics interface file.

# Authority chain, parent -> SOA fields.
declare -a auth_chain

# Reference chain, parent name -> child name
declare -a ref_chain

# DNS chain - domain name -> address
declare -a name_address

# Name and service pairs - domain name -> service
declare -a name_srvc

# Name and resource pairs - domain name -> Resource Record
declare -a name_resource

# Parent and Child pairs - parent name -> child name
# This MAY NOT be the same as the ref_chain followed!
declare -a parent_child

# Address and Blacklist hit pairs - address->server
declare -a address_hits

# Dump interface file data
declare -f _dot_dump
_dot_dump=pend_dummy   # Initially a no-op

#  Data dump is enabled by setting the environment variable SPAMMER_DATA
#+ to the name of a writable file.
declare _dot_file

# Helper function for the dump-to-dot-file function
# dump_to_dot <array_name> <prefix>
dump_to_dot() {
    local -a _dda_tmp
    local -i _dda_cnt
    local _dda_form='    '${2}'%04u %s\n'
    local IFS=${NO_WSP}
    eval _dda_tmp=\(\ \$\{$1\[@\]\}\ \)
    _dda_cnt=${#_dda_tmp[@]}
    if [ ${_dda_cnt} -gt 0 ]
    then
        for (( _dda = 0 ; _dda < _dda_cnt ; _dda++ ))
        do
            printf "${_dda_form}" \
                   "${_dda}" "${_dda_tmp[${_dda}]}" >>${_dot_file}
        done
    fi
}

# Which will also set _dot_dump to this function . . .
dump_dot() {
    local -i _dd_cnt
    echo '# Data vintage: '$(date -R) >${_dot_file}
    echo '# ABS Guide: is_spammer.bash; v2, 2004-msz' >>${_dot_file}
    echo >>${_dot_file}
    echo 'digraph G {' >>${_dot_file}

    if [ ${#known_name[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known domain name nodes' >>${_dot_file}
        _dd_cnt=${#known_name[@]}
        for (( _dd = 0 ; _dd < _dd_cnt ; _dd++ ))
        do
            printf '    N%04u [label="%s"] ;\n' \
                   "${_dd}" "${known_name[${_dd}]}" >>${_dot_file}
        done
    fi

    if [ ${#known_address[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known address nodes' >>${_dot_file}
        _dd_cnt=${#known_address[@]}
        for (( _dd = 0 ; _dd < _dd_cnt ; _dd++ ))
        do
            printf '    A%04u [label="%s"] ;\n' \
                   "${_dd}" "${known_address[${_dd}]}" >>${_dot_file}
        done
    fi

    echo                                   >>${_dot_file}
    echo '/*'                              >>${_dot_file}
    echo ' * Known relationships :: User conversion to'  >>${_dot_file}
    echo ' * graphic form by hand or program required.'  >>${_dot_file}
    echo ' *'                              >>${_dot_file}

    if [ ${#auth_chain[@]} -gt 0 ]
    then
      echo >>${_dot_file}
      echo '# Authority ref. edges followed & field source.' >>${_dot_file}
        dump_to_dot auth_chain AC
    fi

    if [ ${#ref_chain[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Name ref. edges followed and field source.' >>${_dot_file}
        dump_to_dot ref_chain RC
    fi

    if [ ${#name_address[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known name->address edges' >>${_dot_file}
        dump_to_dot name_address NA
    fi

    if [ ${#name_srvc[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known name->service edges' >>${_dot_file}
        dump_to_dot name_srvc NS
    fi

    if [ ${#name_resource[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known name->resource edges' >>${_dot_file}
        dump_to_dot name_resource NR
    fi

    if [ ${#parent_child[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known parent->child edges' >>${_dot_file}
        dump_to_dot parent_child PC
    fi

    if [ ${#list_server[@]} -gt 0 ]
    then
        echo >>${_dot_file}
        echo '# Known Blacklist nodes' >>${_dot_file}
        _dd_cnt=${#list_server[@]}
        for (( _dd = 0 ; _dd < _dd_cnt ; _dd++ ))
        do
            printf '    LS%04u [label="%s"] ;\n' \
                   "${_dd}" "${list_server[${_dd}]}" >>${_dot_file}
        done
    fi

    unique_lines address_hits address_hits
    if [ ${#address_hits[@]} -gt 0 ]
    then
      echo >>${_dot_file}
      echo '# Known address->Blacklist_hit edges' >>${_dot_file}
      echo '# CAUTION: dig warnings can trigger false hits.' >>${_dot_file}
       dump_to_dot address_hits AH
    fi
    echo          >>${_dot_file}
    echo ' *'     >>${_dot_file}
    echo ' * That is a lot of relationships. Happy graphing.' >>${_dot_file}
    echo ' */'    >>${_dot_file}
    echo '}'      >>${_dot_file}
    return 0
}

# # # # 'Hunt the Spammer' execution flow # # # #

#  Execution trace is enabled by setting the
#+ environment variable SPAMMER_TRACE to the name of a writable file.
declare -a _trace_log
declare _log_file

# Function to fill the trace log
trace_logger() {
    _trace_log[${#_trace_log[@]}]=${_pend_current_}
}

# Dump trace log to file function variable.
declare -f _log_dump
_log_dump=pend_dummy   # Initially a no-op.

# Dump the trace log to a file.
dump_log() {
    local -i _dl_cnt
    _dl_cnt=${#_trace_log[@]}
    for (( _dl = 0 ; _dl < _dl_cnt ; _dl++ ))
    do
        echo ${_trace_log[${_dl}]} >> ${_log_file}
    done
    _dl_cnt=${#_pending_[@]}
    if [ ${_dl_cnt} -gt 0 ]
    then
        _dl_cnt=${_dl_cnt}-1
        echo '# # # Operations stack not empty # # #' >> ${_log_file}
        for (( _dl = ${_dl_cnt} ; _dl >= 0 ; _dl-- ))
        do
            echo ${_pending_[${_dl}]} >> ${_log_file}
        done
    fi
}

# # # Utility program 'dig' wrappers # # #
#
#  These wrappers are derived from the
#+ examples shown in dig_wrappers.bash.
#
#  The major difference is these return
#+ their results as a list in an array.
#
#  See dig_wrappers.bash for details and
#+ use that script to develop any changes.
#
# # #

# Short form answer: 'dig' parses answer.

# Forward lookup :: Name -> Address
# short_fwd <domain_name> <array_name>
short_fwd() {
    local -a _sf_reply
    local -i _sf_rc
    local -i _sf_cnt
    IFS=${NO_WSP}
echo -n '.'
# echo 'sfwd: '${1}
  _sf_reply=( $(dig +short ${1} -c in -t a 2>/dev/null) )
  _sf_rc=$?
  if [ ${_sf_rc} -ne 0 ]
  then
    _trace_log[${#_trace_log[@]}]='## Lookup error '${_sf_rc}' on '${1}' ##'
# [ ${_sf_rc} -ne 9 ] && pend_drop
        return ${_sf_rc}
    else
        # Some versions of 'dig' return warnings on stdout.
        _sf_cnt=${#_sf_reply[@]}
        for (( _sf = 0 ; _sf < ${_sf_cnt} ; _sf++ ))
        do
            [ 'x'${_sf_reply[${_sf}]:0:2} == 'x;;' ] &&
                unset _sf_reply[${_sf}]
        done
        eval $2=\( \$\{_sf_reply\[@\]\} \)
    fi
    return 0
}

# Reverse lookup :: Address -> Name
# short_rev <ip_address> <array_name>
short_rev() {
    local -a _sr_reply
    local -i _sr_rc
    local -i _sr_cnt
    IFS=${NO_WSP}
echo -n '.'
# echo 'srev: '${1}
  _sr_reply=( $(dig +short -x ${1} 2>/dev/null) )
  _sr_rc=$?
  if [ ${_sr_rc} -ne 0 ]
  then
    _trace_log[${#_trace_log[@]}]='## Lookup error '${_sr_rc}' on '${1}' ##'
# [ ${_sr_rc} -ne 9 ] && pend_drop
        return ${_sr_rc}
    else
        # Some versions of 'dig' return warnings on stdout.
        _sr_cnt=${#_sr_reply[@]}
        for (( _sr = 0 ; _sr < ${_sr_cnt} ; _sr++ ))
        do
            [ 'x'${_sr_reply[${_sr}]:0:2} == 'x;;' ] &&
                unset _sr_reply[${_sr}]
        done
        eval $2=\( \$\{_sr_reply\[@\]\} \)
    fi
    return 0
}

# Special format lookup used to query blacklist servers.
# short_text <ip_address> <array_name>
short_text() {
    local -a _st_reply
    local -i _st_rc
    local -i _st_cnt
    IFS=${NO_WSP}
# echo 'stxt: '${1}
  _st_reply=( $(dig +short ${1} -c in -t txt 2>/dev/null) )
  _st_rc=$?
  if [ ${_st_rc} -ne 0 ]
  then
    _trace_log[${#_trace_log[@]}]='##Text lookup error '${_st_rc}' on '${1}'##'
# [ ${_st_rc} -ne 9 ] && pend_drop
        return ${_st_rc}
    else
        # Some versions of 'dig' return warnings on stdout.
        _st_cnt=${#_st_reply[@]}
        for (( _st = 0 ; _st < ${#_st_cnt} ; _st++ ))
        do
            [ 'x'${_st_reply[${_st}]:0:2} == 'x;;' ] &&
                unset _st_reply[${_st}]
        done
        eval $2=\( \$\{_st_reply\[@\]\} \)
    fi
    return 0
}

# The long forms, a.k.a., the parse it yourself versions

# RFC 2782   Service lookups
# dig +noall +nofail +answer _ldap._tcp.openldap.org -t srv
# _<service>._<protocol>.<domain_name>
# _ldap._tcp.openldap.org. 3600   IN     SRV    0 0 389 ldap.openldap.org.
# domain TTL Class SRV Priority Weight Port Target

# Forward lookup :: Name -> poor man's zone transfer
# long_fwd <domain_name> <array_name>
long_fwd() {
    local -a _lf_reply
    local -i _lf_rc
    local -i _lf_cnt
    IFS=${NO_WSP}
echo -n ':'
# echo 'lfwd: '${1}
  _lf_reply=( $(
     dig +noall +nofail +answer +authority +additional \
         ${1} -t soa ${1} -t mx ${1} -t any 2>/dev/null) )
  _lf_rc=$?
  if [ ${_lf_rc} -ne 0 ]
  then
    _trace_log[${#_trace_log[@]}]='# Zone lookup err '${_lf_rc}' on '${1}' #'
# [ ${_lf_rc} -ne 9 ] && pend_drop
        return ${_lf_rc}
    else
        # Some versions of 'dig' return warnings on stdout.
        _lf_cnt=${#_lf_reply[@]}
        for (( _lf = 0 ; _lf < ${_lf_cnt} ; _lf++ ))
        do
            [ 'x'${_lf_reply[${_lf}]:0:2} == 'x;;' ] &&
                unset _lf_reply[${_lf}]
        done
        eval $2=\( \$\{_lf_reply\[@\]\} \)
    fi
    return 0
}
#  The reverse lookup domain name corresponding to the IPv6 address:
#      4321:0:1:2:3:4:567:89ab
#  would be (nibble, I.E: Hexdigit) reversed:
#  b.a.9.8.7.6.5.0.4.0.0.0.3.0.0.0.2.0.0.0.1.0.0.0.0.0.0.0.1.2.3.4.IP6.ARPA.

# Reverse lookup :: Address -> poor man's delegation chain
# long_rev <rev_ip_address> <array_name>
long_rev() {
    local -a _lr_reply
    local -i _lr_rc
    local -i _lr_cnt
    local _lr_dns
    _lr_dns=${1}'.in-addr.arpa.'
    IFS=${NO_WSP}
echo -n ':'
# echo 'lrev: '${1}
  _lr_reply=( $(
       dig +noall +nofail +answer +authority +additional \
           ${_lr_dns} -t soa ${_lr_dns} -t any 2>/dev/null) )
  _lr_rc=$?
  if [ ${_lr_rc} -ne 0 ]
  then
    _trace_log[${#_trace_log[@]}]='# Deleg lkp error '${_lr_rc}' on '${1}' #'
# [ ${_lr_rc} -ne 9 ] && pend_drop
        return ${_lr_rc}
    else
        # Some versions of 'dig' return warnings on stdout.
        _lr_cnt=${#_lr_reply[@]}
        for (( _lr = 0 ; _lr < ${_lr_cnt} ; _lr++ ))
        do
            [ 'x'${_lr_reply[${_lr}]:0:2} == 'x;;' ] &&
                unset _lr_reply[${_lr}]
        done
        eval $2=\( \$\{_lr_reply\[@\]\} \)
    fi
    return 0
}

# # # Application specific functions # # #

# Mung a possible name; suppresses root and TLDs.
# name_fixup <string>
name_fixup(){
    local -a _nf_tmp
    local -i _nf_end
    local _nf_str
    local IFS
    _nf_str=$(to_lower ${1})
    _nf_str=$(to_dot ${_nf_str})
    _nf_end=${#_nf_str}-1
    [ ${_nf_str:${_nf_end}} != '.' ] &&
        _nf_str=${_nf_str}'.'
    IFS=${ADR_IFS}
    _nf_tmp=( ${_nf_str} )
    IFS=${WSP_IFS}
    _nf_end=${#_nf_tmp[@]}
    case ${_nf_end} in
    0) # No dots, only dots.
        echo
        return 1
    ;;
    1) # Only a TLD.
        echo
        return 1
    ;;
    2) # Maybe okay.
       echo ${_nf_str}
       return 0
       # Needs a lookup table?
       if [ ${#_nf_tmp[1]} -eq 2 ]
       then # Country coded TLD.
           echo
           return 1
       else
           echo ${_nf_str}
           return 0
       fi
    ;;
    esac
    echo ${_nf_str}
    return 0
}

# Grope and mung original input(s).
split_input() {
    [ ${#uc_name[@]} -gt 0 ] || return 0
    local -i _si_cnt
    local -i _si_len
    local _si_str
    unique_lines uc_name uc_name
    _si_cnt=${#uc_name[@]}
    for (( _si = 0 ; _si < _si_cnt ; _si++ ))
    do
        _si_str=${uc_name[$_si]}
        if is_address ${_si_str}
        then
            uc_address[${#uc_address[@]}]=${_si_str}
            unset uc_name[$_si]
        else
            if ! uc_name[$_si]=$(name_fixup ${_si_str})
            then
                unset ucname[$_si]
            fi
        fi
    done
  uc_name=( ${uc_name[@]} )
  _si_cnt=${#uc_name[@]}
  _trace_log[${#_trace_log[@]}]='#Input '${_si_cnt}' unchkd name input(s).#'
  _si_cnt=${#uc_address[@]}
  _trace_log[${#_trace_log[@]}]='#Input '${_si_cnt}' unchkd addr input(s).#'
    return 0
}

# # # Discovery functions -- recursively interlocked by external data # # #
# # # The leading 'if list is empty; return 0' in each is required. # # #

# Recursion limiter
# limit_chk() <next_level>
limit_chk() {
    local -i _lc_lmt
    # Check indirection limit.
    if [ ${indirect} -eq 0 ] || [ $# -eq 0 ]
    then
        # The 'do-forever' choice
        echo 1                 # Any value will do.
        return 0               # OK to continue.
    else
        # Limiting is in effect.
        if [ ${indirect} -lt ${1} ]
        then
            echo ${1}          # Whatever.
            return 1           # Stop here.
        else
            _lc_lmt=${1}+1     # Bump the given limit.
            echo ${_lc_lmt}    # Echo it.
            return 0           # OK to continue.
        fi
    fi
}

# For each name in uc_name:
#     Move name to chk_name.
#     Add addresses to uc_address.
#     Pend expand_input_address.
#     Repeat until nothing new found.
# expand_input_name <indirection_limit>
expand_input_name() {
    [ ${#uc_name[@]} -gt 0 ] || return 0
    local -a _ein_addr
    local -a _ein_new
    local -i _ucn_cnt
    local -i _ein_cnt
    local _ein_tst
    _ucn_cnt=${#uc_name[@]}

    if  ! _ein_cnt=$(limit_chk ${1})
    then
        return 0
    fi

    for (( _ein = 0 ; _ein < _ucn_cnt ; _ein++ ))
    do
        if short_fwd ${uc_name[${_ein}]} _ein_new
        then
          for (( _ein_cnt = 0 ; _ein_cnt < ${#_ein_new[@]}; _ein_cnt++ ))
          do
              _ein_tst=${_ein_new[${_ein_cnt}]}
              if is_address ${_ein_tst}
              then
                  _ein_addr[${#_ein_addr[@]}]=${_ein_tst}
              fi
    done
        fi
    done
    unique_lines _ein_addr _ein_addr     # Scrub duplicates.
    edit_exact chk_address _ein_addr     # Scrub pending detail.
    edit_exact known_address _ein_addr   # Scrub already detailed.
 if [ ${#_ein_addr[@]} -gt 0 ]        # Anything new?
 then
   uc_address=( ${uc_address[@]} ${_ein_addr[@]} )
   pend_func expand_input_address ${1}
   _trace_log[${#_trace_log[@]}]='#Add '${#_ein_addr[@]}' unchkd addr inp.#'
    fi
    edit_exact chk_name uc_name          # Scrub pending detail.
    edit_exact known_name uc_name        # Scrub already detailed.
    if [ ${#uc_name[@]} -gt 0 ]
    then
        chk_name=( ${chk_name[@]} ${uc_name[@]}  )
        pend_func detail_each_name ${1}
    fi
    unset uc_name[@]
    return 0
}

# For each address in uc_address:
#     Move address to chk_address.
#     Add names to uc_name.
#     Pend expand_input_name.
#     Repeat until nothing new found.
# expand_input_address <indirection_limit>
expand_input_address() {
    [ ${#uc_address[@]} -gt 0 ] || return 0
    local -a _eia_addr
    local -a _eia_name
    local -a _eia_new
    local -i _uca_cnt
    local -i _eia_cnt
    local _eia_tst
    unique_lines uc_address _eia_addr
    unset uc_address[@]
    edit_exact been_there_addr _eia_addr
    _uca_cnt=${#_eia_addr[@]}
    [ ${_uca_cnt} -gt 0 ] &&
        been_there_addr=( ${been_there_addr[@]} ${_eia_addr[@]} )

    for (( _eia = 0 ; _eia < _uca_cnt ; _eia++ ))
     do
       if short_rev ${_eia_addr[${_eia}]} _eia_new
       then
         for (( _eia_cnt = 0 ; _eia_cnt < ${#_eia_new[@]} ; _eia_cnt++ ))
         do
           _eia_tst=${_eia_new[${_eia_cnt}]}
           if _eia_tst=$(name_fixup ${_eia_tst})
           then
             _eia_name[${#_eia_name[@]}]=${_eia_tst}
       fi
     done
           fi
    done
    unique_lines _eia_name _eia_name     # Scrub duplicates.
    edit_exact chk_name _eia_name        # Scrub pending detail.
    edit_exact known_name _eia_name      # Scrub already detailed.
 if [ ${#_eia_name[@]} -gt 0 ]        # Anything new?
 then
   uc_name=( ${uc_name[@]} ${_eia_name[@]} )
   pend_func expand_input_name ${1}
   _trace_log[${#_trace_log[@]}]='#Add '${#_eia_name[@]}' unchkd name inp.#'
    fi
    edit_exact chk_address _eia_addr     # Scrub pending detail.
    edit_exact known_address _eia_addr   # Scrub already detailed.
    if [ ${#_eia_addr[@]} -gt 0 ]        # Anything new?
    then
        chk_address=( ${chk_address[@]} ${_eia_addr[@]} )
        pend_func detail_each_address ${1}
    fi
    return 0
}

# The parse-it-yourself zone reply.
# The input is the chk_name list.
# detail_each_name <indirection_limit>
detail_each_name() {
    [ ${#chk_name[@]} -gt 0 ] || return 0
    local -a _den_chk       # Names to check
    local -a _den_name      # Names found here
    local -a _den_address   # Addresses found here
    local -a _den_pair      # Pairs found here
    local -a _den_rev       # Reverse pairs found here
    local -a _den_tmp       # Line being parsed
    local -a _den_auth      # SOA contact being parsed
    local -a _den_new       # The zone reply
    local -a _den_pc        # Parent-Child gets big fast
    local -a _den_ref       # So does reference chain
    local -a _den_nr        # Name-Resource can be big
    local -a _den_na        # Name-Address
    local -a _den_ns        # Name-Service
    local -a _den_achn      # Chain of Authority
    local -i _den_cnt       # Count of names to detail
    local -i _den_lmt       # Indirection limit
    local _den_who          # Named being processed
    local _den_rec          # Record type being processed
    local _den_cont         # Contact domain
    local _den_str          # Fixed up name string
    local _den_str2         # Fixed up reverse
    local IFS=${WSP_IFS}

    # Local, unique copy of names to check
    unique_lines chk_name _den_chk
    unset chk_name[@]       # Done with globals.

    # Less any names already known
    edit_exact known_name _den_chk
    _den_cnt=${#_den_chk[@]}

    # If anything left, add to known_name.
    [ ${_den_cnt} -gt 0 ] &&
        known_name=( ${known_name[@]} ${_den_chk[@]} )

    # for the list of (previously) unknown names . . .
    for (( _den = 0 ; _den < _den_cnt ; _den++ ))
    do
        _den_who=${_den_chk[${_den}]}
        if long_fwd ${_den_who} _den_new
        then
            unique_lines _den_new _den_new
            if [ ${#_den_new[@]} -eq 0 ]
            then
                _den_pair[${#_den_pair[@]}]='0.0.0.0 '${_den_who}
            fi

            # Parse each line in the reply.
            for (( _line = 0 ; _line < ${#_den_new[@]} ; _line++ ))
            do
                IFS=${NO_WSP}$'\x09'$'\x20'
                _den_tmp=( ${_den_new[${_line}]} )
                IFS=${WSP_IFS}
              # If usable record and not a warning message . . .
              if [ ${#_den_tmp[@]} -gt 4 ] && [ 'x'${_den_tmp[0]} != 'x;;' ]
              then
                    _den_rec=${_den_tmp[3]}
                    _den_nr[${#_den_nr[@]}]=${_den_who}' '${_den_rec}
                    # Begin at RFC1033 (+++)
                    case ${_den_rec} in

#<name> [<ttl>]  [<class>] SOA <origin> <person>
                    SOA) # Start Of Authority
    if _den_str=$(name_fixup ${_den_tmp[0]})
    then
      _den_name[${#_den_name[@]}]=${_den_str}
      _den_achn[${#_den_achn[@]}]=${_den_who}' '${_den_str}' SOA'
      # SOA origin -- domain name of master zone record
      if _den_str2=$(name_fixup ${_den_tmp[4]})
      then
        _den_name[${#_den_name[@]}]=${_den_str2}
        _den_achn[${#_den_achn[@]}]=${_den_who}' '${_den_str2}' SOA.O'
      fi
      # Responsible party e-mail address (possibly bogus).
      # Possibility of first.last@domain.name ignored.
      set -f
      if _den_str2=$(name_fixup ${_den_tmp[5]})
      then
        IFS=${ADR_IFS}
        _den_auth=( ${_den_str2} )
        IFS=${WSP_IFS}
        if [ ${#_den_auth[@]} -gt 2 ]
        then
          _den_cont=${_den_auth[1]}
          for (( _auth = 2 ; _auth < ${#_den_auth[@]} ; _auth++ ))
          do
            _den_cont=${_den_cont}'.'${_den_auth[${_auth}]}
          done
          _den_name[${#_den_name[@]}]=${_den_cont}'.'
          _den_achn[${#_den_achn[@]}]=${_den_who}' '${_den_cont}'. SOA.C'
                                fi
        fi
        set +f
                        fi
                    ;;


      A) # IP(v4) Address Record
      if _den_str=$(name_fixup ${_den_tmp[0]})
      then
        _den_name[${#_den_name[@]}]=${_den_str}
        _den_pair[${#_den_pair[@]}]=${_den_tmp[4]}' '${_den_str}
        _den_na[${#_den_na[@]}]=${_den_str}' '${_den_tmp[4]}
        _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' A'
      else
        _den_pair[${#_den_pair[@]}]=${_den_tmp[4]}' unknown.domain'
        _den_na[${#_den_na[@]}]='unknown.domain '${_den_tmp[4]}
        _den_ref[${#_den_ref[@]}]=${_den_who}' unknown.domain A'
      fi
      _den_address[${#_den_address[@]}]=${_den_tmp[4]}
      _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_tmp[4]}
             ;;

             NS) # Name Server Record
             # Domain name being serviced (may be other than current)
               if _den_str=$(name_fixup ${_den_tmp[0]})
                 then
                   _den_name[${#_den_name[@]}]=${_den_str}
                   _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' NS'

             # Domain name of service provider
             if _den_str2=$(name_fixup ${_den_tmp[4]})
             then
               _den_name[${#_den_name[@]}]=${_den_str2}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str2}' NSH'
               _den_ns[${#_den_ns[@]}]=${_den_str2}' NS'
               _den_pc[${#_den_pc[@]}]=${_den_str}' '${_den_str2}
              fi
               fi
                    ;;

             MX) # Mail Server Record
                 # Domain name being serviced (wildcards not handled here)
             if _den_str=$(name_fixup ${_den_tmp[0]})
             then
               _den_name[${#_den_name[@]}]=${_den_str}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' MX'
             fi
             # Domain name of service provider
             if _den_str=$(name_fixup ${_den_tmp[5]})
             then
               _den_name[${#_den_name[@]}]=${_den_str}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' MXH'
               _den_ns[${#_den_ns[@]}]=${_den_str}' MX'
               _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_str}
             fi
                    ;;

             PTR) # Reverse address record
                  # Special name
             if _den_str=$(name_fixup ${_den_tmp[0]})
             then
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' PTR'
               # Host name (not a CNAME)
               if _den_str2=$(name_fixup ${_den_tmp[4]})
               then
                 _den_rev[${#_den_rev[@]}]=${_den_str}' '${_den_str2}
                 _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str2}' PTRH'
                 _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_str}
               fi
             fi
                    ;;

             AAAA) # IP(v6) Address Record
             if _den_str=$(name_fixup ${_den_tmp[0]})
             then
               _den_name[${#_den_name[@]}]=${_den_str}
               _den_pair[${#_den_pair[@]}]=${_den_tmp[4]}' '${_den_str}
               _den_na[${#_den_na[@]}]=${_den_str}' '${_den_tmp[4]}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' AAAA'
               else
                 _den_pair[${#_den_pair[@]}]=${_den_tmp[4]}' unknown.domain'
                 _den_na[${#_den_na[@]}]='unknown.domain '${_den_tmp[4]}
                 _den_ref[${#_den_ref[@]}]=${_den_who}' unknown.domain'
               fi
               # No processing for IPv6 addresses
               _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_tmp[4]}
                    ;;

             CNAME) # Alias name record
                    # Nickname
             if _den_str=$(name_fixup ${_den_tmp[0]})
             then
               _den_name[${#_den_name[@]}]=${_den_str}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' CNAME'
               _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_str}
             fi
                    # Hostname
             if _den_str=$(name_fixup ${_den_tmp[4]})
             then
               _den_name[${#_den_name[@]}]=${_den_str}
               _den_ref[${#_den_ref[@]}]=${_den_who}' '${_den_str}' CHOST'
               _den_pc[${#_den_pc[@]}]=${_den_who}' '${_den_str}
             fi
                    ;;
#            TXT)
#            ;;
                    esac
                fi
            done
        else # Lookup error == 'A' record 'unknown address'
            _den_pair[${#_den_pair[@]}]='0.0.0.0 '${_den_who}
        fi
    done

    # Control dot array growth.
    unique_lines _den_achn _den_achn      # Works best, all the same.
    edit_exact auth_chain _den_achn       # Works best, unique items.
    if [ ${#_den_achn[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        auth_chain=( ${auth_chain[@]} ${_den_achn[@]} )
        IFS=${WSP_IFS}
    fi

    unique_lines _den_ref _den_ref      # Works best, all the same.
    edit_exact ref_chain _den_ref       # Works best, unique items.
    if [ ${#_den_ref[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        ref_chain=( ${ref_chain[@]} ${_den_ref[@]} )
        IFS=${WSP_IFS}
    fi

    unique_lines _den_na _den_na
    edit_exact name_address _den_na
    if [ ${#_den_na[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        name_address=( ${name_address[@]} ${_den_na[@]} )
        IFS=${WSP_IFS}
    fi

    unique_lines _den_ns _den_ns
    edit_exact name_srvc _den_ns
    if [ ${#_den_ns[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        name_srvc=( ${name_srvc[@]} ${_den_ns[@]} )
        IFS=${WSP_IFS}
    fi

    unique_lines _den_nr _den_nr
    edit_exact name_resource _den_nr
    if [ ${#_den_nr[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        name_resource=( ${name_resource[@]} ${_den_nr[@]} )
        IFS=${WSP_IFS}
    fi

    unique_lines _den_pc _den_pc
    edit_exact parent_child _den_pc
    if [ ${#_den_pc[@]} -gt 0 ]
    then
        IFS=${NO_WSP}
        parent_child=( ${parent_child[@]} ${_den_pc[@]} )
        IFS=${WSP_IFS}
    fi

    # Update list known_pair (Address and Name).
    unique_lines _den_pair _den_pair
    edit_exact known_pair _den_pair
    if [ ${#_den_pair[@]} -gt 0 ]  # Anything new?
    then
        IFS=${NO_WSP}
        known_pair=( ${known_pair[@]} ${_den_pair[@]} )
        IFS=${WSP_IFS}
    fi

    # Update list of reverse pairs.
    unique_lines _den_rev _den_rev
    edit_exact reverse_pair _den_rev
    if [ ${#_den_rev[@]} -gt 0 ]   # Anything new?
    then
        IFS=${NO_WSP}
        reverse_pair=( ${reverse_pair[@]} ${_den_rev[@]} )
        IFS=${WSP_IFS}
    fi

    # Check indirection limit -- give up if reached.
    if ! _den_lmt=$(limit_chk ${1})
    then
        return 0
    fi

# Execution engine is LIFO. Order of pend operations is important.
# Did we define any new addresses?
unique_lines _den_address _den_address    # Scrub duplicates.
edit_exact known_address _den_address     # Scrub already processed.
edit_exact un_address _den_address        # Scrub already waiting.
if [ ${#_den_address[@]} -gt 0 ]          # Anything new?
then
  uc_address=( ${uc_address[@]} ${_den_address[@]} )
  pend_func expand_input_address ${_den_lmt}
  _trace_log[${#_trace_log[@]}]='# Add '${#_den_address[@]}' unchkd addr. #'
    fi

# Did we find any new names?
unique_lines _den_name _den_name          # Scrub duplicates.
edit_exact known_name _den_name           # Scrub already processed.
edit_exact uc_name _den_name              # Scrub already waiting.
if [ ${#_den_name[@]} -gt 0 ]             # Anything new?
then
  uc_name=( ${uc_name[@]} ${_den_name[@]} )
  pend_func expand_input_name ${_den_lmt}
  _trace_log[${#_trace_log[@]}]='#Added '${#_den_name[@]}' unchkd name#'
    fi
    return 0
}

# The parse-it-yourself delegation reply
# Input is the chk_address list.
# detail_each_address <indirection_limit>
detail_each_address() {
    [ ${#chk_address[@]} -gt 0 ] || return 0
    unique_lines chk_address chk_address
    edit_exact known_address chk_address
    if [ ${#chk_address[@]} -gt 0 ]
    then
        known_address=( ${known_address[@]} ${chk_address[@]} )
        unset chk_address[@]
    fi
    return 0
}

# # # Application specific output functions # # #

# Pretty print the known pairs.
report_pairs() {
    echo
    echo 'Known network pairs.'
    col_print known_pair 2 5 30

    if [ ${#auth_chain[@]} -gt 0 ]
    then
        echo
        echo 'Known chain of authority.'
        col_print auth_chain 2 5 30 55
    fi

    if [ ${#reverse_pair[@]} -gt 0 ]
    then
        echo
        echo 'Known reverse pairs.'
        col_print reverse_pair 2 5 55
    fi
    return 0
}

# Check an address against the list of blacklist servers.
# A good place to capture for GraphViz: address->status(server(reports))
# check_lists <ip_address>
check_lists() {
    [ $# -eq 1 ] || return 1
    local -a _cl_fwd_addr
    local -a _cl_rev_addr
    local -a _cl_reply
    local -i _cl_rc
    local -i _ls_cnt
    local _cl_dns_addr
    local _cl_lkup

    split_ip ${1} _cl_fwd_addr _cl_rev_addr
    _cl_dns_addr=$(dot_array _cl_rev_addr)'.'
    _ls_cnt=${#list_server[@]}
    echo '    Checking address '${1}
    for (( _cl = 0 ; _cl < _ls_cnt ; _cl++ ))
    do
      _cl_lkup=${_cl_dns_addr}${list_server[${_cl}]}
      if short_text ${_cl_lkup} _cl_reply
      then
        if [ ${#_cl_reply[@]} -gt 0 ]
        then
          echo '        Records from '${list_server[${_cl}]}
          address_hits[${#address_hits[@]}]=${1}' '${list_server[${_cl}]}
          _hs_RC=2
          for (( _clr = 0 ; _clr < ${#_cl_reply[@]} ; _clr++ ))
          do
            echo '            '${_cl_reply[${_clr}]}
          done
        fi
      fi
    done
    return 0
}

# # # The usual application glue # # #

# Who did it?
credits() {
   echo
   echo 'Advanced Bash Scripting Guide: is_spammer.bash, v2, 2004-msz'
}

# How to use it?
# (See also, "Quickstart" at end of script.)
usage() {
    cat <<-'_usage_statement_'
    The script is_spammer.bash requires either one or two arguments.

    arg 1) May be one of:
        a) A domain name
        b) An IPv4 address
        c) The name of a file with any mix of names
           and addresses, one per line.

    arg 2) May be one of:
        a) A Blacklist server domain name
        b) The name of a file with Blacklist server
           domain names, one per line.
        c) If not present, a default list of (free)
           Blacklist servers is used.
        d) If a filename of an empty, readable, file
           is given,
           Blacklist server lookup is disabled.

    All script output is written to stdout.

    Return codes: 0 -> All OK, 1 -> Script failure,
                  2 -> Something is Blacklisted.

    Requires the external program 'dig' from the 'bind-9'
    set of DNS programs.  See: http://www.isc.org

    The domain name lookup depth limit defaults to 2 levels.
    Set the environment variable SPAMMER_LIMIT to change.
    SPAMMER_LIMIT=0 means 'unlimited'

    Limit may also be set on the command-line.
    If arg#1 is an integer, the limit is set to that value
    and then the above argument rules are applied.

    Setting the environment variable 'SPAMMER_DATA' to a filename
    will cause the script to write a GraphViz graphic file.

    For the development version;
    Setting the environment variable 'SPAMMER_TRACE' to a filename
    will cause the execution engine to log a function call trace.

_usage_statement_
}

# The default list of Blacklist servers:
# Many choices, see: http://www.spews.org/lists.html

declare -a default_servers
# See: http://www.spamhaus.org (Conservative, well maintained)
default_servers[0]='sbl-xbl.spamhaus.org'
# See: http://ordb.org (Open mail relays)
default_servers[1]='relays.ordb.org'
# See: http://www.spamcop.net/ (You can report spammers here)
default_servers[2]='bl.spamcop.net'
# See: http://www.spews.org (An 'early detect' system)
default_servers[3]='l2.spews.dnsbl.sorbs.net'
# See: http://www.dnsbl.us.sorbs.net/using.shtml
default_servers[4]='dnsbl.sorbs.net'
# See: http://dsbl.org/usage (Various mail relay lists)
default_servers[5]='list.dsbl.org'
default_servers[6]='multihop.dsbl.org'
default_servers[7]='unconfirmed.dsbl.org'

# User input argument #1
setup_input() {
    if [ -e ${1} ] && [ -r ${1} ]  # Name of readable file
    then
        file_to_array ${1} uc_name
        echo 'Using filename >'${1}'< as input.'
    else
        if is_address ${1}          # IP address?
        then
            uc_address=( ${1} )
            echo 'Starting with address >'${1}'<'
        else                       # Must be a name.
            uc_name=( ${1} )
            echo 'Starting with domain name >'${1}'<'
        fi
    fi
    return 0
}

# User input argument #2
setup_servers() {
    if [ -e ${1} ] && [ -r ${1} ]  # Name of a readable file
    then
        file_to_array ${1} list_server
        echo 'Using filename >'${1}'< as blacklist server list.'
    else
        list_server=( ${1} )
        echo 'Using blacklist server >'${1}'<'
    fi
    return 0
}

# User environment variable SPAMMER_TRACE
live_log_die() {
    if [ ${SPAMMER_TRACE:=} ]    # Wants trace log?
    then
        if [ ! -e ${SPAMMER_TRACE} ]
        then
            if ! touch ${SPAMMER_TRACE} 2>/dev/null
            then
                pend_func echo $(printf '%q\n' \
                'Unable to create log file >'${SPAMMER_TRACE}'<')
                pend_release
                exit 1
            fi
            _log_file=${SPAMMER_TRACE}
            _pend_hook_=trace_logger
            _log_dump=dump_log
        else
            if [ ! -w ${SPAMMER_TRACE} ]
            then
                pend_func echo $(printf '%q\n' \
                'Unable to write log file >'${SPAMMER_TRACE}'<')
                pend_release
                exit 1
            fi
            _log_file=${SPAMMER_TRACE}
            echo '' > ${_log_file}
            _pend_hook_=trace_logger
            _log_dump=dump_log
        fi
    fi
    return 0
}

# User environment variable SPAMMER_DATA
data_capture() {
    if [ ${SPAMMER_DATA:=} ]    # Wants a data dump?
    then
        if [ ! -e ${SPAMMER_DATA} ]
        then
            if ! touch ${SPAMMER_DATA} 2>/dev/null
            then
                pend_func echo $(printf '%q]n' \
                'Unable to create data output file >'${SPAMMER_DATA}'<')
                pend_release
                exit 1
            fi
            _dot_file=${SPAMMER_DATA}
            _dot_dump=dump_dot
        else
            if [ ! -w ${SPAMMER_DATA} ]
            then
                pend_func echo $(printf '%q\n' \
                'Unable to write data output file >'${SPAMMER_DATA}'<')
                pend_release
                exit 1
            fi
            _dot_file=${SPAMMER_DATA}
            _dot_dump=dump_dot
        fi
    fi
    return 0
}

# Grope user specified arguments.
do_user_args() {
    if [ $# -gt 0 ] && is_number $1
    then
        indirect=$1
        shift
    fi

    case $# in                     # Did user treat us well?
        1)
            if ! setup_input $1    # Needs error checking.
            then
                pend_release
                $_log_dump
                exit 1
            fi
            list_server=( ${default_servers[@]} )
            _list_cnt=${#list_server[@]}
            echo 'Using default blacklist server list.'
            echo 'Search depth limit: '${indirect}
            ;;
        2)
            if ! setup_input $1    # Needs error checking.
            then
                pend_release
                $_log_dump
                exit 1
            fi
            if ! setup_servers $2  # Needs error checking.
            then
                pend_release
                $_log_dump
                exit 1
            fi
            echo 'Search depth limit: '${indirect}
            ;;
        *)
            pend_func usage
            pend_release
            $_log_dump
            exit 1
            ;;
    esac
    return 0
}

# A general purpose debug tool.
# list_array <array_name>
list_array() {
    [ $# -eq 1 ] || return 1  # One argument required.

    local -a _la_lines
    set -f
    local IFS=${NO_WSP}
    eval _la_lines=\(\ \$\{$1\[@\]\}\ \)
    echo
    echo "Element count "${#_la_lines[@]}" array "${1}
    local _ln_cnt=${#_la_lines[@]}

    for (( _i = 0; _i < ${_ln_cnt}; _i++ ))
    do
        echo 'Element '$_i' >'${_la_lines[$_i]}'<'
    done
    set +f
    return 0
}

# # # 'Hunt the Spammer' program code # # #
pend_init                               # Ready stack engine.
pend_func credits                       # Last thing to print.

# # # Deal with user # # #
live_log_die                            # Setup debug trace log.
data_capture                            # Setup data capture file.
echo
do_user_args $@

# # # Haven't exited yet - There is some hope # # #
# Discovery group - Execution engine is LIFO - pend
# in reverse order of execution.
_hs_RC=0                                # Hunt the Spammer return code
pend_mark
    pend_func report_pairs              # Report name-address pairs.

    # The two detail_* are mutually recursive functions.
    # They also pend expand_* functions as required.
    # These two (the last of ???) exit the recursion.
    pend_func detail_each_address       # Get all resources of addresses.
    pend_func detail_each_name          # Get all resources of names.

    #  The two expand_* are mutually recursive functions,
    #+ which pend additional detail_* functions as required.
    pend_func expand_input_address 1    # Expand input names by address.
    pend_func expand_input_name 1       # #xpand input addresses by name.

    # Start with a unique set of names and addresses.
    pend_func unique_lines uc_address uc_address
    pend_func unique_lines uc_name uc_name

    # Separate mixed input of names and addresses.
    pend_func split_input
pend_release

# # # Pairs reported -- Unique list of IP addresses found
echo
_ip_cnt=${#known_address[@]}
if [ ${#list_server[@]} -eq 0 ]
then
    echo 'Blacklist server list empty, none checked.'
else
    if [ ${_ip_cnt} -eq 0 ]
    then
        echo 'Known address list empty, none checked.'
    else
        _ip_cnt=${_ip_cnt}-1   # Start at top.
        echo 'Checking Blacklist servers.'
        for (( _ip = _ip_cnt ; _ip >= 0 ; _ip-- ))
        do
          pend_func check_lists $( printf '%q\n' ${known_address[$_ip]} )
        done
    fi
fi
pend_release
$_dot_dump                   # Graphics file dump
$_log_dump                   # Execution trace
echo


##############################
# Example output from script #
##############################
:<<-'_is_spammer_outputs_'

./is_spammer.bash 0 web4.alojamentos7.com

Starting with domain name >web4.alojamentos7.com<
Using default blacklist server list.
Search depth limit: 0
.:....::::...:::...:::.......::..::...:::.......::
Known network pairs.
    66.98.208.97             web4.alojamentos7.com.
    66.98.208.97             ns1.alojamentos7.com.
    69.56.202.147            ns2.alojamentos.ws.
    66.98.208.97             alojamentos7.com.
    66.98.208.97             web.alojamentos7.com.
    69.56.202.146            ns1.alojamentos.ws.
    69.56.202.146            alojamentos.ws.
    66.235.180.113           ns1.alojamentos.org.
    66.235.181.192           ns2.alojamentos.org.
    66.235.180.113           alojamentos.org.
    66.235.180.113           web6.alojamentos.org.
    216.234.234.30           ns1.theplanet.com.
    12.96.160.115            ns2.theplanet.com.
    216.185.111.52           mail1.theplanet.com.
    69.56.141.4              spooling.theplanet.com.
    216.185.111.40           theplanet.com.
    216.185.111.40           www.theplanet.com.
    216.185.111.52           mail.theplanet.com.

Checking Blacklist servers.
  Checking address 66.98.208.97
      Records from dnsbl.sorbs.net
  "Spam Received See: http://www.dnsbl.sorbs.net/lookup.shtml?66.98.208.97"
    Checking address 69.56.202.147
    Checking address 69.56.202.146
    Checking address 66.235.180.113
    Checking address 66.235.181.192
    Checking address 216.185.111.40
    Checking address 216.234.234.30
    Checking address 12.96.160.115
    Checking address 216.185.111.52
    Checking address 69.56.141.4

Advanced Bash Scripting Guide: is_spammer.bash, v2, 2004-msz

_is_spammer_outputs_

exit ${_hs_RC}

####################################################
#  The script ignores everything from here on down #
#+ because of the 'exit' command, just above.      #
####################################################



Quickstart
==========

 Prerequisites

  Bash version 2.05b or 3.00 (bash --version)
  A version of Bash which supports arrays. Array 
  support is included by default Bash configurations.

  'dig,' version 9.x.x (dig $HOSTNAME, see first line of output)
  A version of dig which supports the +short options. 
  See: dig_wrappers.bash for details.


 Optional Prerequisites

  'named,' a local DNS caching program. Any flavor will do.
  Do twice: dig $HOSTNAME 
  Check near bottom of output for: SERVER: 127.0.0.1#53
  That means you have one running.


 Optional Graphics Support

  'date,' a standard *nix thing. (date -R)

  dot Program to convert graphic description file to a 
  diagram. (dot -V)
  A part of the Graph-Viz set of programs.
  See: [http://www.research.att.com/sw/tools/graphviz||GraphViz]

  'dotty,' a visual editor for graphic description files.
  Also a part of the Graph-Viz set of programs.




 Quick Start

In the same directory as the is_spammer.bash script; 
Do: ./is_spammer.bash

 Usage Details

1. Blacklist server choices.

  (a) To use default, built-in list: Do nothing.

  (b) To use your own list: 

    i. Create a file with a single Blacklist server 
       domain name per line.

    ii. Provide that filename as the last argument to 
        the script.

  (c) To use a single Blacklist server: Last argument 
      to the script.

  (d) To disable Blacklist lookups:

    i. Create an empty file (touch spammer.nul)
       Your choice of filename.

    ii. Provide the filename of that empty file as the 
        last argument to the script.

2. Search depth limit.

  (a) To use the default value of 2: Do nothing.

  (b) To set a different limit: 
      A limit of 0 means: no limit.

    i. export SPAMMER_LIMIT=1
       or whatever limit you want.

    ii. OR provide the desired limit as the first 
       argument to the script.

3. Optional execution trace log.

  (a) To use the default setting of no log output: Do nothing.

  (b) To write an execution trace log:
      export SPAMMER_TRACE=spammer.log
      or whatever filename you want.

4. Optional graphic description file.

  (a) To use the default setting of no graphic file: Do nothing.

  (b) To write a Graph-Viz graphic description file:
      export SPAMMER_DATA=spammer.dot
      or whatever filename you want.

5. Where to start the search.

  (a) Starting with a single domain name:

    i. Without a command-line search limit: First 
       argument to script.

    ii. With a command-line search limit: Second 
        argument to script.

  (b) Starting with a single IP address:

    i. Without a command-line search limit: First 
       argument to script.

    ii. With a command-line search limit: Second 
        argument to script.

  (c) Starting with (mixed) multiple name(s) and/or address(es):
      Create a file with one name or address per line.
      Your choice of filename.

    i. Without a command-line search limit: Filename as 
       first argument to script.

    ii. With a command-line search limit: Filename as 
        second argument to script.

6. What to do with the display output.

  (a) To view display output on screen: Do nothing.

  (b) To save display output to a file: Redirect stdout to a filename.

  (c) To discard display output: Redirect stdout to /dev/null.

7. Temporary end of decision making. 
   press RETURN 
   wait (optionally, watch the dots and colons).

8. Optionally check the return code.

  (a) Return code 0: All OK

  (b) Return code 1: Script setup failure

  (c) Return code 2: Something was blacklisted.

9. Where is my graph (diagram)?

The script does not directly produce a graph (diagram). 
It only produces a graphic description file. You can 
process the graphic descriptor file that was output 
with the 'dot' program.

Until you edit that descriptor file, to describe the 
relationships you want shown, all that you will get is 
a bunch of labeled name and address nodes.

All of the script's discovered relationships are within 
a comment block in the graphic descriptor file, each 
with a descriptive heading.

The editing required to draw a line between a pair of 
nodes from the information in the descriptor file may 
be done with a text editor. 

Given these lines somewhere in the descriptor file:

# Known domain name nodes

N0000 [label="guardproof.info."] ;

N0002 [label="third.guardproof.info."] ;



# Known address nodes

A0000 [label="61.141.32.197"] ;



/*

# Known name->address edges

NA0000 third.guardproof.info. 61.141.32.197



# Known parent->child edges

PC0000 guardproof.info. third.guardproof.info.

 */

Turn that into the following lines by substituting node 
identifiers into the relationships:

# Known domain name nodes

N0000 [label="guardproof.info."] ;

N0002 [label="third.guardproof.info."] ;



# Known address nodes

A0000 [label="61.141.32.197"] ;



# PC0000 guardproof.info. third.guardproof.info.

N0000->N0002 ;



# NA0000 third.guardproof.info. 61.141.32.197

N0002->A0000 ;



/*

# Known name->address edges

NA0000 third.guardproof.info. 61.141.32.197



# Known parent->child edges

PC0000 guardproof.info. third.guardproof.info.

 */

Process that with the 'dot' program, and you have your 
first network diagram.

In addition to the conventional graphic edges, the 
descriptor file includes similar format pair-data that 
describes services, zone records (sub-graphs?), 
blacklisted addresses, and other things which might be 
interesting to include in your graph. This additional 
information could be displayed as different node 
shapes, colors, line sizes, etc.

The descriptor file can also be read and edited by a 
Bash script (of course). You should be able to find 
most of the functions required within the 
"is_spammer.bash" script.

# End Quickstart.



Additional Note
========== ====

Michael Zick points out that there is a "makeviz.bash" interactive
Web site at rediris.es. Can't give the full URL, since this is not
a publically accessible site.
```

The tessh port keeps the script's design, functions and names, and most
of the machinery becomes Python's. Arrays are lists, handed to the
functions themselves and edited in place (`$target[:] = …`, `.remove`,
`.extend`) rather than by name through `eval`; `unique_lines` is
`dict.fromkeys`; the `IFS` juggling that splits and joins strings is
`.split` and `.join`. The stack holds functions and their arguments, not
command lines to `eval`, so the `printf '%q'` quoting goes. A `dig`
wrapper gets the command's `Output`, whose `.ok`, `.code` and `.lines`
replace the `$?` tests and the splitting, and gives back a list, or
`None` when `dig` failed. `match` sorts the record types, and the eight
"unique, less what's known, then add" passes at the end of
`detail_each_name` share one function, `add_new`. The script and its
Quickstart are as long as before, but the code is about a third of the
bash's.

```tessh no-run
#!/usr/bin/env tessh

# $Id: is_spammer.bash,v 1.12.2.11 2004/10/01 21:42:33 mszick Exp $
# (Ported to tessh from the ABS Guide's is_spammer.bash.)
#
# Spammer-identification
# by Michael S. Zick
# Used in the ABS Guide with permission.

#######################################################
# Documentation
# See also "Quickstart" at end of script.
#######################################################

#   Copyright (c) Michael S. Zick, 2004
#   License: Unrestricted reuse in any form, for any purpose.
#   Warranty: None -{Its a script; the user is on their own.}-
#
# Impatient?
#   Application code: goto "# # # Hunt the Spammer' program code # # #"
#   Example output: "Example output from script", after the code.
#   How to use: Enter script name without arguments.
#               Or goto "Quickstart" at end of script.
#
# Provides
#   Given a domain name or IP(v4) address as input:
#   Does an exhaustive set of queries to find the associated
#   network resources (short of recursing into TLDs).
#   Checks the IP(v4) addresses found against Blacklist nameservers.
#   If found to be a blacklisted IP(v4) address,
#   reports the blacklist text records.
#   (Usually hyper-links to the specific report.)
#
# Requires
#   A working Internet connection.
#   The external program 'dig' -- part of the 'bind' set of programs
#   (Bind series 9.x). See: http://www.isc.org
#   All usages of 'dig' are limited to wrapper functions,
#   which may be rewritten as required.
#
# Usage
#   Script requires a single argument, which may be:
#   1) A domain name;
#   2) An IP(v4) address;
#   3) A filename, with one name or address per line.
#   Script accepts an optional second argument, which may be:
#   1) A Blacklist server name;
#   2) A filename, with one Blacklist server name per line.
#   If the second argument is not provided, the script uses
#   a built-in set of (free) Blacklist servers.
#
# Return Codes
#   0 - All OK
#   1 - Script failure
#   2 - Something is Blacklisted
#
# Optional environment variables
#   SPAMMER_TRACE  If set to a writable file,
#                  script will log an execution flow trace.
#   SPAMMER_DATA   If set to a writable file, script will dump its
#                  discovered data in the form of GraphViz file.
#   SPAMMER_LIMIT  Limits the depth of resource tracing.
#                  Default is 2 levels. A setting of 0 (zero) means
#                  'unlimited' . . . Caution: script might recurse
#                  the whole Internet!
#
# Scripting project
#   Add support for IP(v6) addresses.
#   IP(v6) addresses are recognized but not processed.
#
# Advanced project
#   Add the reverse lookup detail to the discovered information.
#   Report the delegation chain and abuse contacts.
#   Modify the GraphViz file output to include the
#   newly discovered information.

import os fnmatch contextlib

# (bash's special IFS settings for splitting and joining strings are
# .split and .join here.)

# # # Pending operations stack machine # # #
# The stack holds [function arguments] pairs: the functions themselves,
# not command lines to eval.

_pending_ = []           # Global stack of pending operations.
_p_ctrl_ = 0             # Global sentinel for stack runners.
_pend_current_ = None    # Global holder for currently executing function.

# The do nothing function
def pend_dummy
  pass

# # # Debug version only - remove for regular use # # #
# The function in _pend_hook_ is called immediately before each pending
# function is run. Stack clean, _pend_current_ set.
_pend_hook_ = $pend_dummy

# Clear and initialize the function stack.
def pend_init
  global _pend_hook_
  $_pending_.clear
  pend_func $pend_stop_mark
  _pend_hook_ = $pend_dummy      # Debug only.

# Discard the top function on the stack.
def pend_pop
  if $_pending_
    $_pending_.pop

# pend_func function [arguments]
def pend_func f *args
  $_pending_.append [$f $args]

# The function which stops the release:
def pend_stop_mark
  global _p_ctrl_
  _p_ctrl_ = 0

def pend_mark
  pend_func $pend_stop_mark

# Run functions until 'pend_stop_mark' . . .
def pend_release
  global _p_ctrl_ _pend_current_
  _p_ctrl_ = len $_pending_
  while $_p_ctrl_ > 0 and $_pending_
    _pend_current_ = ($_pending_.pop)
    _pend_hook_                    # Debug only.
    [f args] = $_pend_current_
    f *$args

# Drop functions until 'pend_stop_mark' . . .
def pend_drop
  while $_pending_
    [f _] = ($_pending_.pop)
    if $f is $pend_stop_mark
      break
  if not $_pending_
    pend_func $pend_stop_mark

# A pending operation as text, for the trace log: "name arg …"
def op_text op
  [f args] = $op
  " ".join ([$f.__name__] + [(str $a) for a in $args])

#### List editors ####
# The bash versions take array names and eval; these take the lists, and
# change the target in place.

# edit_exact <excludes> <target> [every]
# Remove each exclude's first match from the target (with every=True,
# all its matches).
def edit_exact excludes target every=False
  for x in $excludes
    if $every
      $target[:] = [$t for t in $target if $t != $x]
    elif $x in $target
      $target.remove $x

# edit_by_glob <excludes> <target> [prefix]
# Remove the items an exclude pattern matches (with prefix=True,
# the items that start with a match).
def edit_by_glob excludes target prefix=False
  for x in $excludes
    pattern = if $prefix then $x + "*" else $x
    $target[:] = [$t for t in $target if not (fnmatch.fnmatchcase $t $pattern)]

# unique_lines <in> [<into>]
# The non-empty items, each once, in order: given back, and put in
# the list `into` too if there is one.
def unique_lines lines into=None
  out = list (dict.fromkeys ($l for l in $lines if $l))
  if $into is not None
    $into[:] = $out
  $out

# to_lower <string>  (the bash version replaces A to Z one letter at a time)
def to_lower s = ($s.lower)

#### Application helper functions ####

# Not everybody uses dots as separators (APNIC, for example).
# to_dot <string>
def to_dot s = re'[#|@%]'.sub "." $s

# is_number <input>
def is_number s
  try
    int $s
    True
  except ValueError
    False

# is_address <input>
def is_address s
  parts = $s.split "."
  (len $parts) == 4 and (all ((is_number $p) and (int $p) < 256 for p in $parts))

# split_ip <IP_address> : [the parts, the parts in DNS query order]
def split_ip ip
  parts = $ip.split "."
  [$parts (list (reversed $parts))]

# dot_array <list>
def dot_array parts = ".".join $parts

# file_to_array <file_name> : its lines, less the empty ones
def file_to_array name
  with (open $name) as f
    [$l for l in (($f.read).splitlines) if $l]

# Columnized print of a list of multi-field strings.
# col_print <lines> <min_space> <tab_stop [tab_stops]>
def col_print lines min_space *stops
  if not $stops
    return
  for line in $lines
    pos = 1
    out = ""
    for [i field] in (enumerate ($line.split))
      width = if $i < (len $stops) then $stops[$i] - $pos else 0
      if $width < $min_space
        width = $min_space
      out += " " * $width + $field
      pos += $width + (len $field)
    print $out

# # # # 'Hunt the Spammer' data flow # # # #

_hs_RC = 0               # Application return code

uc_name = []             # Original input, from which IP addresses are removed.
                         # After which, domain names to check.
uc_address = []          # Original input IP addresses are moved here.
                         # After which, IP addresses to check.
chk_name = []            # Names against which address expansion run.
                         # Ready for name detail lookup.
chk_address = []         # Addresses against which name expansion run.
                         # Ready for address detail lookup.

# Recursion is depth-first-by-name. The expand_input_address maintains
# this list to prohibit looking up addresses twice during domain name
# recursion.
been_there_addr = ['127.0.0.1']   # Whitelist localhost

known_name = []          # Names which we have checked (or given up on)
known_address = []       # Addresses which we have checked (or given up on)

# List of zero or more Blacklist servers to check. Each 'known_address'
# will be checked against each server, with negative replies and failures
# suppressed.
list_server = []

# Indirection limit - set to zero == no limit
indirect = int (($env.get SPAMMER_LIMIT) or 2)

# # # # 'Hunt the Spammer' information output data # # # #

# Any domain name may have multiple IP addresses.
# Any IP address may have multiple domain names.
# Therefore, track unique address-name pairs.
known_pair = []
reverse_pair = []

# In addition to the data flow lists known_address, known_name and
# list_server, the following are output to the external graphics file.
auth_chain = []          # Authority chain, parent -> SOA fields.
ref_chain = []           # Reference chain, parent name -> child name
name_address = []        # DNS chain - domain name -> address
name_srvc = []           # Name and service pairs - domain name -> service
name_resource = []       # Name and resource pairs - domain name -> Resource Record
parent_child = []        # Parent and Child pairs - parent name -> child name
                         # This MAY NOT be the same as the ref_chain followed!
address_hits = []        # Address and Blacklist hit pairs - address->server

# Dump interface file data
_dot_dump = $pend_dummy  # Initially a no-op

# Data dump is enabled by setting the environment variable SPAMMER_DATA
# to the name of a writable file.
_dot_file = None

# Helper function for the dump-to-dot-file function
# dump_to_dot <list> <prefix>
def dump_to_dot lines prefix
  for [i line] in (enumerate $lines)
    print "    $prefix${i:04d} $line"

# Which will also set _dot_dump to this function . . .
def dump_dot
  with (open $_dot_file w) as f, (contextlib.redirect_stdout $f)
    print "# Data vintage: $(date -R)"
    print '# ABS Guide: is_spammer.bash; v2, 2004-msz'
    print
    print 'digraph G {'

    if $known_name
      print
      print '# Known domain name nodes'
      for [i name] in (enumerate $known_name)
        print "    N${i:04d} [label=\"$name\"] ;"

    if $known_address
      print
      print '# Known address nodes'
      for [i addr] in (enumerate $known_address)
        print "    A${i:04d} [label=\"$addr\"] ;"

    print
    print '/*'
    print ' * Known relationships :: User conversion to'
    print ' * graphic form by hand or program required.'
    print ' *'

    for [lines prefix title] in [[$auth_chain AC 'Authority ref. edges followed & field source.']
                                 [$ref_chain RC 'Name ref. edges followed and field source.']
                                 [$name_address NA 'Known name->address edges']
                                 [$name_srvc NS 'Known name->service edges']
                                 [$name_resource NR 'Known name->resource edges']
                                 [$parent_child PC 'Known parent->child edges']]
      if $lines
        print
        print "# $title"
        dump_to_dot $lines $prefix

    if $list_server
      print
      print '# Known Blacklist nodes'
      for [i server] in (enumerate $list_server)
        print "    LS${i:04d} [label=\"$server\"] ;"

    unique_lines $address_hits $address_hits
    if $address_hits
      print
      print '# Known address->Blacklist_hit edges'
      print '# CAUTION: dig warnings can trigger false hits.'
      dump_to_dot $address_hits AH
    print
    print ' *'
    print ' * That is a lot of relationships. Happy graphing.'
    print ' */'
    print '}'

# # # # 'Hunt the Spammer' execution flow # # # #

# Execution trace is enabled by setting the environment variable
# SPAMMER_TRACE to the name of a writable file.
_trace_log = []
_log_file = None

# Function to fill the trace log
def trace_logger
  $_trace_log.append (op_text $_pend_current_)

# Dump trace log to file function variable.
_log_dump = $pend_dummy  # Initially a no-op.

# Dump the trace log to a file.
def dump_log
  with (open $_log_file a) as f
    for line in $_trace_log
      print $line file=$f
    if $_pending_
      print '# # # Operations stack not empty # # #' file=$f
      for op in (reversed $_pending_)
        print (op_text $op) file=$f

# # # Utility program 'dig' wrappers # # #
#
# Each gives back the reply's lines, or None when dig fails (and the
# failure goes in the trace log).

# Some versions of 'dig' return warnings on stdout: leave them out.
def dig_reply reply message name
  if not $reply.ok
    $_trace_log.append ($message.format $reply.code $name)
    return None
  [$l for l in $reply.lines if $l and not ($l.startswith ";;")]

# Short form answer: 'dig' parses answer.

# Forward lookup :: Name -> Address
def short_fwd name
  print "." end="" flush=True
  dig_reply (dig +short $name -c 'in' -t a 2>/dev/null) '## Lookup error {} on {} ##' $name

# Reverse lookup :: Address -> Name
def short_rev ip
  print "." end="" flush=True
  dig_reply (dig +short -x $ip 2>/dev/null) '## Lookup error {} on {} ##' $ip

# Special format lookup used to query blacklist servers.
# (The bash version's warning filter counts to ${#_st_cnt}, the length of
# the count, so it looks at the first line or so only.)
def short_text name
  dig_reply (dig +short $name -c 'in' -t txt 2>/dev/null) '##Text lookup error {} on {}##' $name

# The long forms, a.k.a., the parse it yourself versions

# RFC 2782   Service lookups
# dig +noall +nofail +answer _ldap._tcp.openldap.org -t srv
# _<service>._<protocol>.<domain_name>
# _ldap._tcp.openldap.org. 3600   IN     SRV    0 0 389 ldap.openldap.org.
# domain TTL Class SRV Priority Weight Port Target

# Forward lookup :: Name -> poor man's zone transfer
def long_fwd name
  print ":" end="" flush=True
  reply = dig +noall +nofail +answer +authority +additional \
            $name -t soa $name -t mx $name -t any 2>/dev/null
  dig_reply $reply '# Zone lookup err {} on {} #' $name

# The reverse lookup domain name corresponding to the IPv6 address:
#     4321:0:1:2:3:4:567:89ab
# would be (nibble, I.E: Hexdigit) reversed:
# b.a.9.8.7.6.5.0.4.0.0.0.3.0.0.0.2.0.0.0.1.0.0.0.0.0.0.0.1.2.3.4.IP6.ARPA.

# Reverse lookup :: Address -> poor man's delegation chain
def long_rev rev_ip
  print ":" end="" flush=True
  dns = $rev_ip + '.in-addr.arpa.'
  reply = dig +noall +nofail +answer +authority +additional \
            $dns -t soa $dns -t any 2>/dev/null
  dig_reply $reply '# Deleg lkp error {} on {} #' $rev_ip

# # # Application specific functions # # #

# Mung a possible name; suppresses root and TLDs.
# name_fixup <string> : the name, in lower case and ending in a dot;
# None for nothing, only dots, or only a TLD.
def name_fixup s
  s = to_dot (to_lower $s)
  if not ($s.endswith ".")
    s += "."
  labels = ($s.split ".")[:-1]
  match (len $labels)
    case 0 | 1              # No dots, only dots; only a TLD.
      return None
    case 2                  # Maybe okay.
      # Needs a lookup table? (The bash version returns here, before its
      # check for a country-coded TLD: two letters, return failure.)
      return $s
  $s

# Grope and mung original input(s).
def split_input
  if not $uc_name
    return
  names = []
  for s in (unique_lines $uc_name)
    if is_address $s
      $uc_address.append $s
    elif (fixed := name_fixup $s)
      $names.append $fixed
  $uc_name[:] = $names
  $_trace_log.append "#Input $(len $uc_name) unchkd name input(s).#"
  $_trace_log.append "#Input $(len $uc_address) unchkd addr input(s).#"

# # # Discovery functions -- recursively interlocked by external data # # #
# # # The leading 'if list is empty; return' in each is required. # # #

# Recursion limiter: the next level, or None to stop here.
# limit_chk <next_level>
def limit_chk level=None
  if $indirect == 0 or $level is None
    1                       # The 'do-forever' choice: any value will do.
  elif $indirect < $level
    None                    # Stop here.
  else
    $level + 1              # Bump the given limit.

# add_new <list> <found> : add the found items, each once, that the list
# doesn't have yet (and give them back).
def add_new target found
  new = unique_lines $found
  edit_exact $target $new
  $target.extend $new
  $new

# For each name in uc_name:
#     Move name to chk_name.
#     Add addresses to uc_address.
#     Pend expand_input_address.
#     Repeat until nothing new found.
# expand_input_name <indirection_limit>
def expand_input_name level
  if not $uc_name
    return
  if (limit_chk $level) is None
    return
  addrs = []
  for name in $uc_name
    reply = short_fwd $name
    if $reply is not None
      $addrs.extend [$a for a in $reply if (is_address $a)]
  addrs = unique_lines $addrs         # Scrub duplicates.
  edit_exact $chk_address $addrs      # Scrub pending detail.
  edit_exact $known_address $addrs    # Scrub already detailed.
  if $addrs                           # Anything new?
    $uc_address.extend $addrs
    pend_func $expand_input_address $level
    $_trace_log.append "#Add $(len $addrs) unchkd addr inp.#"
  edit_exact $chk_name $uc_name       # Scrub pending detail.
  edit_exact $known_name $uc_name     # Scrub already detailed.
  if $uc_name
    $chk_name.extend $uc_name
    pend_func $detail_each_name $level
  $uc_name.clear

# For each address in uc_address:
#     Move address to chk_address.
#     Add names to uc_name.
#     Pend expand_input_name.
#     Repeat until nothing new found.
# expand_input_address <indirection_limit>
def expand_input_address level
  if not $uc_address
    return
  addrs = unique_lines $uc_address
  $uc_address.clear
  edit_exact $been_there_addr $addrs
  $been_there_addr.extend $addrs
  names = []
  for addr in $addrs
    reply = short_rev $addr
    if $reply is not None
      $names.extend [$n for n in (map $name_fixup $reply) if $n]
  names = unique_lines $names         # Scrub duplicates.
  edit_exact $chk_name $names         # Scrub pending detail.
  edit_exact $known_name $names       # Scrub already detailed.
  if $names                           # Anything new?
    $uc_name.extend $names
    pend_func $expand_input_name $level
    $_trace_log.append "#Add $(len $names) unchkd name inp.#"
  edit_exact $chk_address $addrs      # Scrub pending detail.
  edit_exact $known_address $addrs    # Scrub already detailed.
  if $addrs                           # Anything new?
    $chk_address.extend $addrs
    pend_func $detail_each_address $level

# The parse-it-yourself zone reply.
# The input is the chk_name list.
# detail_each_name <indirection_limit>
def detail_each_name level=None
  if not $chk_name
    return
  names = []        # Names found here
  addresses = []    # Addresses found here
  pair = []         # Pairs found here
  rev = []          # Reverse pairs found here
  pc = []           # Parent-Child gets big fast
  ref = []          # So does reference chain
  nr = []           # Name-Resource can be big
  na = []           # Name-Address
  ns = []           # Name-Service
  achn = []         # Chain of Authority

  # Local, unique copy of names to check
  chk = unique_lines $chk_name
  $chk_name.clear                     # Done with globals.
  # Less any names already known; if anything left, add to known_name.
  edit_exact $known_name $chk
  $known_name.extend $chk

  # for the list of (previously) unknown names . . .
  for who in $chk
    reply = long_fwd $who
    if $reply is None       # Lookup error == 'A' record 'unknown address'
      $pair.append "0.0.0.0 $who"
      continue
    reply = unique_lines $reply
    if not $reply
      $pair.append "0.0.0.0 $who"

    # Parse each line in the reply.
    for line in $reply
      f = ($line.split)
      # If usable record and not a warning message . . .
      if (len $f) <= 4 or $f[0] == ';;'
        continue
      f += ["" ""]          # (A missing field reads as empty, as in bash.)
      rec = $f[3]
      $nr.append "$who $rec"
      # Begin at RFC1033 (+++)
      match $rec
        case SOA            # Start Of Authority
          # <name> [<ttl>] [<class>] SOA <origin> <person>
          if (s := name_fixup $f[0])
            $names.append $s
            $achn.append "$who $s SOA"
            # SOA origin -- domain name of master zone record
            if (origin := name_fixup $f[4])
              $names.append $origin
              $achn.append "$who $origin SOA.O"
            # Responsible party e-mail address (possibly bogus).
            # Possibility of first.last@domain.name ignored.
            if (person := name_fixup $f[5])
              auth = ($person.split ".")[:-1]
              if (len $auth) > 2
                contact = ".".join $auth[1:]
                $names.append "$contact."
                $achn.append "$who $contact. SOA.C"

        case A              # IP(v4) Address Record
          if (s := name_fixup $f[0])
            $names.append $s
            $pair.append "$f[4] $s"
            $na.append "$s $f[4]"
            $ref.append "$who $s A"
          else
            $pair.append "$f[4] unknown.domain"
            $na.append "unknown.domain $f[4]"
            $ref.append "$who unknown.domain A"
          $addresses.append $f[4]
          $pc.append "$who $f[4]"

        case NS             # Name Server Record
          # Domain name being serviced (may be other than current)
          if (s := name_fixup $f[0])
            $names.append $s
            $ref.append "$who $s NS"
            # Domain name of service provider
            if (host := name_fixup $f[4])
              $names.append $host
              $ref.append "$who $host NSH"
              $ns.append "$host NS"
              $pc.append "$s $host"

        case MX             # Mail Server Record
          # Domain name being serviced (wildcards not handled here)
          if (s := name_fixup $f[0])
            $names.append $s
            $ref.append "$who $s MX"
          # Domain name of service provider
          if (s := name_fixup $f[5])
            $names.append $s
            $ref.append "$who $s MXH"
            $ns.append "$s MX"
            $pc.append "$who $s"

        case PTR            # Reverse address record
          # Special name
          if (s := name_fixup $f[0])
            $ref.append "$who $s PTR"
            # Host name (not a CNAME)
            if (host := name_fixup $f[4])
              $rev.append "$s $host"
              $ref.append "$who $host PTRH"
              $pc.append "$who $s"

        case AAAA           # IP(v6) Address Record
          if (s := name_fixup $f[0])
            $names.append $s
            $pair.append "$f[4] $s"
            $na.append "$s $f[4]"
            $ref.append "$who $s AAAA"
          else
            $pair.append "$f[4] unknown.domain"
            $na.append "unknown.domain $f[4]"
            $ref.append "$who unknown.domain"
          # No processing for IPv6 addresses
          $pc.append "$who $f[4]"

        case CNAME          # Alias name record
          # Nickname
          if (s := name_fixup $f[0])
            $names.append $s
            $ref.append "$who $s CNAME"
            $pc.append "$who $s"
          # Hostname
          if (s := name_fixup $f[4])
            $names.append $s
            $ref.append "$who $s CHOST"
            $pc.append "$who $s"
        # case TXT

  # Control dot array growth.
  add_new $auth_chain $achn
  add_new $ref_chain $ref
  add_new $name_address $na
  add_new $name_srvc $ns
  add_new $name_resource $nr
  add_new $parent_child $pc
  add_new $known_pair $pair           # Update list known_pair (Address and Name).
  add_new $reverse_pair $rev          # Update list of reverse pairs.

  # Check indirection limit -- give up if reached.
  lmt = limit_chk $level
  if $lmt is None
    return

  # Execution engine is LIFO. Order of pend operations is important.
  # Did we define any new addresses?
  addresses = unique_lines $addresses # Scrub duplicates.
  edit_exact $known_address $addresses  # Scrub already processed.
  # (bash: "edit_exact un_address …  # Scrub already waiting." un_address
  # is a typo for uc_address, so it scrubs nothing; left out here, since
  # the search depth depends on it.)
  if $addresses                       # Anything new?
    $uc_address.extend $addresses
    pend_func $expand_input_address $lmt
    $_trace_log.append "# Add $(len $addresses) unchkd addr. #"

  # Did we find any new names?
  names = unique_lines $names         # Scrub duplicates.
  edit_exact $known_name $names       # Scrub already processed.
  edit_exact $uc_name $names          # Scrub already waiting.
  if $names                           # Anything new?
    $uc_name.extend $names
    pend_func $expand_input_name $lmt
    $_trace_log.append "#Added $(len $names) unchkd name#"

# The parse-it-yourself delegation reply
# Input is the chk_address list.
# detail_each_address <indirection_limit>
def detail_each_address level=None
  if not $chk_address
    return
  unique_lines $chk_address $chk_address
  edit_exact $known_address $chk_address
  $known_address.extend $chk_address
  $chk_address.clear

# # # Application specific output functions # # #

# Pretty print the known pairs.
def report_pairs
  print
  print 'Known network pairs.'
  col_print $known_pair 2 5 30

  if $auth_chain
    print
    print 'Known chain of authority.'
    col_print $auth_chain 2 5 30 55

  if $reverse_pair
    print
    print 'Known reverse pairs.'
    col_print $reverse_pair 2 5 55

# Check an address against the list of blacklist servers.
# A good place to capture for GraphViz: address->status(server(reports))
# check_lists <ip_address>
def check_lists ip
  global _hs_RC
  [fwd rev] = split_ip $ip
  dns_addr = (dot_array $rev) + "."
  print "    Checking address $ip"
  for server in $list_server
    reply = short_text "$dns_addr$server"
    if $reply
      print "        Records from $server"
      $address_hits.append "$ip $server"
      _hs_RC = 2
      for r in $reply
        print "            $r"

# # # The usual application glue # # #

# Who did it?
def credits
  print
  print 'Advanced Bash Scripting Guide: is_spammer.bash, v2, 2004-msz'

# How to use it?
# (See also, "Quickstart" at end of script.)
def usage
  cat <<'_usage_statement_'
    The script is_spammer.tsh requires either one or two arguments.

    arg 1) May be one of:
        a) A domain name
        b) An IPv4 address
        c) The name of a file with any mix of names
           and addresses, one per line.

    arg 2) May be one of:
        a) A Blacklist server domain name
        b) The name of a file with Blacklist server
           domain names, one per line.
        c) If not present, a default list of (free)
           Blacklist servers is used.
        d) If a filename of an empty, readable, file
           is given,
           Blacklist server lookup is disabled.

    All script output is written to stdout.

    Return codes: 0 -> All OK, 1 -> Script failure,
                  2 -> Something is Blacklisted.

    Requires the external program 'dig' from the 'bind-9'
    set of DNS programs.  See: http://www.isc.org

    The domain name lookup depth limit defaults to 2 levels.
    Set the environment variable SPAMMER_LIMIT to change.
    SPAMMER_LIMIT=0 means 'unlimited'

    Limit may also be set on the command-line.
    If arg#1 is an integer, the limit is set to that value
    and then the above argument rules are applied.

    Setting the environment variable 'SPAMMER_DATA' to a filename
    will cause the script to write a GraphViz graphic file.

    For the development version;
    Setting the environment variable 'SPAMMER_TRACE' to a filename
    will cause the execution engine to log a function call trace.

_usage_statement_

# The default list of Blacklist servers:
# Many choices, see: http://www.spews.org/lists.html
default_servers = [
  'sbl-xbl.spamhaus.org'        # See: http://www.spamhaus.org (Conservative, well maintained)
  'relays.ordb.org'             # See: http://ordb.org (Open mail relays)
  'bl.spamcop.net'              # See: http://www.spamcop.net/ (You can report spammers here)
  'l2.spews.dnsbl.sorbs.net'    # See: http://www.spews.org (An 'early detect' system)
  'dnsbl.sorbs.net'             # See: http://www.dnsbl.us.sorbs.net/using.shtml
  'list.dsbl.org'               # See: http://dsbl.org/usage (Various mail relay lists)
  'multihop.dsbl.org'
  'unconfirmed.dsbl.org'
]

# Name of a readable file?
def readable_file name = (os.path.isfile $name) and (os.access $name $os.R_OK)

# User input argument #1
def setup_input arg
  if readable_file $arg
    $uc_name[:] = (file_to_array $arg)
    print "Using filename >$arg< as input."
  elif is_address $arg             # IP address?
    $uc_address[:] = [$arg]
    print "Starting with address >$arg<"
  else                             # Must be a name.
    $uc_name[:] = [$arg]
    print "Starting with domain name >$arg<"

# User input argument #2
def setup_servers arg
  if readable_file $arg
    $list_server[:] = (file_to_array $arg)
    print "Using filename >$arg< as blacklist server list."
  else
    $list_server[:] = [$arg]
    print "Using blacklist server >$arg<"

# A file the script can write: an existing writable one, or one it can
# make. If not: pend the complaint, release it, and exit 1.
# writable <file_name> <what>
def writable name what
  existed = os.path.exists $name
  try
    with (open $name a)
      pass
  except OSError
    verb = if $existed then write else create
    pend_func $print "Unable to $verb $what >$name<"
    pend_release
    exit 1

# User environment variable SPAMMER_TRACE
def live_log_die
  global _log_file _pend_hook_ _log_dump
  trace = $env.get SPAMMER_TRACE
  if $trace                        # Wants trace log?
    existed = os.path.exists $trace
    writable $trace 'log file'
    _log_file = $trace
    if $existed
      print >$_log_file            # (Empties it.)
    _pend_hook_ = $trace_logger
    _log_dump = $dump_log

# User environment variable SPAMMER_DATA
def data_capture
  global _dot_file _dot_dump
  data = $env.get SPAMMER_DATA
  if $data                         # Wants a data dump?
    writable $data 'data output file'
    _dot_file = $data
    _dot_dump = $dump_dot

# Grope user specified arguments.
def do_user_args args
  global indirect
  if $args and (is_number $args[0])
    indirect = int $args[0]
    args = $args[1:]

  match $args                      # Did user treat us well?
    case [$input]
      setup_input $input           # (The bash version has room for error checking.)
      $list_server[:] = $default_servers
      print 'Using default blacklist server list.'
      print "Search depth limit: $indirect"
    case [$input $servers]
      setup_input $input
      setup_servers $servers
      print "Search depth limit: $indirect"
    case _
      pend_func $usage
      pend_release
      _log_dump
      exit 1

# A general purpose debug tool.
# list_array <list> <name>
def list_array lines name
  print
  print "Element count $(len $lines) array $name"
  for [i line] in (enumerate $lines)
    print "Element $i >$line<"

# # # 'Hunt the Spammer' program code # # #
pend_init                                # Ready stack engine.
pend_func $credits                       # Last thing to print.

# # # Deal with user # # #
live_log_die                             # Setup debug trace log.
data_capture                             # Setup data capture file.
print
do_user_args $argv[1:]

# # # Haven't exited yet - There is some hope # # #
# Discovery group - Execution engine is LIFO - pend
# in reverse order of execution.
_hs_RC = 0                               # Hunt the Spammer return code
pend_mark
pend_func $report_pairs                  # Report name-address pairs.

# The two detail_* are mutually recursive functions.
# They also pend expand_* functions as required.
# These two (the last of ???) exit the recursion.
pend_func $detail_each_address           # Get all resources of addresses.
pend_func $detail_each_name              # Get all resources of names.

# The two expand_* are mutually recursive functions,
# which pend additional detail_* functions as required.
pend_func $expand_input_address 1        # Expand input names by address.
pend_func $expand_input_name 1           # Expand input addresses by name.

# Start with a unique set of names and addresses.
pend_func $unique_lines $uc_address $uc_address
pend_func $unique_lines $uc_name $uc_name

# Separate mixed input of names and addresses.
pend_func $split_input
pend_release

# # # Pairs reported -- Unique list of IP addresses found
print
if not $list_server
  print 'Blacklist server list empty, none checked.'
elif not $known_address
  print 'Known address list empty, none checked.'
else
  print 'Checking Blacklist servers.'
  for ip in (reversed $known_address)    # Pended from the top: run in order.
    pend_func $check_lists $ip
pend_release
_dot_dump                                # Graphics file dump
_log_dump                                # Execution trace
print

exit $_hs_RC

##############################
# Example output from script #
##############################
#
# ./is_spammer.bash 0 web4.alojamentos7.com
#
# Starting with domain name >web4.alojamentos7.com<
# Using default blacklist server list.
# Search depth limit: 0
# .:....::::...:::...:::.......::..::...:::.......::
# Known network pairs.
#     66.98.208.97             web4.alojamentos7.com.
#     66.98.208.97             ns1.alojamentos7.com.
#     69.56.202.147            ns2.alojamentos.ws.
#     66.98.208.97             alojamentos7.com.
#     66.98.208.97             web.alojamentos7.com.
#     69.56.202.146            ns1.alojamentos.ws.
#     69.56.202.146            alojamentos.ws.
#     66.235.180.113           ns1.alojamentos.org.
#     66.235.181.192           ns2.alojamentos.org.
#     66.235.180.113           alojamentos.org.
#     66.235.180.113           web6.alojamentos.org.
#     216.234.234.30           ns1.theplanet.com.
#     12.96.160.115            ns2.theplanet.com.
#     216.185.111.52           mail1.theplanet.com.
#     69.56.141.4              spooling.theplanet.com.
#     216.185.111.40           theplanet.com.
#     216.185.111.40           www.theplanet.com.
#     216.185.111.52           mail.theplanet.com.
#
# Checking Blacklist servers.
#   Checking address 66.98.208.97
#       Records from dnsbl.sorbs.net
#   "Spam Received See: http://www.dnsbl.sorbs.net/lookup.shtml?66.98.208.97"
#     Checking address 69.56.202.147
#     Checking address 69.56.202.146
#     Checking address 66.235.180.113
#     Checking address 66.235.181.192
#     Checking address 216.185.111.40
#     Checking address 216.234.234.30
#     Checking address 12.96.160.115
#     Checking address 216.185.111.52
#     Checking address 69.56.141.4
#
# Advanced Bash Scripting Guide: is_spammer.bash, v2, 2004-msz

####################################################
#  The script ignores everything from here on down #
#+ because of the 'exit' command, just above.      #
####################################################

# Quickstart
# ==========
#
#  Prerequisites
#
#   tessh, and 'dig,' version 9.x.x (dig $HOSTNAME, see first line of
#   output): a version of dig which supports the +short options.
#
#  Optional Prerequisites
#
#   'named,' a local DNS caching program. Any flavor will do.
#   Do twice: dig $HOSTNAME
#   Check near bottom of output for: SERVER: 127.0.0.1#53
#   That means you have one running.
#
#  Optional Graphics Support
#
#   'date,' a standard *nix thing. (date -R)
#   dot Program to convert graphic description file to a
#   diagram. (dot -V) A part of the Graph-Viz set of programs.
#   'dotty,' a visual editor for graphic description files.
#   Also a part of the Graph-Viz set of programs.
#
#  Quick Start
#
# In the same directory as the is_spammer.tsh script;
# Do: ./is_spammer.tsh
#
#  Usage Details
#
# 1. Blacklist server choices.
#   (a) To use default, built-in list: Do nothing.
#   (b) To use your own list:
#     i. Create a file with a single Blacklist server
#        domain name per line.
#     ii. Provide that filename as the last argument to the script.
#   (c) To use a single Blacklist server: Last argument to the script.
#   (d) To disable Blacklist lookups:
#     i. Create an empty file (touch spammer.nul)
#        Your choice of filename.
#     ii. Provide the filename of that empty file as the
#         last argument to the script.
#
# 2. Search depth limit.
#   (a) To use the default value of 2: Do nothing.
#   (b) To set a different limit: A limit of 0 means: no limit.
#     i. export SPAMMER_LIMIT=1
#        or whatever limit you want.
#     ii. OR provide the desired limit as the first argument to the script.
#
# 3. Optional execution trace log.
#   (a) To use the default setting of no log output: Do nothing.
#   (b) To write an execution trace log:
#       export SPAMMER_TRACE=spammer.log
#       or whatever filename you want.
#
# 4. Optional graphic description file.
#   (a) To use the default setting of no graphic file: Do nothing.
#   (b) To write a Graph-Viz graphic description file:
#       export SPAMMER_DATA=spammer.dot
#       or whatever filename you want.
#
# 5. Where to start the search.
#   (a) Starting with a single domain name:
#     i. Without a command-line search limit: First argument to script.
#     ii. With a command-line search limit: Second argument to script.
#   (b) Starting with a single IP address:
#     i. Without a command-line search limit: First argument to script.
#     ii. With a command-line search limit: Second argument to script.
#   (c) Starting with (mixed) multiple name(s) and/or address(es):
#       Create a file with one name or address per line.
#       Your choice of filename.
#     i. Without a command-line search limit: Filename as first
#        argument to script.
#     ii. With a command-line search limit: Filename as second
#         argument to script.
#
# 6. What to do with the display output.
#   (a) To view display output on screen: Do nothing.
#   (b) To save display output to a file: Redirect stdout to a filename.
#   (c) To discard display output: Redirect stdout to /dev/null.
#
# 7. Temporary end of decision making.
#    press RETURN
#    wait (optionally, watch the dots and colons).
#
# 8. Optionally check the return code.
#   (a) Return code 0: All OK
#   (b) Return code 1: Script setup failure
#   (c) Return code 2: Something was blacklisted.
#
# 9. Where is my graph (diagram)?
#
# The script does not directly produce a graph (diagram). It only
# produces a graphic description file. You can process the graphic
# descriptor file that was output with the 'dot' program.
#
# Until you edit that descriptor file, to describe the relationships
# you want shown, all that you will get is a bunch of labeled name and
# address nodes.
#
# All of the script's discovered relationships are within a comment
# block in the graphic descriptor file, each with a descriptive heading.
#
# The editing required to draw a line between a pair of nodes from the
# information in the descriptor file may be done with a text editor.
#
# Given these lines somewhere in the descriptor file:
#
#   # Known domain name nodes
#   N0000 [label="guardproof.info."] ;
#   N0002 [label="third.guardproof.info."] ;
#
#   # Known address nodes
#   A0000 [label="61.141.32.197"] ;
#
#   /*
#   # Known name->address edges
#   NA0000 third.guardproof.info. 61.141.32.197
#
#   # Known parent->child edges
#   PC0000 guardproof.info. third.guardproof.info.
#    */
#
# Turn that into the following lines by substituting node identifiers
# into the relationships:
#
#   # Known domain name nodes
#   N0000 [label="guardproof.info."] ;
#   N0002 [label="third.guardproof.info."] ;
#
#   # Known address nodes
#   A0000 [label="61.141.32.197"] ;
#
#   # PC0000 guardproof.info. third.guardproof.info.
#   N0000->N0002 ;
#
#   # NA0000 third.guardproof.info. 61.141.32.197
#   N0002->A0000 ;
#
#   /*
#   # Known name->address edges
#   NA0000 third.guardproof.info. 61.141.32.197
#
#   # Known parent->child edges
#   PC0000 guardproof.info. third.guardproof.info.
#    */
#
# Process that with the 'dot' program, and you have your first network
# diagram.
#
# In addition to the conventional graphic edges, the descriptor file
# includes similar format pair-data that describes services, zone
# records (sub-graphs?), blacklisted addresses, and other things which
# might be interesting to include in your graph. This additional
# information could be displayed as different node shapes, colors, line
# sizes, etc.
#
# The descriptor file can also be read and edited by a tessh script
# (of course). You should be able to find most of the functions
# required within the "is_spammer.tsh" script.
#
# End Quickstart.
#
# Additional Note
# ========== ====
#
# Michael Zick points out that there is a "makeviz.bash" interactive
# Web site at rediris.es. Can't give the full URL, since this is not
# a publically accessible site.
```

Both were run against a stand-in `dig` that answers for a made-up zone,
and printed the same reports and dot files, with one change to the bash
first. In today's bash, `dot_array`'s `local _da_output=${_da_input[@]}`
joins the parts with spaces (only `${_da_input[*]}` joins with the first
`IFS` character), so the blacklist query for 192.0.2.10 is
`10 2 0 192.bl.example.test` and no address is ever reported listed
(see the helpers below). With `[*]` there it works; the tessh `".".join`
has no such trap. Of the original's other slips, the port mends the
harmless ones (`split_input` unsets `ucname`, `short_text` filters only
the first line or so) and keeps one: `detail_each_name` scrubs the
addresses "already waiting" from `un_address`, an array that doesn't
exist, and the search depth depends on that. (bash 5 also says
`declare: -a: invalid option` about `declare -f -a _pending_`, and goes
on.)

The pending-operations stack: functions are pushed in the reverse order
of running, `pend_release` runs them last-in first-out down to a mark,
and `pend_drop` throws away what's above the mark. In tessh the
`pend_func` lines under `pend_mark` can't be indented as the bash has
them: indented lines under a call are more arguments to it.

```bash
# (is_spammer.bash's stack machine, and a few things to run on it.)
NO_WSP=$'\x0A'$'\x0D'

# The do nothing function
pend_dummy() { : ; }

# Clear and initialize the function stack.
pend_init() {
    unset _pending_[@]
    pend_func pend_stop_mark
    _pend_hook_='pend_dummy'  # Debug only.
}

# pend_func function_name [$(printf '%q\n' arguments)]
pend_func() {
    local IFS=${NO_WSP}
    set -f
    _pending_[${#_pending_[@]}]=$@
    set +f
}

# The function which stops the release:
pend_stop_mark() {
    _p_ctrl_=0
}

pend_mark() {
    pend_func pend_stop_mark
}

# Execute functions until 'pend_stop_mark' . . .
pend_release() {
    local -i _top_             # Declare _top_ as integer.
    _p_ctrl_=${#_pending_[@]}
    while [ ${_p_ctrl_} -gt 0 ]
    do
       _top_=${#_pending_[@]}-1
       _pend_current_=${_pending_[$_top_]}
       unset _pending_[$_top_]
       $_pend_hook_            # Debug only.
       eval $_pend_current_
    done
}

# Drop functions until 'pend_stop_mark' . . .
pend_drop() {
    local -i _top_
    local _pd_ctrl_=${#_pending_[@]}
    while [ ${_pd_ctrl_} -gt 0 ]
    do
       _top_=$_pd_ctrl_-1
       if [ "${_pending_[$_top_]}" == 'pend_stop_mark' ]
       then
           unset _pending_[$_top_]
           break
       else
           unset _pending_[$_top_]
           _pd_ctrl_=$_top_
       fi
    done
    if [ ${#_pending_[@]} -eq 0 ]
    then
        pend_func pend_stop_mark
    fi
}

pend_init
pend_func echo "Printed last."
pend_mark                  # Pend in reverse order of execution:
    pend_func echo three
    pend_func echo two
    pend_func echo one
pend_release               # Runs up to the mark.
echo "--"
pend_mark
    pend_func echo "Never printed."
pend_drop                  # Drops down to the mark.
pend_release               # Runs the rest.
```

```tessh
_pending_ = []           # Global stack of pending operations.
_p_ctrl_ = 0             # Global sentinel for stack runners.
_pend_current_ = None    # Global holder for currently executing function.

# The do nothing function
def pend_dummy
  pass

_pend_hook_ = $pend_dummy

# Clear and initialize the function stack.
def pend_init
  global _pend_hook_
  $_pending_.clear
  pend_func $pend_stop_mark
  _pend_hook_ = $pend_dummy      # Debug only.

# pend_func function [arguments]
def pend_func f *args
  $_pending_.append [$f $args]

# The function which stops the release:
def pend_stop_mark
  global _p_ctrl_
  _p_ctrl_ = 0

def pend_mark
  pend_func $pend_stop_mark

# Run functions until 'pend_stop_mark' . . .
def pend_release
  global _p_ctrl_ _pend_current_
  _p_ctrl_ = len $_pending_
  while $_p_ctrl_ > 0 and $_pending_
    _pend_current_ = ($_pending_.pop)
    _pend_hook_                    # Debug only.
    [f args] = $_pend_current_
    f *$args

# Drop functions until 'pend_stop_mark' . . .
def pend_drop
  while $_pending_
    [f _] = ($_pending_.pop)
    if $f is $pend_stop_mark
      break
  if not $_pending_
    pend_func $pend_stop_mark

pend_init
pend_func $print "Printed last."
pend_mark                  # Pend in reverse order of execution:
pend_func $print three
pend_func $print two
pend_func $print one
pend_release               # Runs up to the mark.
print "--"
pend_mark
pend_func $print "Never printed."
pend_drop                  # Drops down to the mark.
pend_release               # Runs the rest.
```

```output
one
two
three
--
Printed last.
```

The array editors take array names in bash and copy the arrays in and
out with `eval`. The tessh ones take the lists and change them;
`edit_by_glob`'s patterns are `fnmatch`'s, which match as bash's
`${t/#pattern/}` does when it empties the whole item.

```bash
# (is_spammer.bash's array editors, tried on made-up names.)
NO_WSP=$'\x0A'$'\x0D'

# This function described in edit_exact.bash.
# (See "Additional documentation," above.)
# edit_exact <excludes_array_name> <target_array_name>
edit_exact() {
    [ $# -eq 2 ] ||
    [ $# -eq 3 ] || return 1
    local -a _ee_Excludes
    local -a _ee_Target
    local _ee_x
    local _ee_t
    local IFS=${NO_WSP}
    set -f
    eval _ee_Excludes=\( \$\{$1\[@\]\} \)
    eval _ee_Target=\( \$\{$2\[@\]\} \)
    local _ee_len=${#_ee_Target[@]}     # Original length.
    local _ee_cnt=${#_ee_Excludes[@]}   # Exclude list length.
    [ ${_ee_len} -ne 0 ] || return 0    # Can't edit zero length.
    [ ${_ee_cnt} -ne 0 ] || return 0    # Can't edit zero length.
    for (( x = 0; x < ${_ee_cnt} ; x++ ))
    do
        _ee_x=${_ee_Excludes[$x]}
        for (( n = 0 ; n < ${_ee_len} ; n++ ))
        do
            _ee_t=${_ee_Target[$n]}
            if [ x"${_ee_t}" == x"${_ee_x}" ]
            then
                unset _ee_Target[$n]     # Discard match.
                [ $# -eq 2 ] && break    # If 2 arguments, then done.
            fi
        done
    done
    eval $2=\( \$\{_ee_Target\[@\]\} \)
    set +f
    return 0
}

# This function described in edit_by_glob.bash.
# edit_by_glob <excludes_array_name> <target_array_name>
edit_by_glob() {
    [ $# -eq 2 ] ||
    [ $# -eq 3 ] || return 1
    local -a _ebg_Excludes
    local -a _ebg_Target
    local _ebg_x
    local _ebg_t
    local IFS=${NO_WSP}
    set -f
    eval _ebg_Excludes=\( \$\{$1\[@\]\} \)
    eval _ebg_Target=\( \$\{$2\[@\]\} \)
    local _ebg_len=${#_ebg_Target[@]}
    local _ebg_cnt=${#_ebg_Excludes[@]}
    [ ${_ebg_len} -ne 0 ] || return 0
    [ ${_ebg_cnt} -ne 0 ] || return 0
    for (( x = 0; x < ${_ebg_cnt} ; x++ ))
    do
        _ebg_x=${_ebg_Excludes[$x]}
        for (( n = 0 ; n < ${_ebg_len} ; n++ ))
        do
            [ $# -eq 3 ] && _ebg_x=${_ebg_x}'*'  #  Do prefix edit
            if [ ${_ebg_Target[$n]:=} ]          #+ if defined & set.
            then
                _ebg_t=${_ebg_Target[$n]/#${_ebg_x}/}
                [ ${#_ebg_t} -eq 0 ] && unset _ebg_Target[$n]
            fi
        done
    done
    eval $2=\( \$\{_ebg_Target\[@\]\} \)
    set +f
    return 0
}

# This function described in unique_lines.bash.
# unique_lines <in_name> <out_name>
unique_lines() {
    [ $# -eq 2 ] || return 1
    local -a _ul_in
    local -a _ul_out
    local -i _ul_cnt
    local -i _ul_pos
    local _ul_tmp
    local IFS=${NO_WSP}
    set -f
    eval _ul_in=\( \$\{$1\[@\]\} \)
    _ul_cnt=${#_ul_in[@]}
    for (( _ul_pos = 0 ; _ul_pos < ${_ul_cnt} ; _ul_pos++ ))
    do
        if [ ${_ul_in[${_ul_pos}]:=} ]      # If defined & not empty
        then
            _ul_tmp=${_ul_in[${_ul_pos}]}
            _ul_out[${#_ul_out[@]}]=${_ul_tmp}
            for (( zap = _ul_pos ; zap < ${_ul_cnt} ; zap++ ))
            do
                [ ${_ul_in[${zap}]:=} ] &&
                [ 'x'${_ul_in[${zap}]} == 'x'${_ul_tmp} ] &&
                    unset _ul_in[${zap}]
            done
        fi
    done
    eval $2=\( \$\{_ul_out\[@\]\} \)
    set +f
    return 0
}

names=( a.test b.test a.test c.test b.test )
unique_lines names uniq             # Each once, in order.
echo "${uniq[@]}"

known=( b.test )
edit_exact known uniq               # Less the known ones.
echo "${uniq[@]}"

hosts=( mail.a.test www.a.test mail.b.test ns.mail.test )
patterns=( 'mail.*' )
edit_by_glob patterns hosts         # Less those 'mail.*' matches.
echo "${hosts[@]}"

hosts=( mail.a.test www.a.test mail.b.test )
prefixes=( 'www.' )
edit_by_glob prefixes hosts prefix  # Less those starting with 'www.'.
echo "${hosts[@]}"
```

```tessh
import fnmatch

# edit_exact <excludes> <target> [every]
# Remove each exclude's first match from the target (with every=True,
# all its matches).
def edit_exact excludes target every=False
  for x in $excludes
    if $every
      $target[:] = [$t for t in $target if $t != $x]
    elif $x in $target
      $target.remove $x

# edit_by_glob <excludes> <target> [prefix]
# Remove the items an exclude pattern matches (with prefix=True,
# the items that start with a match).
def edit_by_glob excludes target prefix=False
  for x in $excludes
    pattern = if $prefix then $x + "*" else $x
    $target[:] = [$t for t in $target if not (fnmatch.fnmatchcase $t $pattern)]

# unique_lines <in> [<into>]
# The non-empty items, each once, in order: given back, and put in
# the list `into` too if there is one.
def unique_lines lines into=None
  out = list (dict.fromkeys ($l for l in $lines if $l))
  if $into is not None
    $into[:] = $out
  $out

names = [a.test b.test a.test c.test b.test]
uniq = unique_lines $names               # Each once, in order.
print *$uniq

known = [b.test]
edit_exact $known $uniq                  # Less the known ones.
print *$uniq

hosts = [mail.a.test www.a.test mail.b.test ns.mail.test]
edit_by_glob ['mail.*'] $hosts           # Less those 'mail.*' matches.
print *$hosts

hosts = [mail.a.test www.a.test mail.b.test]
edit_by_glob ['www.'] $hosts prefix=True # Less those starting with 'www.'.
print *$hosts
```

```output
a.test b.test c.test
a.test c.test
www.a.test ns.mail.test
mail.a.test mail.b.test
```

The name and address helpers. `to_lower`'s 26 replacements are
`.lower`, and `to_dot`'s `${1//[#|@|%]/.}` is a regex `.sub`. The last
line shows the `dot_array` trouble: bash 5 joins with spaces.

```bash
# (is_spammer.bash's name and address helpers, tried on made-up input.)
# Whitespace == :Space:Tab:Line Feed:Carriage Return:
WSP_IFS=$'\x20'$'\x09'$'\x0A'$'\x0D'

# No Whitespace == Line Feed:Carriage Return
NO_WSP=$'\x0A'$'\x0D'

# Field separator for dotted decimal IP addresses
ADR_IFS=${NO_WSP}'.'

# Array to dotted string conversions
DOT_IFS='.'${WSP_IFS}

# This function described in char_convert.bash.
# to_lower <string>
to_lower() {
    [ $# -eq 1 ] || return 1
    local _tl_out
    _tl_out=${1//A/a}
    _tl_out=${_tl_out//B/b}
    _tl_out=${_tl_out//C/c}
    _tl_out=${_tl_out//D/d}
    _tl_out=${_tl_out//E/e}
    _tl_out=${_tl_out//F/f}
    _tl_out=${_tl_out//G/g}
    _tl_out=${_tl_out//H/h}
    _tl_out=${_tl_out//I/i}
    _tl_out=${_tl_out//J/j}
    _tl_out=${_tl_out//K/k}
    _tl_out=${_tl_out//L/l}
    _tl_out=${_tl_out//M/m}
    _tl_out=${_tl_out//N/n}
    _tl_out=${_tl_out//O/o}
    _tl_out=${_tl_out//P/p}
    _tl_out=${_tl_out//Q/q}
    _tl_out=${_tl_out//R/r}
    _tl_out=${_tl_out//S/s}
    _tl_out=${_tl_out//T/t}
    _tl_out=${_tl_out//U/u}
    _tl_out=${_tl_out//V/v}
    _tl_out=${_tl_out//W/w}
    _tl_out=${_tl_out//X/x}
    _tl_out=${_tl_out//Y/y}
    _tl_out=${_tl_out//Z/z}
    echo ${_tl_out}
    return 0
}

#### Application helper functions ####

# Not everybody uses dots as separators (APNIC, for example).
# This function described in to_dot.bash
# to_dot <string>
to_dot() {
    [ $# -eq 1 ] || return 1
    echo ${1//[#|@|%]/.}
    return 0
}

# This function described in is_number.bash.
# is_number <input>
is_number() {
    [ "$#" -eq 1 ]    || return 1  # is blank?
    [ x"$1" == 'x0' ] && return 0  # is zero?
    local -i tst
    let tst=$1 2>/dev/null         # else is numeric!
    return $?
}

# This function described in is_address.bash.
# is_address <input>
is_address() {
    [ $# -eq 1 ] || return 1    # Blank ==> false
    local -a _ia_input
    local IFS=${ADR_IFS}
    _ia_input=( $1 )
    if  [ ${#_ia_input[@]} -eq 4 ]  &&
        is_number ${_ia_input[0]}   &&
        is_number ${_ia_input[1]}   &&
        is_number ${_ia_input[2]}   &&
        is_number ${_ia_input[3]}   &&
        [ ${_ia_input[0]} -lt 256 ] &&
        [ ${_ia_input[1]} -lt 256 ] &&
        [ ${_ia_input[2]} -lt 256 ] &&
        [ ${_ia_input[3]} -lt 256 ]
    then
        return 0
    else
        return 1
    fi
}

#  This function described in split_ip.bash.
#  split_ip <IP_address>
#+ <array_name_norm> [<array_name_rev>]
split_ip() {
    [ $# -eq 3 ] ||              #  Either three
    [ $# -eq 2 ] || return 1     #+ or two arguments
    local -a _si_input
    local IFS=${ADR_IFS}
    _si_input=( $1 )
    IFS=${WSP_IFS}
    eval $2=\(\ \$\{_si_input\[@\]\}\ \)
    if [ $# -eq 3 ]
    then
        # Build query order array.
        local -a _dns_ip
        _dns_ip[0]=${_si_input[3]}
        _dns_ip[1]=${_si_input[2]}
        _dns_ip[2]=${_si_input[1]}
        _dns_ip[3]=${_si_input[0]}
        eval $3=\(\ \$\{_dns_ip\[@\]\}\ \)
    fi
    return 0
}

# This function described in dot_array.bash.
# dot_array <array_name>
dot_array() {
    [ $# -eq 1 ] || return 1     # Single argument required.
    local -a _da_input
    eval _da_input=\(\ \$\{$1\[@\]\}\ \)
    local IFS=${DOT_IFS}
    local _da_output=${_da_input[@]}
    IFS=${WSP_IFS}
    echo ${_da_output}
    return 0
}

# Mung a possible name; suppresses root and TLDs.
# name_fixup <string>
name_fixup(){
    local -a _nf_tmp
    local -i _nf_end
    local _nf_str
    local IFS
    _nf_str=$(to_lower ${1})
    _nf_str=$(to_dot ${_nf_str})
    _nf_end=${#_nf_str}-1
    [ ${_nf_str:${_nf_end}} != '.' ] &&
        _nf_str=${_nf_str}'.'
    IFS=${ADR_IFS}
    _nf_tmp=( ${_nf_str} )
    IFS=${WSP_IFS}
    _nf_end=${#_nf_tmp[@]}
    case ${_nf_end} in
    0) # No dots, only dots.
        echo
        return 1
    ;;
    1) # Only a TLD.
        echo
        return 1
    ;;
    2) # Maybe okay.
       echo ${_nf_str}
       return 0
       # Needs a lookup table?
       if [ ${#_nf_tmp[1]} -eq 2 ]
       then # Country coded TLD.
           echo
           return 1
       else
           echo ${_nf_str}
           return 0
       fi
    ;;
    esac
    echo ${_nf_str}
    return 0
}

for s in 192.0.2.10 300.1.2.3 10.1.2 www.example.test
do
  if is_address $s
  then echo "$s: an address"
  else echo "$s: not an address"
  fi
done

for s in WWW.Example.TEST mail.example.test. test Host@Example.org
do
  if n=$(name_fixup $s)
  then echo "$s -> $n"
  else echo "$s -> (not a name)"
  fi
done

split_ip 192.0.2.10 fwd rev           # The blacklist query for an address:
echo $(dot_array rev).bl.example.test
```

```output bash
192.0.2.10: an address
300.1.2.3: not an address
10.1.2: not an address
www.example.test: not an address
WWW.Example.TEST -> www.example.test.
mail.example.test. -> mail.example.test.
test -> (not a name)
Host@Example.org -> host.example.org.
10 2 0 192.bl.example.test
```

```tessh
# to_lower <string>  (the bash version replaces A to Z one letter at a time)
def to_lower s = ($s.lower)

# Not everybody uses dots as separators (APNIC, for example).
# to_dot <string>
def to_dot s = re'[#|@%]'.sub "." $s

# is_number <input>
def is_number s
  try
    int $s
    True
  except ValueError
    False

# is_address <input>
def is_address s
  parts = $s.split "."
  (len $parts) == 4 and (all ((is_number $p) and (int $p) < 256 for p in $parts))

# split_ip <IP_address> : [the parts, the parts in DNS query order]
def split_ip ip
  parts = $ip.split "."
  [$parts (list (reversed $parts))]

# dot_array <list>
def dot_array parts = ".".join $parts

# Mung a possible name; suppresses root and TLDs.
# name_fixup <string> : the name, in lower case and ending in a dot;
# None for nothing, only dots, or only a TLD.
def name_fixup s
  s = to_dot (to_lower $s)
  if not ($s.endswith ".")
    s += "."
  labels = ($s.split ".")[:-1]
  match (len $labels)
    case 0 | 1              # No dots, only dots; only a TLD.
      return None
    case 2                  # Maybe okay.
      # Needs a lookup table? (The bash version returns here, before its
      # check for a country-coded TLD: two letters, return failure.)
      return $s
  $s

for s in [192.0.2.10 300.1.2.3 10.1.2 www.example.test]
  if is_address $s
    print "$s: an address"
  else
    print "$s: not an address"

for s in [WWW.Example.TEST mail.example.test. test Host@Example.org]
  if (n := name_fixup $s)
    print "$s -> $n"
  else
    print "$s -> (not a name)"

[fwd rev] = split_ip 192.0.2.10       # The blacklist query for an address:
print "${dot_array $rev}.bl.example.test"
```

```output
192.0.2.10: an address
300.1.2.3: not an address
10.1.2: not an address
www.example.test: not an address
WWW.Example.TEST -> www.example.test.
mail.example.test. -> mail.example.test.
test -> (not a name)
Host@Example.org -> host.example.org.
10.2.0.192.bl.example.test
```

`col_print` puts the fields of each line at tab stops, with at least
`min_space` spaces between them. The bash builds its padding by cutting
pieces off a string of 60 spaces; in tessh it's `" " * $width`.

```bash
# (is_spammer.bash's col_print, on made-up pairs.)
NO_WSP=$'\x0A'$'\x0D'

#  Columnized print of an array of multi-field strings.
#  col_print <array_name> <min_space> <
#+ tab_stop [tab_stops]>
col_print() {
    [ $# -gt 2 ] || return 0
    local -a _cp_inp
    local -a _cp_spc
    local -a _cp_line
    local _cp_min
    local _cp_mcnt
    local _cp_pos
    local _cp_cnt
    local _cp_tab
    local -i _cp
    local -i _cpf
    local _cp_fld
    # WARNING: FOLLOWING LINE NOT BLANK -- IT IS QUOTED SPACES.
    local _cp_max='                                                            '
    set -f
    local IFS=${NO_WSP}
    eval _cp_inp=\(\ \$\{$1\[@\]\}\ \)
    [ ${#_cp_inp[@]} -gt 0 ] || return 0 # Empty is easy.
    _cp_mcnt=$2
    _cp_min=${_cp_max:1:${_cp_mcnt}}
    shift
    shift
    _cp_cnt=$#
    for (( _cp = 0 ; _cp < _cp_cnt ; _cp++ ))
    do
        _cp_spc[${#_cp_spc[@]}]="${_cp_max:2:$1}" #"
        shift
    done
    _cp_cnt=${#_cp_inp[@]}
    for (( _cp = 0 ; _cp < _cp_cnt ; _cp++ ))
    do
        _cp_pos=1
        IFS=${NO_WSP}$'\x20'
        _cp_line=( ${_cp_inp[${_cp}]} )
        IFS=${NO_WSP}
        for (( _cpf = 0 ; _cpf < ${#_cp_line[@]} ; _cpf++ ))
        do
            _cp_tab=${_cp_spc[${_cpf}]:${_cp_pos}}
            if [ ${#_cp_tab} -lt ${_cp_mcnt} ]
            then
                _cp_tab="${_cp_min}"
            fi
            echo -n "${_cp_tab}"
            (( _cp_pos = ${_cp_pos} + ${#_cp_tab} ))
            _cp_fld="${_cp_line[${_cpf}]}"
            echo -n ${_cp_fld}
            (( _cp_pos = ${_cp_pos} + ${#_cp_fld} ))
        done
        echo
    done
    set +f
    return 0
}

pairs=( '192.0.2.10 spammer.test.' '198.51.100.6 ns2.hoster.test.' )
col_print pairs 2 5 30

chain=( 'spammer.test. ns1.spammer.test. SOA.O'
        'a-rather-long-host-name.test. hoster.test. SOA' )
col_print chain 2 5 30 55
```

```tessh
# Columnized print of a list of multi-field strings.
# col_print <lines> <min_space> <tab_stop [tab_stops]>
def col_print lines min_space *stops
  if not $stops
    return
  for line in $lines
    pos = 1
    out = ""
    for [i field] in (enumerate ($line.split))
      width = if $i < (len $stops) then $stops[$i] - $pos else 0
      if $width < $min_space
        width = $min_space
      out += " " * $width + $field
      pos += $width + (len $field)
    print $out

pairs = ['192.0.2.10 spammer.test.' '198.51.100.6 ns2.hoster.test.']
col_print $pairs 2 5 30

chain = ['spammer.test. ns1.spammer.test. SOA.O'
         'a-rather-long-host-name.test. hoster.test. SOA']
col_print $chain 2 5 30 55
```

```output
    192.0.2.10               spammer.test.
    198.51.100.6             ns2.hoster.test.
    spammer.test.            ns1.spammer.test.        SOA.O
    a-rather-long-host-name.test.  hoster.test.       SOA
```

The heart of `detail_each_name`: each line of `dig`'s zone reply is split
into fields, and the record type (the fourth field) decides which names,
addresses and relations it gives. Here on a made-up reply, with a shorter
`name_fixup`:

```tessh
# (A reply as `dig +noall +answer … -t any` gives it, for a made-up zone.)
reply = '''
spammer.test.      3600  IN  SOA    ns1.spammer.test. hostmaster.spammer.test. 2004 3600 600 86400 300
spammer.test.      3600  IN  A      192.0.2.10
spammer.test.      3600  IN  NS     ns2.hoster.test.
spammer.test.      3600  IN  MX     10 mail.spammer.test.
www.spammer.test.  3600  IN  CNAME  spammer.test.
;; Warning: a made-up warning
'''

# name_fixup, shortened: lower case, dots for # | @ %, a dot at the end;
# None for only a TLD.
def name_fixup s
  s = (re'[#|@%]'.sub "." ($s.lower)).rstrip "."
  if "." in $s then $s + "." else None

who = spammer.test.
names = [] ; addresses = [] ; ref = [] ; achn = []
for line in ($reply.splitlines)
  f = ($line.split)
  # If usable record and not a warning message . . .
  if (len $f) <= 4 or $f[0] == ';;'
    continue
  f += ["" ""]
  match $f[3]
    case SOA                    # Start Of Authority
      if (s := name_fixup $f[0])
        $achn.append "$who $s SOA"
        if (origin := name_fixup $f[4])
          $achn.append "$who $origin SOA.O"
    case A                      # IP(v4) Address Record
      $addresses.append $f[4]
      $ref.append "$who $(name_fixup $f[0]) A"
    case NS                     # Name Server Record
      if (host := name_fixup $f[4])
        $names.append $host
        $ref.append "$who $host NSH"
    case MX                     # Mail Server Record (host after the preference)
      if (host := name_fixup $f[5])
        $names.append $host
        $ref.append "$who $host MXH"
    case CNAME                  # Alias name record
      if (s := name_fixup $f[0])
        $names.append $s
        $ref.append "$who $s CNAME"

print names: *$names
print addresses: *$addresses
print "authority:" ; print *$achn sep="\n"
print "references:" ; print *$ref sep="\n"
```

```output
names: ns2.hoster.test. mail.spammer.test. www.spammer.test.
addresses: 192.0.2.10
authority:
spammer.test. spammer.test. SOA
spammer.test. ns1.spammer.test. SOA.O
references:
spammer.test. spammer.test. A
spammer.test. ns2.hoster.test. NSH
spammer.test. mail.spammer.test. MXH
spammer.test. www.spammer.test. CNAME
```

And the argument handling, `do_user_args` with `setup_input` and
`setup_servers`, on sample command lines and made-up files (the usage
message and exit left out):

```tessh
import os
# (A made-up input file and blacklist server file to try it on.)
print "spammer.test\n192.0.2.11\n\nwww.spammer.test" >hunt.txt
print "bl.example.test\nrelays.example.test" >servers.txt

uc_name = [] ; uc_address = [] ; list_server = []
indirect = 2
default_servers = ['sbl-xbl.spamhaus.org' 'relays.ordb.org' 'bl.spamcop.net']

def is_number s
  try
    int $s
    True
  except ValueError
    False

def is_address s
  parts = $s.split "."
  (len $parts) == 4 and (all ((is_number $p) and (int $p) < 256 for p in $parts))

def file_to_array name
  with (open $name) as f
    [$l for l in (($f.read).splitlines) if $l]

def readable_file name = (os.path.isfile $name) and (os.access $name $os.R_OK)

# User input argument #1
def setup_input arg
  if readable_file $arg
    $uc_name[:] = (file_to_array $arg)
    print "Using filename >$arg< as input."
  elif is_address $arg             # IP address?
    $uc_address[:] = [$arg]
    print "Starting with address >$arg<"
  else                             # Must be a name.
    $uc_name[:] = [$arg]
    print "Starting with domain name >$arg<"

# User input argument #2
def setup_servers arg
  if readable_file $arg
    $list_server[:] = (file_to_array $arg)
    print "Using filename >$arg< as blacklist server list."
  else
    $list_server[:] = [$arg]
    print "Using blacklist server >$arg<"

# Grope user specified arguments.
def do_user_args args
  global indirect
  if $args and (is_number $args[0])
    indirect = int $args[0]
    args = $args[1:]

  match $args                      # Did user treat us well?
    case [$input]
      setup_input $input
      $list_server[:] = $default_servers
      print 'Using default blacklist server list.'
      print "Search depth limit: $indirect"
    case [$input $servers]
      setup_input $input
      setup_servers $servers
      print "Search depth limit: $indirect"
    case _
      print "(usage, and exit 1)"

# (Sample command lines, as $argv[1:] would give them.)
for args in [[spammer.test] ["0" 192.0.2.10 bl.example.test] [hunt.txt servers.txt] [spammer.test bl.example.test extra]]
  for xs in [$uc_name $uc_address $list_server]
    $xs.clear
  indirect = 2
  print "\$ is_spammer.tsh" *$args
  do_user_args $args
  print "  names:" *$uc_name
  print "  addresses:" *$uc_address
  print "  servers:" *$list_server
```

```output
$ is_spammer.tsh spammer.test
Starting with domain name >spammer.test<
Using default blacklist server list.
Search depth limit: 2
  names: spammer.test
  addresses:
  servers: sbl-xbl.spamhaus.org relays.ordb.org bl.spamcop.net
$ is_spammer.tsh 0 192.0.2.10 bl.example.test
Starting with address >192.0.2.10<
Using blacklist server >bl.example.test<
Search depth limit: 0
  names:
  addresses: 192.0.2.10
  servers: bl.example.test
$ is_spammer.tsh hunt.txt servers.txt
Using filename >hunt.txt< as input.
Using filename >servers.txt< as blacklist server list.
Search depth limit: 2
  names: spammer.test 192.0.2.11 www.spammer.test
  addresses:
  servers: bl.example.test relays.example.test
$ is_spammer.tsh spammer.test bl.example.test extra
(usage, and exit 1)
  names:
  addresses:
  servers:
```

## Spammer hunt with whois
<!-- help: abs-whx whx whois spammer registry rwhois arin ripe apnic -->

Walter Dnes's `whx.sh` finds who's responsible for an address: it looks
up a domain name's address with `host`, skips the reserved ranges, asks
the regional registry that holds the address with `whois` (ARIN, or the
one ARIN refers to, and the national registries some of them refer to in
turn), and shows the result. It needs the network and `whois`.

### Example A-29. Spammer Hunt (`whx.sh`)

```bash no-run
#!/bin/bash
# whx.sh: "whois" spammer lookup
# Author: Walter Dnes
# Slight revisions (first section) by ABS Guide author.
# Used in ABS Guide with permission.

# Needs version 3.x or greater of Bash to run (because of =~ operator).
# Commented by script author and ABS Guide author.



E_BADARGS=85        # Missing command-line arg.
E_NOHOST=86         # Host not found.
E_TIMEOUT=87        # Host lookup timed out.
E_UNDEF=88          # Some other (undefined) error.

HOSTWAIT=10         # Specify up to 10 seconds for host query reply.
                    # The actual wait may be a bit longer.
OUTFILE=whois.txt   # Output file.
PORT=4321


if [ -z "$1" ]      # Check for (required) command-line arg.
then
  echo "Usage: $0 domain name or IP address"
  exit $E_BADARGS
fi


if [[ "$1" =~ [a-zA-Z][a-zA-Z]$ ]]  #  Ends in two alpha chars?
then                                  #  It's a domain name &&
                                      #+ must do host lookup.
  IPADDR=$(host -W $HOSTWAIT $1 | awk '{print $4}')
                                      #  Doing host lookup
                                      #+ to get IP address.
				      #  Extract final field.
else
  IPADDR="$1"                         #  Command-line arg was IP address.
fi

echo; echo "IP Address is: "$IPADDR""; echo

if [ -e "$OUTFILE" ]
then
  rm -f "$OUTFILE"
  echo "Stale output file \"$OUTFILE\" removed."; echo
fi


#  Sanity checks.
#  (This section needs more work.)
#  ===============================
if [ -z "$IPADDR" ]
# No response.
then
  echo "Host not found!"
  exit $E_NOHOST    # Bail out.
fi

if [[ "$IPADDR" =~ ^[;;] ]]
#  ;; Connection timed out; no servers could be reached.
then
  echo "Host lookup timed out!"
  exit $E_TIMEOUT   # Bail out.
fi

if [[ "$IPADDR" =~ [(NXDOMAIN)]$ ]]
#  Host xxxxxxxxx.xxx not found: 3(NXDOMAIN)
then
  echo "Host not found!"
  exit $E_NOHOST    # Bail out.
fi

if [[ "$IPADDR" =~ [(SERVFAIL)]$ ]]
#  Host xxxxxxxxx.xxx not found: 2(SERVFAIL)
then
  echo "Host not found!"
  exit $E_NOHOST    # Bail out.
fi




# ======================== Main body of script ========================

AFRINICquery() {
#  Define the function that queries AFRINIC. Echo a notification to the
#+ screen, and then run the actual query, redirecting output to $OUTFILE.

  echo "Searching for $IPADDR in whois.afrinic.net"
  whois -h whois.afrinic.net "$IPADDR" > $OUTFILE

#  Check for presence of reference to an rwhois.
#  Warn about non-functional rwhois.infosat.net server
#+ and attempt rwhois query.
  if grep -e "^remarks: .*rwhois\.[^ ]\+" "$OUTFILE"
  then
    echo " " >> $OUTFILE
    echo "***" >> $OUTFILE
    echo "***" >> $OUTFILE
    echo "Warning: rwhois.infosat.net was not working \
      as of 2005/02/02" >> $OUTFILE
    echo "         when this script was written." >> $OUTFILE
    echo "***" >> $OUTFILE
    echo "***" >> $OUTFILE
    echo " " >> $OUTFILE
    RWHOIS=`grep "^remarks: .*rwhois\.[^ ]\+" "$OUTFILE" | tail -n 1 |\
    sed "s/\(^.*\)\(rwhois\..*\)\(:4.*\)/\2/"`
    whois -h ${RWHOIS}:${PORT} "$IPADDR" >> $OUTFILE
  fi
}

APNICquery() {
  echo "Searching for $IPADDR in whois.apnic.net"
  whois -h whois.apnic.net "$IPADDR" > $OUTFILE

#  Just  about  every  country has its own internet registrar.
#  I don't normally bother consulting them, because the regional registry
#+ usually supplies sufficient information.
#  There are a few exceptions, where the regional registry simply
#+ refers to the national registry for direct data.
#  These are Japan and South Korea in APNIC, and Brasil in LACNIC.
#  The following if statement checks $OUTFILE (whois.txt) for the presence
#+ of "KR" (South Korea) or "JP" (Japan) in the country field.
#  If either is found, the query is re-run against the appropriate
#+ national registry.

  if grep -E "^country:[ ]+KR$" "$OUTFILE"
  then
    echo "Searching for $IPADDR in whois.krnic.net"
    whois -h whois.krnic.net "$IPADDR" >> $OUTFILE
  elif grep -E "^country:[ ]+JP$" "$OUTFILE"
  then
    echo "Searching for $IPADDR in whois.nic.ad.jp"
    whois -h whois.nic.ad.jp "$IPADDR"/e >> $OUTFILE
  fi
}

ARINquery() {
  echo "Searching for $IPADDR in whois.arin.net"
  whois -h whois.arin.net "$IPADDR" > $OUTFILE

#  Several large internet providers listed by ARIN have their own
#+ internal whois service, referred to as "rwhois".
#  A large block of IP addresses is listed with the provider
#+ under the ARIN registry.
#  To get the IP addresses of 2nd-level ISPs or other large customers,
#+ one has to refer to the rwhois server on port 4321.
#  I originally started with a bunch of "if" statements checking for
#+ the larger providers.
#  This approach is unwieldy, and there's always another rwhois server
#+ that I didn't know about.
#  A more elegant approach is to check $OUTFILE for a reference
#+ to a whois server, parse that server name out of the comment section,
#+ and re-run the query against the appropriate rwhois server.
#  The parsing looks a bit ugly, with a long continued line inside
#+ backticks.
#  But it only has to be done once, and will work as new servers are added.
#@   ABS Guide author comment: it isn't all that ugly, and is, in fact,
#@+  an instructive use of Regular Expressions.

  if grep -E "^Comment: .*rwhois.[^ ]+" "$OUTFILE"
  then
    RWHOIS=`grep -e "^Comment:.*rwhois\.[^ ]\+" "$OUTFILE" | tail -n 1 |\
    sed "s/^\(.*\)\(rwhois\.[^ ]\+\)\(.*$\)/\2/"`
    echo "Searching for $IPADDR in ${RWHOIS}"
    whois -h ${RWHOIS}:${PORT} "$IPADDR" >> $OUTFILE
  fi
}

LACNICquery() {
  echo "Searching for $IPADDR in whois.lacnic.net"
  whois -h whois.lacnic.net "$IPADDR" > $OUTFILE

#  The  following if statement checks $OUTFILE (whois.txt) for
#+ the presence of "BR" (Brasil) in the country field.
#  If it is found, the query is re-run against whois.registro.br.

  if grep -E "^country:[ ]+BR$" "$OUTFILE"
  then
    echo "Searching for $IPADDR in whois.registro.br"
    whois -h whois.registro.br "$IPADDR" >> $OUTFILE
  fi
}

RIPEquery() {
  echo "Searching for $IPADDR in whois.ripe.net"
  whois -h whois.ripe.net "$IPADDR" > $OUTFILE
}

#  Initialize a few variables.
#  * slash8 is the most significant octet
#  * slash16 consists of the two most significant octets
#  * octet2 is the second most significant octet




slash8=`echo $IPADDR | cut -d. -f 1`
  if [ -z "$slash8" ]  # Yet another sanity check.
  then
    echo "Undefined error!"
    exit $E_UNDEF
  fi
slash16=`echo $IPADDR | cut -d. -f 1-2`
#                             ^ Period specified as 'cut" delimiter.
  if [ -z "$slash16" ]
  then
    echo "Undefined error!"
    exit $E_UNDEF
  fi
octet2=`echo $slash16 | cut -d. -f 2`
  if [ -z "$octet2" ]
  then
    echo "Undefined error!"
    exit $E_UNDEF
  fi


#  Check for various odds and ends of reserved space.
#  There is no point in querying for those addresses.

if [ $slash8 == 0 ]; then
  echo $IPADDR is '"This Network"' space\; Not querying
elif [ $slash8 == 10 ]; then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash8 == 14 ]; then
  echo $IPADDR is '"Public Data Network"' space\; Not querying
elif [ $slash8 == 127 ]; then
  echo $IPADDR is loopback space\; Not querying
elif [ $slash16 == 169.254 ]; then
  echo $IPADDR is link-local space\; Not querying
elif [ $slash8 == 172 ] && [ $octet2 -ge 16 ] && [ $octet2 -le 31 ];then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash16 == 192.168 ]; then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash8 -ge 224 ]; then
  echo $IPADDR is either Multicast or reserved space\; Not querying
elif [ $slash8 -ge 200 ] && [ $slash8 -le 201 ]; then LACNICquery "$IPADDR"
elif [ $slash8 -ge 202 ] && [ $slash8 -le 203 ]; then APNICquery "$IPADDR"
elif [ $slash8 -ge 210 ] && [ $slash8 -le 211 ]; then APNICquery "$IPADDR"
elif [ $slash8 -ge 218 ] && [ $slash8 -le 223 ]; then APNICquery "$IPADDR"

#  If we got this far without making a decision, query ARIN.
#  If a reference is found in $OUTFILE to APNIC, AFRINIC, LACNIC, or RIPE,
#+ query the appropriate whois server.

else
  ARINquery "$IPADDR"
  if grep "whois.afrinic.net" "$OUTFILE"; then
    AFRINICquery "$IPADDR"
  elif grep -E "^OrgID:[ ]+RIPE$" "$OUTFILE"; then
    RIPEquery "$IPADDR"
  elif grep -E "^OrgID:[ ]+APNIC$" "$OUTFILE"; then
    APNICquery "$IPADDR"
  elif grep -E "^OrgID:[ ]+LACNIC$" "$OUTFILE"; then
    LACNICquery "$IPADDR"
  fi
fi

#@  ---------------------------------------------------------------
#   Try also:
#   wget http://logi.cc/nw/whois.php3?ACTION=doQuery&DOMAIN=$IPADDR
#@  ---------------------------------------------------------------

#  We've  now  finished  the querying.
#  Echo a copy of the final result to the screen.

cat $OUTFILE
# Or "less $OUTFILE" . . .


exit 0

#@  ABS Guide author comments:
#@  Nothing fancy here, but still a very useful tool for hunting spammers.
#@  Sure, the script can be cleaned up some, and it's still a bit buggy,
#@+ (exercise for reader), but all the same, it's a nice piece of coding
#@+ by Walter Dnes.
#@  Thank you!
```

As the ABS Guide says, it's "still a bit buggy". Today's bash doesn't
run it at all: the unquoted `;;` in `[[ "$IPADDR" =~ ^[;;] ]]` is a
syntax error that ends the script at line 60. With that quoted, it
works for addresses, but not well for names: `awk '{print $4}'` takes
the fourth word of *every* line `host` prints, so `IPADDR` is the address
plus words like `address` and `handled`; and a name that isn't found
gives `found:` (from `Host x not found: 3(NXDOMAIN)`), which none of the
checks catch, since they test `IPADDR` and their brackets make character
classes. The tessh port takes the last field of the first `has address`
line, and looks for the timeout and `NXDOMAIN`/`SERVFAIL` in what `host`
said. The reserved-space `if` chain is a `match` on the first two
octets, with guards for the ranges; `int` failing is the "undefined
error"; `grep`, `sed` and `whois` run as they do in bash.

```tessh no-run
#!/usr/bin/env tessh
# whx.tsh: "whois" spammer lookup
# Author: Walter Dnes
# Slight revisions (first section) by ABS Guide author.
# Used in ABS Guide with permission.
# (Ported to tessh from whx.sh.)

E_BADARGS = 85        # Missing command-line arg.
E_NOHOST = 86         # Host not found.
E_TIMEOUT = 87        # Host lookup timed out.
E_UNDEF = 88          # Some other (undefined) error.

HOSTWAIT = 10         # Specify up to 10 seconds for host query reply.
                      # The actual wait may be a bit longer.
OUTFILE = Path whois.txt    # Output file.
PORT = 4321

if (len $argv) < 2 or not $argv[1]    # Check for (required) command-line arg.
  print "Usage: $argv[0] domain name or IP address"
  exit $E_BADARGS
arg = $argv[1]

reply = ""
if $arg =~ re'[a-zA-Z][a-zA-Z]$'      # Ends in two alpha chars?
  # It's a domain name && must do host lookup to get IP address:
  # the last field of host's first "has address" line.
  reply = host -W $HOSTWAIT $arg
  addrs = [($l.split)[-1] for l in $reply.lines if ' has address ' in $l]
  IPADDR = if $addrs then $addrs[0] else ""
else
  IPADDR = $arg                       # Command-line arg was IP address.

print ; print "IP Address is: $IPADDR" ; print

if ($OUTFILE.exists)
  $OUTFILE.unlink
  print "Stale output file \"$OUTFILE\" removed." ; print

# Sanity checks, on what host said.
# (This section needs more work.)
# ===============================
if $reply =~ re'^;;'m
  # ;; connection timed out; no servers could be reached
  print "Host lookup timed out!"
  exit $E_TIMEOUT                     # Bail out.

if $reply =~ re'\((NXDOMAIN|SERVFAIL)\)$'m
  # Host xxxxxxxxx.xxx not found: 3(NXDOMAIN)
  # Host xxxxxxxxx.xxx not found: 2(SERVFAIL)
  print "Host not found!"
  exit $E_NOHOST                      # Bail out.

if not $IPADDR                        # No response.
  print "Host not found!"
  exit $E_NOHOST                      # Bail out.

# ======================== Main body of script ========================

def AFRINICquery ip
  # Echo a notification to the screen, and then run the actual query,
  # redirecting output to $OUTFILE.
  print "Searching for $ip in whois.afrinic.net"
  whois -h whois.afrinic.net $ip > $OUTFILE

  # Check for presence of reference to an rwhois.
  # Warn about non-functional rwhois.infosat.net server
  # and attempt rwhois query.
  if grep -e '^remarks: .*rwhois\.[^ ]\+' $OUTFILE
    with (open $OUTFILE a) as out
      print " " "***" "***" sep="\n" file=$out
      print "Warning: rwhois.infosat.net was not working as of 2005/02/02" file=$out
      print "         when this script was written." file=$out
      print "***" "***" " " sep="\n" file=$out
    RWHOIS = grep '^remarks: .*rwhois\.[^ ]\+' $OUTFILE | tail -n 1 | \
             sed 's/\(^.*\)\(rwhois\..*\)\(:4.*\)/\2/'
    whois -h "$RWHOIS:$PORT" $ip >> $OUTFILE

def APNICquery ip
  print "Searching for $ip in whois.apnic.net"
  whois -h whois.apnic.net $ip > $OUTFILE

  # Just about every country has its own internet registrar.
  # The regional registry usually supplies sufficient information, with a
  # few exceptions, where it simply refers to the national registry:
  # Japan and South Korea in APNIC, and Brasil in LACNIC.
  # If "KR" or "JP" is in the country field, re-run the query against
  # the national registry.
  if grep -E '^country:[ ]+KR$' $OUTFILE
    print "Searching for $ip in whois.krnic.net"
    whois -h whois.krnic.net $ip >> $OUTFILE
  elif grep -E '^country:[ ]+JP$' $OUTFILE
    print "Searching for $ip in whois.nic.ad.jp"
    whois -h whois.nic.ad.jp "$ip/e" >> $OUTFILE

def ARINquery ip
  print "Searching for $ip in whois.arin.net"
  whois -h whois.arin.net $ip > $OUTFILE

  # Several large internet providers listed by ARIN have their own
  # internal whois service, referred to as "rwhois", on port 4321.
  # Rather than a bunch of "if" statements for the larger providers,
  # check $OUTFILE for a reference to a whois server, parse that server
  # name out of the comment section, and re-run the query against it.
  # It will work as new servers are added.
  if grep -E '^Comment: .*rwhois.[^ ]+' $OUTFILE
    RWHOIS = grep -e '^Comment:.*rwhois\.[^ ]\+' $OUTFILE | tail -n 1 | \
             sed 's/^\(.*\)\(rwhois\.[^ ]\+\)\(.*$\)/\2/'
    print "Searching for $ip in $RWHOIS"
    whois -h "$RWHOIS:$PORT" $ip >> $OUTFILE

def LACNICquery ip
  print "Searching for $ip in whois.lacnic.net"
  whois -h whois.lacnic.net $ip > $OUTFILE

  # If "BR" (Brasil) is in the country field, re-run the query against
  # whois.registro.br.
  if grep -E '^country:[ ]+BR$' $OUTFILE
    print "Searching for $ip in whois.registro.br"
    whois -h whois.registro.br $ip >> $OUTFILE

def RIPEquery ip
  print "Searching for $ip in whois.ripe.net"
  whois -h whois.ripe.net $ip > $OUTFILE

# The two most significant octets, as numbers.
# (Yet another sanity check: not numbers is an undefined error.)
octets = $IPADDR.split "."
try
  slash8 = int $octets[0]
  octet2 = int $octets[1]
except (ValueError, IndexError)
  print "Undefined error!"
  exit $E_UNDEF

# Check for various odds and ends of reserved space.
# There is no point in querying for those addresses.
match [$slash8 $octet2]
  case [0 _]
    print "$IPADDR is \"This Network\" space; Not querying"
  case [10 _]
    print "$IPADDR is RFC1918 space; Not querying"
  case [14 _]
    print "$IPADDR is \"Public Data Network\" space; Not querying"
  case [127 _]
    print "$IPADDR is loopback space; Not querying"
  case [169 254]
    print "$IPADDR is link-local space; Not querying"
  case [172 $o] if 16 <= $o <= 31
    print "$IPADDR is RFC1918 space; Not querying"
  case [192 168]
    print "$IPADDR is RFC1918 space; Not querying"
  case [$s _] if $s >= 224
    print "$IPADDR is either Multicast or reserved space; Not querying"
  case [$s _] if 200 <= $s <= 201
    LACNICquery $IPADDR
  case [$s _] if 202 <= $s <= 203 or 210 <= $s <= 211 or 218 <= $s <= 223
    APNICquery $IPADDR
  case _
    # If we got this far without making a decision, query ARIN.
    # If a reference is found in $OUTFILE to APNIC, AFRINIC, LACNIC,
    # or RIPE, query the appropriate whois server.
    ARINquery $IPADDR
    if grep whois.afrinic.net $OUTFILE
      AFRINICquery $IPADDR
    elif grep -E '^OrgID:[ ]+RIPE$' $OUTFILE
      RIPEquery $IPADDR
    elif grep -E '^OrgID:[ ]+APNIC$' $OUTFILE
      APNICquery $IPADDR
    elif grep -E '^OrgID:[ ]+LACNIC$' $OUTFILE
      LACNICquery $IPADDR

#   Try also:
#   wget http://logi.cc/nw/whois.php3?ACTION=doQuery&DOMAIN=$IPADDR

# We've now finished the querying.
# Echo a copy of the final result to the screen.
if ($OUTFILE.exists)
  cat $OUTFILE          # Or "less $OUTFILE" . . .

exit 0

#@  ABS Guide author comments:
#@  Nothing fancy here, but still a very useful tool for hunting spammers.
#@  Sure, the script can be cleaned up some, and it's still a bit buggy,
#@+ (exercise for reader), but all the same, it's a nice piece of coding
#@+ by Walter Dnes.
#@  Thank you!
```

Run against a stand-in `host` and `whois`, it printed what the bash (with
the `;;` quoted) printed for addresses, and the right address for names.
Its parts on made-up data: the choice of registry,

```bash
# (whx.sh's choice of registry, for made-up addresses; the queries only
#+ say which they'd be.)
LACNICquery() { echo "$1: ask LACNIC"; }
APNICquery()  { echo "$1: ask APNIC"; }
ARINquery()   { echo "$1: ask ARIN, then the registry it refers to"; }

for IPADDR in 0.1.2.3 10.9.8.7 172.20.1.1 172.40.1.1 169.254.3.4 \
              192.168.0.1 230.1.2.3 200.1.2.3 211.1.2.3 64.1.2.3
do
slash8=`echo $IPADDR | cut -d. -f 1`
slash16=`echo $IPADDR | cut -d. -f 1-2`
octet2=`echo $slash16 | cut -d. -f 2`

if [ $slash8 == 0 ]; then
  echo $IPADDR is '"This Network"' space\; Not querying
elif [ $slash8 == 10 ]; then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash8 == 14 ]; then
  echo $IPADDR is '"Public Data Network"' space\; Not querying
elif [ $slash8 == 127 ]; then
  echo $IPADDR is loopback space\; Not querying
elif [ $slash16 == 169.254 ]; then
  echo $IPADDR is link-local space\; Not querying
elif [ $slash8 == 172 ] && [ $octet2 -ge 16 ] && [ $octet2 -le 31 ];then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash16 == 192.168 ]; then
  echo $IPADDR is RFC1918 space\; Not querying
elif [ $slash8 -ge 224 ]; then
  echo $IPADDR is either Multicast or reserved space\; Not querying
elif [ $slash8 -ge 200 ] && [ $slash8 -le 201 ]; then LACNICquery "$IPADDR"
elif [ $slash8 -ge 202 ] && [ $slash8 -le 203 ]; then APNICquery "$IPADDR"
elif [ $slash8 -ge 210 ] && [ $slash8 -le 211 ]; then APNICquery "$IPADDR"
elif [ $slash8 -ge 218 ] && [ $slash8 -le 223 ]; then APNICquery "$IPADDR"
else
  ARINquery "$IPADDR"
fi
done
```

```tessh
# (whx's choice of registry, for made-up addresses; the queries only
# say which they'd be.)
def LACNICquery ip = print "$ip: ask LACNIC"
def APNICquery ip = print "$ip: ask APNIC"
def ARINquery ip = print "$ip: ask ARIN, then the registry it refers to"

def registry IPADDR
  octets = $IPADDR.split "."
  slash8 = int $octets[0]
  octet2 = int $octets[1]
  match [$slash8 $octet2]
    case [0 _]
      print "$IPADDR is \"This Network\" space; Not querying"
    case [10 _]
      print "$IPADDR is RFC1918 space; Not querying"
    case [14 _]
      print "$IPADDR is \"Public Data Network\" space; Not querying"
    case [127 _]
      print "$IPADDR is loopback space; Not querying"
    case [169 254]
      print "$IPADDR is link-local space; Not querying"
    case [172 $o] if 16 <= $o <= 31
      print "$IPADDR is RFC1918 space; Not querying"
    case [192 168]
      print "$IPADDR is RFC1918 space; Not querying"
    case [$s _] if $s >= 224
      print "$IPADDR is either Multicast or reserved space; Not querying"
    case [$s _] if 200 <= $s <= 201
      LACNICquery $IPADDR
    case [$s _] if 202 <= $s <= 203 or 210 <= $s <= 211 or 218 <= $s <= 223
      APNICquery $IPADDR
    case _
      ARINquery $IPADDR

for ip in [0.1.2.3 10.9.8.7 172.20.1.1 172.40.1.1 169.254.3.4
           192.168.0.1 230.1.2.3 200.1.2.3 211.1.2.3 64.1.2.3]
  registry $ip
```

```output
0.1.2.3 is "This Network" space; Not querying
10.9.8.7 is RFC1918 space; Not querying
172.20.1.1 is RFC1918 space; Not querying
172.40.1.1: ask ARIN, then the registry it refers to
169.254.3.4 is link-local space; Not querying
192.168.0.1 is RFC1918 space; Not querying
230.1.2.3 is either Multicast or reserved space; Not querying
200.1.2.3: ask LACNIC
211.1.2.3: ask APNIC
64.1.2.3: ask ARIN, then the registry it refers to
```

reading `host`'s reply (the bash first, showing its trouble),

```bash
# (What `host` says, made up, and whx.sh's way of getting the address
#+ from it: the fourth field of each line.)
for reply in \
'spam.example.co has address 202.1.2.3
spam.example.co has IPv6 address 2001:db8::1
spam.example.co mail is handled by 10 mx.example.co.' \
'Host nothere.example.co not found: 3(NXDOMAIN)'
do
  IPADDR=$(echo "$reply" | awk '{print $4}')
  echo "IP Address is: "$IPADDR""
  if [[ "$IPADDR" =~ [(NXDOMAIN)]$ ]]
  then
    echo "Host not found!"
  fi
done
```

```output bash
IP Address is: 202.1.2.3 address handled
IP Address is: found:
```

```tessh
# (What `host` says, made up, and the address taken from it: the last
# field of the first "has address" line. The checks look at the reply.)
for reply in ['spam.example.co has address 202.1.2.3
spam.example.co has IPv6 address 2001:db8::1
spam.example.co mail is handled by 10 mx.example.co.'
              'Host nothere.example.co not found: 3(NXDOMAIN)'
              ';; connection timed out; no servers could be reached']
  addrs = [($l.split)[-1] for l in ($reply.splitlines) if ' has address ' in $l]
  IPADDR = if $addrs then $addrs[0] else ""
  print "IP Address is: $IPADDR"
  if $reply =~ re'^;;'m
    print "Host lookup timed out!"
  elif $reply =~ re'\((NXDOMAIN|SERVFAIL)\)$'m
    print "Host not found!"
```

```output
IP Address is: 202.1.2.3
IP Address is: 
Host not found!
IP Address is: 
Host lookup timed out!
```

and finding the rwhois server an ARIN reply refers to, the same pipeline
in both, and then with a regex in tessh. (`grep` without `-q` shows the
lines it found, as in the script.)

```bash
# (A made-up ARIN reply that refers to an rwhois server.)
OUTFILE=whois.txt
cat >$OUTFILE <<'END'
OrgName:    Example Hosting
OrgID:      EXHO
Comment:    Customer details are in our rwhois server,
Comment:    rwhois.example.net port 4321.
END

if grep -E "^Comment: .*rwhois.[^ ]+" "$OUTFILE"
then
  RWHOIS=`grep -e "^Comment:.*rwhois\.[^ ]\+" "$OUTFILE" | tail -n 1 |\
  sed "s/^\(.*\)\(rwhois\.[^ ]\+\)\(.*$\)/\2/"`
  echo "Searching in ${RWHOIS}"
fi
```

```output bash
Comment:    Customer details are in our rwhois server,
Comment:    rwhois.example.net port 4321.
Searching in rwhois.example.net
```

```tessh
# (A made-up ARIN reply that refers to an rwhois server.)
OUTFILE = Path whois.txt
cat >$OUTFILE <<'END'
OrgName:    Example Hosting
OrgID:      EXHO
Comment:    Customer details are in our rwhois server,
Comment:    rwhois.example.net port 4321.
END

if grep -E '^Comment: .*rwhois.[^ ]+' $OUTFILE
  RWHOIS = grep -e '^Comment:.*rwhois\.[^ ]\+' $OUTFILE | tail -n 1 | \
           sed 's/^\(.*\)\(rwhois\.[^ ]\+\)\(.*$\)/\2/'
  print "Searching in $RWHOIS"

# The same in Python: the server named on the last Comment line.
comments = [$l for l in (($OUTFILE.read_text).splitlines) if $l =~ re'^Comment:.*rwhois\.\S']
print "Searching in $(($comments[-1] =~ re'rwhois\.\S+').group)"
```

```output
Comment:    Customer details are in our rwhois server,
Comment:    rwhois.example.net port 4321.
Searching in rwhois.example.net
Searching in rwhois.example.net
```

## A friendlier wget
<!-- help: abs-wgetter2 wgetter2 wget download getopts front-end cookies -->

Little Monster's `wgetter2.bash` is a front end to `wget`: it builds a
long `wget` command from a few questions (how deep to go, which page
refers, whether to log to the terminal), and either runs it or saves it
to a file to run later (`-r file`, or `-i` to be asked for one). It
remembers which cookie file to use in a configuration file,
`~/.wgetter2rc`, that it `source`s. It needs the network and a terminal.

### Example A-30. Making *wget* easier to use (`wgetter2.bash`)

```bash no-run
#!/bin/bash
# wgetter2.bash

# Author: Little Monster [monster@monstruum.co.uk]
# ==> Used in ABS Guide with permission of script author.
# ==> This script still needs debugging and fixups (exercise for reader).
# ==> It could also use some additional editing in the comments.


#  This is wgetter2 --
#+ a Bash script to make wget a bit more friendly, and save typing.

#  Carefully crafted by Little Monster.
#  More or less complete on 02/02/2005.
#  If you think this script can be improved,
#+ email me at: monster@monstruum.co.uk
# ==> and cc: to the author of the ABS Guide, please.
#  This script is licenced under the GPL.
#  You are free to copy, alter and re-use it,
#+ but please don't try to claim you wrote it.
#  Log your changes here instead.

# =======================================================================
# changelog:

# 07/02/2005.  Fixups by Little Monster.
# 02/02/2005.  Minor additions by Little Monster.
#              (See after # +++++++++++ )
# 29/01/2005.  Minor stylistic edits and cleanups by author of ABS Guide.
#              Added exit error codes.
# 22/11/2004.  Finished initial version of second version of wgetter:
#              wgetter2 is born.
# 01/12/2004.  Changed 'runn' function so it can be run 2 ways --
#              either ask for a file name or have one input on the CL.
# 01/12/2004.  Made sensible handling of no URL's given.
# 01/12/2004.  Made loop of main options, so you don't
#              have to keep calling wgetter 2 all the time.
#              Runs as a session instead.
# 01/12/2004.  Added looping to 'runn' function.
#              Simplified and improved.
# 01/12/2004.  Added state to recursion setting.
#              Enables re-use of previous value.
# 05/12/2004.  Modified the file detection routine in the 'runn' function
#              so it's not fooled by empty values, and is cleaner.
# 01/02/2004.  Added cookie finding routine from later version (which 
#              isn't ready yet), so as not to have hard-coded paths.
# =======================================================================

# Error codes for abnormal exit.
E_USAGE=67        # Usage message, then quit.
E_NO_OPTS=68      # No command-line args entered.
E_NO_URLS=69      # No URLs passed to script.
E_NO_SAVEFILE=70  # No save filename passed to script.
E_USER_EXIT=71    # User decides to quit.


#  Basic default wget command we want to use.
#  This is the place to change it, if required.
#  NB: if using a proxy, set http_proxy = yourproxy in .wgetrc.
#  Otherwise delete --proxy=on, below.
# ====================================================================
CommandA="wget -nc -c -t 5 --progress=bar --random-wait --proxy=on -r"
# ====================================================================



# --------------------------------------------------------------------
# Set some other variables and explain them.

pattern=" -A .jpg,.JPG,.jpeg,.JPEG,.gif,.GIF,.htm,.html,.shtml,.php"
                    # wget's option to only get certain types of file.
                    # comment out if not using
today=`date +%F`    # Used for a filename.
home=$HOME          # Set HOME to an internal variable.
                    # In case some other path is used, change it here.
depthDefault=3      # Set a sensible default recursion.
Depth=$depthDefault # Otherwise user feedback doesn't tie in properly.
RefA=""             # Set blank referring page.
Flag=""             #  Default to not saving anything,
                    #+ or whatever else might be wanted in future.
lister=""           # Used for passing a list of urls directly to wget.
Woptions=""         # Used for passing wget some options for itself.
inFile=""           # Used for the run function.
newFile=""          # Used for the run function.
savePath="$home/w-save"
Config="$home/.wgetter2rc"
                    #  This is where some variables can be stored, 
                    #+ if permanently changed from within the script.
Cookie_List="$home/.cookielist"
                    # So we know where the cookies are kept . . .
cFlag=""            # Part of the cookie file selection routine.

# Define the options available. Easy to change letters here if needed.
# These are the optional options; you don't just wait to be asked.

save=s   # Save command instead of executing it.
cook=c   # Change cookie file for this session.
help=h   # Usage guide.
list=l   # Pass wget the -i option and URL list.
runn=r   # Run saved commands as an argument to the option.
inpu=i   # Run saved commands interactively.
wopt=w   # Allow to enter options to pass directly to wget.
# --------------------------------------------------------------------


if [ -z "$1" ]; then   # Make sure we get something for wget to eat.
   echo "You must at least enter a URL or option!"
   echo "-$help for usage."
   exit $E_NO_OPTS
fi



# +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
# added added added added added added added added added added added added

if [ ! -e "$Config" ]; then   # See if configuration file exists.
   echo "Creating configuration file, $Config"
   echo "# This is the configuration file for wgetter2" > "$Config"
   echo "# Your customised settings will be saved in this file" >> "$Config"
else
   source $Config             # Import variables we set outside the script.
fi

if [ ! -e "$Cookie_List" ]; then
   # Set up a list of cookie files, if there isn't one.
   echo "Hunting for cookies . . ."
   find -name cookies.txt >> $Cookie_List # Create the list of cookie files.
fi #  Isolate this in its own 'if' statement,
   #+ in case we got interrupted while searching.

if [ -z "$cFlag" ]; then # If we haven't already done this . . .
   echo                  # Make a nice space after the command prompt.
   echo "Looks like you haven't set up your source of cookies yet."
   n=0                   #  Make sure the counter
                         #+ doesn't contain random values.
   while read; do
      Cookies[$n]=$REPLY # Put the cookie files we found into an array.
      echo "$n) ${Cookies[$n]}"  # Create a menu.
      n=$(( n + 1 ))     # Increment the counter.
   done < $Cookie_List   # Feed the read statement.
   echo "Enter the number of the cookie file you want to use."
   echo "If you won't be using cookies, just press RETURN."
   echo
   echo "I won't be asking this again. Edit $Config"
   echo "If you decide to change at a later date"
   echo "or use the -${cook} option for per session changes."
   read
   if [ ! -z $REPLY ]; then   # User didn't just press return.
      Cookie=" --load-cookies ${Cookies[$REPLY]}"
      # Set the variable here as well as in the config file.

      echo "Cookie=\" --load-cookies ${Cookies[$REPLY]}\"" >> $Config
   fi
   echo "cFlag=1" >> $Config  # So we know not to ask again.
fi

# end added section end added section end added section end added section
# +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++



# Another variable.
# This one may or may not be subject to variation.
# A bit like the small print.
CookiesON=$Cookie
# echo "cookie file is $CookiesON" # For debugging.
# echo "home is ${home}"           # For debugging.
                                   # Got caught with this one!


wopts()
{
echo "Enter options to pass to wget."
echo "It is assumed you know what you're doing."
echo
echo "You can pass their arguments here too."
# That is to say, everything passed here is passed to wget.

read Wopts
# Read in the options to be passed to wget.

Woptions=" $Wopts"
#         ^  Why the leading space?
# Assign to another variable.
# Just for fun, or something . . .

echo "passing options ${Wopts} to wget"
# Mainly for debugging.
# Is cute.

return
}


save_func()
{
echo "Settings will be saved."
if [ ! -d $savePath ]; then  #  See if directory exists.
   mkdir $savePath           #  Create the directory to save things in
                             #+ if it isn't already there.
fi

Flag=S
# Tell the final bit of code what to do.
# Set a flag since stuff is done in main.

return
}


usage() # Tell them how it works.
{
    echo "Welcome to wgetter.  This is a front end to wget."
    echo "It will always run wget with these options:"
    echo "$CommandA"
    echo "and the pattern to match: $pattern \
(which you can change at the top of this script)."
    echo "It will also ask you for recursion depth, \
and if you want to use a referring page."
    echo "Wgetter accepts the following options:"
    echo ""
    echo "-$help : Display this help."
    echo "-$save : Save the command to a file $savePath/wget-($today) \
instead of running it."
    echo "-$runn : Run saved wget commands instead of starting a new one -"
    echo "Enter filename as argument to this option."
    echo "-$inpu : Run saved wget commands interactively --"
    echo "The script will ask you for the filename."
    echo "-$cook : Change the cookies file for this session."
    echo "-$list : Tell wget to use URL's from a list instead of \
from the command-line."
    echo "-$wopt : Pass any other options direct to wget."
    echo ""
    echo "See the wget man page for additional options \
you can pass to wget."
    echo ""

    exit $E_USAGE  # End here. Don't process anything else.
}



list_func() #  Gives the user the option to use the -i option to wget,
            #+ and a list of URLs.
{
while [ 1 ]; do
   echo "Enter the name of the file containing URL's (press q to change
your mind)."
   read urlfile
   if [ ! -e "$urlfile" ] && [ "$urlfile" != q ]; then
       # Look for a file, or the quit option.
       echo "That file does not exist!"
   elif [ "$urlfile" = q ]; then # Check quit option.
       echo "Not using a url list."
       return
   else
      echo "using $urlfile."
      echo "If you gave url's on the command-line, I'll use those first."
                            # Report wget standard behaviour to the user.
      lister=" -i $urlfile" # This is what we want to pass to wget.
      return
   fi
done
}


cookie_func() # Give the user the option to use a different cookie file.
{
while [ 1 ]; do
   echo "Change the cookies file. Press return if you don't want to change 
it."
   read Cookies
   # NB: this is not the same as Cookie, earlier.
   # There is an 's' on the end.
   # Bit like chocolate chips.
   if [ -z "$Cookies" ]; then                 # Escape clause for wusses.
      return
   elif [ ! -e "$Cookies" ]; then
      echo "File does not exist.  Try again." # Keep em going . . .
   else
       CookiesON=" --load-cookies $Cookies"   # File is good -- use it!
       return
   fi
done
}



run_func()
{
if [ -z "$OPTARG" ]; then
# Test to see if we used the in-line option or the query one.
   if [ ! -d "$savePath" ]; then      # If directory doesn't exist . . .
      echo "$savePath does not appear to exist."
      echo "Please supply path and filename of saved wget commands:"
      read newFile
         until [ -f "$newFile" ]; do  # Keep going till we get something.
            echo "Sorry, that file does not exist.  Please try again."
            # Try really hard to get something.
            read newFile
         done


# -----------------------------------------------------------------------
#       if [ -z ( grep wget ${newfile} ) ]; then
        # Assume they haven't got the right file and bail out.
#       echo "Sorry, that file does not contain wget commands.  Aborting."
#       exit
#       fi
#
# This is bogus code.
# It doesn't actually work.
# If anyone wants to fix it, feel free!
# -----------------------------------------------------------------------


      filePath="${newFile}"
   else
   echo "Save path is $savePath"
     echo "Please enter name of the file which you want to use."
     echo "You have a choice of:"
     ls $savePath                                    # Give them a choice.
     read inFile
       until [ -f "$savePath/$inFile" ]; do         #  Keep going till
                                                    #+ we get something.
          if [ ! -f "${savePath}/${inFile}" ]; then # If file doesn't exist.
             echo "Sorry, that file does not exist.  Please choose from:"
             ls $savePath                           # If a mistake is made.
             read inFile
          fi
         done
      filePath="${savePath}/${inFile}"  # Make one variable . . .
   fi
else filePath="${savePath}/${OPTARG}"   # Which can be many things . . .
fi

if [ ! -f "$filePath" ]; then           # If a bogus file got through.
   echo "You did not specify a suitable file."
   echo "Run this script with the -${save} option first."
   echo "Aborting."
   exit $E_NO_SAVEFILE
fi
echo "Using: $filePath"
while read; do
    eval $REPLY
    echo "Completed: $REPLY"
done < $filePath  # Feed the actual file we are using into a 'while' loop.

exit
}



# Fish out any options we are using for the script.
# This is based on the demo in "Learning The Bash Shell" (O'Reilly).
while getopts ":$save$cook$help$list$runn:$inpu$wopt" opt
do
  case $opt in
     $save) save_func;;   #  Save some wgetter sessions for later.
     $cook) cookie_func;; #  Change cookie file.
     $help) usage;;       #  Get help.
     $list) list_func;;   #  Allow wget to use a list of URLs.
     $runn) run_func;;    #  Useful if you are calling wgetter from,
                          #+ for example, a cron script.
     $inpu) run_func;;    #  When you don't know what your files are named.
     $wopt) wopts;;       #  Pass options directly to wget.
        \?) echo "Not a valid option."
            echo "Use -${wopt} to pass options directly to wget,"
            echo "or -${help} for help";;      # Catch anything else.
  esac
done
shift $((OPTIND - 1))     # Do funky magic stuff with $#.


if [ -z "$1" ] && [ -z "$lister" ]; then
                          #  We should be left with at least one URL
                          #+ on the command-line, unless a list is 
			  #+ being used -- catch empty CL's.
   echo "No URL's given! You must enter them on the same line as wgetter2."
   echo "E.g.,  wgetter2 http://somesite http://anothersite."
   echo "Use $help option for more information."
   exit $E_NO_URLS        # Bail out, with appropriate error code.
fi

URLS=" $@"
# Use this so that URL list can be changed if we stay in the option loop.

while [ 1 ]; do
   # This is where we ask for the most used options.
   # (Mostly unchanged from version 1 of wgetter)
   if [ -z $curDepth ]; then
      Current=""
   else Current=" Current value is $curDepth"
   fi
       echo "How deep should I go? \
(integer: Default is $depthDefault.$Current)"
       read Depth   # Recursion -- how far should we go?
       inputB=""    # Reset this to blank on each pass of the loop.
       echo "Enter the name of the referring page (default is none)."
       read inputB  # Need this for some sites.

       echo "Do you want to have the output logged to the terminal"
       echo "(y/n, default is yes)?"
       read noHide  # Otherwise wget will just log it to a file.

       case $noHide in    # Now you see me, now you don't.
          y|Y ) hide="";;
          n|N ) hide=" -b";;
            * ) hide="";;
       esac

       if [ -z ${Depth} ]; then
       #  User accepted either default or current depth,
       #+ in which case Depth is now empty.
          if [ -z ${curDepth} ]; then
          #  See if a depth was set on a previous iteration.
             Depth="$depthDefault"
             #  Set the default recursion depth if nothing
             #+ else to use.
          else Depth="$curDepth" #  Otherwise, set the one we used before.
          fi
       fi
   Recurse=" -l $Depth"          # Set how deep we want to go.
   curDepth=$Depth               # Remember setting for next time.

       if [ ! -z $inputB ]; then
          RefA=" --referer=$inputB"   # Option to use referring page.
       fi

   WGETTER="${CommandA}${pattern}${hide}${RefA}${Recurse}\
${CookiesON}${lister}${Woptions}${URLS}"
   #  Just string the whole lot together . . .
   #  NB: no embedded spaces.
   #  They are in the individual elements so that if any are empty,
   #+ we don't get an extra space.

   if [ -z "${CookiesON}" ] && [ "$cFlag" = "1" ] ; then
       echo "Warning -- can't find cookie file"
       #  This should be changed,
       #+ in case the user has opted to not use cookies.
   fi

   if [ "$Flag" = "S" ]; then
      echo "$WGETTER" >> $savePath/wget-${today}
      #  Create a unique filename for today, or append to it if it exists.
      echo "$inputB" >> $savePath/site-list-${today}
      #  Make a list, so it's easy to refer back to,
      #+ since the whole command is a bit confusing to look at.
      echo "Command saved to the file $savePath/wget-${today}"
           # Tell the user.
      echo "Referring page URL saved to the file$ \
savePath/site-list-${today}"
           # Tell the user.
      Saver=" with save option"
      # Stick this somewhere, so it appears in the loop if set.
   else
       echo "*****************"
       echo "*****Getting*****"
       echo "*****************"
       echo ""
       echo "$WGETTER"
       echo ""
       echo "*****************"
       eval "$WGETTER"
   fi

       echo ""
       echo "Starting over$Saver."
       echo "If you want to stop, press q."
       echo "Otherwise, enter some URL's:"
       # Let them go again. Tell about save option being set.

       read
       case $REPLY in
       # Need to change this to a 'trap' clause.
          q|Q ) exit $E_USER_EXIT;;  # Exercise for the reader?
            * ) URLS=" $REPLY";;
       esac

       echo ""
done


exit 0
```

In tessh the script keeps its shape: the same globals, functions and
messages. `read` is `input` (tessh's `read` would do too); the option letters live on one namespace,
`opt`, so `$opt.save` reads like bash's `$save` without the letters
taking names such as `list` and `help` from Python, and a `match` case
`opt.save` compares with it. Python has no `getopts` that goes on past an
option it doesn't know (`getopt.getopt` stops with an error), so the port
has a short one of its own that gives what bash's does, `?` and all.
(tessh's own `getopts` takes bash's arguments too; the port shows the
Python way.)
The configuration file holds lines such as `cFlag=1` and
`Cookie=" --load-cookies …"`, which are tessh as well as bash, so
`source` reads it as before. Where the bash `eval`s a saved command line,
tessh splits it as the shell would with `shlex.split` and runs the words
with `env`, since a program named by a value can't head a command.

```tessh no-run
#!/usr/bin/env tessh
# wgetter2.tsh (ported from wgetter2.bash)

# Author: Little Monster [monster@monstruum.co.uk]
# ==> Used in ABS Guide with permission of script author.
# ==> This script still needs debugging and fixups (exercise for reader).
# ==> It could also use some additional editing in the comments.

#  This is wgetter2 --
#+ a script to make wget a bit more friendly, and save typing.

#  Carefully crafted by Little Monster.
#  More or less complete on 02/02/2005.
#  If you think this script can be improved,
#+ email me at: monster@monstruum.co.uk
# ==> and cc: to the author of the ABS Guide, please.
#  This script is licenced under the GPL.
#  You are free to copy, alter and re-use it,
#+ but please don't try to claim you wrote it.
#  Log your changes here instead.

# =======================================================================
# changelog:

# 07/02/2005.  Fixups by Little Monster.
# 02/02/2005.  Minor additions by Little Monster.
#              (See after # +++++++++++ )
# 29/01/2005.  Minor stylistic edits and cleanups by author of ABS Guide.
#              Added exit error codes.
# 22/11/2004.  Finished initial version of second version of wgetter:
#              wgetter2 is born.
# 01/12/2004.  Changed 'runn' function so it can be run 2 ways --
#              either ask for a file name or have one input on the CL.
# 01/12/2004.  Made sensible handling of no URL's given.
# 01/12/2004.  Made loop of main options, so you don't
#              have to keep calling wgetter 2 all the time.
#              Runs as a session instead.
# 01/12/2004.  Added looping to 'runn' function.
#              Simplified and improved.
# 01/12/2004.  Added state to recursion setting.
#              Enables re-use of previous value.
# 05/12/2004.  Modified the file detection routine in the 'runn' function
#              so it's not fooled by empty values, and is cleaner.
# 01/02/2004.  Added cookie finding routine from later version (which
#              isn't ready yet), so as not to have hard-coded paths.
# =======================================================================

import os shlex types

# Error codes for abnormal exit.
E_USAGE = 67        # Usage message, then quit.
E_NO_OPTS = 68      # No command-line args entered.
E_NO_URLS = 69      # No URLs passed to script.
E_NO_SAVEFILE = 70  # No save filename passed to script.
E_USER_EXIT = 71    # User decides to quit.

#  Basic default wget command we want to use.
#  This is the place to change it, if required.
#  NB: if using a proxy, set http_proxy = yourproxy in .wgetrc.
#  Otherwise delete --proxy=on, below.
# ====================================================================
CommandA = "wget -nc -c -t 5 --progress=bar --random-wait --proxy=on -r"
# ====================================================================

# --------------------------------------------------------------------
# Set some other variables and explain them.

pattern = " -A .jpg,.JPG,.jpeg,.JPEG,.gif,.GIF,.htm,.html,.shtml,.php"
                    # wget's option to only get certain types of file.
                    # comment out if not using
today = date +%F    # Used for a filename.
home = $env.HOME    # Set HOME to an internal variable.
                    # In case some other path is used, change it here.
depthDefault = 3    # Set a sensible default recursion.
Depth = $depthDefault # Otherwise user feedback doesn't tie in properly.
RefA = ""           # Set blank referring page.
Flag = ""           #  Default to not saving anything,
                    #+ or whatever else might be wanted in future.
lister = ""         # Used for passing a list of urls directly to wget.
Woptions = ""       # Used for passing wget some options for itself.
savePath = "$home/w-save"
Config = "$home/.wgetter2rc"
                    #  This is where some variables can be stored,
                    #+ if permanently changed from within the script.
Cookie_List = "$home/.cookielist"
                    # So we know where the cookies are kept . . .
cFlag = ""          # Part of the cookie file selection routine.
Cookie = ""         # The configuration file may set it.

# Define the options available. Easy to change letters here if needed.
# These are the optional options; you don't just wait to be asked.
opt = types.SimpleNamespace
  save=s   # Save command instead of executing it.
  cook=c   # Change cookie file for this session.
  help=h   # Usage guide.
  list=l   # Pass wget the -i option and URL list.
  runn=r   # Run saved commands as an argument to the option.
  inpu=i   # Run saved commands interactively.
  wopt=w   # Allow to enter options to pass directly to wget.
# --------------------------------------------------------------------

if (len $argv) < 2 or not $argv[1]   # Make sure we get something for wget to eat.
  print "You must at least enter a URL or option!"
  print "-$opt.help for usage."
  exit $E_NO_OPTS

# +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
# added added added added added added added added added added added added

if not (os.path.exists $Config)      # See if configuration file exists.
  print "Creating configuration file, $Config"
  print "# This is the configuration file for wgetter2" >$Config
  print "# Your customised settings will be saved in this file" >>$Config
else
  source $Config                     # Import variables we set outside the script.
                                     # (Its lines, Cookie="…" and cFlag=1, are tessh.)

if not (os.path.exists $Cookie_List)
  # Set up a list of cookie files, if there isn't one.
  print "Hunting for cookies . . ."
  find -name cookies.txt >>$Cookie_List  # Create the list of cookie files.
# Isolate this in its own 'if' statement,
# in case we got interrupted while searching.

if not $cFlag                        # If we haven't already done this . . .
  print                              # Make a nice space after the command prompt.
  print "Looks like you haven't set up your source of cookies yet."
  Cookies = (((Path $Cookie_List).read_text).splitlines)
  for [n c] in (enumerate $Cookies)  # Put the cookie files we found in a list,
    print "$n) $c"                   # and make a menu of them.
  print "Enter the number of the cookie file you want to use."
  print "If you won't be using cookies, just press RETURN."
  print
  print "I won't be asking this again. Edit $Config"
  print "If you decide to change at a later date"
  print "or use the -$opt.cook option for per session changes."
  REPLY = (input)
  if $REPLY                          # User didn't just press return.
    Cookie = " --load-cookies " + $Cookies[(int $REPLY)]
    # Set the variable here as well as in the config file.
    print "Cookie=\"$Cookie\"" >>$Config
  print "cFlag=1" >>$Config          # So we know not to ask again.

# end added section end added section end added section end added section
# +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

# Another variable.
# This one may or may not be subject to variation.
# A bit like the small print.
CookiesON = $Cookie
# print "cookie file is $CookiesON" # For debugging.
# print "home is $home"             # For debugging.
                                    # Got caught with this one!

def wopts
  global Woptions
  print "Enter options to pass to wget."
  print "It is assumed you know what you're doing."
  print
  print "You can pass their arguments here too."
  # That is to say, everything passed here is passed to wget.

  Wopts = (input)       # Read in the options to be passed to wget.

  Woptions = " $Wopts"
  #           ^  Why the leading space?
  # (So that the pieces of the command join with spaces between them.)

  print "passing options $Wopts to wget"
  # Mainly for debugging.
  # Is cute.

def save_func
  global Flag
  print "Settings will be saved."
  mkdir -p $savePath    # Create the directory to save things in
                        # if it isn't already there.
  Flag = S
  # Tell the final bit of code what to do.
  # Set a flag since stuff is done in main.

def usage               # Tell them how it works.
  print "Welcome to wgetter.  This is a front end to wget."
  print "It will always run wget with these options:"
  print $CommandA
  print "and the pattern to match: $pattern (which you can change at the top of this script)."
  print "It will also ask you for recursion depth, and if you want to use a referring page."
  print "Wgetter accepts the following options:"
  print ""
  print "-$opt.help : Display this help."
  print "-$opt.save : Save the command to a file $savePath/wget-($today) instead of running it."
  print "-$opt.runn : Run saved wget commands instead of starting a new one -"
  print "Enter filename as argument to this option."
  print "-$opt.inpu : Run saved wget commands interactively --"
  print "The script will ask you for the filename."
  print "-$opt.cook : Change the cookies file for this session."
  print "-$opt.list : Tell wget to use URL's from a list instead of from the command-line."
  print "-$opt.wopt : Pass any other options direct to wget."
  print ""
  print "See the wget man page for additional options you can pass to wget."
  print ""

  exit $E_USAGE         # End here. Don't process anything else.

def list_func           # Gives the user the option to use the -i option to wget,
                        # and a list of URLs.
  global lister
  while True
    print "Enter the name of the file containing URL's (press q to change\nyour mind)."
    urlfile = (input)
    if $urlfile == q    # Check quit option.
      print "Not using a url list."
      return
    elif not (os.path.exists $urlfile)
      print "That file does not exist!"
    else
      print "using $urlfile."
      print "If you gave url's on the command-line, I'll use those first."
                        # Report wget standard behaviour to the user.
      lister = " -i $urlfile"   # This is what we want to pass to wget.
      return

def cookie_func         # Give the user the option to use a different cookie file.
  global CookiesON
  while True
    print "Change the cookies file. Press return if you don't want to change \nit."
    Cookies = (input)
    # NB: this is not the same as Cookie, earlier.
    # There is an 's' on the end.
    # Bit like chocolate chips.
    if not $Cookies                       # Escape clause for wusses.
      return
    elif not (os.path.exists $Cookies)
      print "File does not exist.  Try again."   # Keep em going . . .
    else
      CookiesON = " --load-cookies $Cookies"     # File is good -- use it!
      return

# run_func [file name]
def run_func OPTARG=""
  if not $OPTARG
    # Test to see if we used the in-line option or the query one.
    if not (os.path.isdir $savePath)       # If directory doesn't exist . . .
      print "$savePath does not appear to exist."
      print "Please supply path and filename of saved wget commands:"
      newFile = (input)
      while not (os.path.isfile $newFile)  # Keep going till we get something.
        print "Sorry, that file does not exist.  Please try again."
        # Try really hard to get something.
        newFile = (input)
      # (The bash version has some bogus code here, commented out, to bail
      # out when the file holds no wget commands. If anyone wants to fix
      # it, feel free!)
      filePath = $newFile
    else
      print "Save path is $savePath"
      print "Please enter name of the file which you want to use."
      print "You have a choice of:"
      ls $savePath                         # Give them a choice.
      inFile = (input)
      while not (os.path.isfile "$savePath/$inFile")   # Keep going till
                                                       # we get something.
        print "Sorry, that file does not exist.  Please choose from:"
        ls $savePath                       # If a mistake is made.
        inFile = (input)
      filePath = "$savePath/$inFile"       # Make one variable . . .
  else
    filePath = "$savePath/$OPTARG"         # Which can be many things . . .

  if not (os.path.isfile $filePath)        # If a bogus file got through.
    print "You did not specify a suitable file."
    print "Run this script with the -$opt.save option first."
    print "Aborting."
    exit $E_NO_SAVEFILE
  print "Using: $filePath"
  for line in (((Path $filePath).read_text).splitlines)
    env (shlex.split $line)                # (bash: eval $REPLY)
    print "Completed: $line"
  exit

# getopts, as bash's runs with a leading ":" in its spec: [letter OPTARG]
# for each option, ["?" letter] for one it doesn't know and [":" letter]
# for one missing its argument; then the arguments after the options.
def getopts spec args
  opts = []
  while $args and ($args[0].startswith "-") and $args[0] != "-"
    [word *args] = $args
    if $word == "--"
      break
    letters = $word[1:]
    while $letters
      [c letters] = [$letters[0] $letters[1:]]
      if $c == ":" or $c not in $spec
        $opts.append ["?" $c]
      elif ($c + ":") not in $spec
        $opts.append [$c ""]
      elif $letters or $args               # Its argument: the rest of the
        optarg = if $letters then $letters else ($args.pop 0)   # word, or the next.
        $opts.append [$c $optarg]
        letters = ""
      else
        $opts.append [":" $c]
  [$opts $args]

# Fish out any options we are using for the script.
# This is based on the demo in "Learning The Bash Shell" (O'Reilly).
[opts args] = getopts ":$opt.save$opt.cook$opt.help$opt.list$opt.runn:$opt.inpu$opt.wopt" $argv[1:]
for [o OPTARG] in $opts
  match $o
    case opt.save
      save_func         #  Save some wgetter sessions for later.
    case opt.cook
      cookie_func       #  Change cookie file.
    case opt.help
      usage             #  Get help.
    case opt.list
      list_func         #  Allow wget to use a list of URLs.
    case opt.runn
      run_func $OPTARG  #  Useful if you are calling wgetter from,
                        #+ for example, a cron script.
    case opt.inpu
      run_func          #  When you don't know what your files are named.
    case opt.wopt
      wopts             #  Pass options directly to wget.
    case "?"
      print "Not a valid option."
      print "Use -$opt.wopt to pass options directly to wget,"
      print "or -$opt.help for help"      # Catch anything else.

if not $args and not $lister
  # We should be left with at least one URL on the command-line,
  # unless a list is being used -- catch empty CL's.
  print "No URL's given! You must enter them on the same line as wgetter2."
  print "E.g.,  wgetter2 http://somesite http://anothersite."
  print "Use $opt.help option for more information."
  exit $E_NO_URLS       # Bail out, with appropriate error code.

URLS = " " + (" ".join $args)
# Use this so that URL list can be changed if we stay in the option loop.

curDepth = ""
Saver = ""
while True
  # This is where we ask for the most used options.
  # (Mostly unchanged from version 1 of wgetter)
  Current = if $curDepth then " Current value is $curDepth" else ""
  print "How deep should I go? (integer: Default is $depthDefault.$Current)"
  Depth = (input)       # Recursion -- how far should we go?
  inputB = ""           # Reset this to blank on each pass of the loop.
  print "Enter the name of the referring page (default is none)."
  inputB = (input)      # Need this for some sites.

  print "Do you want to have the output logged to the terminal"
  print "(y/n, default is yes)?"
  noHide = (input)      # Otherwise wget will just log it to a file.

  match $noHide         # Now you see me, now you don't.
    case y | Y
      hide = ""
    case n | N
      hide = " -b"
    case _
      hide = ""

  if not $Depth
    # User accepted either default or current depth,
    # in which case Depth is now empty.
    # Use the one from a previous pass if there is one, else the default.
    Depth = $curDepth or $depthDefault
  Recurse = " -l $Depth"          # Set how deep we want to go.
  curDepth = $Depth               # Remember setting for next time.

  if $inputB
    RefA = " --referer=$inputB"   # Option to use referring page.

  WGETTER = "$CommandA$pattern$hide$RefA$Recurse$CookiesON$lister$Woptions$URLS"
  # Just string the whole lot together . . .
  # NB: no embedded spaces.
  # They are in the individual elements so that if any are empty,
  # we don't get an extra space.

  if not $CookiesON and $cFlag
    print "Warning -- can't find cookie file"
    # This should be changed,
    # in case the user has opted to not use cookies.

  if $Flag == S
    commandFile = "$savePath/wget-$today"
    siteFile = "$savePath/site-list-$today"
    print $WGETTER >>$commandFile
    # Create a unique filename for today, or append to it if it exists.
    print $inputB >>$siteFile
    # Make a list, so it's easy to refer back to,
    # since the whole command is a bit confusing to look at.
    print "Command saved to the file $commandFile"     # Tell the user.
    print "Referring page URL saved to the file $siteFile"
    Saver = " with save option"
    # Stick this somewhere, so it appears in the loop if set.
  else
    print "*****************"
    print "*****Getting*****"
    print "*****************"
    print ""
    print $WGETTER
    print ""
    print "*****************"
    env (shlex.split $WGETTER)    # (bash: eval "$WGETTER")

  print ""
  print "Starting over$Saver."
  print "If you want to stop, press q."
  print "Otherwise, enter some URL's:"
  # Let them go again. Tell about save option being set.

  REPLY = (input)
  match $REPLY
    # Need to change this to a 'trap' clause.
    case q | Q
      exit $E_USER_EXIT           # Exercise for the reader?
    case _
      URLS = " $REPLY"

  print ""

exit 0
```

Run with a stand-in `wget`, answers on its input and the same command
lines, the two printed the same, except where the bash prints
`saved to the file$ savePath/…` (a stray `$` and space before a line
break) and where `eval` squeezes the spaces in a saved command's quoted
argument. Its parts, on sample command lines and canned answers. The
options (the functions only say they were called):

```bash
# (wgetter2.bash's option loop, on sample command lines; the functions
#+ only say they were called.)
save=s; cook=c; help=h; list=l; runn=r; inpu=i; wopt=w
save_func()   { echo "  save_func"; }
cookie_func() { echo "  cookie_func"; }
usage()       { echo "  usage"; }
list_func()   { echo "  list_func"; }
run_func()    { echo "  run_func ${OPTARG:-(asks)}"; }
wopts()       { echo "  wopts"; }

for cl in "-s http://a" "-sc -w http://b http://c" "-r saved-cmds" \
          "-x -l" "-rsaved -- -notanoption"
do
  echo "wgetter2 $cl"
  set -- $cl
  OPTIND=1
  while getopts ":$save$cook$help$list$runn:$inpu$wopt" opt
  do
    case $opt in
       $save) save_func;;   #  Save some wgetter sessions for later.
       $cook) cookie_func;; #  Change cookie file.
       $help) usage;;       #  Get help.
       $list) list_func;;   #  Allow wget to use a list of URLs.
       $runn) run_func;;    #  Useful if you are calling wgetter from,
                            #+ for example, a cron script.
       $inpu) run_func;;    #  When you don't know what your files are named.
       $wopt) wopts;;       #  Pass options directly to wget.
          \?) echo "Not a valid option."
              echo "Use -${wopt} to pass options directly to wget,"
              echo "or -${help} for help";;      # Catch anything else.
    esac
  done
  shift $((OPTIND - 1))     # Do funky magic stuff with $#.
  echo "  URLs:" "$@"
done
```

```tessh
# (wgetter2's option loop, on sample command lines; the functions only
# say they were called.)
import types
opt = types.SimpleNamespace save=s cook=c help=h list=l runn=r inpu=i wopt=w
def save_func = print "  save_func"
def cookie_func = print "  cookie_func"
def usage = print "  usage"
def list_func = print "  list_func"
def run_func OPTARG="" = print "  run_func" ($OPTARG or "(asks)")
def wopts = print "  wopts"

# getopts, as bash's runs with a leading ":" in its spec: [letter OPTARG]
# for each option, ["?" letter] for one it doesn't know and [":" letter]
# for one missing its argument; then the arguments after the options.
def getopts spec args
  opts = []
  while $args and ($args[0].startswith "-") and $args[0] != "-"
    [word *args] = $args
    if $word == "--"
      break
    letters = $word[1:]
    while $letters
      [c letters] = [$letters[0] $letters[1:]]
      if $c == ":" or $c not in $spec
        $opts.append ["?" $c]
      elif ($c + ":") not in $spec
        $opts.append [$c ""]
      elif $letters or $args               # Its argument: the rest of the
        optarg = if $letters then $letters else ($args.pop 0)   # word, or the next.
        $opts.append [$c $optarg]
        letters = ""
      else
        $opts.append [":" $c]
  [$opts $args]

for cl in ["-s http://a" "-sc -w http://b http://c" "-r saved-cmds"
           "-x -l" "-rsaved -- -notanoption"]
  print "wgetter2 $cl"
  [opts args] = getopts ":$opt.save$opt.cook$opt.help$opt.list$opt.runn:$opt.inpu$opt.wopt" ($cl.split)
  for [o OPTARG] in $opts
    match $o
      case opt.save
        save_func         #  Save some wgetter sessions for later.
      case opt.cook
        cookie_func       #  Change cookie file.
      case opt.help
        usage             #  Get help.
      case opt.list
        list_func         #  Allow wget to use a list of URLs.
      case opt.runn
        run_func $OPTARG  #  Useful if you are calling wgetter from,
                          #+ for example, a cron script.
      case opt.inpu
        run_func          #  When you don't know what your files are named.
      case opt.wopt
        wopts             #  Pass options directly to wget.
      case "?"
        print "Not a valid option."
        print "Use -$opt.wopt to pass options directly to wget,"
        print "or -$opt.help for help"      # Catch anything else.
  print "  URLs:" *$args
```

```output
wgetter2 -s http://a
  save_func
  URLs: http://a
wgetter2 -sc -w http://b http://c
  save_func
  cookie_func
  wopts
  URLs: http://b http://c
wgetter2 -r saved-cmds
  run_func saved-cmds
  URLs:
wgetter2 -x -l
Not a valid option.
Use -w to pass options directly to wget,
or -h for help
  list_func
  URLs:
wgetter2 -rsaved -- -notanoption
  run_func saved
  URLs: -notanoption
```

Building the `wget` command, two passes of the main loop: the depth
asked for the first time is remembered as the second's default. The
bash feeds `read` from a heredoc; tessh puts its own `input` in front of
the real one.

```bash
# (Two passes of wgetter2.bash's main loop, with canned answers, a shorter
#+ wget command, and the command shown instead of run.)
CommandA="wget -nc -r"
pattern=" -A .jpg,.html"
depthDefault=3
URLS=" http://example.test/"
CookiesON=" --load-cookies cookies.txt"

for pass in 1 2
do
   if [ -z $curDepth ]; then
      Current=""
   else Current=" Current value is $curDepth"
   fi
       echo "How deep should I go? \
(integer: Default is $depthDefault.$Current)"
       read Depth   # Recursion -- how far should we go?
       inputB=""    # Reset this to blank on each pass of the loop.
       echo "Enter the name of the referring page (default is none)."
       read inputB  # Need this for some sites.

       echo "Do you want to have the output logged to the terminal"
       echo "(y/n, default is yes)?"
       read noHide  # Otherwise wget will just log it to a file.

       case $noHide in    # Now you see me, now you don't.
          y|Y ) hide="";;
          n|N ) hide=" -b";;
            * ) hide="";;
       esac

       if [ -z ${Depth} ]; then
          if [ -z ${curDepth} ]; then
             Depth="$depthDefault"
          else Depth="$curDepth"
          fi
       fi
   Recurse=" -l $Depth"          # Set how deep we want to go.
   curDepth=$Depth               # Remember setting for next time.

       if [ ! -z $inputB ]; then
          RefA=" --referer=$inputB"   # Option to use referring page.
       fi

   WGETTER="${CommandA}${pattern}${hide}${RefA}${Recurse}\
${CookiesON}${lister}${Woptions}${URLS}"
   echo "> $WGETTER"
   URLS=" http://example.test/more"
done <<'ANSWERS'
2
http://example.test/start
n


y
ANSWERS
```

```tessh
# (Two passes of wgetter2's main loop, with canned answers, a shorter
# wget command, and the command shown instead of run.)
answers = iter ["2" "http://example.test/start" "n"
                "" "" "y"]
def input = next $answers        # (Instead of reading the terminal.)

CommandA = "wget -nc -r"
pattern = " -A .jpg,.html"
depthDefault = 3
URLS = " http://example.test/"
CookiesON = " --load-cookies cookies.txt"
RefA = "" ; lister = "" ; Woptions = ""

curDepth = ""
for pass in [1 2]
  Current = if $curDepth then " Current value is $curDepth" else ""
  print "How deep should I go? (integer: Default is $depthDefault.$Current)"
  Depth = (input)       # Recursion -- how far should we go?
  inputB = ""           # Reset this to blank on each pass of the loop.
  print "Enter the name of the referring page (default is none)."
  inputB = (input)      # Need this for some sites.

  print "Do you want to have the output logged to the terminal"
  print "(y/n, default is yes)?"
  noHide = (input)      # Otherwise wget will just log it to a file.

  match $noHide         # Now you see me, now you don't.
    case y | Y
      hide = ""
    case n | N
      hide = " -b"
    case _
      hide = ""

  if not $Depth
    Depth = $curDepth or $depthDefault
  Recurse = " -l $Depth"          # Set how deep we want to go.
  curDepth = $Depth               # Remember setting for next time.

  if $inputB
    RefA = " --referer=$inputB"   # Option to use referring page.

  WGETTER = "$CommandA$pattern$hide$RefA$Recurse$CookiesON$lister$Woptions$URLS"
  print "> $WGETTER"
  URLS = " http://example.test/more"
```

```output
How deep should I go? (integer: Default is 3.)
Enter the name of the referring page (default is none).
Do you want to have the output logged to the terminal
(y/n, default is yes)?
> wget -nc -r -A .jpg,.html -b --referer=http://example.test/start -l 2 --load-cookies cookies.txt http://example.test/
How deep should I go? (integer: Default is 3. Current value is 2)
Enter the name of the referring page (default is none).
Do you want to have the output logged to the terminal
(y/n, default is yes)?
> wget -nc -r -A .jpg,.html --referer=http://example.test/start -l 2 --load-cookies cookies.txt http://example.test/more
```

And `run_func`'s last part, running the saved commands:

```bash
# (A made-up file of saved commands: echo instead of wget.)
filePath=wget-saved
cat >$filePath <<'END'
echo getting http://example.test/ -l 2
echo getting 'http://example.test/a page' -l 3
END

echo "Using: $filePath"
while read; do
    eval $REPLY
    echo "Completed: $REPLY"
done < $filePath  # Feed the actual file we are using into a 'while' loop.
```

```tessh
import shlex
# (A made-up file of saved commands: echo instead of wget.)
filePath = Path wget-saved
cat >$filePath <<'END'
echo getting http://example.test/ -l 2
echo getting 'http://example.test/a page' -l 3
END

print "Using: $filePath"
for line in (($filePath.read_text).splitlines)
  env (shlex.split $line)        # The words of the line, as the shell splits them.
  print "Completed: $line"
```

```output
Using: wget-saved
getting http://example.test/ -l 2
Completed: echo getting http://example.test/ -l 2
getting http://example.test/a page -l 3
Completed: echo getting 'http://example.test/a page' -l 3
```

## A podcast fetcher
<!-- help: abs-bashpodder bashpodder podcast rss feed wget m3u playlist -->

Linc Fessenden's `bashpodder.sh` (with "many contributors") reads the
feed addresses in `bp.conf`, pulls the enclosure URLs out of each feed
with `tr` and `sed`, downloads the episodes it hasn't fetched before into
a folder named for the day, keeps the list of fetched URLs in
`podcast.log`, and writes an `m3u` playlist. It needs the network.

### Example A-31. A *podcasting* script (`bashpodder.sh`)

```bash no-run
#!/bin/bash

#  bashpodder.sh:
#  By Linc 10/1/2004
#  Find the latest script at
#+ http://linc.homeunix.org:8080/scripts/bashpodder
#  Last revision 12/14/2004 - Many Contributors!
#  If you use this and have made improvements or have comments
#+ drop me an email at linc dot fessenden at gmail dot com
#  I'd appreciate it!

# ==>  ABS Guide extra comments.

# ==>  Author of this script has kindly granted permission
# ==>+ for inclusion in ABS Guide.


# ==> ################################################################
# 
# ==> What is "podcasting"?

# ==> It's broadcasting "radio shows" over the Internet.
# ==> These shows can be played on iPods and other music file players.

# ==> This script makes it possible.
# ==> See documentation at the script author's site, above.

# ==> ################################################################


# Make script crontab friendly:
cd $(dirname $0)
# ==> Change to directory where this script lives.

# datadir is the directory you want podcasts saved to:
datadir=$(date +%Y-%m-%d)
# ==> Will create a date-labeled directory, named: YYYY-MM-DD

# Check for and create datadir if necessary:
if test ! -d $datadir
        then
        mkdir $datadir
fi

# Delete any temp file:
rm -f temp.log

#  Read the bp.conf file and wget any url not already
#+ in the podcast.log file:
while read podcast
  do # ==> Main action follows.
  file=$(wget -q $podcast -O - | tr '\r' '\n' | tr \' \" | \
sed -n 's/.*url="\([^"]*\)".*/\1/p')
  for url in $file
                do
                echo $url >> temp.log
                if ! grep "$url" podcast.log > /dev/null
                        then
                        wget -q -P $datadir "$url"
                fi
                done
    done < bp.conf

# Move dynamically created log file to permanent log file:
cat podcast.log >> temp.log
sort temp.log | uniq > podcast.log
rm temp.log
# Create an m3u playlist:
ls $datadir | grep -v m3u > $datadir/podcast.m3u


exit 0

#################################################
For a different scripting approach to Podcasting,
see Phil Salkie's article, 
"Internet Radio to Podcast with Shell Tools"
in the September, 2005 issue of LINUX JOURNAL,
http://www.linuxjournal.com/article/8171
#################################################
```

The pipelines are the same in tessh. `while read podcast … done <
bp.conf` is a `for` over the file's words, `$(dirname $0)` is
`(Path $argv[0]).parent`, and `if ! grep …` is `if not (grep …)`: a
command in parentheses is its `Output`, true when it succeeded. bash's
`tr \' \"` is `tr "'" '"'`.

```tessh no-run
#!/usr/bin/env tessh

#  bashpodder.tsh: (ported from bashpodder.sh)
#  By Linc 10/1/2004
#  Find the latest script at
#+ http://linc.homeunix.org:8080/scripts/bashpodder
#  Last revision 12/14/2004 - Many Contributors!
#  If you use this and have made improvements or have comments
#+ drop me an email at linc dot fessenden at gmail dot com
#  I'd appreciate it!

# ==>  ABS Guide extra comments.

# ==>  Author of this script has kindly granted permission
# ==>+ for inclusion in ABS Guide.

# ==> What is "podcasting"?
# ==> It's broadcasting "radio shows" over the Internet.
# ==> These shows can be played on iPods and other music file players.
# ==> This script makes it possible.

import os

# Make script crontab friendly:
cd (Path $argv[0]).parent
# ==> Change to directory where this script lives.

# datadir is the directory you want podcasts saved to:
datadir = date +%Y-%m-%d
# ==> Will create a date-labeled directory, named: YYYY-MM-DD

# Check for and create datadir if necessary:
if not (os.path.isdir $datadir)
  mkdir $datadir

# Delete any temp file:
rm -f temp.log

# Read the bp.conf file and wget any url not already
# in the podcast.log file:
for podcast in (((Path bp.conf).read_text).split)
  # ==> Main action follows.
  file = wget -q $podcast -O - | tr '\r' '\n' | tr "'" '"' | \
         sed -n 's/.*url="\([^"]*\)".*/\1/p'
  for url in ($file.split)
    print $url >>temp.log
    if not (grep $url podcast.log >/dev/null)
      wget -q -P $datadir $url

# Move dynamically created log file to permanent log file:
cat podcast.log >>temp.log
sort temp.log | uniq >podcast.log
rm temp.log
# Create an m3u playlist:
ls $datadir | grep -v m3u >$datadir/podcast.m3u

exit 0

#################################################
# For a different scripting approach to Podcasting,
# see Phil Salkie's article,
# "Internet Radio to Podcast with Shell Tools"
# in the September, 2005 issue of LINUX JOURNAL,
# http://www.linuxjournal.com/article/8171
#################################################
```

Here it is on two made-up feeds, one with its items on one line split by
carriage returns (which the first `tr` is for), and a stand-in `wget` in
the current folder that reads the feeds from files and only touches the
episodes it "fetches". One episode is in the log already, so it isn't
fetched again.

```bash
# (Made up: two feeds, a log of one episode already fetched, and a
#+ stand-in wget that "downloads" from these files instead of the net.)
cat >show1.xml <<'END'
<rss><channel><title>Show One</title>
<item><enclosure url="http://example.test/one/ep1.mp3" type="audio/mpeg"/></item>
<item><enclosure url='http://example.test/one/ep2.mp3' type='audio/mpeg'/></item>
</channel></rss>
END
printf '<rss><item><enclosure url="http://example.test/two/a.ogg"/></item>\r<item><enclosure url="http://example.test/two/b.ogg"/></item></rss>\r' >show2.xml
printf '%s\n' show1.xml show2.xml >bp.conf
echo http://example.test/one/ep1.mp3 >podcast.log
cat >wget <<'END'
#!/bin/sh
# wget -q FEED -O -   or   wget -q -P DIR URL
if [ "$3" = -O ]; then cat "$2"; else echo "  fetching $4"; touch "$3/${4##*/}"; fi
END
chmod +x wget; PATH=$PWD:$PATH

# cd $(dirname $0)        (Not here: the made-up files are in this folder.)

# datadir is the directory you want podcasts saved to:
datadir=$(date +%Y-%m-%d)
# ==> Will create a date-labeled directory, named: YYYY-MM-DD

# Check for and create datadir if necessary:
if test ! -d $datadir
        then
        mkdir $datadir
fi

# Delete any temp file:
rm -f temp.log

#  Read the bp.conf file and wget any url not already
#+ in the podcast.log file:
while read podcast
  do # ==> Main action follows.
  file=$(wget -q $podcast -O - | tr '\r' '\n' | tr \' \" | \
sed -n 's/.*url="\([^"]*\)".*/\1/p')
  for url in $file
                do
                echo $url >> temp.log
                if ! grep "$url" podcast.log > /dev/null
                        then
                        wget -q -P $datadir "$url"
                fi
                done
    done < bp.conf

# Move dynamically created log file to permanent log file:
cat podcast.log >> temp.log
sort temp.log | uniq > podcast.log
rm temp.log
# Create an m3u playlist:
ls $datadir | grep -v m3u > $datadir/podcast.m3u

echo "podcast.log:"; cat podcast.log           # (To see what it did.)
echo "podcast.m3u:"; cat $datadir/podcast.m3u
```

```tessh
import os
# (Made up: two feeds, a log of one episode already fetched, and a
# stand-in wget that "downloads" from these files instead of the net.)
cat >show1.xml <<'END'
<rss><channel><title>Show One</title>
<item><enclosure url="http://example.test/one/ep1.mp3" type="audio/mpeg"/></item>
<item><enclosure url='http://example.test/one/ep2.mp3' type='audio/mpeg'/></item>
</channel></rss>
END
printf '<rss><item><enclosure url="http://example.test/two/a.ogg"/></item>\r<item><enclosure url="http://example.test/two/b.ogg"/></item></rss>\r' >show2.xml
print show1.xml show2.xml sep="\n" >bp.conf
print http://example.test/one/ep1.mp3 >podcast.log
cat >wget <<'END'
#!/bin/sh
# wget -q FEED -O -   or   wget -q -P DIR URL
if [ "$3" = -O ]; then cat "$2"; else echo "  fetching $4"; touch "$3/${4##*/}"; fi
END
chmod +x wget ; $env.PATH = "$(pwd):$env.PATH"

# cd (Path $argv[0]).parent  (Not here: the made-up files are in this folder.)

# datadir is the directory you want podcasts saved to:
datadir = date +%Y-%m-%d
# ==> Will create a date-labeled directory, named: YYYY-MM-DD

# Check for and create datadir if necessary:
if not (os.path.isdir $datadir)
  mkdir $datadir

# Delete any temp file:
rm -f temp.log

# Read the bp.conf file and wget any url not already
# in the podcast.log file:
for podcast in (((Path bp.conf).read_text).split)
  # ==> Main action follows.
  file = wget -q $podcast -O - | tr '\r' '\n' | tr "'" '"' | \
         sed -n 's/.*url="\([^"]*\)".*/\1/p'
  for url in ($file.split)
    print $url >>temp.log
    if not (grep $url podcast.log >/dev/null)
      wget -q -P $datadir $url

# Move dynamically created log file to permanent log file:
cat podcast.log >>temp.log
sort temp.log | uniq >podcast.log
rm temp.log
# Create an m3u playlist:
ls $datadir | grep -v m3u >$datadir/podcast.m3u

print "podcast.log:" ; cat podcast.log           # (To see what it did.)
print "podcast.m3u:" ; cat $datadir/podcast.m3u
```

```output
  fetching http://example.test/one/ep2.mp3
  fetching http://example.test/two/a.ogg
  fetching http://example.test/two/b.ogg
podcast.log:
http://example.test/one/ep1.mp3
http://example.test/one/ep2.mp3
http://example.test/two/a.ogg
http://example.test/two/b.ogg
podcast.m3u:
a.ogg
b.ogg
ep2.mp3
```

The `sed` takes one URL per line, the last one on it. In tessh a regex
can take them all from the feed's text at once:
`re"url=[\"']([^\"']*)".findall $text`.

## Nightly backup with rsync
<!-- help: abs-nightly-backup nightly-backup backup rsync rotate snapshots cron -->

Richard Neill's `nightly-backup.sh` backs up a home folder to a backup
disk with `rsync`, run from cron: it mounts the disk if need be, makes
the new backup with hard links to the last one (`--link-dest`), and then
rotates the old ones, keeping one a day for a week, one a week for a
month and one a month for three months. It needs root and the disk, and
it deletes old backups.

### Example A-32. Nightly backup to a firewire HD (`nightly-backup.sh`)

```bash no-run
#!/bin/bash
# nightly-backup.sh
# http://www.richardneill.org/source.php#nightly-backup-rsync
# Copyright (c) 2005 Richard Neill <backup@richardneill.org>.
# This is Free Software licensed under the GNU GPL.
# ==> Included in ABS Guide with script author's kind permission.
# ==> (Thanks!)

#  This does a backup from the host computer to a locally connected
#+ firewire HDD using rsync and ssh.
#  (Script should work with USB-connected device (see lines 40-43).
#  It then rotates the backups.
#  Run it via cron every night at 5am.
#  This only backs up the home directory.
#  If ownerships (other than the user's) should be preserved,
#+ then run the rsync process as root (and re-instate the -o).
#  We save every day for 7 days, then every week for 4 weeks,
#+ then every month for 3 months.

#  See: http://www.mikerubel.org/computers/rsync_snapshots/
#+ for more explanation of the theory.
#  Save as: $HOME/bin/nightly-backup_firewire-hdd.sh

#  Known bugs:
#  ----------
#  i)  Ideally, we want to exclude ~/.tmp and the browser caches.

#  ii) If the user is sitting at the computer at 5am,
#+     and files are modified while the rsync is occurring,
#+     then the BACKUP_JUSTINCASE branch gets triggered.
#      To some extent, this is a 
#+     feature, but it also causes a "disk-space leak".





##### BEGIN CONFIGURATION SECTION ############################################
LOCAL_USER=rjn                # User whose home directory should be backed up.
MOUNT_POINT=/backup           # Mountpoint of backup drive.
                              # NO trailing slash!
                              # This must be unique (eg using a udev symlink)
# MOUNT_POINT=/media/disk     # For USB-connected device.
SOURCE_DIR=/home/$LOCAL_USER  # NO trailing slash - it DOES matter to rsync.
BACKUP_DEST_DIR=$MOUNT_POINT/backup/`hostname -s`.${LOCAL_USER}.nightly_backup
DRY_RUN=false                 #If true, invoke rsync with -n, to do a dry run.
                              # Comment out or set to false for normal use.
VERBOSE=false                 # If true, make rsync verbose.
                              # Comment out or set to false otherwise.
COMPRESS=false                # If true, compress.
                              # Good for internet, bad on LAN.
                              # Comment out or set to false otherwise.

### Exit Codes ###
E_VARS_NOT_SET=64
E_COMMANDLINE=65
E_MOUNT_FAIL=70
E_NOSOURCEDIR=71
E_UNMOUNTED=72
E_BACKUP=73
##### END CONFIGURATION SECTION ##############################################


# Check that all the important variables have been set:
if [ -z "$LOCAL_USER" ] ||
   [ -z "$SOURCE_DIR" ] ||
   [ -z "$MOUNT_POINT" ]  ||
   [ -z "$BACKUP_DEST_DIR" ]
then
   echo 'One of the variables is not set! Edit the file: $0. BACKUP FAILED.'
   exit $E_VARS_NOT_SET
fi

if [ "$#" != 0 ]  # If command-line param(s) . . .
then              # Here document(ation).
  cat <<-ENDOFTEXT
    Automatic Nightly backup run from cron.
    Read the source for more details: $0
    The backup directory is $BACKUP_DEST_DIR .
    It will be created if necessary; initialisation is no longer required.

    WARNING: Contents of $BACKUP_DEST_DIR are rotated.
    Directories named 'backup.\$i' will eventually be DELETED.
    We keep backups from every day for 7 days (1-8),
    then every week for 4 weeks (9-12),
    then every month for 3 months (13-15).

    You may wish to add this to your crontab using 'crontab -e'
    #  Back up files: $SOURCE_DIR to $BACKUP_DEST_DIR
    #+ every night at 3:15 am
         15 03 * * * /home/$LOCAL_USER/bin/nightly-backup_firewire-hdd.sh

    Don't forget to verify the backups are working,
    especially if you don't read cron's mail!"
	ENDOFTEXT
   exit $E_COMMANDLINE
fi


# Parse the options.
# ==================

if [ "$DRY_RUN" == "true" ]; then
  DRY_RUN="-n"
  echo "WARNING:"
  echo "THIS IS A 'DRY RUN'!"
  echo "No data will actually be transferred!"
else
  DRY_RUN=""
fi

if [ "$VERBOSE" == "true" ]; then
  VERBOSE="-v"
else
  VERBOSE=""
fi

if [ "$COMPRESS" == "true" ]; then
  COMPRESS="-z"
else
  COMPRESS=""
fi


#  Every week (actually of 8 days) and every month,
#+ extra backups are preserved.
DAY_OF_MONTH=`date +%d`            # Day of month (01..31).
if [ $DAY_OF_MONTH = 01 ]; then    # First of month.
  MONTHSTART=true
elif [ $DAY_OF_MONTH = 08 \
    -o $DAY_OF_MONTH = 16 \
    -o $DAY_OF_MONTH = 24 ]; then
    # Day 8,16,24  (use 8, not 7 to better handle 31-day months)
      WEEKSTART=true
fi



#  Check that the HDD is mounted.
#  At least, check that *something* is mounted here!
#  We can use something unique to the device, rather than just guessing
#+ the scsi-id by having an appropriate udev rule in
#+ /etc/udev/rules.d/10-rules.local
#+ and by putting a relevant entry in /etc/fstab.
#  Eg: this udev rule:
# BUS="scsi", KERNEL="sd*", SYSFS{vendor}="WDC WD16",
# SYSFS{model}="00JB-00GVA0     ", NAME="%k", SYMLINK="lacie_1394d%n"

if mount | grep $MOUNT_POINT >/dev/null; then
  echo "Mount point $MOUNT_POINT is indeed mounted. OK"
else
  echo -n "Attempting to mount $MOUNT_POINT..."	
           # If it isn't mounted, try to mount it.
  sudo mount $MOUNT_POINT 2>/dev/null

  if mount | grep $MOUNT_POINT >/dev/null; then
    UNMOUNT_LATER=TRUE
    echo "OK"
    #  Note: Ensure that this is also unmounted
    #+ if we exit prematurely with failure.
  else
    echo "FAILED"
    echo -e "Nothing is mounted at $MOUNT_POINT. BACKUP FAILED!"
    exit $E_MOUNT_FAIL
  fi
fi


# Check that source dir exists and is readable.
if [ ! -r  $SOURCE_DIR ] ; then
  echo "$SOURCE_DIR does not exist, or cannot be read. BACKUP FAILED."
  exit $E_NOSOURCEDIR
fi


# Check that the backup directory structure is as it should be.
# If not, create it.
# Create the subdirectories.
# Note that backup.0 will be created as needed by rsync.

for ((i=1;i<=15;i++)); do
  if [ ! -d $BACKUP_DEST_DIR/backup.$i ]; then
    if /bin/mkdir -p $BACKUP_DEST_DIR/backup.$i ; then
    #  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  No [ ] test brackets. Why?
      echo "Warning: directory $BACKUP_DEST_DIR/backup.$i is missing,"
      echo "or was not initialised. (Re-)creating it."
    else
      echo "ERROR: directory $BACKUP_DEST_DIR/backup.$i"
      echo "is missing and could not be created."
    if  [ "$UNMOUNT_LATER" == "TRUE" ]; then
        # Before we exit, unmount the mount point if necessary.
        cd
	sudo umount $MOUNT_POINT &&
	echo "Unmounted $MOUNT_POINT again. Giving up."
    fi
      exit $E_UNMOUNTED
  fi
fi
done


#  Set the permission to 700 for security
#+ on an otherwise permissive multi-user system.
if ! /bin/chmod 700 $BACKUP_DEST_DIR ; then
  echo "ERROR: Could not set permissions on $BACKUP_DEST_DIR to 700."

  if  [ "$UNMOUNT_LATER" == "TRUE" ]; then
  # Before we exit, unmount the mount point if necessary.
     cd ; sudo umount $MOUNT_POINT \
     && echo "Unmounted $MOUNT_POINT again. Giving up."
  fi

  exit $E_UNMOUNTED
fi

# Create the symlink: current -> backup.1 if required.
# A failure here is not critical.
cd $BACKUP_DEST_DIR
if [ ! -h current ] ; then
  if ! /bin/ln -s backup.1 current ; then
    echo "WARNING: could not create symlink current -> backup.1"
  fi
fi


# Now, do the rsync.
echo "Now doing backup with rsync..."
echo "Source dir: $SOURCE_DIR"
echo -e "Backup destination dir: $BACKUP_DEST_DIR\n"


/usr/bin/rsync $DRY_RUN $VERBOSE -a -S --delete --modify-window=60 \
--link-dest=../backup.1 $SOURCE_DIR $BACKUP_DEST_DIR/backup.0/

#  Only warn, rather than exit if the rsync failed,
#+ since it may only be a minor problem.
#  E.g., if one file is not readable, rsync will fail.
#  This shouldn't prevent the rotation.
#  Not using, e.g., `date +%a`  since these directories
#+ are just full of links and don't consume *that much* space.

if [ $? != 0 ]; then
  BACKUP_JUSTINCASE=backup.`date +%F_%T`.justincase
  echo "WARNING: the rsync process did not entirely succeed."
  echo "Something might be wrong."
  echo "Saving an extra copy at: $BACKUP_JUSTINCASE"
  echo "WARNING: if this occurs regularly, a LOT of space will be consumed,"
  echo "even though these are just hard-links!"
fi

# Save a readme in the backup parent directory.
# Save another one in the recent subdirectory.
echo "Backup of $SOURCE_DIR on `hostname` was last run on \
`date`" > $BACKUP_DEST_DIR/README.txt
echo "This backup of $SOURCE_DIR on `hostname` was created on \
`date`" > $BACKUP_DEST_DIR/backup.0/README.txt

# If we are not in a dry run, rotate the backups.
[ -z "$DRY_RUN" ] &&

  #  Check how full the backup disk is.
  #  Warn if 90%. if 98% or more, we'll probably fail, so give up.
  #  (Note: df can output to more than one line.)
  #  We test this here, rather than before
  #+ so that rsync may possibly have a chance.
  DISK_FULL_PERCENT=`/bin/df $BACKUP_DEST_DIR |
  tr "\n" ' ' | awk '{print $12}' | grep -oE [0-9]+ `
  echo "Disk space check on backup partition \
  $MOUNT_POINT $DISK_FULL_PERCENT% full."
  if [ $DISK_FULL_PERCENT -gt 90 ]; then
    echo "Warning: Disk is greater than 90% full."
  fi
  if [ $DISK_FULL_PERCENT -gt 98 ]; then
    echo "Error: Disk is full! Giving up."
      if  [ "$UNMOUNT_LATER" == "TRUE" ]; then
        # Before we exit, unmount the mount point if necessary.
        cd; sudo umount $MOUNT_POINT &&
        echo "Unmounted $MOUNT_POINT again. Giving up."
      fi
    exit $E_UNMOUNTED
  fi


 # Create an extra backup.
 # If this copy fails, give up.
 if [ -n "$BACKUP_JUSTINCASE" ]; then
   if ! /bin/cp -al $BACKUP_DEST_DIR/backup.0 \
      $BACKUP_DEST_DIR/$BACKUP_JUSTINCASE
   then
     echo "ERROR: Failed to create extra copy \
     $BACKUP_DEST_DIR/$BACKUP_JUSTINCASE"
     if  [ "$UNMOUNT_LATER" == "TRUE" ]; then
       # Before we exit, unmount the mount point if necessary.
       cd ;sudo umount $MOUNT_POINT &&
       echo "Unmounted $MOUNT_POINT again. Giving up."
     fi
     exit $E_UNMOUNTED
   fi
 fi


 # At start of month, rotate the oldest 8.
 if [ "$MONTHSTART" == "true" ]; then
   echo -e "\nStart of month. \
   Removing oldest backup: $BACKUP_DEST_DIR/backup.15"  &&
   /bin/rm -rf  $BACKUP_DEST_DIR/backup.15  &&
   echo "Rotating monthly,weekly backups: \
   $BACKUP_DEST_DIR/backup.[8-14] -> $BACKUP_DEST_DIR/backup.[9-15]"  &&
     /bin/mv $BACKUP_DEST_DIR/backup.14 $BACKUP_DEST_DIR/backup.15  &&
     /bin/mv $BACKUP_DEST_DIR/backup.13 $BACKUP_DEST_DIR/backup.14  &&
     /bin/mv $BACKUP_DEST_DIR/backup.12 $BACKUP_DEST_DIR/backup.13  &&
     /bin/mv $BACKUP_DEST_DIR/backup.11 $BACKUP_DEST_DIR/backup.12  &&
     /bin/mv $BACKUP_DEST_DIR/backup.10 $BACKUP_DEST_DIR/backup.11  &&
     /bin/mv $BACKUP_DEST_DIR/backup.9 $BACKUP_DEST_DIR/backup.10  &&
     /bin/mv $BACKUP_DEST_DIR/backup.8 $BACKUP_DEST_DIR/backup.9

 # At start of week, rotate the second-oldest 4.
 elif [ "$WEEKSTART" == "true" ]; then
   echo -e "\nStart of week. \
   Removing oldest weekly backup: $BACKUP_DEST_DIR/backup.12"  &&
   /bin/rm -rf  $BACKUP_DEST_DIR/backup.12  &&

   echo "Rotating weekly backups: \
   $BACKUP_DEST_DIR/backup.[8-11] -> $BACKUP_DEST_DIR/backup.[9-12]"  &&
     /bin/mv $BACKUP_DEST_DIR/backup.11 $BACKUP_DEST_DIR/backup.12  &&
     /bin/mv $BACKUP_DEST_DIR/backup.10 $BACKUP_DEST_DIR/backup.11  &&
     /bin/mv $BACKUP_DEST_DIR/backup.9 $BACKUP_DEST_DIR/backup.10  &&
     /bin/mv $BACKUP_DEST_DIR/backup.8 $BACKUP_DEST_DIR/backup.9

 else
   echo -e "\nRemoving oldest daily backup: $BACKUP_DEST_DIR/backup.8"  &&
     /bin/rm -rf  $BACKUP_DEST_DIR/backup.8

 fi  &&

 # Every day, rotate the newest 8.
 echo "Rotating daily backups: \
 $BACKUP_DEST_DIR/backup.[1-7] -> $BACKUP_DEST_DIR/backup.[2-8]"  &&
     /bin/mv $BACKUP_DEST_DIR/backup.7 $BACKUP_DEST_DIR/backup.8  &&
     /bin/mv $BACKUP_DEST_DIR/backup.6 $BACKUP_DEST_DIR/backup.7  &&
     /bin/mv $BACKUP_DEST_DIR/backup.5 $BACKUP_DEST_DIR/backup.6  &&
     /bin/mv $BACKUP_DEST_DIR/backup.4 $BACKUP_DEST_DIR/backup.5  &&
     /bin/mv $BACKUP_DEST_DIR/backup.3 $BACKUP_DEST_DIR/backup.4  &&
     /bin/mv $BACKUP_DEST_DIR/backup.2 $BACKUP_DEST_DIR/backup.3  &&
     /bin/mv $BACKUP_DEST_DIR/backup.1 $BACKUP_DEST_DIR/backup.2  &&
     /bin/mv $BACKUP_DEST_DIR/backup.0 $BACKUP_DEST_DIR/backup.1  &&

 SUCCESS=true


if  [ "$UNMOUNT_LATER" == "TRUE" ]; then
  # Unmount the mount point if it wasn't mounted to begin with.
  cd ; sudo umount $MOUNT_POINT && echo "Unmounted $MOUNT_POINT again."
fi


if [ "$SUCCESS" == "true" ]; then
  echo 'SUCCESS!'
  exit 0
fi

# Should have already exited if backup worked.
echo 'BACKUP FAILED! Is this just a dry run? Is the disk full?) '
exit $E_BACKUP
```

In tessh the switches are Python booleans that make a list of `rsync`
options (the bash works out `COMPRESS` but never passes it on; here it's
passed), the day of the month is a number, and the folders are Paths:
`mkdir`, `chmod`, the symlink and the rotation are Path methods in `try`
blocks, so a failure is an `OSError` to catch rather than a status to
test after each `&&`. The four copies of "unmount if we mounted, then
exit" are one function, `give_up`. The disk check uses
`shutil.disk_usage` instead of picking a field out of `df`'s output.
One fix: in the bash, `[ -z "$DRY_RUN" ] &&` guards only the line after
it (the `df`), so a dry run goes on to delete and rotate the old backups
until a `mv` of the missing `backup.0` fails; here a dry run rotates
nothing.

```tessh no-run
#!/usr/bin/env tessh
# nightly-backup.tsh (ported from nightly-backup.sh)
# http://www.richardneill.org/source.php#nightly-backup-rsync
# Copyright (c) 2005 Richard Neill <backup@richardneill.org>.
# This is Free Software licensed under the GNU GPL.
# ==> Included in ABS Guide with script author's kind permission.
# ==> (Thanks!)

#  This does a backup from the host computer to a locally connected
#+ firewire HDD using rsync and ssh.
#  (Script should work with USB-connected device (see MOUNT_POINT).
#  It then rotates the backups.
#  Run it via cron every night at 5am.
#  This only backs up the home directory.
#  If ownerships (other than the user's) should be preserved,
#+ then run the rsync process as root (and re-instate the -o).
#  We save every day for 7 days, then every week for 4 weeks,
#+ then every month for 3 months.

#  See: http://www.mikerubel.org/computers/rsync_snapshots/
#+ for more explanation of the theory.
#  Save as: $HOME/bin/nightly-backup_firewire-hdd.tsh

#  Known bugs:
#  ----------
#  i)  Ideally, we want to exclude ~/.tmp and the browser caches.

#  ii) If the user is sitting at the computer at 5am,
#+     and files are modified while the rsync is occurring,
#+     then the BACKUP_JUSTINCASE branch gets triggered.
#      To some extent, this is a
#+     feature, but it also causes a "disk-space leak".

import os shutil math datetime

##### BEGIN CONFIGURATION SECTION ############################################
LOCAL_USER = rjn              # User whose home directory should be backed up.
MOUNT_POINT = /backup         # Mountpoint of backup drive.
                              # NO trailing slash!
                              # This must be unique (eg using a udev symlink)
# MOUNT_POINT = /media/disk   # For USB-connected device.
SOURCE_DIR = Path "/home/$LOCAL_USER"   # NO trailing slash - it DOES matter to rsync.
BACKUP_DEST_DIR = $MOUNT_POINT / backup / "$(hostname -s).${LOCAL_USER}.nightly_backup"
DRY_RUN = False               # If True, invoke rsync with -n, to do a dry run.
VERBOSE = False               # If True, make rsync verbose.
COMPRESS = False              # If True, compress.
                              # Good for internet, bad on LAN.

### Exit Codes ###
E_VARS_NOT_SET = 64
E_COMMANDLINE = 65
E_MOUNT_FAIL = 70
E_NOSOURCEDIR = 71
E_UNMOUNTED = 72
E_BACKUP = 73
##### END CONFIGURATION SECTION ##############################################

# Check that all the important variables have been set:
if not (all ((str $v) for v in [$LOCAL_USER $SOURCE_DIR $MOUNT_POINT $BACKUP_DEST_DIR]))
  print "One of the variables is not set! Edit the file: $argv[0]. BACKUP FAILED."
  exit $E_VARS_NOT_SET

if (len $argv) > 1            # If command-line param(s) . . .
  me = $argv[0]               # Here document(ation).
  print """
    Automatic Nightly backup run from cron.
    Read the source for more details: $me
    The backup directory is $BACKUP_DEST_DIR .
    It will be created if necessary; initialisation is no longer required.

    WARNING: Contents of $BACKUP_DEST_DIR are rotated.
    Directories named 'backup.\$i' will eventually be DELETED.
    We keep backups from every day for 7 days (1-8),
    then every week for 4 weeks (9-12),
    then every month for 3 months (13-15).

    You may wish to add this to your crontab using 'crontab -e'
    #  Back up files: $SOURCE_DIR to $BACKUP_DEST_DIR
    #+ every night at 3:15 am
         15 03 * * * /home/$LOCAL_USER/bin/nightly-backup_firewire-hdd.tsh

    Don't forget to verify the backups are working,
    especially if you don't read cron's mail!"""
  exit $E_COMMANDLINE

# Parse the options.
# ==================

if $DRY_RUN
  print "WARNING:"
  print "THIS IS A 'DRY RUN'!"
  print "No data will actually be transferred!"
# rsync's options: -n, -v and -z as set above.
rsync_options = [$o for [on o] in [[$DRY_RUN -n] [$VERBOSE -v] [$COMPRESS -z]] if $on]

#  Every week (actually of 8 days) and every month,
#+ extra backups are preserved.
DAY_OF_MONTH = (datetime.date.today).day
MONTHSTART = $DAY_OF_MONTH == 1                  # First of month.
WEEKSTART = $DAY_OF_MONTH in [8 16 24]
# Day 8,16,24  (use 8, not 7 to better handle 31-day months)

#  Check that the HDD is mounted.
#  At least, check that *something* is mounted here!
#  We can use something unique to the device, rather than just guessing
#+ the scsi-id by having an appropriate udev rule in
#+ /etc/udev/rules.d/10-rules.local
#+ and by putting a relevant entry in /etc/fstab.
#  Eg: this udev rule:
# BUS="scsi", KERNEL="sd*", SYSFS{vendor}="WDC WD16",
# SYSFS{model}="00JB-00GVA0     ", NAME="%k", SYMLINK="lacie_1394d%n"

def mounted = (str $MOUNT_POINT) in (mount)

UNMOUNT_LATER = False
if (mounted)
  print "Mount point $MOUNT_POINT is indeed mounted. OK"
else
  print "Attempting to mount $MOUNT_POINT..." end="" flush=True
  # If it isn't mounted, try to mount it.
  sudo mount $MOUNT_POINT 2>/dev/null

  if (mounted)
    UNMOUNT_LATER = True
    print "OK"
    # Note: Ensure that this is also unmounted
    # if we exit prematurely with failure (give_up, below).
  else
    print "FAILED"
    print "Nothing is mounted at $MOUNT_POINT. BACKUP FAILED!"
    exit $E_MOUNT_FAIL

# Before we exit, unmount the mount point if necessary.
def give_up status
  if $UNMOUNT_LATER
    cd
    sudo umount $MOUNT_POINT && echo "Unmounted $MOUNT_POINT again. Giving up."
  exit $status

# Check that source dir exists and is readable.
if not (os.access $SOURCE_DIR $os.R_OK)
  print "$SOURCE_DIR does not exist, or cannot be read. BACKUP FAILED."
  exit $E_NOSOURCEDIR

# Check that the backup directory structure is as it should be.
# If not, create it.
# Create the subdirectories.
# Note that backup.0 will be created as needed by rsync.
for i in (range 1 16)
  d = $BACKUP_DEST_DIR / "backup.$i"
  if not ($d.is_dir)
    try
      $d.mkdir parents=True
      print "Warning: directory $d is missing,"
      print "or was not initialised. (Re-)creating it."
    except OSError
      print "ERROR: directory $d"
      print "is missing and could not be created."
      give_up $E_UNMOUNTED

# Set the permission to 700 for security
# on an otherwise permissive multi-user system.
try
  $BACKUP_DEST_DIR.chmod 0o700
except OSError
  print "ERROR: Could not set permissions on $BACKUP_DEST_DIR to 700."
  give_up $E_UNMOUNTED

# Create the symlink: current -> backup.1 if required.
# A failure here is not critical.
cd $BACKUP_DEST_DIR
current = Path current
if not ($current.is_symlink)
  try
    $current.symlink_to backup.1
  except OSError
    print "WARNING: could not create symlink current -> backup.1"

# Now, do the rsync.
print "Now doing backup with rsync..."
print "Source dir: $SOURCE_DIR"
print "Backup destination dir: $BACKUP_DEST_DIR\n"

rsync $rsync_options -a -S --delete --modify-window=60 \
  --link-dest=../backup.1 $SOURCE_DIR "$BACKUP_DEST_DIR/backup.0/"

#  Only warn, rather than exit if the rsync failed,
#+ since it may only be a minor problem.
#  E.g., if one file is not readable, rsync will fail.
#  This shouldn't prevent the rotation.
#  Not using, e.g., `date +%a`  since these directories
#+ are just full of links and don't consume *that much* space.
BACKUP_JUSTINCASE = ""
if $? != 0
  BACKUP_JUSTINCASE = "backup.$(date +%F_%T).justincase"
  print "WARNING: the rsync process did not entirely succeed."
  print "Something might be wrong."
  print "Saving an extra copy at: $BACKUP_JUSTINCASE"
  print "WARNING: if this occurs regularly, a LOT of space will be consumed,"
  print "even though these are just hard-links!"

# Save a readme in the backup parent directory.
# Save another one in the recent subdirectory.
# (There's no backup.0 after a dry run: bash complains and goes on.)
try
  ($BACKUP_DEST_DIR / README.txt).write_text "Backup of $SOURCE_DIR on $(hostname) was last run on $(date)\n"
  ($BACKUP_DEST_DIR / backup.0 / README.txt).write_text "This backup of $SOURCE_DIR on $(hostname) was created on $(date)\n"
except OSError as e
  print "WARNING: $e"

# remove <n> : throw away backup.n
def remove n
  shutil.rmtree ($BACKUP_DEST_DIR / "backup.$n") ignore_errors=True

# move_up <low> <high> : backup.high … backup.low, each one up
# (an OSError if one can't be moved)
def move_up low high
  for i in (range $high ($low - 1) -1)
    ($BACKUP_DEST_DIR / "backup.$i").rename ($BACKUP_DEST_DIR / "backup.${$i + 1}")

SUCCESS = False
d = $BACKUP_DEST_DIR
# If we are not in a dry run, rotate the backups.
# (In the bash version the test guards only the next line, the df.)
if not $DRY_RUN
  # Check how full the backup disk is.
  # Warn if 90%. if 98% or more, we'll probably fail, so give up.
  # We test this here, rather than before
  # so that rsync may possibly have a chance.
  disk = shutil.disk_usage $BACKUP_DEST_DIR
  DISK_FULL_PERCENT = math.ceil (100 * $disk.used / ($disk.used + $disk.free))   # As df counts it.
  print "Disk space check on backup partition $MOUNT_POINT $DISK_FULL_PERCENT% full."
  if $DISK_FULL_PERCENT > 90
    print "Warning: Disk is greater than 90% full."
  if $DISK_FULL_PERCENT > 98
    print "Error: Disk is full! Giving up."
    give_up $E_UNMOUNTED

  # Create an extra backup.
  # If this copy fails, give up.
  if $BACKUP_JUSTINCASE
    if not (cp -al "$d/backup.0" "$d/$BACKUP_JUSTINCASE")
      print "ERROR: Failed to create extra copy $d/$BACKUP_JUSTINCASE"
      give_up $E_UNMOUNTED

  try
    if $MONTHSTART                 # At start of month, rotate the oldest 8.
      print "\nStart of month. Removing oldest backup: $d/backup.15"
      remove 15
      print "Rotating monthly,weekly backups: $d/backup.[8-14] -> $d/backup.[9-15]"
      move_up 8 14
    elif $WEEKSTART                # At start of week, rotate the second-oldest 4.
      print "\nStart of week. Removing oldest weekly backup: $d/backup.12"
      remove 12
      print "Rotating weekly backups: $d/backup.[8-11] -> $d/backup.[9-12]"
      move_up 8 11
    else
      print "\nRemoving oldest daily backup: $d/backup.8"
      remove 8
    # Every day, rotate the newest 8.
    print "Rotating daily backups: $d/backup.[1-7] -> $d/backup.[2-8]"
    move_up 0 7
    SUCCESS = True
  except OSError as e
    print "Rotation stopped: $e"

if $UNMOUNT_LATER
  # Unmount the mount point if it wasn't mounted to begin with.
  cd
  sudo umount $MOUNT_POINT && echo "Unmounted $MOUNT_POINT again."

if $SUCCESS
  print 'SUCCESS!'
  exit 0

# Should have already exited if backup worked.
print 'BACKUP FAILED! Is this just a dry run? Is the disk full?) '
exit $E_BACKUP
```

With its mount point and source folder in a temporary folder, and
`mounted` always true, it made the folders, the backup and the
rotations as described. The rotation itself, on a made-up backup folder
for three days of the month (each `backup.N` holds a file saying which
it was before):

```bash
# (nightly-backup.sh's rotation, on a made-up backup folder for three
#+ days of the month; mv and rm from PATH rather than /bin.)
show() {           # Which day's backup each backup.N now holds.
  for ((i=0;i<=15;i++)); do
    [ -d $BACKUP_DEST_DIR/backup.$i ] && echo -n " $i:$(ls $BACKUP_DEST_DIR/backup.$i)"
  done
  echo
}

for DAY_OF_MONTH in 05 08 01
do
BACKUP_DEST_DIR=dest-$DAY_OF_MONTH
for ((i=0;i<=15;i++)); do mkdir -p $BACKUP_DEST_DIR/backup.$i; touch $BACKUP_DEST_DIR/backup.$i/was$i; done
MONTHSTART= ; WEEKSTART=
if [ $DAY_OF_MONTH = 01 ]; then    # First of month.
  MONTHSTART=true
elif [ $DAY_OF_MONTH = 08 \
    -o $DAY_OF_MONTH = 16 \
    -o $DAY_OF_MONTH = 24 ]; then
    # Day 8,16,24  (use 8, not 7 to better handle 31-day months)
      WEEKSTART=true
fi
echo "Day $DAY_OF_MONTH:"

 # At start of month, rotate the oldest 8.
 if [ "$MONTHSTART" == "true" ]; then
   echo -e "Start of month. \
   Removing oldest backup: $BACKUP_DEST_DIR/backup.15"  &&
   rm -rf  $BACKUP_DEST_DIR/backup.15  &&
   echo "Rotating monthly,weekly backups: \
   $BACKUP_DEST_DIR/backup.[8-14] -> $BACKUP_DEST_DIR/backup.[9-15]"  &&
     mv $BACKUP_DEST_DIR/backup.14 $BACKUP_DEST_DIR/backup.15  &&
     mv $BACKUP_DEST_DIR/backup.13 $BACKUP_DEST_DIR/backup.14  &&
     mv $BACKUP_DEST_DIR/backup.12 $BACKUP_DEST_DIR/backup.13  &&
     mv $BACKUP_DEST_DIR/backup.11 $BACKUP_DEST_DIR/backup.12  &&
     mv $BACKUP_DEST_DIR/backup.10 $BACKUP_DEST_DIR/backup.11  &&
     mv $BACKUP_DEST_DIR/backup.9 $BACKUP_DEST_DIR/backup.10  &&
     mv $BACKUP_DEST_DIR/backup.8 $BACKUP_DEST_DIR/backup.9

 # At start of week, rotate the second-oldest 4.
 elif [ "$WEEKSTART" == "true" ]; then
   echo -e "Start of week. \
   Removing oldest weekly backup: $BACKUP_DEST_DIR/backup.12"  &&
   rm -rf  $BACKUP_DEST_DIR/backup.12  &&

   echo "Rotating weekly backups: \
   $BACKUP_DEST_DIR/backup.[8-11] -> $BACKUP_DEST_DIR/backup.[9-12]"  &&
     mv $BACKUP_DEST_DIR/backup.11 $BACKUP_DEST_DIR/backup.12  &&
     mv $BACKUP_DEST_DIR/backup.10 $BACKUP_DEST_DIR/backup.11  &&
     mv $BACKUP_DEST_DIR/backup.9 $BACKUP_DEST_DIR/backup.10  &&
     mv $BACKUP_DEST_DIR/backup.8 $BACKUP_DEST_DIR/backup.9

 else
   echo -e "Removing oldest daily backup: $BACKUP_DEST_DIR/backup.8"  &&
     rm -rf  $BACKUP_DEST_DIR/backup.8

 fi  &&

 # Every day, rotate the newest 8.
 echo "Rotating daily backups: \
 $BACKUP_DEST_DIR/backup.[1-7] -> $BACKUP_DEST_DIR/backup.[2-8]"  &&
     mv $BACKUP_DEST_DIR/backup.7 $BACKUP_DEST_DIR/backup.8  &&
     mv $BACKUP_DEST_DIR/backup.6 $BACKUP_DEST_DIR/backup.7  &&
     mv $BACKUP_DEST_DIR/backup.5 $BACKUP_DEST_DIR/backup.6  &&
     mv $BACKUP_DEST_DIR/backup.4 $BACKUP_DEST_DIR/backup.5  &&
     mv $BACKUP_DEST_DIR/backup.3 $BACKUP_DEST_DIR/backup.4  &&
     mv $BACKUP_DEST_DIR/backup.2 $BACKUP_DEST_DIR/backup.3  &&
     mv $BACKUP_DEST_DIR/backup.1 $BACKUP_DEST_DIR/backup.2  &&
     mv $BACKUP_DEST_DIR/backup.0 $BACKUP_DEST_DIR/backup.1  &&

 SUCCESS=true
 show
done
```

```output bash
Day 05:
Removing oldest daily backup: dest-05/backup.8
Rotating daily backups:  dest-05/backup.[1-7] -> dest-05/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was9 10:was10 11:was11 12:was12 13:was13 14:was14 15:was15
Day 08:
Start of week.    Removing oldest weekly backup: dest-08/backup.12
Rotating weekly backups:    dest-08/backup.[8-11] -> dest-08/backup.[9-12]
Rotating daily backups:  dest-08/backup.[1-7] -> dest-08/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was8 10:was9 11:was10 12:was11 13:was13 14:was14 15:was15
Day 01:
Start of month.    Removing oldest backup: dest-01/backup.15
Rotating monthly,weekly backups:    dest-01/backup.[8-14] -> dest-01/backup.[9-15]
Rotating daily backups:  dest-01/backup.[1-7] -> dest-01/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was8 10:was9 11:was10 12:was11 13:was12 14:was13 15:was14
```

(The extra spaces in bash's messages are the indentation after its `\`
line breaks inside the quotes.)

```tessh
import os shutil
# (nightly-backup's rotation, on a made-up backup folder for three days
# of the month.)
def show dest           # Which day's backup each backup.N now holds.
  held = []
  for i in (range 16)
    b = $dest / "backup.$i"
    if ($b.is_dir)
      $held.append "$i:$(' '.join (os.listdir $b))"
  print "" *$held

# remove <dest> <n> : throw away backup.n
def remove dest n
  shutil.rmtree ($dest / "backup.$n") ignore_errors=True

# move_up <dest> <low> <high> : backup.high … backup.low each one up
def move_up dest low high
  for i in (range $high ($low - 1) -1)
    ($dest / "backup.$i").rename ($dest / "backup.${$i + 1}")

for DAY_OF_MONTH in [5 8 1]
  BACKUP_DEST_DIR = Path "dest-$DAY_OF_MONTH"
  for i in (range 16)
    mkdir -p "$BACKUP_DEST_DIR/backup.$i" ; touch "$BACKUP_DEST_DIR/backup.$i/was$i"
  MONTHSTART = $DAY_OF_MONTH == 1                # First of month.
  WEEKSTART = $DAY_OF_MONTH in [8 16 24]         # (8, not 7, to better handle 31-day months)
  print "Day $DAY_OF_MONTH:"
  d = $BACKUP_DEST_DIR
  SUCCESS = try
    if $MONTHSTART                 # At start of month, rotate the oldest 8.
      print "Start of month. Removing oldest backup: $d/backup.15"
      remove $d 15
      print "Rotating monthly,weekly backups: $d/backup.[8-14] -> $d/backup.[9-15]"
      move_up $d 8 14
    elif $WEEKSTART                # At start of week, rotate the second-oldest 4.
      print "Start of week. Removing oldest weekly backup: $d/backup.12"
      remove $d 12
      print "Rotating weekly backups: $d/backup.[8-11] -> $d/backup.[9-12]"
      move_up $d 8 11
    else
      print "Removing oldest daily backup: $d/backup.8"
      remove $d 8
    # Every day, rotate the newest 8.
    print "Rotating daily backups: $d/backup.[1-7] -> $d/backup.[2-8]"
    move_up $d 0 7
    True
  except OSError as e
    print "Rotation failed: $e"
    False
  show $d
```

```output
Day 5:
Removing oldest daily backup: dest-5/backup.8
Rotating daily backups: dest-5/backup.[1-7] -> dest-5/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was9 10:was10 11:was11 12:was12 13:was13 14:was14 15:was15
Day 8:
Start of week. Removing oldest weekly backup: dest-8/backup.12
Rotating weekly backups: dest-8/backup.[8-11] -> dest-8/backup.[9-12]
Rotating daily backups: dest-8/backup.[1-7] -> dest-8/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was8 10:was9 11:was10 12:was11 13:was13 14:was14 15:was15
Day 1:
Start of month. Removing oldest backup: dest-1/backup.15
Rotating monthly,weekly backups: dest-1/backup.[8-14] -> dest-1/backup.[9-15]
Rotating daily backups: dest-1/backup.[1-7] -> dest-1/backup.[2-8]
 1:was0 2:was1 3:was2 4:was3 5:was4 6:was5 7:was6 8:was7 9:was8 10:was9 11:was10 12:was11 13:was12 14:was13 15:was14
```

## An expanded cd
<!-- help: abs-cdll cdll cd_new cd-history special-directories directory-stack -->

Phil Braham's `cdll` isn't a script to run but a file to source in a
login shell: it defines `cd_new`, to alias as `cd`, which keeps a history
of the last 50 folders (`cd 3` goes back to the fourth latest), ten
"special" entries to keep folders in (`cd -r2 dir`, `cd -s2`), suggests
folders when a name isn't found (`cd -a0` takes the first), lists all
that (`cd @`), and saves and loads it to a file (`cd -f`, `cd -u`).

### Example A-33. An expanded *cd* command (`cdll`)

```bash no-run
###########################################################################
#
#       cdll
#       by Phil Braham
#
#       ############################################
#       Latest version of this script available from
#       http://freshmeat.net/projects/cd/
#       ############################################
#
#       .cd_new
#
#       An enhancement of the Unix cd command
#
#       There are unlimited stack entries and special entries. The stack
#       entries keep the last cd_maxhistory
#       directories that have been used. The special entries can be
#       assigned to commonly used directories.
#
#       The special entries may be pre-assigned by setting the environment
#       variables CDSn or by using the -u or -U command.
#
#       The following is a suggestion for the .profile file:
#
#               . cdll              #  Set up the cd command
#       alias cd='cd_new'           #  Replace the cd command
#               cd -U               #  Upload pre-assigned entries for
#                                   #+ the stack and special entries
#               cd -D               #  Set non-default mode
#               alias @="cd_new @"  #  Allow @ to be used to get history
#
#       For help type:
#
#               cd -h or
#               cd -H
#
#
###########################################################################
#
#       Version 1.2.1
#
#       Written by Phil Braham - Realtime Software Pty Ltd
#       (realtime@mpx.com.au)
#       Please send any suggestions or enhancements to the author (also at
#       phil@braham.net)
#
############################################################################

cd_hm ()
{
        ${PRINTF} "%s" "cd [dir] [0-9] [@[s|h] [-g [<dir>]] [-d] \
[-D] [-r<n>] [dir|0-9] [-R<n>] [<dir>|0-9]
   [-s<n>] [-S<n>] [-u] [-U] [-f] [-F] [-h] [-H] [-v]
    <dir> Go to directory
    0-n         Go to previous directory (0 is previous, 1 is last but 1 etc)
                n is up to max history (default is 50)
    @           List history and special entries
    @h          List history entries
    @s          List special entries
    -g [<dir>]  Go to literal name (bypass special names)
                This is to allow access to dirs called '0','1','-h' etc
    -d          Change default action - verbose. (See note)
    -D          Change default action - silent. (See note)
    -s<n> Go to the special entry <n>*
    -S<n> Go to the special entry <n>
                and replace it with the current dir*
    -r<n> [<dir>] Go to directory <dir>
                              and then put it on special entry <n>*
    -R<n> [<dir>] Go to directory <dir>
                              and put current dir on special entry <n>*
    -a<n>       Alternative suggested directory. See note below.
    -f [<file>] File entries to <file>.
    -u [<file>] Update entries from <file>.
                If no filename supplied then default file
                (${CDPath}${2:-"$CDFile"}) is used
                -F and -U are silent versions
    -v          Print version number
    -h          Help
    -H          Detailed help

    *The special entries (0 - 9) are held until log off, replaced by another
     entry or updated with the -u command

    Alternative suggested directories:
    If a directory is not found then CD will suggest any
    possibilities. These are directories starting with the same letters
    and if any are found they are listed prefixed with -a<n>
    where <n> is a number.
    It's possible to go to the directory by entering cd -a<n>
    on the command line.
    
    The directory for -r<n> or -R<n> may be a number.
    For example:
        $ cd -r3 4  Go to history entry 4 and put it on special entry 3
        $ cd -R3 4  Put current dir on the special entry 3
                    and go to history entry 4
        $ cd -s3    Go to special entry 3
    
    Note that commands R,r,S and s may be used without a number
    and refer to 0:
        $ cd -s     Go to special entry 0
        $ cd -S     Go to special entry 0 and make special
                    entry 0 current dir
        $ cd -r 1   Go to history entry 1 and put it on special entry 0
        $ cd -r     Go to history entry 0 and put it on special entry 0
    "
        if ${TEST} "$CD_MODE" = "PREV"
        then
                ${PRINTF} "$cd_mnset"
        else
                ${PRINTF} "$cd_mset"
        fi
}

cd_Hm ()
{
        cd_hm
        ${PRINTF} "%s" "
        The previous directories (0-$cd_maxhistory) are stored in the
        environment variables CD[0] - CD[$cd_maxhistory]
        Similarly the special directories S0 - $cd_maxspecial are in
        the environment variable CDS[0] - CDS[$cd_maxspecial]
        and may be accessed from the command line

        The default pathname for the -f and -u commands is $CDPath
        The default filename for the -f and -u commands is $CDFile

        Set the following environment variables:
            CDL_PROMPTLEN  - Set to the length of prompt you require.
                Prompt string is set to the right characters of the
                current directory.
                If not set then prompt is left unchanged
            CDL_PROMPT_PRE - Set to the string to prefix the prompt.
                Default is:
                    non-root:  \"\\[\\e[01;34m\\]\"  (sets colour to blue).
                    root:      \"\\[\\e[01;31m\\]\"  (sets colour to red).
            CDL_PROMPT_POST    - Set to the string to suffix the prompt.
                Default is:
                    non-root:  \"\\[\\e[00m\\]$\"
                                (resets colour and displays $).
                    root:      \"\\[\\e[00m\\]#\"
                                (resets colour and displays #).
            CDPath - Set the default path for the -f & -u options.
                     Default is home directory
            CDFile - Set the default filename for the -f & -u options.
                     Default is cdfile
        
"
    cd_version

}

cd_version ()
{
 printf "Version: ${VERSION_MAJOR}.${VERSION_MINOR} Date: ${VERSION_DATE}\n"
}

#
# Truncate right.
#
# params:
#   p1 - string
#   p2 - length to truncate to
#
# returns string in tcd
#
cd_right_trunc ()
{
    local tlen=${2}
    local plen=${#1}
    local str="${1}"
    local diff
    local filler="<--"
    if ${TEST} ${plen} -le ${tlen}
    then
        tcd="${str}"
    else
        let diff=${plen}-${tlen}
        elen=3
        if ${TEST} ${diff} -le 2
        then
            let elen=${diff}
        fi
        tlen=-${tlen}
        let tlen=${tlen}+${elen}
        tcd=${filler:0:elen}${str:tlen}
    fi
}

#
# Three versions of do history:
#    cd_dohistory  - packs history and specials side by side
#    cd_dohistoryH - Shows only hstory
#    cd_dohistoryS - Shows only specials
#
cd_dohistory ()
{
    cd_getrc
        ${PRINTF} "History:\n"
    local -i count=${cd_histcount}
    while ${TEST} ${count} -ge 0
    do
        cd_right_trunc "${CD[count]}" ${cd_lchar}
            ${PRINTF} "%2d %-${cd_lchar}.${cd_lchar}s " ${count} "${tcd}"

        cd_right_trunc "${CDS[count]}" ${cd_rchar}
            ${PRINTF} "S%d %-${cd_rchar}.${cd_rchar}s\n" ${count} "${tcd}"
        count=${count}-1
    done
}

cd_dohistoryH ()
{
    cd_getrc
        ${PRINTF} "History:\n"
        local -i count=${cd_maxhistory}
        while ${TEST} ${count} -ge 0
        do
          ${PRINTF} "${count} %-${cd_flchar}.${cd_flchar}s\n" ${CD[$count]}
          count=${count}-1
        done
}

cd_dohistoryS ()
{
    cd_getrc
        ${PRINTF} "Specials:\n"
        local -i count=${cd_maxspecial}
        while ${TEST} ${count} -ge 0
        do
          ${PRINTF} "S${count} %-${cd_flchar}.${cd_flchar}s\n" ${CDS[$count]}
          count=${count}-1
        done
}

cd_getrc ()
{
    cd_flchar=$(stty -a | awk -F \;
    '/rows/ { print $2 $3 }' | awk -F \  '{ print $4 }')
    if ${TEST} ${cd_flchar} -ne 0
    then
        cd_lchar=${cd_flchar}/2-5
        cd_rchar=${cd_flchar}/2-5
            cd_flchar=${cd_flchar}-5
    else
            cd_flchar=${FLCHAR:=75}
	    # cd_flchar is used for for the @s & @h history
            cd_lchar=${LCHAR:=35}
            cd_rchar=${RCHAR:=35}
    fi
}

cd_doselection ()
{
        local -i nm=0
        cd_doflag="TRUE"
        if ${TEST} "${CD_MODE}" = "PREV"
        then
                if ${TEST} -z "$cd_npwd"
                then
                        cd_npwd=0
                fi
        fi
        tm=$(echo "${cd_npwd}" | cut -b 1)
    if ${TEST} "${tm}" = "-"
    then
        pm=$(echo "${cd_npwd}" | cut -b 2)
        nm=$(echo "${cd_npwd}" | cut -d $pm -f2)
        case "${pm}" in
             a) cd_npwd=${cd_sugg[$nm]} ;;
             s) cd_npwd="${CDS[$nm]}" ;;
             S) cd_npwd="${CDS[$nm]}" ; CDS[$nm]=`pwd` ;;
             r) cd_npwd="$2" ; cd_specDir=$nm ; cd_doselection "$1" "$2";;
             R) cd_npwd="$2" ; CDS[$nm]=`pwd` ; cd_doselection "$1" "$2";;
        esac
    fi

    if ${TEST} "${cd_npwd}" != "." -a "${cd_npwd}" \
!= ".." -a "${cd_npwd}" -le ${cd_maxhistory} >>/dev/null 2>&1
    then
      cd_npwd=${CD[$cd_npwd]}
     else
       case "$cd_npwd" in
                @)  cd_dohistory ; cd_doflag="FALSE" ;;
               @h) cd_dohistoryH ; cd_doflag="FALSE" ;;
               @s) cd_dohistoryS ; cd_doflag="FALSE" ;;
               -h) cd_hm ; cd_doflag="FALSE" ;;
               -H) cd_Hm ; cd_doflag="FALSE" ;;
               -f) cd_fsave "SHOW" $2 ; cd_doflag="FALSE" ;;
               -u) cd_upload "SHOW" $2 ; cd_doflag="FALSE" ;;
               -F) cd_fsave "NOSHOW" $2 ; cd_doflag="FALSE" ;;
               -U) cd_upload "NOSHOW" $2 ; cd_doflag="FALSE" ;;
               -g) cd_npwd="$2" ;;
               -d) cd_chdefm 1; cd_doflag="FALSE" ;;
               -D) cd_chdefm 0; cd_doflag="FALSE" ;;
               -r) cd_npwd="$2" ; cd_specDir=0 ; cd_doselection "$1" "$2";;
               -R) cd_npwd="$2" ; CDS[0]=`pwd` ; cd_doselection "$1" "$2";;
               -s) cd_npwd="${CDS[0]}" ;;
               -S) cd_npwd="${CDS[0]}"  ; CDS[0]=`pwd` ;;
               -v) cd_version ; cd_doflag="FALSE";;
       esac
    fi
}

cd_chdefm ()
{
        if ${TEST} "${CD_MODE}" = "PREV"
        then
                CD_MODE=""
                if ${TEST} $1 -eq 1
                then
                        ${PRINTF} "${cd_mset}"
                fi
        else
                CD_MODE="PREV"
                if ${TEST} $1 -eq 1
                then
                        ${PRINTF} "${cd_mnset}"
                fi
        fi
}

cd_fsave ()
{
        local sfile=${CDPath}${2:-"$CDFile"}
        if ${TEST} "$1" = "SHOW"
        then
                ${PRINTF} "Saved to %s\n" $sfile
        fi
        ${RM} -f ${sfile}
        local -i count=0
        while ${TEST} ${count} -le ${cd_maxhistory}
        do
                echo "CD[$count]=\"${CD[$count]}\"" >> ${sfile}
                count=${count}+1
        done
        count=0
        while ${TEST} ${count} -le ${cd_maxspecial}
        do
                echo "CDS[$count]=\"${CDS[$count]}\"" >> ${sfile}
                count=${count}+1
        done
}

cd_upload ()
{
        local sfile=${CDPath}${2:-"$CDFile"}
        if ${TEST} "${1}" = "SHOW"
        then
                ${PRINTF} "Loading from %s\n" ${sfile}
        fi
        . ${sfile}
}

cd_new ()
{
    local -i count
    local -i choose=0

        cd_npwd="${1}"
        cd_specDir=-1
        cd_doselection "${1}" "${2}"

        if ${TEST} ${cd_doflag} = "TRUE"
        then
                if ${TEST} "${CD[0]}" != "`pwd`"
                then
                        count=$cd_maxhistory
                        while ${TEST} $count -gt 0
                        do
                                CD[$count]=${CD[$count-1]}
                                count=${count}-1
                        done
                        CD[0]=`pwd`
                fi
                command cd "${cd_npwd}" 2>/dev/null
        if ${TEST} $? -eq 1
        then
            ${PRINTF} "Unknown dir: %s\n" "${cd_npwd}"
            local -i ftflag=0
            for i in "${cd_npwd}"*
            do
                if ${TEST} -d "${i}"
                then
                    if ${TEST} ${ftflag} -eq 0
                    then
                        ${PRINTF} "Suggest:\n"
                        ftflag=1
                fi
                    ${PRINTF} "\t-a${choose} %s\n" "$i"
                                        cd_sugg[$choose]="${i}"
                    choose=${choose}+1
        fi
            done
        fi
        fi

        if ${TEST} ${cd_specDir} -ne -1
        then
                CDS[${cd_specDir}]=`pwd`
        fi

        if ${TEST} ! -z "${CDL_PROMPTLEN}"
        then
        cd_right_trunc "${PWD}" ${CDL_PROMPTLEN}
            cd_rp=${CDL_PROMPT_PRE}${tcd}${CDL_PROMPT_POST}
                export PS1="$(echo -ne ${cd_rp})"
        fi
}
#########################################################################
#                                                                       #
#                        Initialisation here                            #
#                                                                       #
#########################################################################
#
VERSION_MAJOR="1"
VERSION_MINOR="2.1"
VERSION_DATE="24-MAY-2003"
#
alias cd=cd_new
#
# Set up commands
RM=/bin/rm
TEST=test
PRINTF=printf              # Use builtin printf

#########################################################################
#                                                                       #
# Change this to modify the default pre- and post prompt strings.       #
# These only come into effect if CDL_PROMPTLEN is set.                  #
#                                                                       #
#########################################################################
if ${TEST} ${EUID} -eq 0
then
#   CDL_PROMPT_PRE=${CDL_PROMPT_PRE:="$HOSTNAME@"}
    CDL_PROMPT_PRE=${CDL_PROMPT_PRE:="\\[\\e[01;31m\\]"}  # Root is in red
    CDL_PROMPT_POST=${CDL_PROMPT_POST:="\\[\\e[00m\\]#"}
else
    CDL_PROMPT_PRE=${CDL_PROMPT_PRE:="\\[\\e[01;34m\\]"}  # Users in blue
    CDL_PROMPT_POST=${CDL_PROMPT_POST:="\\[\\e[00m\\]$"}
fi
#########################################################################
#
# cd_maxhistory defines the max number of history entries allowed.
typeset -i cd_maxhistory=50

#########################################################################
#
# cd_maxspecial defines the number of special entries.
typeset -i cd_maxspecial=9
#
#
#########################################################################
#
#  cd_histcount defines the number of entries displayed in
#+ the history command.
typeset -i cd_histcount=9
#
#########################################################################
export CDPath=${HOME}/
#  Change these to use a different                                      #
#+ default path and filename                                            #
export CDFile=${CDFILE:=cdfile}           # for the -u and -f commands  #
#
#########################################################################
                                                                        #
typeset -i cd_lchar cd_rchar cd_flchar
                        #  This is the number of chars to allow for the #
cd_flchar=${FLCHAR:=75} #+ cd_flchar is used for for the @s & @h history#

typeset -ax CD CDS
#
cd_mset="\n\tDefault mode is now set - entering cd with no parameters \
has the default action\n\tUse cd -d or -D for cd to go to \
previous directory with no parameters\n"
cd_mnset="\n\tNon-default mode is now set - entering cd with no \
parameters is the same as entering cd 0\n\tUse cd -d or \
-D to change default cd action\n"

# ==================================================================== #



: <<DOCUMENTATION

Written by Phil Braham. Realtime Software Pty Ltd.
Released under GNU license. Free to use. Please pass any modifications
or comments to the author Phil Braham:

realtime@mpx.com.au
=======================================================================

cdll is a replacement for cd and incorporates similar functionality to
the bash pushd and popd commands but is independent of them.

This version of cdll has been tested on Linux using Bash. It will work
on most Linux versions but will probably not work on other shells without
modification.

Introduction
============

cdll allows easy moving about between directories. When changing to a new
directory the current one is automatically put onto a stack. By default
50 entries are kept, but this is configurable. Special directories can be
kept for easy access - by default up to 10, but this is configurable. The
most recent stack entries and the special entries can be easily viewed.

The directory stack and special entries can be saved to, and loaded from,
a file. This allows them to be set up on login, saved before logging out
or changed when moving project to project.

In addition, cdll provides a flexible command prompt facility that allows,
for example, a directory name in colour that is truncated from the left
if it gets too long.


Setting up cdll
===============

Copy cdll to either your local home directory or a central directory
such as /usr/bin (this will require root access).

Copy the file cdfile to your home directory. It will require read and
write access. This a default file that contains a directory stack and
special entries.

To replace the cd command you must add commands to your login script.
The login script is one or more of:

    /etc/profile
    ~/.bash_profile
    ~/.bash_login
    ~/.profile
    ~/.bashrc
    /etc/bash.bashrc.local
    
To setup your login, ~/.bashrc is recommended, for global (and root) setup
add the commands to /etc/bash.bashrc.local
    
To set up on login, add the command:
    . <dir>/cdll
For example if cdll is in your local home directory:
    . ~/cdll
If in /usr/bin then:
    . /usr/bin/cdll

If you want to use this instead of the buitin cd command then add:
    alias cd='cd_new'
We would also recommend the following commands:
    alias @='cd_new @'
    cd -U
    cd -D

If you want to use cdll's prompt facilty then add the following:
    CDL_PROMPTLEN=nn
Where nn is a number described below. Initially 99 would be suitable
number.

Thus the script looks something like this:

    ######################################################################
    # CD Setup
    ######################################################################
    CDL_PROMPTLEN=21        # Allow a prompt length of up to 21 characters
    . /usr/bin/cdll         # Initialise cdll
    alias cd='cd_new'       # Replace the built in cd command
    alias @='cd_new @'      # Allow @ at the prompt to display history
    cd -U                   # Upload directories
    cd -D                   # Set default action to non-posix
    ######################################################################

The full meaning of these commands will become clear later.

There are a couple of caveats. If another program changes the directory
without calling cdll, then the directory won't be put on the stack and
also if the prompt facility is used then this will not be updated. Two
programs that can do this are pushd and popd. To update the prompt and
stack simply enter:

    cd .
    
Note that if the previous entry on the stack is the current directory
then the stack is not updated.

Usage
=====  
cd [dir] [0-9] [@[s|h] [-g <dir>] [-d] [-D] [-r<n>]
   [dir|0-9] [-R<n>] [<dir>|0-9] [-s<n>] [-S<n>]
   [-u] [-U] [-f] [-F] [-h] [-H] [-v]

    <dir>       Go to directory
    0-n         Goto previous directory (0 is previous,
                1 is last but 1, etc.)
                n is up to max history (default is 50)
    @           List history and special entries (Usually available as $ @)
    @h          List history entries
    @s          List special entries
    -g [<dir>]  Go to literal name (bypass special names)
                This is to allow access to dirs called '0','1','-h' etc
    -d          Change default action - verbose. (See note)
    -D          Change default action - silent. (See note)
    -s<n>       Go to the special entry <n>
    -S<n>       Go to the special entry <n>
                      and replace it with the current dir
    -r<n> [<dir>] Go to directory <dir>
                              and then put it on special entry <n>
    -R<n> [<dir>] Go to directory <dir>
                              and put current dir on special entry <n>
    -a<n>       Alternative suggested directory. See note below.
    -f [<file>] File entries to <file>.
    -u [<file>] Update entries from <file>.
                If no filename supplied then default file (~/cdfile) is used
                -F and -U are silent versions
    -v          Print version number
    -h          Help
    -H          Detailed help



Examples
========

These examples assume non-default mode is set (that is, cd with no
parameters will go to the most recent stack directory), that aliases
have been set up for cd and @ as described above and that cd's prompt
facility is active and the prompt length is 21 characters.

    /home/phil$ @
    # List the entries with the @
    History:
    # Output of the @ command
    .....
    # Skipped these entries for brevity
    1 /home/phil/ummdev               S1 /home/phil/perl
    # Most recent two history entries
    0 /home/phil/perl/eg              S0 /home/phil/umm/ummdev
    # and two special entries are shown
    
    /home/phil$ cd /home/phil/utils/Cdll
    # Now change directories
    /home/phil/utils/Cdll$ @
    # Prompt reflects the directory.
    History:
    # New history
    .....   
    1 /home/phil/perl/eg              S1 /home/phil/perl
    # History entry 0 has moved to 1
    0 /home/phil                      S0 /home/phil/umm/ummdev
    # and the most recent has entered
       
To go to a history entry:

    /home/phil/utils/Cdll$ cd 1
    # Go to history entry 1.
    /home/phil/perl/eg$
    # Current directory is now what was 1
    
To go to a special entry:

    /home/phil/perl/eg$ cd -s1
    # Go to special entry 1
    /home/phil/umm/ummdev$
    # Current directory is S1

To go to a directory called, for example, 1:

    /home/phil$ cd -g 1
    # -g ignores the special meaning of 1
    /home/phil/1$
    
To put current directory on the special list as S1:
    cd -r1 .        #  OR
    cd -R1 .        #  These have the same effect if the directory is
                    #+ . (the current directory)

To go to a directory and add it as a special  
  The directory for -r<n> or -R<n> may be a number.
  For example:
        $ cd -r3 4  Go to history entry 4 and put it on special entry 3
        $ cd -R3 4  Put current dir on the special entry 3 and go to
                    history entry 4
        $ cd -s3    Go to special entry 3

    Note that commands R,r,S and s may be used without a number and
    refer to 0:
        $ cd -s     Go to special entry 0
        $ cd -S     Go to special entry 0 and make special entry 0
                    current dir
        $ cd -r 1   Go to history entry 1 and put it on special entry 0
        $ cd -r     Go to history entry 0 and put it on special entry 0


    Alternative suggested directories:

    If a directory is not found, then CD will suggest any
    possibilities. These are directories starting with the same letters
    and if any are found they are listed prefixed with -a<n>
    where <n> is a number. It's possible to go to the directory
    by entering cd -a<n> on the command line.

        Use cd -d or -D to change default cd action. cd -H will show
        current action.

        The history entries (0-n) are stored in the environment variables
        CD[0] - CD[n]
        Similarly the special directories S0 - 9 are in the environment
        variable CDS[0] - CDS[9]
        and may be accessed from the command line, for example:
        
            ls -l ${CDS[3]}
            cat ${CD[8]}/file.txt

        The default pathname for the -f and -u commands is ~
        The default filename for the -f and -u commands is cdfile


Configuration
=============

    The following environment variables can be set:
    
        CDL_PROMPTLEN  - Set to the length of prompt you require.
            Prompt string is set to the right characters of the current
            directory. If not set, then prompt is left unchanged. Note
            that this is the number of characters that the directory is
            shortened to, not the total characters in the prompt.

            CDL_PROMPT_PRE - Set to the string to prefix the prompt.
                Default is:
                    non-root:  "\\[\\e[01;34m\\]"  (sets colour to blue).
                    root:      "\\[\\e[01;31m\\]"  (sets colour to red).

            CDL_PROMPT_POST    - Set to the string to suffix the prompt.
                Default is:
                    non-root:  "\\[\\e[00m\\]$"
                               (resets colour and displays $).
                    root:      "\\[\\e[00m\\]#"
                               (resets colour and displays #).

        Note:
            CDL_PROMPT_PRE & _POST only t

        CDPath - Set the default path for the -f & -u options.
                 Default is home directory
        CDFile - Set the default filename for the -f & -u options.
                 Default is cdfile


    There are three variables defined in the file cdll which control the
    number of entries stored or displayed. They are in the section labeled
    'Initialisation here' towards the end of the file.

        cd_maxhistory       - The number of history entries stored.
                              Default is 50.
        cd_maxspecial       - The number of special entries allowed.
                              Default is 9.
        cd_histcount        - The number of history and special entries
                              displayed. Default is 9.

    Note that cd_maxspecial should be >= cd_histcount to avoid displaying
    special entries that can't be set.


Version: 1.2.1 Date: 24-MAY-2003

DOCUMENTATION
```

The tessh version is a file to `source` from `~/.config/tessh/rc.tsh`.
The arrays are lists and the `printf` formats are f-strings and
`.ljust`; `@command` lets `cd_new` take its arguments as words, and
`cd = $cd_new` stands in for the alias. Inside it, the real `cd` is
`builtin_cd`, taken from `$cd` before that (bash's `command cd`); a
folder that isn't there is an `OSError`. The terminal width comes from
`shutil.get_terminal_size`: the bash's `stty -a | awk -F \;` has a line
break before the awk program, so `awk` only prints its usage, and the
listing always falls back to 75 columns. `cd -u` reads the
`CD[n]="dir"` lines that `cd -f` writes rather than sourcing them, so the
same file works with both versions.

**Gap:** there is no alias for `@` (the bash suggests `alias @="cd_new
@"`): `@` can't be a name in tessh. Type `cd @`. And tessh's prompt
isn't `PS1`: the `CDL_PROMPTLEN` prompt is worked out and put in
`$env.PS1` as in bash, for the programs started from the shell, but the
REPL doesn't show it.

```tessh no-run
###########################################################################
#
#       cdll.tsh (ported from cdll)
#       by Phil Braham
#
#       ############################################
#       Latest version of this script available from
#       http://freshmeat.net/projects/cd/
#       ############################################
#
#       An enhancement of the Unix cd command
#
#       There are unlimited stack entries and special entries. The stack
#       entries keep the last cd_maxhistory
#       directories that have been used. The special entries can be
#       assigned to commonly used directories.
#
#       The special entries may be pre-assigned by using the -u or -U
#       command.
#
#       The following is a suggestion for ~/.config/tessh/rc.tsh:
#
#               source ~/cdll.tsh   #  Set up the cd command
#               cd = $cd_new        #  Replace the cd command
#               cd -U               #  Upload pre-assigned entries for
#                                   #+ the stack and special entries
#               cd -D               #  Set non-default mode
#
#       For help type:
#
#               cd -h or
#               cd -H
#
###########################################################################
#
#       Version 1.2.1
#
#       Written by Phil Braham - Realtime Software Pty Ltd
#       (realtime@mpx.com.au)
#       Please send any suggestions or enhancements to the author (also at
#       phil@braham.net)
#
############################################################################

import os re glob shutil

def cd_hm
  print """cd [dir] [0-9] [@[s|h] [-g [<dir>]] [-d] [-D] [-r<n>] [dir|0-9] [-R<n>] [<dir>|0-9]
   [-s<n>] [-S<n>] [-u] [-U] [-f] [-F] [-h] [-H] [-v]
    <dir> Go to directory
    0-n         Go to previous directory (0 is previous, 1 is last but 1 etc)
                n is up to max history (default is 50)
    @           List history and special entries
    @h          List history entries
    @s          List special entries
    -g [<dir>]  Go to literal name (bypass special names)
                This is to allow access to dirs called '0','1','-h' etc
    -d          Change default action - verbose. (See note)
    -D          Change default action - silent. (See note)
    -s<n> Go to the special entry <n>*
    -S<n> Go to the special entry <n>
                and replace it with the current dir*
    -r<n> [<dir>] Go to directory <dir>
                              and then put it on special entry <n>*
    -R<n> [<dir>] Go to directory <dir>
                              and put current dir on special entry <n>*
    -a<n>       Alternative suggested directory. See note below.
    -f [<file>] File entries to <file>.
    -u [<file>] Update entries from <file>.
                If no filename supplied then default file
                ($CDPath$CDFile) is used
                -F and -U are silent versions
    -v          Print version number
    -h          Help
    -H          Detailed help

    *The special entries (0 - 9) are held until log off, replaced by another
     entry or updated with the -u command

    Alternative suggested directories:
    If a directory is not found then CD will suggest any
    possibilities. These are directories starting with the same letters
    and if any are found they are listed prefixed with -a<n>
    where <n> is a number.
    It's possible to go to the directory by entering cd -a<n>
    on the command line.

    The directory for -r<n> or -R<n> may be a number.
    For example:
        \$ cd -r3 4  Go to history entry 4 and put it on special entry 3
        \$ cd -R3 4  Put current dir on the special entry 3
                    and go to history entry 4
        \$ cd -s3    Go to special entry 3

    Note that commands R,r,S and s may be used without a number
    and refer to 0:
        \$ cd -s     Go to special entry 0
        \$ cd -S     Go to special entry 0 and make special
                    entry 0 current dir
        \$ cd -r 1   Go to history entry 1 and put it on special entry 0
        \$ cd -r     Go to history entry 0 and put it on special entry 0
    """ end=""
  print (if $CD_MODE == PREV then $cd_mnset else $cd_mset) end=""

def cd_Hm
  cd_hm
  print """
        The previous directories (0-$cd_maxhistory) are stored in the
        list CD[0] - CD[$cd_maxhistory]
        Similarly the special directories S0 - $cd_maxspecial are in
        the list CDS[0] - CDS[$cd_maxspecial]
        and may be accessed from the command line

        The default pathname for the -f and -u commands is $CDPath
        The default filename for the -f and -u commands is $CDFile

        Set the following environment variables:
            CDL_PROMPTLEN  - Set to the length of prompt you require.
                Prompt string is set to the right characters of the
                current directory.
                If not set then prompt is left unchanged
            CDL_PROMPT_PRE - Set to the string to prefix the prompt.
            CDL_PROMPT_POST    - Set to the string to suffix the prompt.
            CDPath - Set the default path for the -f & -u options.
                     Default is home directory
            CDFile - Set the default filename for the -f & -u options.
                     Default is cdfile

""" end=""
  cd_version

def cd_version
  print "Version: $VERSION_MAJOR.$VERSION_MINOR Date: $VERSION_DATE"

#
# Truncate right: the right-hand end of a string, at most `length` long,
# with "<--" in front when it's cut.
#
def cd_right_trunc s length
  if (len $s) <= $length
    return $s
  diff = (len $s) - $length
  elen = min 3 $diff
  "<--"[:$elen] + $s[($elen - $length):]

# printf's %-n.ns: s, cut or padded to n.
def pad s n = ($s.ljust $n)[:$n]

#
# Three versions of do history:
#    cd_dohistory  - packs history and specials side by side
#    cd_dohistoryH - Shows only history
#    cd_dohistoryS - Shows only specials
#
def cd_dohistory
  [flchar lchar rchar] = (cd_getrc)
  print "History:"
  for count in (range $cd_histcount -1 -1)
    left = cd_right_trunc $CD[$count] $lchar
    right = cd_right_trunc $CDS[$count] $rchar
    print "${count:2d} $(pad $left $lchar) S$count $(pad $right $rchar)"

def cd_dohistoryH
  [flchar lchar rchar] = (cd_getrc)
  print "History:"
  for count in (range $cd_maxhistory -1 -1)
    print "$count $(pad $CD[$count] $flchar)"

def cd_dohistoryS
  [flchar lchar rchar] = (cd_getrc)
  print "Specials:"
  for count in (range $cd_maxspecial -1 -1)
    print "S$count $(pad $CDS[$count] $flchar)"

# The widths to show: [whole line, history, specials], from the
# terminal's width (80 when there's no terminal).
def cd_getrc
  columns = (shutil.get_terminal_size).columns
  [($columns - 5) ($columns // 2 - 5) ($columns // 2 - 5)]

def cd_doselection a1 a2
  global cd_npwd cd_specDir cd_doflag
  cd_doflag = True
  if $CD_MODE == "PREV" and not $cd_npwd
    cd_npwd = "0"
  if $cd_npwd =~ re'^-[asSrR]'
    # -s3: the letter, then the number of the entry (0 if none).
    pm = $cd_npwd[1]
    nm = int ($cd_npwd[2:] or 0)
    match $pm
      case a
        cd_npwd = $cd_sugg[$nm]
      case s
        cd_npwd = $CDS[$nm]
      case S
        cd_npwd = $CDS[$nm] ; $CDS[$nm] = (pwd)
      case r
        cd_npwd = $a2 ; cd_specDir = $nm ; cd_doselection $a1 $a2
      case R
        cd_npwd = $a2 ; $CDS[$nm] = (pwd) ; cd_doselection $a1 $a2

  if $cd_npwd =~ re'^-?\d+$' and (int $cd_npwd) <= $cd_maxhistory
    cd_npwd = $CD[(int $cd_npwd)]
  else
    match $cd_npwd
      case "@"
        cd_dohistory ; cd_doflag = False
      case "@h"
        cd_dohistoryH ; cd_doflag = False
      case "@s"
        cd_dohistoryS ; cd_doflag = False
      case "-h"
        cd_hm ; cd_doflag = False
      case "-H"
        cd_Hm ; cd_doflag = False
      case "-f"
        cd_fsave SHOW $a2 ; cd_doflag = False
      case "-u"
        cd_upload SHOW $a2 ; cd_doflag = False
      case "-F"
        cd_fsave NOSHOW $a2 ; cd_doflag = False
      case "-U"
        cd_upload NOSHOW $a2 ; cd_doflag = False
      case "-g"
        cd_npwd = $a2
      case "-d"
        cd_chdefm 1 ; cd_doflag = False
      case "-D"
        cd_chdefm 0 ; cd_doflag = False
      case "-v"
        cd_version ; cd_doflag = False
      # (-r, -R, -s and -S without a number are done above, as 0.)

def cd_chdefm verbose
  global CD_MODE
  if $CD_MODE == PREV
    CD_MODE = ""
    if $verbose
      print $cd_mset end=""
  else
    CD_MODE = PREV
    if $verbose
      print $cd_mnset end=""

# The entries file: CD[n]="dir" and CDS[n]="dir" lines, as the bash
# version writes and reads them.
def cd_file name = Path ($CDPath + ($name or $CDFile))

def cd_fsave show name=""
  sfile = cd_file $name
  if $show == SHOW
    print "Saved to $sfile"
  with (open $sfile w) as f
    for [i d] in (enumerate $CD)
      print "CD[$i]=\"$d\"" file=$f
    for [i d] in (enumerate $CDS)
      print "CDS[$i]=\"$d\"" file=$f

def cd_upload show name=""
  sfile = cd_file $name
  if $show == SHOW
    print "Loading from $sfile"
  # (bash sources the file; here its lines are read.)
  for m in (re'^(CDS?)\[(\d+)\]="(.*)"$'m.finditer ($sfile.read_text))
    entries = if ($m.group 1) == CD then $CD else $CDS
    i = int ($m.group 2)
    if $i < (len $entries)
      $entries[$i] = $m.group 3

builtin_cd = $cd             # (bash: command cd)

@command
def cd_new a1="" a2=""
  global cd_npwd cd_specDir
  choose = 0
  cd_npwd = $a1
  cd_specDir = -1
  cd_doselection $a1 $a2

  if $cd_doflag
    if $CD[0] != (pwd)
      $CD[1:] = $CD[:-1]          # Everything down one . . .
      $CD[0] = (pwd)              # . . . and here on top.
    try
      if $cd_npwd
        builtin_cd $cd_npwd
      else
        builtin_cd                # (Plain cd: go home.)
    except OSError
      print "Unknown dir: $cd_npwd"
      found = [$d for d in (sorted (glob.glob "$cd_npwd*")) if (os.path.isdir $d)]
      if $found
        print "Suggest:"
      for [choose d] in (enumerate $found)
        print "\t-a$choose $d"
        $cd_sugg[$choose] = $d

  if $cd_specDir != -1
    $CDS[$cd_specDir] = (pwd)

  if ($env.get CDL_PROMPTLEN)
    tcd = cd_right_trunc (pwd) (int $env.CDL_PROMPTLEN)
    $env.PS1 = $CDL_PROMPT_PRE + $tcd + $CDL_PROMPT_POST

#########################################################################
#                                                                       #
#                        Initialisation here                            #
#                                                                       #
#########################################################################
#
VERSION_MAJOR = "1"
VERSION_MINOR = "2.1"
VERSION_DATE = "24-MAY-2003"

#########################################################################
#                                                                       #
# Change this to modify the default pre- and post prompt strings.       #
# These only come into effect if CDL_PROMPTLEN is set.                  #
#                                                                       #
#########################################################################
if (os.geteuid) == 0
  CDL_PROMPT_PRE = $env.get CDL_PROMPT_PRE '\[\e[01;31m\]'    # Root is in red
  CDL_PROMPT_POST = $env.get CDL_PROMPT_POST '\[\e[00m\]#'
else
  CDL_PROMPT_PRE = $env.get CDL_PROMPT_PRE '\[\e[01;34m\]'    # Users in blue
  CDL_PROMPT_POST = $env.get CDL_PROMPT_POST '\[\e[00m\]$'
#########################################################################
#
# cd_maxhistory defines the max number of history entries allowed.
cd_maxhistory = 50

#########################################################################
#
# cd_maxspecial defines the number of special entries.
cd_maxspecial = 9

#########################################################################
#
# cd_histcount defines the number of entries displayed in
# the history command.
cd_histcount = 9

#########################################################################
#  Change these to use a different
#+ default path and filename for the -u and -f commands
CDPath = $env.HOME + "/"
CDFile = $env.get CDFILE cdfile
#########################################################################

CD = [""] * ($cd_maxhistory + 1)   # The history . . .
CDS = [""] * ($cd_maxspecial + 1)  # . . . the special entries,
cd_sugg = {}                       # and the suggestions.
CD_MODE = ""
cd_npwd = "" ; cd_specDir = -1 ; cd_doflag = True

cd_mset = "\n\tDefault mode is now set - entering cd with no parameters has the default action\n\tUse cd -d or -D for cd to go to previous directory with no parameters\n"
cd_mnset = "\n\tNon-default mode is now set - entering cd with no parameters is the same as entering cd 0\n\tUse cd -d or -D to change default cd action\n"

# ==================================================================== #

# (The bash file ends with Phil Braham's documentation of cdll; see
# the ABS Guide. In short: `cd dir` as usual; `cd 3` goes to history
# entry 3; `cd -s2` to special entry 2; `cd -r2 dir` goes to dir and
# keeps it as special 2; `cd @` lists history and specials; `cd -f` and
# `cd -u` save them to and load them from ~/cdfile.)
```

Sourced into a script and tried with the same steps as the bash one,
it went to the same folders and printed the same lists. Two of
its parts here: cutting a path to the prompt's length,

```bash
# (cdll's cd_right_trunc, on made-up paths.)
TEST=test
#
# Truncate right.
#
# params:
#   p1 - string
#   p2 - length to truncate to
#
# returns string in tcd
#
cd_right_trunc ()
{
    local tlen=${2}
    local plen=${#1}
    local str="${1}"
    local diff
    local filler="<--"
    if ${TEST} ${plen} -le ${tlen}
    then
        tcd="${str}"
    else
        let diff=${plen}-${tlen}
        elen=3
        if ${TEST} ${diff} -le 2
        then
            let elen=${diff}
        fi
        tlen=-${tlen}
        let tlen=${tlen}+${elen}
        tcd=${filler:0:elen}${str:tlen}
    fi
}

for n in 30 25 24 20 10
do
  cd_right_trunc /home/phil/utils/Cdll/src $n
  echo "$n: $tcd"
done
```

```tessh
# (cdll's cd_right_trunc, on made-up paths.)
#
# Truncate right: the right-hand end of a string, at most `length` long,
# with "<--" in front when it's cut.
#
def cd_right_trunc s length
  if (len $s) <= $length
    return $s
  diff = (len $s) - $length
  elen = min 3 $diff
  "<--"[:$elen] + $s[($elen - $length):]

for n in [30 25 24 20 10]
  print "$n: $(cd_right_trunc "/home/phil/utils/Cdll/src" $n)"
```

```output
30: /home/phil/utils/Cdll/src
25: /home/phil/utils/Cdll/src
24: <ome/phil/utils/Cdll/src
20: <--il/utils/Cdll/src
10: <--dll/src
```

and a cut-down `cd_new` (history, special entries and suggestions, five
of each) on a made-up tree of folders:

```tessh
import os glob
# (cdll's cd_new, cut down to the history, the special entries and the
# suggestions, with 5 of each; and a made-up tree of folders to try it on.)
mkdir -p projects/alpha projects/beta docs music
top = (pwd)
def here d = if $d then (os.path.relpath $d $top) else "-"

cd_maxhistory = 4 ; cd_maxspecial = 4
CD = [""] * ($cd_maxhistory + 1) ; CDS = [""] * ($cd_maxspecial + 1)
cd_sugg = {}
builtin_cd = $cd

@command
def cd_new a1="" a2=""
  npwd = $a1 ; specDir = -1
  if $npwd =~ re'^-[asr]'           # -s3, -r2 dir, -a0: letter, entry number
    nm = int ($npwd[2:] or 0)
    match $npwd[1]
      case a
        npwd = $cd_sugg[$nm]
      case s
        npwd = $CDS[$nm]
      case r
        npwd = $a2 ; specDir = $nm
  if $npwd =~ re'^\d+$' and (int $npwd) <= $cd_maxhistory
    npwd = $CD[(int $npwd)]           # A history entry.
  elif $npwd == "@"
    for i in (range $cd_maxhistory -1 -1)
      print "$i ${here $CD[$i]:<16} S$i $(here $CDS[$i])"
    return
  if $CD[0] != (pwd)
    $CD[1:] = $CD[:-1]
    $CD[0] = (pwd)
  try
    builtin_cd $npwd
  except OSError
    print "Unknown dir: $(here $npwd)"
    found = [$d for d in (sorted (glob.glob "$npwd*")) if (os.path.isdir $d)]
    if $found
      print "Suggest:"
    for [choose d] in (enumerate $found)
      print "\t-a$choose $(here $d)"
      $cd_sugg[$choose] = $d
  if $specDir != -1
    $CDS[$specDir] = (pwd)
  print "now in $(here (pwd))"

cd = $cd_new
cd projects
cd alpha
cd "$top/docs"
cd 1                     # Back to history entry 1, projects.
cd -r2 "$top/music"      # To music, and keep it as special entry 2.
cd "$top/projects/b"     # No such folder: suggestions.
cd -a0                   # The first suggestion.
cd -s2                   # Special entry 2.
cd @
```

```output
now in projects
now in projects/alpha
now in docs
now in projects
now in music
Unknown dir: projects/b
Suggest:
	-a0 projects/beta
now in music
now in projects/beta
now in music
4 projects/alpha   S4 -
3 docs             S3 -
2 projects         S2 music
1 music            S1 -
0 projects/beta    S0 -
```

## Setting up a PnP sound card
<!-- help: abs-soundcard-on soundcard-on sysfs pnp modprobe device-files -->

Mkarcher's `soundcard-on.sh` gets the sound chip of old ThinkPads
working: it finds the chip's two Plug and Play devices in `/sys`,
switches them on by writing to their `resources` files, reads back the
ports, interrupt and DMA channels they were given, and loads the driver
with those settings. It needs root and that hardware.

### Example A-34. A soundcard setup script (`soundcard-on.sh`)

```bash no-run
#!/bin/bash
# soundcard-on.sh

#  Script author: Mkarcher
#  http://www.thinkwiki.org/wiki  ...
#  /Script_for_configuring_the_CS4239_sound_chip_in_PnP_mode
#  ABS Guide author made minor changes and added comments.
#  Couldn't contact script author to ask for permission to use, but ...
#+ the script was released under the FDL,
#+ so its use here should be both legal and ethical.

#  Sound-via-pnp-script for Thinkpad 600E
#+ and possibly other computers with onboard CS4239/CS4610
#+ that do not work with the PCI driver
#+ and are not recognized by the PnP code of snd-cs4236.
#  Also for some 770-series Thinkpads, such as the 770x.
#  Run as root user, of course.
#
#  These are old and very obsolete laptop computers,
#+ but this particular script is very instructive,
#+ as it shows how to set up and hack device files.



#  Search for sound card pnp device:

for dev in /sys/bus/pnp/devices/*
do
  grep CSC0100 $dev/id > /dev/null && WSSDEV=$dev
  grep CSC0110 $dev/id > /dev/null && CTLDEV=$dev
done
# On 770x:
# WSSDEV = /sys/bus/pnp/devices/00:07
# CTLDEV = /sys/bus/pnp/devices/00:06
# These are symbolic links to /sys/devices/pnp0/ ...


#  Activate devices:
#  Thinkpad boots with devices disabled unless "fast boot" is turned off
#+ (in BIOS).

echo activate > $WSSDEV/resources
echo activate > $CTLDEV/resources


# Parse resource settings.

{ read # Discard "state = active" (see below).
  read bla port1
  read bla port2
  read bla port3
  read bla irq
  read bla dma1
  read bla dma2
 # The "bla's" are labels in the first field: "io," "state," etc.
 # These are discarded.

 #  Hack: with PnPBIOS: ports are: port1: WSS, port2:
 #+ OPL, port3: sb (unneeded)
 #       with ACPI-PnP:ports are: port1: OPL, port2: sb, port3: WSS
 #  (ACPI bios seems to be wrong here, the PnP-card-code in snd-cs4236.c
 #+  uses the PnPBIOS port order)
 #  Detect port order using the fixed OPL port as reference.
  if [ ${port2%%-*} = 0x388 ]
 #            ^^^^  Strip out everything following hyphen in port address.
 #                  So, if port1 is 0x530-0x537
 #+                 we're left with 0x530 -- the start address of the port.
 then
   # PnPBIOS: usual order
   port=${port1%%-*}
   oplport=${port2%%-*}
 else
   # ACPI: mixed-up order
   port=${port3%%-*}
   oplport=${port1%%-*}
 fi
 } < $WSSDEV/resources
# To see what's going on here:
# ---------------------------
#   cat /sys/devices/pnp0/00:07/resources
#
#   state = active
#   io 0x530-0x537
#   io 0x388-0x38b
#   io 0x220-0x233
#   irq 5
#   dma 1
#   dma 0
#   ^^^   "bla" labels in first field (discarded). 


{ read # Discard first line, as above.
  read bla port1
  cport=${port1%%-*}
  #            ^^^^
  # Just want _start_ address of port.
} < $CTLDEV/resources


# Load the module:

modprobe --ignore-install snd-cs4236 port=$port cport=$cport\
fm_port=$oplport irq=$irq dma1=$dma1 dma2=$dma2 isapnp=0 index=0
# See the modprobe manpage.

exit $?
```

The `/sys` files are Paths in tessh: `.read_text` to look for the chip's
ID and to read the settings, `.write_text` to activate. The `{ read …;
read bla port1; … } < file` block, which reads one line per variable and
throws away the labels, is a function that takes the second word of each
line, unpacked into the same names; `${port1%%-*}` is
`.partition "-"`. One fix: in the bash, `cport=$cport\` has no space
before the line break, so `cport` and `fm_port` reach `modprobe` as one
word.

```tessh no-run
#!/usr/bin/env tessh
# soundcard-on.tsh (ported from soundcard-on.sh)

#  Script author: Mkarcher
#  http://www.thinkwiki.org/wiki  ...
#  /Script_for_configuring_the_CS4239_sound_chip_in_PnP_mode
#  ABS Guide author made minor changes and added comments.
#  Couldn't contact script author to ask for permission to use, but ...
#+ the script was released under the FDL,
#+ so its use here should be both legal and ethical.

#  Sound-via-pnp-script for Thinkpad 600E
#+ and possibly other computers with onboard CS4239/CS4610
#+ that do not work with the PCI driver
#+ and are not recognized by the PnP code of snd-cs4236.
#  Also for some 770-series Thinkpads, such as the 770x.
#  Run as root user, of course.
#
#  These are old and very obsolete laptop computers,
#+ but this particular script is very instructive,
#+ as it shows how to set up and hack device files.

#  Search for sound card pnp device:
WSSDEV = None ; CTLDEV = None
for dev in /sys/bus/pnp/devices/*
  id = ($dev / id).read_text
  if "CSC0100" in $id
    WSSDEV = $dev
  if "CSC0110" in $id
    CTLDEV = $dev
# On 770x:
# WSSDEV = /sys/bus/pnp/devices/00:07
# CTLDEV = /sys/bus/pnp/devices/00:06
# These are symbolic links to /sys/devices/pnp0/ ...

#  Activate devices:
#  Thinkpad boots with devices disabled unless "fast boot" is turned off
#+ (in BIOS).
($WSSDEV / resources).write_text "activate\n"
($CTLDEV / resources).write_text "activate\n"

# Parse resource settings.
# A device's settings: the second word of each line after the first
# ("state = active"); the first words are labels: "io", "irq", "dma".
def settings dev = [($line.split)[1] for line in ((($dev / resources).read_text).splitlines)[1:]]

# The start of a port range: 0x530-0x537 -> 0x530
def start ports = ($ports.partition "-")[0]

[port1 port2 port3 irq dma1 dma2] = (settings $WSSDEV)[:6]
#  Hack: with PnPBIOS: ports are: port1: WSS, port2:
#+ OPL, port3: sb (unneeded)
#       with ACPI-PnP:ports are: port1: OPL, port2: sb, port3: WSS
#  (ACPI bios seems to be wrong here, the PnP-card-code in snd-cs4236.c
#+  uses the PnPBIOS port order)
#  Detect port order using the fixed OPL port as reference.
if (start $port2) == "0x388"       # (Quoted: 0x388 bare is a number.)
  # PnPBIOS: usual order
  port = start $port1
  oplport = start $port2
else
  # ACPI: mixed-up order
  port = start $port3
  oplport = start $port1
# To see what's going on here:
# ---------------------------
#   cat /sys/devices/pnp0/00:07/resources
#
#   state = active
#   io 0x530-0x537
#   io 0x388-0x38b
#   io 0x220-0x233
#   irq 5
#   dma 1
#   dma 0
#   ^^^   labels in first field (discarded).

cport = start (settings $CTLDEV)[0]
# Just want _start_ address of port.

# Load the module:
modprobe --ignore-install snd-cs4236 port=$port cport=$cport \
  fm_port=$oplport irq=$irq dma1=$dma1 dma2=$dma2 isapnp=0 index=0
# See the modprobe manpage.

exit $?
```

The parsing, on made-up `resources` files in both port orders:

```bash
# (Made-up resources files of the two PnP devices, as PnPBIOS and as
#+ ACPI list them; the modprobe command is shown, not run.)
mkdir -p pnpbios acpi ctl
printf 'state = active\nio 0x530-0x537\nio 0x388-0x38b\nio 0x220-0x233\nirq 5\ndma 1\ndma 0\n' >pnpbios/resources
printf 'state = active\nio 0x388-0x38b\nio 0x220-0x233\nio 0x530-0x537\nirq 5\ndma 1\ndma 0\n' >acpi/resources
printf 'state = active\nio 0x538-0x53f\n' >ctl/resources
CTLDEV=ctl

for WSSDEV in pnpbios acpi
do
{ read # Discard "state = active" (see below).
  read bla port1
  read bla port2
  read bla port3
  read bla irq
  read bla dma1
  read bla dma2
 #  Detect port order using the fixed OPL port as reference.
  if [ ${port2%%-*} = 0x388 ]
 then
   # PnPBIOS: usual order
   port=${port1%%-*}
   oplport=${port2%%-*}
 else
   # ACPI: mixed-up order
   port=${port3%%-*}
   oplport=${port1%%-*}
 fi
 } < $WSSDEV/resources

{ read # Discard first line, as above.
  read bla port1
  cport=${port1%%-*}
} < $CTLDEV/resources

echo modprobe --ignore-install snd-cs4236 port=$port cport=$cport \
fm_port=$oplport irq=$irq dma1=$dma1 dma2=$dma2 isapnp=0 index=0
done
```

```tessh
# (Made-up resources files of the two PnP devices, as PnPBIOS and as
# ACPI list them; the modprobe command is shown, not run.)
mkdir -p pnpbios acpi ctl
printf 'state = active\nio 0x530-0x537\nio 0x388-0x38b\nio 0x220-0x233\nirq 5\ndma 1\ndma 0\n' >pnpbios/resources
printf 'state = active\nio 0x388-0x38b\nio 0x220-0x233\nio 0x530-0x537\nirq 5\ndma 1\ndma 0\n' >acpi/resources
printf 'state = active\nio 0x538-0x53f\n' >ctl/resources
CTLDEV = Path ctl

# A device's settings: the second word of each line after the first
# ("state = active"); the first words are labels: "io", "irq", "dma".
def settings dev = [($line.split)[1] for line in ((($dev / resources).read_text).splitlines)[1:]]

# The start of a port range: 0x530-0x537 -> 0x530
def start ports = ($ports.partition "-")[0]

for WSSDEV in [(Path pnpbios) (Path acpi)]
  [port1 port2 port3 irq dma1 dma2] = settings $WSSDEV
  # Detect port order using the fixed OPL port as reference.
  if (start $port2) == "0x388"         # (Quoted: 0x388 bare is a number.)
    [port oplport] = [(start $port1) (start $port2)]   # PnPBIOS: usual order
  else
    [port oplport] = [(start $port3) (start $port1)]   # ACPI: mixed-up order
  cport = start (settings $CTLDEV)[0]

  echo modprobe --ignore-install snd-cs4236 port=$port cport=$cport \
    fm_port=$oplport irq=$irq dma1=$dma1 dma2=$dma2 isapnp=0 index=0
```

```output
modprobe --ignore-install snd-cs4236 port=0x530 cport=0x538 fm_port=0x388 irq=5 dma1=1 dma2=0 isapnp=0 index=0
modprobe --ignore-install snd-cs4236 port=0x530 cport=0x538 fm_port=0x388 irq=5 dma1=1 dma2=0 isapnp=0 index=0
```

`0x388` is quoted in the tessh, since a bare `0x388` is the number 904
and the port read from the file is text.

## Finding split paragraphs
<!-- help: abs-find-splitpara find-splitpara split-paragraphs text line-numbers -->

`find-splitpara.sh` finds the places in a text where a paragraph was
split by a stray blank line: a line that starts with a lower-case letter
right after a blank one. It prints them with their line numbers. (The
ABS Guide gives no author for it.)

### Example A-35. Locating split paragraphs in a text file (`find-splitpara.sh`)

```bash no-run
#!/bin/bash
# find-splitpara.sh
#  Finds split paragraphs in a text file,
#+ and tags the line numbers.


ARGCOUNT=1       # Expect one arg.
OFF=0            # Flag states.
ON=1
E_WRONGARGS=85

file="$1"        # Target filename.
lineno=1         # Line number. Start at 1.
Flag=$OFF        # Blank line flag.

if [ $# -ne "$ARGCOUNT" ]
then
  echo "Usage: `basename $0` FILENAME"
  exit $E_WRONGARGS
fi  

file_read ()     # Scan file for pattern, then print line.
{
while read line
do

  if [[ "$line" =~ ^[a-z] && $Flag -eq $ON ]]
     then  # Line begins with lowercase character, following blank line.
     echo -n "$lineno::   "
     echo "$line"
  fi


  if [[ "$line" =~ ^$ ]]
     then       #  If blank line,
     Flag=$ON   #+ set flag.
  else
     Flag=$OFF
  fi

  ((lineno++))

done
} < $file  # Redirect file into function's stdin.

file_read


exit $?


# ----------------------------------------------------------------
This is line one of an example paragraph, bla, bla, bla.
This is line two, and line three should follow on next line, but

there is a blank line separating the two parts of the paragraph.
# ----------------------------------------------------------------

Running this script on a file containing the above paragraph
yields:

4::   there is a blank line separating the two parts of the paragraph.


There will be additional output for all the other split paragraphs
in the target file.
```

In tessh the file is read with `for … in (enumerate $f 1)`, which numbers
the lines, and the flag is a boolean that says whether the line before
was blank. bash's `read` takes the spaces off the ends of each line, so
the tessh `.strip`s it. Here it runs on a made-up text, the script's
own example and more, as its sample argument.

```tessh
#!/usr/bin/env tessh
# find-splitpara.tsh
#  Finds split paragraphs in a text file,
#+ and tags the line numbers.

# (A made-up text to try it on: the script's own example, and more.)
cat >sample.txt <<'END'
This is line one of an example paragraph, bla, bla, bla.
This is line two, and line three should follow on next line, but

there is a blank line separating the two parts of the paragraph.

A paragraph of its own.

  indented, and split too.
Not split.

and this one is.
END

ARGCOUNT = 1     # Expect one arg.
E_WRONGARGS = 85

args = $argv[1:] or [sample.txt]    # (sample argument, when none is given)
if (len $args) != $ARGCOUNT
  print "Usage: $((Path $argv[0]).name) FILENAME"
  exit $E_WRONGARGS
file = $args[0]  # Target filename.

def file_read path    # Scan file for pattern, then print line.
  after_blank = False                 # Blank line flag.
  with (open $path) as f
    for [lineno line] in (enumerate $f 1)
      line = ($line.strip)            # (As read takes off the spaces.)
      if $after_blank and $line =~ re'^[a-z]'
        # Line begins with lowercase character, following blank line.
        print "$lineno::   $line"
      after_blank = $line == ""       # If blank line, set flag.

file_read $file
```

```output
4::   there is a blank line separating the two parts of the paragraph.
8::   indented, and split too.
11::   and this one is.
```

## Insertion sort
<!-- help: abs-insertion-sort insertion-sort sort sorting array-slicing -->

JuanJo Ciarlante's `insertion-sort.bash` sorts a line of numbers read
from its input (or random ones, with `-t`) by insertion: each element in
turn is moved left past the larger ones before it. It shows off bash's
array slicing: the move is one assignment that puts slices of the array
back together. With `DEBUG` set it shows each step, the pivot in `[ ]`
and where it went in `< >`.

### Example A-36. Insertion sort (`insertion-sort.bash`)

```bash no-run
#!/bin/bash
# insertion-sort.bash: Insertion sort implementation in Bash
#                      Heavy use of Bash array features:
#+                     (string) slicing, merging, etc
# URL: http://www.lugmen.org.ar/~jjo/jjotip/insertion-sort.bash.d
#+          /insertion-sort.bash.sh
#
# Author: JuanJo Ciarlante <jjo@irrigacion.gov.ar>
# Lightly reformatted by ABS Guide author.
# License: GPLv2
# Used in ABS Guide with author's permission (thanks!).
#
# Test with:   ./insertion-sort.bash -t
# Or:          bash insertion-sort.bash -t
# The following *doesn't* work:
#              sh insertion-sort.bash -t
#  Why not? Hint: which Bash-specific features are disabled
#+ when running a script by 'sh script.sh'?
#
: ${DEBUG:=0}  # Debug, override with:  DEBUG=1 ./scriptname . . .
# Parameter substitution -- set DEBUG to 0 if not previously set.

# Global array: "list"
typeset -a list
# Load whitespace-separated numbers from stdin.
if [ "$1" = "-t" ]; then
DEBUG=1
        read -a list < <( od -Ad -w24 -t u2 /dev/urandom ) # Random list.
#                    ^ ^  process substition
else
        read -a list
fi
numelem=${#list[*]}

#  Shows the list, marking the element whose index is $1
#+ by surrounding it with the two chars passed as $2.
#  Whole line prefixed with $3.
showlist()
  {
  echo "$3"${list[@]:0:$1} ${2:0:1}${list[$1]}${2:1:1} ${list[@]:$1+1};
  }

# Loop _pivot_ -- from second element to end of list.
for(( i=1; i<numelem; i++ )) do
        ((DEBUG))&&showlist i "[]" " "
        # From current _pivot_, back to first element.
        for(( j=i; j; j-- )) do
                # Search for the 1st elem. less than current "pivot" . . .
                [[ "${list[j-1]}" -le "${list[i]}" ]] && break
        done
	(( i==j )) && continue ## No insertion was needed for this element.
	# . . . Move list[i] (pivot) to the left of list[j]:
        list=(${list[@]:0:j} ${list[i]} ${list[j]}\
	#         {0,j-1}        {i}       {j}
              ${list[@]:j+1:i-(j+1)} ${list[@]:i+1})
	#         {j+1,i-1}              {i+1,last}
	((DEBUG))&&showlist j "<>" "*"
done


echo
echo  "------"
echo $'Result:\n'${list[@]}

exit $?
```

The tessh list slices are bash's array slices with Python's bounds
(`$list[$j:$i]` is the elements j to i-1, where bash's
`${list[@]:j:i-j}` counts them), and `+` joins them, with no word
splitting to worry about. The inner `for (( j=i; j; j-- ))` loop is a
`while`, since `j` is needed after it. The numbers come from a line in
the script rather than from the input, and `DEBUG` is on. The bash,
given the same line on its input with `DEBUG=1`, printed the same.

```tessh
#!/usr/bin/env tessh
import os random
# insertion-sort.tsh: Insertion sort implementation in tessh
#                     (ported from insertion-sort.bash: list slicing
#                     and joining instead of bash's array features)
#
# Author: JuanJo Ciarlante <jjo@irrigacion.gov.ar>
# Lightly reformatted by ABS Guide author.
# License: GPLv2
# Used in ABS Guide with author's permission (thanks!).
#
# Test with:   ./insertion-sort.tsh -t
#
DEBUG = int ($env.get DEBUG 1)   # Debug, override with: DEBUG=0 ./scriptname
                                 # (1 here, to see the steps; 0 in the bash.)

# Global list: "list"
if $argv[1:2] == ["-t"]
  DEBUG = 1
  list = [(random.randrange 65536) for _ in (range 12)]   # Random list.
else
  # Load whitespace-separated numbers from stdin.
  # (Here a line given in the script.)
  list = [(int $n) for n in ("73 31 52 9 31 88 4".split)]
numelem = len $list

# Shows the list, marking the element whose index is i
# by surrounding it with the two chars passed as marks.
# Whole line prefixed with prefix.
def showlist i marks prefix
  words = [(str $n) for n in $list]
  $words[$i] = $marks[0] + $words[$i] + $marks[1]
  if $i > 0
    $words[0] = $prefix + $words[0]
  else
    words = [$prefix] + $words
  print *$words

# Loop _pivot_ -- from second element to end of list.
for i in (range 1 $numelem)
  if $DEBUG
    showlist $i "[]" " "
  # From current _pivot_, back to first element.
  j = $i
  while $j
    # Search for the 1st elem. less than current "pivot" . . .
    if $list[$j - 1] <= $list[$i]
      break
    j -= 1
  if $i == $j
    continue           ## No insertion was needed for this element.
  # . . . Move list[i] (pivot) to the left of list[j]:
  list = $list[:$j] + [$list[$i]] + $list[$j:$i] + $list[$i + 1:]
  #        {0,j-1}       {i}          {j,i-1}       {i+1,last}
  if $DEBUG
    showlist $j "<>" "*"

print
print "------"
print "Result:"
print *$list
```

```output
 73 [31] 52 9 31 88 4
* <31> 73 52 9 31 88 4
 31 73 [52] 9 31 88 4
*31 <52> 73 9 31 88 4
 31 52 73 [9] 31 88 4
* <9> 31 52 73 31 88 4
 9 31 52 73 [31] 88 4
*9 31 <31> 52 73 88 4
 9 31 31 52 73 [88] 4
 9 31 31 52 73 88 [4]
* <4> 9 31 31 52 73 88

------
Result:
4 9 31 31 52 73 88
```

## Standard deviation
<!-- help: abs-stddev sd standard-deviation mean variance statistics bc decimal -->

`sd.sh` (the ABS Guide gives no author) works out the arithmetic mean and
the standard deviation of the numbers in a data file, one per line, with
`bc` doing each sum to nine decimal places. Its `arith_mean` "returns"
two values, the mean on its output and the count as its exit status,
which breaks past 255 points.

### Example A-37. Standard Deviation (`sd.sh`)

```bash no-run
#!/bin/bash
# sd.sh: Standard Deviation

#  The Standard Deviation indicates how consistent a set of data is.
#  It shows to what extent the individual data points deviate from the
#+ arithmetic mean, i.e., how much they "bounce around" (or cluster).
#  It is essentially the average deviation-distance of the
#+ data points from the mean.

# =========================================================== #
#    To calculate the Standard Deviation:
#
# 1  Find the arithmetic mean (average) of all the data points.
# 2  Subtract each data point from the arithmetic mean,
#    and square that difference.
# 3  Add all of the individual difference-squares in # 2.
# 4  Divide the sum in # 3 by the number of data points.
#    This is known as the "variance."
# 5  The square root of # 4 gives the Standard Deviation.
# =========================================================== #

count=0         # Number of data points; global.
SC=9            # Scale to be used by bc. Nine decimal places.
E_DATAFILE=90   # Data file error.

# ----------------- Set data file ---------------------
if [ ! -z "$1" ]  # Specify filename as cmd-line arg?
then
  datafile="$1" #  ASCII text file,
else            #+ one (numerical) data point per line!
  datafile=sample.dat
fi              #  See example data file, below.

if [ ! -e "$datafile" ]
then
  echo "\""$datafile"\" does not exist!"
  exit $E_DATAFILE
fi
# -----------------------------------------------------


arith_mean ()
{
  local rt=0         # Running total.
  local am=0         # Arithmetic mean.
  local ct=0         # Number of data points.

  while read value   # Read one data point at a time.
  do
    rt=$(echo "scale=$SC; $rt + $value" | bc)
    (( ct++ ))
  done

  am=$(echo "scale=$SC; $rt / $ct" | bc)

  echo $am; return $ct   # This function "returns" TWO values!
  #  Caution: This little trick will not work if $ct > 255!
  #  To handle a larger number of data points,
  #+ simply comment out the "return $ct" above.
} <"$datafile"   # Feed in data file.

sd ()
{
  mean1=$1  # Arithmetic mean (passed to function).
  n=$2      # How many data points.
  sum2=0    # Sum of squared differences ("variance").
  avg2=0    # Average of $sum2.
  sdev=0    # Standard Deviation.

  while read value   # Read one line at a time.
  do
    diff=$(echo "scale=$SC; $mean1 - $value" | bc)
    # Difference between arith. mean and data point.
    dif2=$(echo "scale=$SC; $diff * $diff" | bc) # Squared.
    sum2=$(echo "scale=$SC; $sum2 + $dif2" | bc) # Sum of squares.
  done

    avg2=$(echo "scale=$SC; $sum2 / $n" | bc)  # Avg. of sum of squares.
    sdev=$(echo "scale=$SC; sqrt($avg2)" | bc) # Square root =
    echo $sdev                                 # Standard Deviation.

} <"$datafile"   # Rewinds data file.


# ======================================================= #
mean=$(arith_mean); count=$?   # Two returns from function!
std_dev=$(sd $mean $count)

echo
echo "Number of data points in \""$datafile"\" = $count"
echo "Arithmetic mean (average) = $mean"
echo "Standard Deviation = $std_dev"
echo
# ======================================================= #

exit

#  This script could stand some drastic streamlining,
#+ but not at the cost of reduced legibility, please.


# ++++++++++++++++++++++++++++++++++++++++ #
# A sample data file (sample1.dat):

# 18.35
# 19.0
# 18.88
# 18.91
# 18.64


# $ sh sd.sh sample1.dat

# Number of data points in "sample1.dat" = 5
# Arithmetic mean (average) = 18.756000000
# Standard Deviation = .235338054
# ++++++++++++++++++++++++++++++++++++++++ #
```

tessh doesn't need `bc` for decimals: Python's `Decimal` is exact, and
`scale` cuts each result to nine places as `bc`'s `scale=9` does, so the
numbers are the same as the ones in the script's comment (`bc` writes
`.235338054`, Python `0.235338054`). The data file is read once, into a
list, and `arith_mean` gives back the mean and the count together. It
runs on the script's own sample data.

```tessh
#!/usr/bin/env tessh
# sd.tsh: Standard Deviation

#  The Standard Deviation indicates how consistent a set of data is.
#  It shows to what extent the individual data points deviate from the
#+ arithmetic mean, i.e., how much they "bounce around" (or cluster).
#  It is essentially the average deviation-distance of the
#+ data points from the mean.

# =========================================================== #
#    To calculate the Standard Deviation:
#
# 1  Find the arithmetic mean (average) of all the data points.
# 2  Subtract each data point from the arithmetic mean,
#    and square that difference.
# 3  Add all of the individual difference-squares in # 2.
# 4  Divide the sum in # 3 by the number of data points.
#    This is known as the "variance."
# 5  The square root of # 4 gives the Standard Deviation.
# =========================================================== #

from decimal import Decimal ROUND_DOWN

# (A made-up data file to try it on: the script's own sample.)
print 18.35 19.0 18.88 18.91 18.64 sep="\n" >sample.dat

SC = 9          # Scale, as bc's: nine decimal places.
E_DATAFILE = 90 # Data file error.

# ----------------- Set data file ---------------------
args = $argv[1:]
datafile = Path (if $args and $args[0] then $args[0] else sample.dat)
                # ASCII text file, one (numerical) data point per line!
if not ($datafile.exists)
  print "\"$datafile\" does not exist!"
  exit $E_DATAFILE
# -----------------------------------------------------

# A number cut to SC decimal places, as bc's scale cuts its results.
def scale x = $x.quantize (Decimal "1e-$SC") rounding=$ROUND_DOWN

# The data points, read once, as exact decimals.
values = [(Decimal $line) for line in (($datafile.read_text).split)]

def arith_mean values
  rt = Decimal 0                 # Running total.
  for value in $values           # One data point at a time.
    rt = scale ($rt + $value)
  am = scale ($rt / (len $values))
  [$am (len $values)]            # This function returns TWO values!
                                 # (No limit of 255, as a return status has.)

def sd mean1 values
  sum2 = Decimal 0               # Sum of squared differences ("variance").
  for value in $values
    diff = scale ($mean1 - $value)    # Difference between arith. mean and data point.
    dif2 = scale ($diff * $diff)      # Squared.
    sum2 = scale ($sum2 + $dif2)      # Sum of squares.
  avg2 = scale ($sum2 / (len $values))  # Avg. of sum of squares.
  scale ($avg2.sqrt)                    # Square root = Standard Deviation.

# ======================================================= #
[mean count] = arith_mean $values    # Two returns from function!
std_dev = sd $mean $values

print
print "Number of data points in \"$datafile\" = $count"
print "Arithmetic mean (average) = $mean"
print "Standard Deviation = $std_dev"
print
# ======================================================= #
```

```output

Number of data points in "sample.dat" = 5
Arithmetic mean (average) = 18.756000000
Standard Deviation = 0.235338054

```

For everyday use Python has it all in `statistics` (`pstdev` is the
population standard deviation, the one computed here), in floating point:

```tessh
import statistics
data = [18.35 19.0 18.88 18.91 18.64]
print (statistics.mean $data) (statistics.pstdev $data)
```

```output
18.756 0.2353380547212875
```
