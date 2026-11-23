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
  dig_reply (dig +short $name -c in -t a 2>/dev/null) '## Lookup error {} on {} ##' $name

# Reverse lookup :: Address -> Name
def short_rev ip
  print "." end="" flush=True
  dig_reply (dig +short -x $ip 2>/dev/null) '## Lookup error {} on {} ##' $ip

# Special format lookup used to query blacklist servers.
# (The bash version's warning filter counts to ${#_st_cnt}, the length of
# the count, so it looks at the first line or so only.)
def short_text name
  dig_reply (dig +short $name -c in -t txt 2>/dev/null) '##Text lookup error {} on {}##' $name

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
