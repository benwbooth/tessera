## A PAD file generator
<!-- help: abs-pad pad.sh pad-file xml shareware form fill-in read -->

Mendel Cooper's `pad.sh` asks for each field of a PAD file (the XML
"Portable Application Description" shareware authors once sent to download
sites) and writes the file as it goes. Every answer is one `read`.

### Example A-38. A "pad" file generator for shareware authors (`pad.sh`)

```bash no-run
#!/bin/bash
# pad.sh

#######################################################
#               PAD (xml) file creator
#+ Written by Mendel Cooper <thegrendel.abs@gmail.com>.
#+ Released to the Public Domain.
#
#  Generates a "PAD" descriptor file for shareware
#+ packages, according to the specifications
#+ of the ASP.
#  http://www.asp-shareware.org/pad
#######################################################


# Accepts (optional) save filename as a command-line argument.
if [ -n "$1" ]
then
  savefile=$1
else
  savefile=save_file.xml               # Default save_file name.
fi  


# ===== PAD file headers =====
HDR1="<?xml version=\"1.0\" encoding=\"Windows-1252\" ?>"
HDR2="<XML_DIZ_INFO>"
HDR3="<MASTER_PAD_VERSION_INFO>"
HDR4="\t<MASTER_PAD_VERSION>1.15</MASTER_PAD_VERSION>"
HDR5="\t<MASTER_PAD_INFO>Portable Application Description, or PAD
for short, is a data set that is used by shareware authors to
disseminate information to anyone interested in their software products.
To find out more go to http://www.asp-shareware.org/pad</MASTER_PAD_INFO>"
HDR6="</MASTER_PAD_VERSION_INFO>"
# ============================


fill_in ()
{
  if [ -z "$2" ]
  then
    echo -n "$1? "     # Get user input.
  else
    echo -n "$1 $2? "  # Additional query?
  fi  

  read var             # May paste to fill in field.
                       # This shows how flexible "read" can be.

  if [ -z "$var" ]
  then
    echo -e "\t\t<$1 />" >>$savefile    # Indent with 2 tabs.
    return
  else
    echo -e "\t\t<$1>$var</$1>" >>$savefile
    return ${#var}     # Return length of input string.
  fi
}    

check_field_length ()  # Check length of program description fields.
{
  # $1 = maximum field length
  # $2 = actual field length
  if [ "$2" -gt "$1" ]
  then
    echo "Warning: Maximum field length of $1 characters exceeded!"
  fi
}  

clear                  # Clear screen.
echo "PAD File Creator"
echo "--- ---- -------"
echo

# Write File Headers to file.
echo $HDR1 >$savefile
echo $HDR2 >>$savefile
echo $HDR3 >>$savefile
echo -e $HDR4 >>$savefile
echo -e $HDR5 >>$savefile
echo $HDR6 >>$savefile


# Company_Info
echo "COMPANY INFO"
CO_HDR="Company_Info"
echo "<$CO_HDR>" >>$savefile

fill_in Company_Name
fill_in Address_1
fill_in Address_2
fill_in City_Town 
fill_in State_Province
fill_in Zip_Postal_Code
fill_in Country

# If applicable:
# fill_in ASP_Member "[Y/N]"
# fill_in ASP_Member_Number
# fill_in ESC_Member "[Y/N]"

fill_in Company_WebSite_URL

clear   # Clear screen between sections.

   # Contact_Info
echo "CONTACT INFO"
CONTACT_HDR="Contact_Info"
echo "<$CONTACT_HDR>" >>$savefile
fill_in Author_First_Name
fill_in Author_Last_Name
fill_in Author_Email
fill_in Contact_First_Name
fill_in Contact_Last_Name
fill_in Contact_Email
echo -e "\t</$CONTACT_HDR>" >>$savefile
   # END Contact_Info

clear

   # Support_Info
echo "SUPPORT INFO"
SUPPORT_HDR="Support_Info"
echo "<$SUPPORT_HDR>" >>$savefile
fill_in Sales_Email
fill_in Support_Email
fill_in General_Email
fill_in Sales_Phone
fill_in Support_Phone
fill_in General_Phone
fill_in Fax_Phone
echo -e "\t</$SUPPORT_HDR>" >>$savefile
   # END Support_Info

echo "</$CO_HDR>" >>$savefile
# END Company_Info

clear

# Program_Info 
echo "PROGRAM INFO"
PROGRAM_HDR="Program_Info"
echo "<$PROGRAM_HDR>" >>$savefile
fill_in Program_Name
fill_in Program_Version
fill_in Program_Release_Month
fill_in Program_Release_Day
fill_in Program_Release_Year
fill_in Program_Cost_Dollars
fill_in Program_Cost_Other
fill_in Program_Type "[Shareware/Freeware/GPL]"
fill_in Program_Release_Status "[Beta, Major Upgrade, etc.]"
fill_in Program_Install_Support
fill_in Program_OS_Support "[Win9x/Win2k/Linux/etc.]"
fill_in Program_Language "[English/Spanish/etc.]"

echo; echo

  # File_Info 
echo "FILE INFO"
FILEINFO_HDR="File_Info"
echo "<$FILEINFO_HDR>" >>$savefile
fill_in Filename_Versioned
fill_in Filename_Previous
fill_in Filename_Generic
fill_in Filename_Long
fill_in File_Size_Bytes
fill_in File_Size_K
fill_in File_Size_MB
echo -e "\t</$FILEINFO_HDR>" >>$savefile
  # END File_Info 

clear

  # Expire_Info 
echo "EXPIRE INFO"
EXPIRE_HDR="Expire_Info"
echo "<$EXPIRE_HDR>" >>$savefile
fill_in Has_Expire_Info "Y/N"
fill_in Expire_Count
fill_in Expire_Based_On
fill_in Expire_Other_Info
fill_in Expire_Month
fill_in Expire_Day
fill_in Expire_Year
echo -e "\t</$EXPIRE_HDR>" >>$savefile
  # END Expire_Info 

clear

  # More Program_Info
echo "ADDITIONAL PROGRAM INFO"
fill_in Program_Change_Info
fill_in Program_Specific_Category
fill_in Program_Categories
fill_in Includes_JAVA_VM "[Y/N]"
fill_in Includes_VB_Runtime "[Y/N]"
fill_in Includes_DirectX "[Y/N]"
  # END More Program_Info

echo "</$PROGRAM_HDR>" >>$savefile
# END Program_Info 

clear

# Program Description
echo "PROGRAM DESCRIPTIONS"
PROGDESC_HDR="Program_Descriptions"
echo "<$PROGDESC_HDR>" >>$savefile

LANG="English"
echo "<$LANG>" >>$savefile

fill_in Keywords "[comma + space separated]"
echo
echo "45, 80, 250, 450, 2000 word program descriptions"
echo "(may cut and paste into field)"
#  It would be highly appropriate to compose the following
#+ "Char_Desc" fields with a text editor,
#+ then cut-and-paste the text into the answer fields.
echo
echo "              |---------------45 characters---------------|"
fill_in Char_Desc_45
check_field_length 45 "$?"
echo
fill_in Char_Desc_80
check_field_length 80 "$?"

fill_in Char_Desc_250
check_field_length 250 "$?"

fill_in Char_Desc_450
fill_in Char_Desc_2000

echo "</$LANG>" >>$savefile
echo "</$PROGDESC_HDR>" >>$savefile
# END Program Description

clear
echo "Done."; echo; echo
echo "Save file is:  \""$savefile"\""

exit 0
```

Here a dict of canned answers stands in for the keyboard; each prompt is
printed with its answer after it. Interactively, `var = input "$prompt"`
is bash's `echo -n "$prompt"; read var` in one: Python's `input` shows the
prompt and reads a line (a pasted line included). The sections become a
small `section` function and the long runs of `fill_in` loops over lists.
bash's `fill_in` hands the length of the answer back as its exit status,
which wraps at 256, so a 300-character description passes the 250 check;
a tessh function gives back the number itself.

```tessh
import sys
# pad.sh: PAD (xml) file creator, by Mendel Cooper (public domain).
# (Canned answers stand in for the keyboard: a field not in here is left empty.)
answers = {
  Company_Name: "Grendel Software"
  City_Town: "Baltimore"
  Country: "USA"
  Author_First_Name: "Mendel"
  Author_Last_Name: "Cooper"
  Program_Name: "pad.sh"
  Program_Version: "1.0"
  Program_Type: "Freeware"
  Keywords: "pad, xml, shareware"
  Char_Desc_45: "Writes a PAD file for shareware packages."
  Char_Desc_80: "Asks for each field of a PAD descriptor and writes the XML file for you, in order."
}

# Accepts (optional) save filename as a command-line argument.
savefile = if (len $argv) > 1 and $argv[1] then $argv[1] else "save_file.xml"

# ===== PAD file headers =====
HDR = [
  '<?xml version="1.0" encoding="Windows-1252" ?>'
  "<XML_DIZ_INFO>"
  "<MASTER_PAD_VERSION_INFO>"
  "\t<MASTER_PAD_VERSION>1.15</MASTER_PAD_VERSION>"
  "\t<MASTER_PAD_INFO>Portable Application Description, or PAD
for short, is a data set that is used by shareware authors to
disseminate information to anyone interested in their software products.
To find out more go to http://www.asp-shareware.org/pad</MASTER_PAD_INFO>"
  "</MASTER_PAD_VERSION_INFO>"
]
# ============================

def clear                       # Clear screen (only when it is one).
  if (sys.stdout.isatty)
    \clear

def fill_in field extra=""
  prompt = if $extra then "$field $extra? " else "$field? "
  var = $answers.get $field ""  # (read var, in the original)
  print "$prompt$var"
  if not $var
    print "\t\t<$field />" >>$savefile    # Indent with 2 tabs.
  else
    print "\t\t<$field>$var</$field>" >>$savefile
  len $var                      # The length of the answer.

def check_field_length max actual
  if $actual > $max
    print "Warning: Maximum field length of $max characters exceeded!"

clear
print "PAD File Creator"
print "--- ---- -------"
print

# Write File Headers to file.
# (bash's unquoted `echo $HDR5` joins its lines with spaces.)
with (open $savefile w) as f
  for h in $HDR
    print (" ".join ($h.split "\n")) file=$f

def section title tag
  clear
  print $title
  print "<$tag>" >>$savefile

def end_section tag indent="\t"
  print "$indent</$tag>" >>$savefile

# Company_Info
section "COMPANY INFO" Company_Info
for f in [Company_Name Address_1 Address_2 City_Town State_Province Zip_Postal_Code Country]
  fill_in $f
# If applicable:
# fill_in ASP_Member "[Y/N]"
fill_in Company_WebSite_URL

section "CONTACT INFO" Contact_Info
for f in [Author_First_Name Author_Last_Name Author_Email Contact_First_Name Contact_Last_Name Contact_Email]
  fill_in $f
end_section Contact_Info

section "SUPPORT INFO" Support_Info
for f in [Sales_Email Support_Email General_Email Sales_Phone Support_Phone General_Phone Fax_Phone]
  fill_in $f
end_section Support_Info
end_section Company_Info indent=""

section "PROGRAM INFO" Program_Info
for f in [Program_Name Program_Version Program_Release_Month Program_Release_Day Program_Release_Year Program_Cost_Dollars Program_Cost_Other]
  fill_in $f
fill_in Program_Type "[Shareware/Freeware/GPL]"
fill_in Program_Release_Status "[Beta, Major Upgrade, etc.]"
fill_in Program_Install_Support
fill_in Program_OS_Support "[Win9x/Win2k/Linux/etc.]"
fill_in Program_Language "[English/Spanish/etc.]"
print ; print

print "FILE INFO"
print "<File_Info>" >>$savefile
for f in [Filename_Versioned Filename_Previous Filename_Generic Filename_Long File_Size_Bytes File_Size_K File_Size_MB]
  fill_in $f
end_section File_Info

section "EXPIRE INFO" Expire_Info
fill_in Has_Expire_Info "Y/N"
for f in [Expire_Count Expire_Based_On Expire_Other_Info Expire_Month Expire_Day Expire_Year]
  fill_in $f
end_section Expire_Info

clear
print "ADDITIONAL PROGRAM INFO"
for f in [Program_Change_Info Program_Specific_Category Program_Categories]
  fill_in $f
for f in [Includes_JAVA_VM Includes_VB_Runtime Includes_DirectX]
  fill_in $f "[Y/N]"
end_section Program_Info indent=""

section "PROGRAM DESCRIPTIONS" Program_Descriptions
LANG = English
print "<$LANG>" >>$savefile
fill_in Keywords "[comma + space separated]"
print
print "45, 80, 250, 450, 2000 word program descriptions"
print "(may cut and paste into field)"
print
print "              |---------------45 characters---------------|"
check_field_length 45 (fill_in Char_Desc_45)
print
check_field_length 80 (fill_in Char_Desc_80)
check_field_length 250 (fill_in Char_Desc_250)
fill_in Char_Desc_450
fill_in Char_Desc_2000
end_section $LANG indent=""
end_section Program_Descriptions indent=""

clear
print "Done." ; print ; print
print "Save file is:  \"$savefile\""

# (What it wrote, the start and the end.)
head -12 $savefile
print "..."
tail -4 $savefile
exit 0
```

```output
PAD File Creator
--- ---- -------

COMPANY INFO
Company_Name? Grendel Software
Address_1? 
Address_2? 
City_Town? Baltimore
State_Province? 
Zip_Postal_Code? 
Country? USA
Company_WebSite_URL? 
CONTACT INFO
Author_First_Name? Mendel
Author_Last_Name? Cooper
Author_Email? 
Contact_First_Name? 
Contact_Last_Name? 
Contact_Email? 
SUPPORT INFO
Sales_Email? 
Support_Email? 
General_Email? 
Sales_Phone? 
Support_Phone? 
General_Phone? 
Fax_Phone? 
PROGRAM INFO
Program_Name? pad.sh
Program_Version? 1.0
Program_Release_Month? 
Program_Release_Day? 
Program_Release_Year? 
Program_Cost_Dollars? 
Program_Cost_Other? 
Program_Type [Shareware/Freeware/GPL]? Freeware
Program_Release_Status [Beta, Major Upgrade, etc.]? 
Program_Install_Support? 
Program_OS_Support [Win9x/Win2k/Linux/etc.]? 
Program_Language [English/Spanish/etc.]? 


FILE INFO
Filename_Versioned? 
Filename_Previous? 
Filename_Generic? 
Filename_Long? 
File_Size_Bytes? 
File_Size_K? 
File_Size_MB? 
EXPIRE INFO
Has_Expire_Info Y/N? 
Expire_Count? 
Expire_Based_On? 
Expire_Other_Info? 
Expire_Month? 
Expire_Day? 
Expire_Year? 
ADDITIONAL PROGRAM INFO
Program_Change_Info? 
Program_Specific_Category? 
Program_Categories? 
Includes_JAVA_VM [Y/N]? 
Includes_VB_Runtime [Y/N]? 
Includes_DirectX [Y/N]? 
PROGRAM DESCRIPTIONS
Keywords [comma + space separated]? pad, xml, shareware

45, 80, 250, 450, 2000 word program descriptions
(may cut and paste into field)

              |---------------45 characters---------------|
Char_Desc_45? Writes a PAD file for shareware packages.

Char_Desc_80? Asks for each field of a PAD descriptor and writes the XML file for you, in order.
Warning: Maximum field length of 80 characters exceeded!
Char_Desc_250? 
Char_Desc_450? 
Char_Desc_2000? 
Done.


Save file is:  "save_file.xml"
<?xml version="1.0" encoding="Windows-1252" ?>
<XML_DIZ_INFO>
<MASTER_PAD_VERSION_INFO>
	<MASTER_PAD_VERSION>1.15</MASTER_PAD_VERSION>
	<MASTER_PAD_INFO>Portable Application Description, or PAD for short, is a data set that is used by shareware authors to disseminate information to anyone interested in their software products. To find out more go to http://www.asp-shareware.org/pad</MASTER_PAD_INFO>
</MASTER_PAD_VERSION_INFO>
<Company_Info>
		<Company_Name>Grendel Software</Company_Name>
		<Address_1 />
		<Address_2 />
		<City_Town>Baltimore</City_Town>
		<State_Province />
...
		<Char_Desc_450 />
		<Char_Desc_2000 />
</English>
</Program_Descriptions>
```

## A man page editor
<!-- help: abs-maned maned.sh man-page groff editor -->

`maned.sh`, also by Mendel Cooper, builds the source of a man page from
answers to a few questions: the `.TH` line, then a `.SH` section for each
field that isn't left empty.

### Example A-39. A "man page" editor (`maned.sh`)

```bash no-run
#!/bin/bash
# maned.sh
# A rudimentary man page editor

# Version: 0.1 (Alpha, probably buggy)
# Author: Mendel Cooper <thegrendel.abs@gmail.com>
# Reldate: 16 June 2008
# License: GPL3


savefile=      # Global, used in multiple functions.
E_NOINPUT=90   # User input missing (error). May or may not be critical.

# =========== Markup Tags ============ #
TopHeader=".TH"
NameHeader=".SH NAME"
SyntaxHeader=".SH SYNTAX"
SynopsisHeader=".SH SYNOPSIS"
InstallationHeader=".SH INSTALLATION"
DescHeader=".SH DESCRIPTION"
OptHeader=".SH OPTIONS"
FilesHeader=".SH FILES"
EnvHeader=".SH ENVIRONMENT"
AuthHeader=".SH AUTHOR"
BugsHeader=".SH BUGS"
SeeAlsoHeader=".SH SEE ALSO"
BOLD=".B"
# Add more tags, as needed.
# See groff docs for markup meanings.
# ==================================== #

start ()
{
clear                  # Clear screen.
echo "ManEd"
echo "-----"
echo
echo "Simple man page creator"
echo "Author: Mendel Cooper"
echo "License: GPL3"
echo; echo; echo
}

progname ()
{
  echo -n "Program name? "
  read name

  echo -n "Manpage section? [Hit RETURN for default (\"1\") ]  "
  read section
  if [ -z "$section" ]
  then
    section=1   # Most man pages are in section 1.
  fi

  if [ -n "$name" ]
  then
    savefile=""$name"."$section""       #  Filename suffix = section.
    echo -n "$1 " >>$savefile
    name1=$(echo "$name" | tr a-z A-Z)  #  Change to uppercase,
                                        #+ per man page convention.
    echo -n "$name1" >>$savefile
  else
    echo "Error! No input."             # Mandatory input.
    exit $E_NOINPUT                     # Critical!
    #  Exercise: The script-abort if no filename input is a bit clumsy.
    #            Rewrite this section so a default filename is used
    #+           if no input.
  fi

  echo -n "  \"$section\"">>$savefile   # Append, always append.

  echo -n "Version? "
  read ver
  echo -n " \"Version $ver \"">>$savefile
  echo >>$savefile

  echo -n "Short description [0 - 5 words]? "
  read sdesc
  echo "$NameHeader">>$savefile
  echo ""$BOLD" "$name"">>$savefile
  echo "\- "$sdesc"">>$savefile

}

fill_in ()
{ # This function more or less copied from "pad.sh" script.
  echo -n "$2? "       # Get user input.
  read var             # May paste (a single line only!) to fill in field.

  if [ -n "$var" ]
  then
    echo "$1 " >>$savefile
    echo -n "$var" >>$savefile
  else                 # Don't append empty field to file.
    return $E_NOINPUT  # Not critical here.
  fi

  echo >>$savefile

}    


end ()
{
clear
echo -n "Would you like to view the saved man page (y/n)? "
read ans
if [ "$ans" = "n" -o "$ans" = "N" ]; then exit; fi
exec less "$savefile"  #  Exit script and hand off control to "less" ...
                       #+ ... which formats for viewing man page source.
}


# ---------------------------------------- #
start
progname "$TopHeader"
fill_in "$SynopsisHeader" "Synopsis"
fill_in "$DescHeader" "Long description"
# May paste in *single line* of text.
fill_in "$OptHeader" "Options"
fill_in "$FilesHeader" "Files"
fill_in "$AuthHeader" "Author"
fill_in "$BugsHeader" "Bugs"
fill_in "$SeeAlsoHeader" "See also"
# fill_in "$OtherHeader" ... as necessary.
end    # ... exit not needed.
# ---------------------------------------- #

#  Note that the generated man page will usually
#+ require manual fine-tuning with a text editor.
#  However, it's a distinct improvement upon
#+ writing man source from scratch
#+ or even editing a blank man page template.

#  The main deficiency of the script is that it permits
#+ pasting only a single text line into the input fields.
#  This may be a long, cobbled-together line, which groff
#  will automatically wrap and hyphenate.
#  However, if you want multiple (newline-separated) paragraphs,
#+ these must be inserted by manual text editing on the
#+ script-generated man page.
#  Exercise (difficult): Fix this!

#  This script is not nearly as elaborate as the
#+ full-featured "manedit" package
#+ http://freshmeat.net/projects/manedit/
#+ but it's much easier to use.
```

The canned answers come from an iterator here, one per question. The
original ends with `exec less "$savefile"`, which replaces the script with
`less`; tessh has no `exec`, but `os.execvp less [less $savefile]` does the
same. Here the page is simply `cat`ed. Writing the `.TH` line is one `print`
to the file opened for appending, not five `echo -n >>`s.

```tessh
import sys
# maned.sh: a rudimentary man page editor, by Mendel Cooper (GPL3).
# (Canned answers stand in for the keyboard, in the order they're asked.)
answers = iter [
  "frob"                          # Program name?
  ""                              # Manpage section? (RETURN: "1")
  "0.1"                           # Version?
  "frobnicate files"              # Short description?
  "frob [-v] file..."             # Synopsis
  "frob frobnicates each file in turn."   # Long description
  "-v  say what it is doing"      # Options
  ""                              # Files (empty: left out)
  "Mendel Cooper"                 # Author
  "None known."                   # Bugs
  "frobnitz(1)"                   # See also
  "y"                             # View the saved man page?
]

def ask prompt
  ans = next $answers             # (read, in the original)
  print "$prompt$ans"
  $ans

savefile = None                   # Global, used in multiple functions.
E_NOINPUT = 90                    # User input missing (error).

# =========== Markup Tags ============ #
TopHeader = ".TH"
NameHeader = ".SH NAME"
SyntaxHeader = ".SH SYNTAX"
SynopsisHeader = ".SH SYNOPSIS"
InstallationHeader = ".SH INSTALLATION"
DescHeader = ".SH DESCRIPTION"
OptHeader = ".SH OPTIONS"
FilesHeader = ".SH FILES"
EnvHeader = ".SH ENVIRONMENT"
AuthHeader = ".SH AUTHOR"
BugsHeader = ".SH BUGS"
SeeAlsoHeader = ".SH SEE ALSO"
BOLD = ".B"
# ==================================== #

def clear                         # Clear screen (only when it is one).
  if (sys.stdout.isatty)
    \clear

def start
  clear
  print ManEd
  print -----
  print
  print "Simple man page creator"
  print "Author: Mendel Cooper"
  print "License: GPL3"
  print ; print ; print

def progname header
  global savefile
  name = ask "Program name? "
  section = ask "Manpage section? [Hit RETURN for default (\"1\") ]  "
  if not $section
    section = 1                   # Most man pages are in section 1.
  if not $name
    print "Error! No input."      # Mandatory input.
    exit $E_NOINPUT               # Critical!
  savefile = "$name.$section"     # Filename suffix = section.
  ver = ask "Version? "
  sdesc = ask "Short description [0 - 5 words]? "
  with (open $savefile a) as f    # Append, always append.
    print "$header $($name.upper)  \"$section\" \"Version $ver \"" file=$f
    print $NameHeader file=$f
    print "$BOLD $name" file=$f
    print "\\- $sdesc" file=$f

def fill_in header prompt
  var = ask "$prompt? "
  if not $var                     # Don't append empty field to file.
    return $E_NOINPUT             # Not critical here.
  print "$header \n$var" >>$savefile

def end
  clear
  ans = ask "Would you like to view the saved man page (y/n)? "
  if $ans in [n N]
    exit
  cat $savefile                   # (exec less "$savefile", in the original)

# ---------------------------------------- #
start
progname $TopHeader
fill_in $SynopsisHeader Synopsis
fill_in $DescHeader "Long description"
fill_in $OptHeader Options
fill_in $FilesHeader Files
fill_in $AuthHeader Author
fill_in $BugsHeader Bugs
fill_in $SeeAlsoHeader "See also"
end
# ---------------------------------------- #
```

```output
ManEd
-----

Simple man page creator
Author: Mendel Cooper
License: GPL3



Program name? frob
Manpage section? [Hit RETURN for default ("1") ]  
Version? 0.1
Short description [0 - 5 words]? frobnicate files
Synopsis? frob [-v] file...
Long description? frob frobnicates each file in turn.
Options? -v  say what it is doing
Files? 
Author? Mendel Cooper
Bugs? None known.
See also? frobnitz(1)
Would you like to view the saved man page (y/n)? y
.TH FROB  "1" "Version 0.1 "
.SH NAME
.B frob
\- frobnicate files
.SH SYNOPSIS 
frob [-v] file...
.SH DESCRIPTION 
frob frobnicates each file in turn.
.SH OPTIONS 
-v  say what it is doing
.SH AUTHOR 
Mendel Cooper
.SH BUGS 
None known.
.SH SEE ALSO 
frobnitz(1)
```

## Petals Around the Rose
<!-- help: abs-petals petals.sh dice game random seed -->

Serghey Rodin's dice puzzle (modded by the ABS Guide's author): five dice
roll, you guess the "number of petals around the rose", and six right in a
row admits you to the Fellowship. How the answer is counted is the puzzle,
and the `calc` function gives it away.

### Example A-40. Petals Around the Rose (`petals.sh`)

```bash no-run
#!/bin/bash -i
# petals.sh

#########################################################################
# Petals Around the Rose                                                #
#                                                                       #
# Version 0.1 Created by Serghey Rodin                                  #
# Version 0.2 Modded by ABS Guide Author                                #
#                                                                       #
# License: GPL3                                                         #
# Used in ABS Guide with permission.                                    #
# ##################################################################### #

hits=0      # Correct guesses.
WIN=6       # Mastered the game.
ALMOST=5    # One short of mastery.
EXIT=exit   # Give up early?

RANDOM=$$   # Seeds the random number generator from PID of script.


# Bones (ASCII graphics for dice)
bone1[1]="|         |"
bone1[2]="|       o |"
bone1[3]="|       o |"
bone1[4]="| o     o |"
bone1[5]="| o     o |"
bone1[6]="| o     o |"
bone2[1]="|    o    |"
bone2[2]="|         |"
bone2[3]="|    o    |"
bone2[4]="|         |"
bone2[5]="|    o    |"
bone2[6]="| o     o |"
bone3[1]="|         |"
bone3[2]="| o       |"
bone3[3]="| o       |"
bone3[4]="| o     o |"
bone3[5]="| o     o |"
bone3[6]="| o     o |"
bone="+---------+"



# Functions

instructions () {

  clear
  echo -n "Do you need instructions? (y/n) "; read ans
  if [ "$ans" = "y" -o "$ans" = "Y" ]; then
    clear
    echo -e '\E[34;47m'  # Blue type.

#  "cat document"
    cat <<INSTRUCTIONSZZZ
The name of the game is Petals Around the Rose,
and that name is significant.
Five dice will roll and you must guess the "answer" for each roll.
It will be zero or an even number.
After your guess, you will be told the answer for the roll, but . . .
that's ALL the information you will get.

Six consecutive correct guesses admits you to the
Fellowship of the Rose.
INSTRUCTIONSZZZ

    echo -e "\033[0m"    # Turn off blue.
    else clear
  fi

}


fortune ()
{
  RANGE=7
  FLOOR=0
  number=0
  while [ "$number" -le $FLOOR ]
  do
    number=$RANDOM
    let "number %= $RANGE"   # 1 - 6.
  done

  return $number
}



throw () { # Calculate each individual die.
  fortune; B1=$?
  fortune; B2=$?
  fortune; B3=$?
  fortune; B4=$?
  fortune; B5=$?

  calc () { # Function embedded within a function!
    case "$1" in
       3   ) rose=2;;
       5   ) rose=4;;
       *   ) rose=0;;
    esac    # Simplified algorithm.
            # Doesn't really get to the heart of the matter.
    return $rose
  }

  answer=0
  calc "$B1"; answer=$(expr $answer + $(echo $?))
  calc "$B2"; answer=$(expr $answer + $(echo $?))
  calc "$B3"; answer=$(expr $answer + $(echo $?))
  calc "$B4"; answer=$(expr $answer + $(echo $?))
  calc "$B5"; answer=$(expr $answer + $(echo $?))
}



game ()
{ # Generate graphic display of dice throw.
  throw
    echo -e "\033[1m"    # Bold.
  echo -e "\n"
  echo -e "$bone\t$bone\t$bone\t$bone\t$bone"
  echo -e \
 "${bone1[$B1]}\t${bone1[$B2]}\t${bone1[$B3]}\t${bone1[$B4]}\t${bone1[$B5]}"
  echo -e \
 "${bone2[$B1]}\t${bone2[$B2]}\t${bone2[$B3]}\t${bone2[$B4]}\t${bone2[$B5]}"
  echo -e \
 "${bone3[$B1]}\t${bone3[$B2]}\t${bone3[$B3]}\t${bone3[$B4]}\t${bone3[$B5]}"
  echo -e "$bone\t$bone\t$bone\t$bone\t$bone"
  echo -e "\n\n\t\t"
    echo -e "\033[0m"    # Turn off bold.
  echo -n "There are how many petals around the rose? "
}



# ============================================================== #

instructions

while [ "$petal" != "$EXIT" ]    # Main loop.
do
  game
  read petal
  echo "$petal" | grep [0-9] >/dev/null  # Filter response for digit.
                                         # Otherwise just roll dice again.
  if [ "$?" -eq 0 ]   # If-loop #1.
  then
    if [ "$petal" == "$answer" ]; then    # If-loop #2.
    	echo -e "\nCorrect. There are $petal petals around the rose.\n"
        (( hits++ ))

        if [ "$hits" -eq "$WIN" ]; then   # If-loop #3.
          echo -e '\E[31;47m'  # Red type.
          echo -e "\033[1m"    # Bold.
          echo "You have unraveled the mystery of the Rose Petals!"
          echo "Welcome to the Fellowship of the Rose!!!"
          echo "(You are herewith sworn to secrecy.)"; echo
          echo -e "\033[0m"    # Turn off red & bold.
          break                # Exit!
        else echo "You have $hits correct so far."; echo

        if [ "$hits" -eq "$ALMOST" ]; then
          echo "Just one more gets you to the heart of the mystery!"; echo
        fi

      fi                                  # Close if-loop #3.

    else
      echo -e "\nWrong. There are $answer petals around the rose.\n"
      hits=0   # Reset number of correct guesses.
    fi                                    # Close if-loop #2.

    echo -n "Hit ENTER for the next roll, or type \"exit\" to end. "
    read
    if [ "$REPLY" = "$EXIT" ]; then exit
    fi

  fi                  # Close if-loop #1.

  clear
done                  # End of main (while) loop.

###

exit $?

# Resources:
# ---------
# 1) http://en.wikipedia.org/wiki/Petals_Around_the_Rose
#    (Wikipedia entry.)
# 2) http://www.borrett.id.au/computing/petals-bg.htm
#    (How Bill Gates coped with the Petals Around the Rose challenge.)
```

The faces are three lists, one per row of the die, indexed by its value,
and the five dice print row by row with `"\t".join`. Each function gives
its result back instead of passing it through `$?` (`fortune` returned the
die as its exit status, `calc` its petals). `random.seed 7` stands in for
`RANDOM=$$` so a run repeats, and a canned player answers: one wild guess,
then the count it has worked out, and Enter after each answer. The colors
are left out when the output isn't a terminal. Interactively, the guess is
`petal = input ""` and "Hit ENTER" is `input ""` again.

```tessh
import sys random
# petals.sh: Petals Around the Rose.
# Version 0.1 Created by Serghey Rodin; 0.2 modded by the ABS Guide author.

hits = 0         # Correct guesses.
WIN = 6          # Mastered the game.
ALMOST = 5       # One short of mastery.
EXIT = exit      # Give up early?

random.seed 7    # (RANDOM=$$ in the original: seeded, so a run repeats.)

# Colors, when the output is a terminal.
tty = (sys.stdout.isatty)
def esc code = if $tty then "\033[${code}m" else ""

# Bones (ASCII graphics for dice): the top, middle and bottom row of each face.
bone_top = [None "|         |" "|       o |" "|       o |" "| o     o |" "| o     o |" "| o     o |"]
bone_mid = [None "|    o    |" "|         |" "|    o    |" "|         |" "|    o    |" "| o     o |"]
bone_bot = [None "|         |" "| o       |" "| o       |" "| o     o |" "| o     o |" "| o     o |"]
bone = "+---------+"

# (The player, canned: a wild guess, then one who has caught on,
#  and Enter after each answer.)
rolls = 0
def read_guess dice
  global rolls
  rolls += 1
  g = if $rolls <= 1 then "4" else (str (sum [{3: 2, 5: 4}.get $d 0 for d in $dice]))
  print $g
  $g
def read_enter
  print
  ""

def clear
  if $tty
    \clear

def instructions
  clear
  ans = n                     # (read ans: the canned player says no)
  print "Do you need instructions? (y/n) $ans"
  if $ans in [y Y]
    clear
    print (esc "34;47")       # Blue type.
    cat <<INSTRUCTIONSZZZ
The name of the game is Petals Around the Rose,
and that name is significant.
Five dice will roll and you must guess the "answer" for each roll.
It will be zero or an even number.
After your guess, you will be told the answer for the roll, but . . .
that's ALL the information you will get.

Six consecutive correct guesses admits you to the
Fellowship of the Rose.
INSTRUCTIONSZZZ
    print (esc 0)             # Turn off blue.
  else
    clear

def fortune = random.randint 1 6

def throw
  # Calculate each individual die.
  dice = [(fortune) for _ in (range 5)]
  def calc d                  # Function embedded within a function!
    match $d
      case 3
        2
      case 5
        4
      case _
        0                     # Simplified algorithm.
  [$dice (sum (map $calc $dice))]

def game
  # Generate graphic display of dice throw.
  [dice answer] = (throw)
  print (esc 1)               # Bold.
  print "\n"
  for row in [[$bone] * 5, [$bone_top[$d] for d in $dice], [$bone_mid[$d] for d in $dice], [$bone_bot[$d] for d in $dice], [$bone] * 5]
    print ("\t".join $row)
  print "\n\n\t\t"
  print (esc 0)               # Turn off bold.
  print "There are how many petals around the rose? " end=""
  [$dice $answer]

# ============================================================== #

instructions
petal = ""
while $petal != $EXIT         # Main loop.
  [dice answer] = (game)
  petal = read_guess $dice
  if $petal =~ re'[0-9]'      # Filter response for digit.
    if $petal == (str $answer)
      print "\nCorrect. There are $petal petals around the rose.\n"
      hits += 1
      if $hits == $WIN
        print (esc "31;47") (esc 1) sep=""   # Red, bold.
        print "You have unraveled the mystery of the Rose Petals!"
        print "Welcome to the Fellowship of the Rose!!!"
        print "(You are herewith sworn to secrecy.)" ; print
        print (esc 0)         # Turn off red & bold.
        break                 # Exit!
      else
        print "You have $hits correct so far." ; print
        if $hits == $ALMOST
          print "Just one more gets you to the heart of the mystery!" ; print
    else
      print "\nWrong. There are $answer petals around the rose.\n"
      hits = 0                # Reset number of correct guesses.
    print "Hit ENTER for the next roll, or type \"exit\" to end. " end=""
    if (read_enter) == $EXIT
      exit
  clear
```

```output
Do you need instructions? (y/n) n



+---------+	+---------+	+---------+	+---------+	+---------+
|       o |	|       o |	| o     o |	| o     o |	|         |
|    o    |	|         |	|         |	| o     o |	|    o    |
| o       |	| o       |	| o     o |	| o     o |	|         |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 4

Wrong. There are 2 petals around the rose.

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
|         |	| o     o |	|         |	|       o |	| o     o |
|    o    |	|    o    |	|    o    |	|    o    |	|    o    |
|         |	| o     o |	|         |	| o       |	| o     o |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 10

Correct. There are 10 petals around the rose.

You have 1 correct so far.

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
|         |	| o     o |	|       o |	|         |	|         |
|    o    |	|    o    |	|         |	|    o    |	|    o    |
|         |	| o     o |	| o       |	|         |	|         |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 4

Correct. There are 4 petals around the rose.

You have 2 correct so far.

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
| o     o |	| o     o |	|         |	|       o |	|         |
|         |	|         |	|    o    |	|         |	|    o    |
| o     o |	| o     o |	|         |	| o       |	|         |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 0

Correct. There are 0 petals around the rose.

You have 3 correct so far.

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
| o     o |	| o     o |	|         |	| o     o |	|         |
|    o    |	|         |	|    o    |	|    o    |	|    o    |
| o     o |	| o     o |	|         |	| o     o |	|         |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 8

Correct. There are 8 petals around the rose.

You have 4 correct so far.

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
|       o |	| o     o |	| o     o |	| o     o |	|         |
|         |	| o     o |	| o     o |	|    o    |	|    o    |
| o       |	| o     o |	| o     o |	| o     o |	|         |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 4

Correct. There are 4 petals around the rose.

You have 5 correct so far.

Just one more gets you to the heart of the mystery!

Hit ENTER for the next roll, or type "exit" to end. 



+---------+	+---------+	+---------+	+---------+	+---------+
| o     o |	| o     o |	| o     o |	|         |	|       o |
|    o    |	|    o    |	|         |	|    o    |	|         |
| o     o |	| o     o |	| o     o |	|         |	| o       |
+---------+	+---------+	+---------+	+---------+	+---------+


		

There are how many petals around the rose? 8

Correct. There are 8 petals around the rose.


You have unraveled the mystery of the Rose Petals!
Welcome to the Fellowship of the Rose!!!
(You are herewith sworn to secrecy.)


```
