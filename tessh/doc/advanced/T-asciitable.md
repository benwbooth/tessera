# Appendix T. ASCII Table
<!-- help: abs-asciitable abs-t ascii ascii-table chr ord character-codes -->

A book like this traditionally has an ASCII table. The ABS Guide gives
instead three scripts that print one, and so does this port. In tessh,
`ord` gives a character's code and `chr` the character for a code, and
`printf`-style formats are Python's `%` or f-string specs (`${n:03o}`).

### Example T-1. A script that generates an ASCII table (`ascii.sh`)

```bash
#!/bin/bash
# ascii.sh
# ver. 0.2, reldate 26 Aug 2008
# Patched by ABS Guide author.

# Original script by Sebastian Arming.
# Used with permission (thanks!).

exec >ASCII.txt         #  Save stdout to file,
                        #+ as in the example scripts
                        #+ reassign-stdout.sh and upperconv.sh.

MAXNUM=256
COLUMNS=5
OCT=8
OCTSQU=64
LITTLESPACE=-3
BIGSPACE=-5

i=1 # Decimal counter
o=1 # Octal counter

while [ "$i" -lt "$MAXNUM" ]; do  # We don't have to count past 400 octal.
        paddi="    $i"
        echo -n "${paddi: $BIGSPACE}  "       # Column spacing.
        paddo="00$o"
#       echo -ne "\\${paddo: $LITTLESPACE}"   # Original.
        echo -ne "\\0${paddo: $LITTLESPACE}"  # Fixup.
#                   ^
        echo -n "     "
        if (( i % $COLUMNS == 0)); then       # New line.
           echo
        fi
        ((i++, o++))
        # The octal notation for 8 is 10, and 64 decimal is 100 octal.
        (( i % $OCT == 0))    && ((o+=2))
        (( i % $OCTSQU == 0)) && ((o+=20))
done

exit $?

# Compare this script with the "pr-asc.sh" example.
# This one handles "unprintable" characters.

# Exercise:
# Rewrite this script to use decimal numbers, rather than octal.
```

```output bash
```

The original sends the rest of its output to `ASCII.txt` with `exec
>ASCII.txt`, which tessh doesn't have; it writes to the file it opens.
The characters above 127 are single bytes, as bash's `\0377` escapes give,
so the file is opened in binary and each character written as `bytes
[$i]`. Counting in decimal (the original's closing exercise) makes the
octal counter and its corrections unnecessary. The table is mostly
unprintable characters, so `cat -v` shows the start of it. (The file is
byte for byte the one the bash script writes.)

```tessh
MAXNUM = 256
COLUMNS = 5

with (open ASCII.txt wb) as out   # Save the table to a file.
  for i in (range 1 $MAXNUM)
    $out.write ((.encode "${i:>5}  ") + (bytes [$i]) + b"     ")
    if $i % $COLUMNS == 0         # New line.
      $out.write b"\n"

head -c 290 ASCII.txt | cat -v
echo
wc -c ASCII.txt
```

```output
    1  ^A         2  ^B         3  ^C         4  ^D         5  ^E     
    6  ^F         7  ^G         8  ^H         9  	        10  
     
   11  ^K        12  ^L        13  ^M        14  ^N        15  ^O     
   16  ^P        17  ^Q        18  ^R        19  ^S        20  ^T     
   21  ^U        22  ^V     
3366 ASCII.txt
```

### Example T-2. Another ASCII table script (`ascii2.sh`)

```bash
#!/bin/bash
# Script author: Joseph Steinhauser
# Lightly edited by ABS Guide author, but not commented.
# Used in ABS Guide with permission.

#-------------------------------------------------------------------------
#-- File:  ascii.sh    Print ASCII chart, base 10/8/16         (JETS-2012)
#-------------------------------------------------------------------------
#-- Usage: ascii [oct|dec|hex|help|8|10|16]
#--
#-- This script prints out a summary of ASCII char codes from Zero to 127.
#-- Numeric values may be printed in Base10, Octal, or Hex.
#--
#-- Format Based on: /usr/share/lib/pub/ascii with base-10 as default.
#-- For more detail, man ascii . . .
#-------------------------------------------------------------------------

[ -n "$BASH_VERSION" ] && shopt -s extglob

case "$1" in
   oct|[Oo]?([Cc][Tt])|8)       Obase=Octal;  Numy=3o;;
   hex|[Hh]?([Ee][Xx])|16|[Xx]) Obase=Hex;    Numy=2X;;
   help|?(-)[h?])        sed -n '2,/^[ ]*$/p' $0;exit;;
   code|[Cc][Oo][Dd][Ee])sed -n '/case/,$p'   $0;exit;;
   *) Obase=Decimal
esac # CODE is actually shorter than the chart!

printf "\t\t## $Obase ASCII Chart ##\n\n"; FM1="|%0${Numy:-3d}"; LD=-1

AB="nul soh stx etx eot enq ack bel bs tab nl vt np cr so si dle"
AD="dc1 dc2 dc3 dc4 nak syn etb can em sub esc fs gs rs us sp"

for TOK in $AB $AD; do ABR[$((LD+=1))]=$TOK; done;
ABR[127]=del

IDX=0
while [ $IDX -le 127 ] && CHR="${ABR[$IDX]}"
   do ((${#CHR}))&& FM2='%-3s'|| FM2=`printf '\\\\%o  ' $IDX`
      printf "$FM1 $FM2" "$IDX" $CHR; (( (IDX+=1)%8))||echo '|'
   done

exit $?
```

```output bash
		## Decimal ASCII Chart ##

|000 nul|001 soh|002 stx|003 etx|004 eot|005 enq|006 ack|007 bel|
|008 bs |009 tab|010 nl |011 vt |012 np |013 cr |014 so |015 si |
|016 dle|017 dc1|018 dc2|019 dc3|020 dc4|021 nak|022 syn|023 etb|
|024 can|025 em |026 sub|027 esc|028 fs |029 gs |030 rs |031 us |
|032 sp |033 !  |034 "  |035 #  |036 $  |037 %  |038 &  |039 '  |
|040 (  |041 )  |042 *  |043 +  |044 ,  |045 -  |046 .  |047 /  |
|048 0  |049 1  |050 2  |051 3  |052 4  |053 5  |054 6  |055 7  |
|056 8  |057 9  |058 :  |059 ;  |060 <  |061 =  |062 >  |063 ?  |
|064 @  |065 A  |066 B  |067 C  |068 D  |069 E  |070 F  |071 G  |
|072 H  |073 I  |074 J  |075 K  |076 L  |077 M  |078 N  |079 O  |
|080 P  |081 Q  |082 R  |083 S  |084 T  |085 U  |086 V  |087 W  |
|088 X  |089 Y  |090 Z  |091 [  |092 \  |093 ]  |094 ^  |095 _  |
|096 `  |097 a  |098 b  |099 c  |100 d  |101 e  |102 f  |103 g  |
|104 h  |105 i  |106 j  |107 k  |108 l  |109 m  |110 n  |111 o  |
|112 p  |113 q  |114 r  |115 s  |116 t  |117 u  |118 v  |119 w  |
|120 x  |121 y  |122 z  |123 {  |124 |  |125 }  |126 ~  |127 del|
```

The argument picks the base with `match`; bash's `extglob` patterns
(`[Oo]?([Cc][Tt])`) become a regular expression on the lower-cased word.
The names of the control characters are a list, so a code's name is
`$ABR[$idx]` when there is one, and its character `chr $idx` otherwise.
`help` and `code` print parts of the script itself, as the original does.

```tessh
import re
arg = (.lower ($argv[1:2] or [""])[0])

[Obase Numy] = match $arg
  case _ if $arg =~ re'^(o(ct)?|8)$'
    [Octal "03o"]
  case _ if $arg =~ re'^(h(ex)?|16|x)$'
    [Hex "02X"]
  case _ if $arg =~ re'^(help|-?[h?])$'
    lines = .splitlines (.read_text (Path $argv[0]))
    print ("\n".join $lines[1:($lines.index "")])
    exit
  case code
    print (cat $argv[0] | sed -n '/match/,$p')
    exit
  case _
    [Decimal "03d"]

print "\t\t## $Obase ASCII Chart ##\n"

ABR = .split ("nul soh stx etx eot enq ack bel bs tab nl vt np cr so si dle " +
              "dc1 dc2 dc3 dc4 nak syn etb can em sub esc fs gs rs us sp")
ABR += [""] * (127 - (len $ABR)) + ["del"]

for idx in (range 128)
  name = if $ABR[$idx] then "%-3s" % $ABR[$idx] else (chr $idx) + "  "
  print "|${format $idx $Numy} $name" end=""
  if ($idx + 1) % 8 == 0
    print "|"
```

```output
		## Decimal ASCII Chart ##

|000 nul|001 soh|002 stx|003 etx|004 eot|005 enq|006 ack|007 bel|
|008 bs |009 tab|010 nl |011 vt |012 np |013 cr |014 so |015 si |
|016 dle|017 dc1|018 dc2|019 dc3|020 dc4|021 nak|022 syn|023 etb|
|024 can|025 em |026 sub|027 esc|028 fs |029 gs |030 rs |031 us |
|032 sp |033 !  |034 "  |035 #  |036 $  |037 %  |038 &  |039 '  |
|040 (  |041 )  |042 *  |043 +  |044 ,  |045 -  |046 .  |047 /  |
|048 0  |049 1  |050 2  |051 3  |052 4  |053 5  |054 6  |055 7  |
|056 8  |057 9  |058 :  |059 ;  |060 <  |061 =  |062 >  |063 ?  |
|064 @  |065 A  |066 B  |067 C  |068 D  |069 E  |070 F  |071 G  |
|072 H  |073 I  |074 J  |075 K  |076 L  |077 M  |078 N  |079 O  |
|080 P  |081 Q  |082 R  |083 S  |084 T  |085 U  |086 V  |087 W  |
|088 X  |089 Y  |090 Z  |091 [  |092 \  |093 ]  |094 ^  |095 _  |
|096 `  |097 a  |098 b  |099 c  |100 d  |101 e  |102 f  |103 g  |
|104 h  |105 i  |106 j  |107 k  |108 l  |109 m  |110 n  |111 o  |
|112 p  |113 q  |114 r  |115 s  |116 t  |117 u  |118 v  |119 w  |
|120 x  |121 y  |122 z  |123 {  |124 |  |125 }  |126 ~  |127 del|
```

### Example T-3. A third ASCII table script, using awk (`ascii3.sh`)

```bash
#!/bin/bash
# ASCII table script, using awk.
# Author: Joseph Steinhauser
# Used in ABS Guide with permission.


#-------------------------------------------------------------------------
#-- File:  ascii     Print ASCII chart, base 10/8/16         (JETS-2010)
#-------------------------------------------------------------------------
#-- Usage: ascii [oct|dec|hex|help|8|10|16]
#--
#-- This script prints a summary of ASCII char codes from Zero to 127.
#-- Numeric values may be printed in Base10, Octal, or Hex (Base16).
#--
#-- Format Based on: /usr/share/lib/pub/ascii with base-10 as default.
#-- For more detail, man ascii
#-------------------------------------------------------------------------

[ -n "$BASH_VERSION" ] && shopt -s extglob

case "$1" in
   oct|[Oo]?([Cc][Tt])|8)       Obase=Octal;  Numy=3o;;
   hex|[Hh]?([Ee][Xx])|16|[Xx]) Obase=Hex;    Numy=2X;;
   help|?(-)[h?])        sed -n '2,/^[ ]*$/p' $0;exit;;
   code|[Cc][Oo][Dd][Ee])sed -n '/case/,$p'   $0;exit;;
   *) Obase=Decimal
esac
export Obase   # CODE is actually shorter than the chart!

awk 'BEGIN{print "\n\t\t## "ENVIRON["Obase"]" ASCII Chart ##\n"
           ab="soh,stx,etx,eot,enq,ack,bel,bs,tab,nl,vt,np,cr,so,si,dle,"
           ad="dc1,dc2,dc3,dc4,nak,syn,etb,can,em,sub,esc,fs,gs,rs,us,sp"
           split(ab ad,abr,",");abr[0]="nul";abr[127]="del";
           fm1="|%0'"${Numy:- 4d}"' %-3s"
           for(idx=0;idx<128;idx++){fmt=fm1 (++colz%8?"":"|\n")
           printf(fmt,idx,(idx in abr)?abr[idx]:sprintf("%c",idx))} }'

exit $?
```

```output bash

		## Decimal ASCII Chart ##

| 000 nul| 001 soh| 002 stx| 003 etx| 004 eot| 005 enq| 006 ack| 007 bel|
| 008 bs | 009 tab| 010 nl | 011 vt | 012 np | 013 cr | 014 so | 015 si |
| 016 dle| 017 dc1| 018 dc2| 019 dc3| 020 dc4| 021 nak| 022 syn| 023 etb|
| 024 can| 025 em | 026 sub| 027 esc| 028 fs | 029 gs | 030 rs | 031 us |
| 032 sp | 033 !  | 034 "  | 035 #  | 036 $  | 037 %  | 038 &  | 039 '  |
| 040 (  | 041 )  | 042 *  | 043 +  | 044 ,  | 045 -  | 046 .  | 047 /  |
| 048 0  | 049 1  | 050 2  | 051 3  | 052 4  | 053 5  | 054 6  | 055 7  |
| 056 8  | 057 9  | 058 :  | 059 ;  | 060 <  | 061 =  | 062 >  | 063 ?  |
| 064 @  | 065 A  | 066 B  | 067 C  | 068 D  | 069 E  | 070 F  | 071 G  |
| 072 H  | 073 I  | 074 J  | 075 K  | 076 L  | 077 M  | 078 N  | 079 O  |
| 080 P  | 081 Q  | 082 R  | 083 S  | 084 T  | 085 U  | 086 V  | 087 W  |
| 088 X  | 089 Y  | 090 Z  | 091 [  | 092 \  | 093 ]  | 094 ^  | 095 _  |
| 096 `  | 097 a  | 098 b  | 099 c  | 100 d  | 101 e  | 102 f  | 103 g  |
| 104 h  | 105 i  | 106 j  | 107 k  | 108 l  | 109 m  | 110 n  | 111 o  |
| 112 p  | 113 q  | 114 r  | 115 s  | 116 t  | 117 u  | 118 v  | 119 w  |
| 120 x  | 121 y  | 122 z  | 123 {  | 124 |  | 125 }  | 126 ~  | 127 del|
```

The awk program stays as it is, in single quotes; the base's format goes
to it with `-v` instead of being spliced into its text, and `$Obase` by
the environment, as in the original.

```tessh
import re
arg = (.lower ($argv[1:2] or [""])[0])

[Obase Numy] = match $arg
  case _ if $arg =~ re'^(o(ct)?|8)$'
    [Octal "3o"]
  case _ if $arg =~ re'^(h(ex)?|16|x)$'
    [Hex "2X"]
  case _
    [Decimal " 4d"]
export Obase   # CODE is actually shorter than the chart!

awk -v "numy=$Numy" 'BEGIN{print "\n\t\t## "ENVIRON["Obase"]" ASCII Chart ##\n"
           ab="soh,stx,etx,eot,enq,ack,bel,bs,tab,nl,vt,np,cr,so,si,dle,"
           ad="dc1,dc2,dc3,dc4,nak,syn,etb,can,em,sub,esc,fs,gs,rs,us,sp"
           split(ab ad,abr,",");abr[0]="nul";abr[127]="del";
           fm1="|%0" numy " %-3s"
           for(idx=0;idx<128;idx++){fmt=fm1 (++colz%8?"":"|\n")
           printf(fmt,idx,(idx in abr)?abr[idx]:sprintf("%c",idx))} }'
```

```output

		## Decimal ASCII Chart ##

| 000 nul| 001 soh| 002 stx| 003 etx| 004 eot| 005 enq| 006 ack| 007 bel|
| 008 bs | 009 tab| 010 nl | 011 vt | 012 np | 013 cr | 014 so | 015 si |
| 016 dle| 017 dc1| 018 dc2| 019 dc3| 020 dc4| 021 nak| 022 syn| 023 etb|
| 024 can| 025 em | 026 sub| 027 esc| 028 fs | 029 gs | 030 rs | 031 us |
| 032 sp | 033 !  | 034 "  | 035 #  | 036 $  | 037 %  | 038 &  | 039 '  |
| 040 (  | 041 )  | 042 *  | 043 +  | 044 ,  | 045 -  | 046 .  | 047 /  |
| 048 0  | 049 1  | 050 2  | 051 3  | 052 4  | 053 5  | 054 6  | 055 7  |
| 056 8  | 057 9  | 058 :  | 059 ;  | 060 <  | 061 =  | 062 >  | 063 ?  |
| 064 @  | 065 A  | 066 B  | 067 C  | 068 D  | 069 E  | 070 F  | 071 G  |
| 072 H  | 073 I  | 074 J  | 075 K  | 076 L  | 077 M  | 078 N  | 079 O  |
| 080 P  | 081 Q  | 082 R  | 083 S  | 084 T  | 085 U  | 086 V  | 087 W  |
| 088 X  | 089 Y  | 090 Z  | 091 [  | 092 \  | 093 ]  | 094 ^  | 095 _  |
| 096 `  | 097 a  | 098 b  | 099 c  | 100 d  | 101 e  | 102 f  | 103 g  |
| 104 h  | 105 i  | 106 j  | 107 k  | 108 l  | 109 m  | 110 n  | 111 o  |
| 112 p  | 113 q  | 114 r  | 115 s  | 116 t  | 117 u  | 118 v  | 119 w  |
| 120 x  | 121 y  | 122 z  | 123 {  | 124 |  | 125 }  | 126 ~  | 127 del|
```
