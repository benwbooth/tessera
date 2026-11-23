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

## Quacky
<!-- help: abs-qky qky.sh quacky perquackey word-game anagram -->

Mendel Cooper's Quacky is a simplified Perquackey: ten random letters, and
you score for each dictionary word you can make from them, more for longer
words, with a bonus for five words of one length and a penalty for a bad
word. The same random seed (the script's argument) gives the same letters,
for "duplicate" play.

### Example A-41. Quacky: a Perquackey-type word game (`qky.sh`)

```bash no-run
#!/bin/bash
# qky.sh

##############################################################
# QUACKEY: a somewhat simplified version of Perquackey [TM]. #
#                                                            #
# Author: Mendel Cooper  <thegrendel.abs@gmail.com>          #
# version 0.1.02      03 May, 2008                           #
# License: GPL3                                              #
##############################################################

WLIST=/usr/share/dict/word.lst
#                     ^^^^^^^^  Word list file found here.
#  ASCII word list, one word per line, UNIX format.
#  A suggested list is the script author's "yawl" word list package.
#  http://bash.deta.in/yawl-0.3.2.tar.gz
#    or
#  http://ibiblio.org/pub/Linux/libs/yawl-0.3.2.tar.gz

NONCONS=0     # Word not constructable from letter set.
CONS=1        # Constructable.
SUCCESS=0
NG=1
FAILURE=''
NULL=0        # Zero out value of letter (if found).
MINWLEN=3     # Minimum word length.
MAXCAT=5      # Maximum number of words in a given category.
PENALTY=200   # General-purpose penalty for unacceptable words.
total=
E_DUP=70      # Duplicate word error.

TIMEOUT=10    # Time for word input.

NVLET=10      # 10 letters for non-vulnerable.
VULET=13      # 13 letters for vulnerable (not yet implemented!).

declare -a Words
declare -a Status
declare -a Score=( 0 0 0 0 0 0 0 0 0 0 0 )


letters=( a n s r t m l k p r b c i d s i d z e w u e t f
e y e r e f e g t g h h i t r s c i t i d i j a t a o l a
m n a n o v n w o s e l n o s p a q e e r a b r s a o d s
t g t i t l u e u v n e o x y m r k )
#  Letter distribution table shamelessly borrowed from "Wordy" game,
#+ ca. 1992, written by a certain fine fellow named Mendel Cooper.

declare -a LS

numelements=${#letters[@]}
randseed="$1"

instructions ()
{
  clear
  echo "Welcome to QUACKEY, the anagramming word construction game."; echo
  echo -n "Do you need instructions? (y/n) "; read ans

   if [ "$ans" = "y" -o "$ans" = "Y" ]; then
     clear
     echo -e '\E[31;47m'  # Red foreground. '\E[34;47m' for blue.
     cat <<INSTRUCTION1

QUACKEY is a variant of Perquackey [TM].
The rules are the same, but the scoring is simplified
and plurals of previously played words are allowed.
"Vulnerable" play is not yet implemented,
but it is otherwise feature-complete.

As the game begins, the player gets 10 letters.
The object is to construct valid dictionary words
of at least 3-letter length from the letterset.
Each word-length category
-- 3-letter, 4-letter, 5-letter, ... --
fills up with the fifth word entered,
and no further words in that category are accepted.

The penalty for too-short (two-letter), duplicate, unconstructable,
and invalid (not in dictionary) words is -200. The same penalty applies
to attempts to enter a word in a filled-up category.

INSTRUCTION1

  echo -n "Hit ENTER for next page of instructions. "; read az1

     cat <<INSTRUCTION2

The scoring mostly corresponds to classic Perquackey:
The first 3-letter word scores    60, plus   10 for each additional one.
The first 4-letter word scores   120, plus   20 for each additional one.
The first 5-letter word scores   200, plus   50 for each additional one.
The first 6-letter word scores   300, plus  100 for each additional one.
The first 7-letter word scores   500, plus  150 for each additional one.
The first 8-letter word scores   750, plus  250 for each additional one.
The first 9-letter word scores  1000, plus  500 for each additional one.
The first 10-letter word scores 2000, plus 2000 for each additional one.

Category completion bonuses are:
3-letter words   100
4-letter words   200
5-letter words   400
6-letter words   800
7-letter words  2000
8-letter words 10000
This is a simplification of the absurdly baroque Perquackey bonus
scoring system.

INSTRUCTION2

  echo -n "Hit ENTER for final page of instructions. "; read az1

     cat <<INSTRUCTION3


Hitting just ENTER for a word entry ends the game.

Individual word entry is timed to a maximum of 10 seconds.
*** Timing out on an entry ends the game. ***
Aside from that, the game is untimed.

--------------------------------------------------
Game statistics are automatically saved to a file.
--------------------------------------------------

For competitive ("duplicate") play, a previous letterset
may be duplicated by repeating the script's random seed,
command-line parameter \$1.
For example, "qky 7633" specifies the letterset 
c a d i f r h u s k ...
INSTRUCTION3

  echo; echo -n "Hit ENTER to begin game. "; read az1

       echo -e "\033[0m"    # Turn off red.
     else clear
  fi

  clear

}



seed_random ()
{                         #  Seed random number generator.
  if [ -n "$randseed" ]   #  Can specify random seed.
  then                    #+ for play in competitive mode.
#   RANDOM="$randseed"
    echo "RANDOM seed set to "$randseed""
  else
    randseed="$$"         # Or get random seed from process ID.
    echo "RANDOM seed not specified, set to Process ID of script ($$)."
  fi

  RANDOM="$randseed"

  echo
}


get_letset ()
{
  element=0
  echo -n "Letterset:"

  for lset in $(seq $NVLET)
  do  # Pick random letters to fill out letterset.
    LS[element]="${letters[$((RANDOM%numelements))]}"
    ((element++))
  done

  echo
  echo "${LS[@]}"

}


add_word ()
{
  wrd="$1"
  local idx=0

  Status[0]=""
  Status[3]=""
  Status[4]=""

  while [ "${Words[idx]}" != '' ]
  do
    if [ "${Words[idx]}" = "$wrd" ]
    then
      Status[3]="Duplicate-word-PENALTY"
      let "Score[0]= 0 - $PENALTY"
      let "Score[1]-=$PENALTY"
      return $E_DUP
    fi

    ((idx++))
  done

  Words[idx]="$wrd"
  get_score

}

get_score()
{
  local wlen=0
  local score=0
  local bonus=0
  local first_word=0
  local add_word=0
  local numwords=0

  wlen=${#wrd}
  numwords=${Score[wlen]}
  Score[2]=0
  Status[4]=""   # Initialize "bonus" to 0.

  case "$wlen" in
    3) first_word=60
       add_word=10;;
    4) first_word=120
       add_word=20;;
    5) first_word=200
       add_word=50;;
    6) first_word=300
       add_word=100;;
    7) first_word=500
       add_word=150;;
    8) first_word=750
       add_word=250;;
    9) first_word=1000
       add_word=500;;
   10) first_word=2000
       add_word=2000;;   # This category modified from original rules!
      esac

  ((Score[wlen]++))
  if [ ${Score[wlen]} -eq $MAXCAT ]
  then   # Category completion bonus scoring simplified!
    case $wlen in
      3 ) bonus=100;;
      4 ) bonus=200;;
      5 ) bonus=400;;
      6 ) bonus=800;;
      7 ) bonus=2000;;
      8 ) bonus=10000;;
    esac  # Needn't worry about 9's and 10's.
    Status[4]="Category-$wlen-completion***BONUS***"
    Score[2]=$bonus
  else
    Status[4]=""   # Erase it.
  fi


    let "score =  $first_word +   $add_word * $numwords"
    if [ "$numwords" -eq 0 ]
    then
      Score[0]=$score
    else
      Score[0]=$add_word
    fi   #  All this to distinguish last-word score
         #+ from total running score.
  let "Score[1] += ${Score[0]}"
  let "Score[1] += ${Score[2]}"

}



get_word ()
{
  local wrd=''
  read -t $TIMEOUT wrd   # Timed read.
  echo $wrd
}

is_constructable ()
{ # This is the most complex and difficult-to-write function.
  local -a local_LS=( "${LS[@]}" )  # Local copy of letter set.
  local is_found=0
  local idx=0
  local pos
  local strlen
  local local_word=( "$1" )
  strlen=${#local_word}

  while [ "$idx" -lt "$strlen" ]
  do
    is_found=$(expr index "${local_LS[*]}" "${local_word:idx:1}")
    if [ "$is_found" -eq "$NONCONS" ] # Not constructable!
    then
      echo "$FAILURE"; return
    else
      ((pos = ($is_found - 1) / 2))   # Compensate for spaces betw. letters!
      local_LS[pos]=$NULL             # Zero out used letters.
      ((idx++))                       # Bump index.
    fi
  done

  echo "$SUCCESS"
  return
}

is_valid ()
{ # Surprisingly easy to check if word in dictionary ...
  fgrep -qw "$1" "$WLIST"   # ... courtesy of 'grep' ...
  echo $?
}

check_word ()
{
  if [ -z "$1" ]
  then
    return
  fi

  Status[1]=""
  Status[2]=""
  Status[3]=""
  Status[4]=""

  iscons=$(is_constructable "$1")
  if [ "$iscons" ]
  then
    Status[1]="constructable" 
    v=$(is_valid "$1")
    if [ "$v" -eq "$SUCCESS" ]
    then
      Status[2]="valid" 
      strlen=${#1}

      if [ ${Score[strlen]} -eq "$MAXCAT" ]   # Category full!
      then
        Status[3]="Category-$strlen-overflow-PENALTY"
        return $NG
      fi

      case "$strlen" in
        1 | 2 )
        Status[3]="Two-letter-word-PENALTY"
        return $NG;;
        * ) 
	Status[3]=""
	return $SUCCESS;;
      esac
    else
      Status[3]="Not-valid-PENALTY"
      return $NG
    fi
  else
    Status[3]="Not-constructable-PENALTY" 
      return $NG
  fi

  ### FIXME: Streamline the above code block.

}


display_words ()
{
  local idx=0
  local wlen0

  clear
  echo "Letterset:   ${LS[@]}"
  echo "Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:"
  echo "------------------------------------------------------------"


   
  while [ "${Words[idx]}" != '' ]
  do
   wlen0=${#Words[idx]}
   case "$wlen0" in
     3) ;;
     4) echo -n "           " ;;
     5) echo -n "                     " ;;
     6) echo -n "                                " ;;
     7) echo -n "                                          " ;;
     8) echo -n "                                                     " ;;
   esac
   echo "${Words[idx]}"
   ((idx++))
  done

  ### FIXME: The word display is pretty crude.
}


play ()
{
  word="Start game"   # Dummy word, to start ...

  while [ "$word" ]   #  If player just hits return (null word),
  do                  #+ then game ends.
    echo "$word: "${Status[@]}""
    echo -n "Last score: [${Score[0]}]   TOTAL score: [${Score[1]}]:     Next word: "
    total=${Score[1]}
    word=$(get_word)
    check_word "$word"

    if [ "$?" -eq "$SUCCESS" ]
    then
      add_word "$word"
    else
      let "Score[0]= 0 - $PENALTY"
      let "Score[1]-=$PENALTY"
    fi

  display_words
  done   # Exit game.

  ### FIXME: The play () function calls too many other functions.
  ### This verges on "spaghetti code" !!!
}

end_of_game ()
{ # Save and display stats.

  #######################Autosave##########################
  savefile=qky.save.$$
  #                 ^^ PID of script
  echo `date` >> $savefile
  echo "Letterset # $randseed  (random seed) ">> $savefile
  echo -n "Letterset: " >> $savefile
  echo "${LS[@]}" >> $savefile
  echo "---------" >> $savefile
  echo "Words constructed:" >> $savefile
  echo "${Words[@]}" >> $savefile
  echo >> $savefile
  echo "Score: $total" >> $savefile

  echo "Statistics for this round saved in \""$savefile"\""
  #########################################################

  echo "Score for this round: $total"
  echo "Words:  ${Words[@]}"
}

# ---------#
instructions
seed_random
get_letset
play
end_of_game
# ---------#

exit $?

# TODO:
#
# 1) Clean up code!
# 2) Prettify the display_words () function (maybe with widgets?).
# 3) Improve the time-out ... maybe change to untimed entry,
#+   but with a time limit for the overall round.   
# 4) An on-screen countdown timer would be nice.
# 5) Implement "vulnerable" mode of play for compatibility with classic
#+   version of the game.
# 6) Improve save-to-file capability (and maybe make it optional).
# 7) Fix bugs!!!

# For more info, reference:
# http://bash.deta.in/qky.README.html
```

A small word list stands in for `/usr/share/dict/word.lst`, the seed is 12
when none is given, and the player is canned: some good words, one too
short, a duplicate, one that can't be made, one not in the list, and a
sixth three-letter word after the category has filled. bash keeps the
scores in slots of one array (`Score[0]` the last, `Score[1]` the total,
`Score[3]`… the count of each length) and the reasons in `Status[1..4]`;
here they're named variables, a count list and a dict. `check_word` gives
back the reason a word is no good, or `None`. Checking that a word can be
made is a loop that takes each letter out of a copy of the set, which is
what bash's `expr index` arithmetic does. The save file is named after the
seed here, not the PID, so the run repeats.

bash's `read -t 10` gives up on a word after ten seconds. In tessh,
`select` waits for input with a timeout:

```tessh no-run
import sys select
def get_word
  print "Next word: " end="" flush=True
  [ready _ _] = select.select [$sys.stdin] [] [] $TIMEOUT
  if $ready then (sys.stdin.readline).strip else ""   # Timed out: "".
```

```tessh
import sys random
# qky.sh: QUACKEY, a somewhat simplified version of Perquackey [TM].
# Author: Mendel Cooper, version 0.1.02, 03 May 2008. License: GPL3.

# (A made-up word list to try it on: /usr/share/dict/word.lst in the original.)
echo "den hat hints it quack set sit stand ten than then tin wish widths" | tr " " "\n" >word.lst
WLIST = ./word.lst
# (The player, canned: the words typed, then just Enter to end.)
typed = iter [hat then it hat quack tisah ten set tin den sit stand widths ""]

MINWLEN = 3        # Minimum word length.
MAXCAT = 5         # Maximum number of words in a given category.
PENALTY = 200      # General-purpose penalty for unacceptable words.
TIMEOUT = 10       # Time for word input.
NVLET = 10         # 10 letters for non-vulnerable.
VULET = 13         # 13 letters for vulnerable (not yet implemented!).

Words = []
Status = {}                       # Why the last word scored what it did.
last = 0                          # Last score.
total = 0                         # TOTAL score.
bonus = 0
count = [0] * 11                  # Words played, by length.

letters = [a n s r t m l k p r b c i d s i d z e w u e t f
e y e r e f e g t g h h i t r s c i t i d i j a t a o l a
m n a n o v n w o s e l n o s p a q e e r a b r s a o d s
t g t i t l u e u v n e o x y m r k]
#  Letter distribution table shamelessly borrowed from "Wordy" game,
#+ ca. 1992, written by a certain fine fellow named Mendel Cooper.

randseed = if (len $argv) > 1 then (int $argv[1]) else 12   # (sample seed, when none is given)

def clear
  if (sys.stdout.isatty)
    \clear

def instructions
  clear
  print "Welcome to QUACKEY, the anagramming word construction game." ; print
  print "Do you need instructions? (y/n) n"   # (The canned player says no;
  clear                                       #  the three pages are as in bash.)

def seed_random
  # Seed random number generator: the same seed gives the same letterset,
  # for play in competitive mode.
  print "RANDOM seed set to $randseed"
  random.seed $randseed
  print

LS = []
def get_letset
  print "Letterset:"
  for _ in (range $NVLET)         # Pick random letters to fill out letterset.
    $LS.append (random.choice $letters)
  print (" ".join $LS)

def get_score wrd
  global last total bonus
  wlen = len $wrd
  numwords = $count[$wlen]
  bonus = 0
  $Status[bonus] = ""
  [first_word add_word] = {3: [60 10], 4: [120 20], 5: [200 50], 6: [300 100],
                           7: [500 150], 8: [750 250], 9: [1000 500], 10: [2000 2000]}[$wlen]
  $count[$wlen] += 1
  if $count[$wlen] == $MAXCAT     # Category completion bonus scoring simplified!
    bonus = {3: 100, 4: 200, 5: 400, 6: 800, 7: 2000, 8: 10000}.get $wlen 0
    $Status[bonus] = "Category-$wlen-completion***BONUS***"
  last = if $numwords == 0 then $first_word else $add_word
  total += $last + $bonus

def penalize why
  global last total
  $Status[penalty] = $why
  last = - $PENALTY
  total -= $PENALTY

def add_word wrd
  $Status[penalty] = ""
  $Status[bonus] = ""
  if $wrd in $Words
    penalize Duplicate-word-PENALTY
    return
  $Words.append $wrd
  get_score $wrd

def get_word
  wrd = next $typed               # (read -t $TIMEOUT wrd, in the original)
  print $wrd
  $wrd

def is_constructable word
  local_LS = (list $LS)           # Local copy of letter set.
  for c in $word
    if $c not in $local_LS        # Not constructable!
      return False
    $local_LS.remove $c           # Use up the letter.
  True

def is_valid word                 # Surprisingly easy to check if word in dictionary ...
  bool (fgrep -qw $word $WLIST)   # ... courtesy of 'grep' ...

def check_word word               # The reason it's no good, or None.
  Status.clear
  if not (is_constructable $word)
    return Not-constructable-PENALTY
  $Status[cons] = constructable
  if not (is_valid $word)
    return Not-valid-PENALTY
  $Status[valid] = valid
  n = len $word
  if $n < $MINWLEN
    return Two-letter-word-PENALTY
  if $count[$n] == $MAXCAT        # Category full!
    return "Category-$n-overflow-PENALTY"
  None

def display_words
  clear
  print "Letterset:   $(' '.join $LS)"
  print "Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:"
  print "------------------------------------------------------------"
  indent = {3: 0, 4: 11, 5: 21, 6: 32, 7: 42, 8: 53}
  for w in $Words
    print (" " * ($indent.get (len $w) 0)) $w sep=""

def play
  word = "Start game"             # Dummy word, to start ...
  while $word                     # If player just hits return, the game ends.
    print "$word: $(' '.join [$s for s in ($Status.values) if $s])"
    print "Last score: [$last]   TOTAL score: [$total]:     Next word: " end=""
    word = (get_word)
    if not $word
      break
    why = check_word $word
    if $why
      penalize $why
    else
      add_word $word
    display_words

def end_of_game                   # Save and display stats.
  savefile = "qky.save.$randseed" # (qky.save.$$, the PID, in the original)
  with (open $savefile a) as f
    print (date) file=$f
    print "Letterset # $randseed  (random seed) " file=$f
    print "Letterset: $(' '.join $LS)" file=$f
    print --------- file=$f
    print "Words constructed:" file=$f
    print (" ".join $Words) file=$f
    print file=$f
    print "Score: $total" file=$f
  print "Statistics for this round saved in \"$savefile\""
  print "Score for this round: $total"
  print "Words:  $(' '.join $Words)"

# ---------#
instructions
seed_random
get_letset
play
end_of_game
# ---------#
```

```output
Welcome to QUACKEY, the anagramming word construction game.

Do you need instructions? (y/n) n
RANDOM seed set to 12

Letterset:
w h t s i d e t n a
Start game: 
Last score: [0]   TOTAL score: [0]:     Next word: hat
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
hat: constructable valid
Last score: [60]   TOTAL score: [60]:     Next word: then
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
then: constructable valid
Last score: [120]   TOTAL score: [180]:     Next word: it
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
it: constructable valid Two-letter-word-PENALTY
Last score: [-200]   TOTAL score: [-20]:     Next word: hat
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
hat: constructable valid Duplicate-word-PENALTY
Last score: [-200]   TOTAL score: [-220]:     Next word: quack
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
quack: Not-constructable-PENALTY
Last score: [-200]   TOTAL score: [-420]:     Next word: tisah
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
tisah: constructable Not-valid-PENALTY
Last score: [-200]   TOTAL score: [-620]:     Next word: ten
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
ten: constructable valid
Last score: [10]   TOTAL score: [-610]:     Next word: set
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
set: constructable valid
Last score: [10]   TOTAL score: [-600]:     Next word: tin
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
tin
tin: constructable valid
Last score: [10]   TOTAL score: [-590]:     Next word: den
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
tin
den
den: constructable valid Category-3-completion***BONUS***
Last score: [10]   TOTAL score: [-480]:     Next word: sit
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
tin
den
sit: constructable valid Category-3-overflow-PENALTY
Last score: [-200]   TOTAL score: [-680]:     Next word: stand
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
tin
den
                     stand
stand: constructable valid
Last score: [200]   TOTAL score: [-480]:     Next word: widths
Letterset:   w h t s i d e t n a
Threes:    Fours:    Fives:     Sixes:    Sevens:    Eights:
------------------------------------------------------------
hat
           then
ten
set
tin
den
                     stand
                                widths
widths: constructable valid
Last score: [300]   TOTAL score: [-180]:     Next word: 
Statistics for this round saved in "qky.save.12"
Score for this round: -180
Words:  hat then ten set tin den stand widths
```

## Nim
<!-- help: abs-nim nim.sh nim game pegs bot -->

Mendel Cooper's Nim: five rows of pegs, you and the bot take turns
removing pegs from one row, and whoever takes the last peg loses. The bot
picks a row at random and leaves one peg in it.

### Example A-42. Nim (`nim.sh`)

```bash no-run
#!/bin/bash
# nim.sh: Game of Nim

# Author: Mendel Cooper
# Reldate: 15 July 2008
# License: GPL3

ROWS=5     # Five rows of pegs (or matchsticks).
WON=91     # Exit codes to keep track of wins/losses.
LOST=92    # Possibly useful if running in batch mode.  
QUIT=99
peg_msg=   # Peg/Pegs?
Rows=( 0 5 4 3 2 1 )   # Array holding play info.
# ${Rows[0]} holds total number of pegs, updated after each turn.
# Other array elements hold number of pegs in corresponding row.

instructions ()
{
  clear
  tput bold
  echo "Welcome to the game of Nim."; echo
  echo -n "Do you need instructions? (y/n) "; read ans

   if [ "$ans" = "y" -o "$ans" = "Y" ]; then
     clear
     echo -e '\E[33;41m'  # Yellow fg., over red bg.; bold.
     cat <<INSTRUCTIONS

Nim is a game with roots in the distant past.
This particular variant starts with five rows of pegs.

1:    | | | | | 
2:     | | | | 
3:      | | | 
4:       | | 
5:        | 

The number at the left identifies the row.

The human player moves first, and alternates turns with the bot.
A turn consists of removing at least one peg from a single row.
It is permissable to remove ALL the pegs from a row.
For example, in row 2, above, the player can remove 1, 2, 3, or 4 pegs.
The player who removes the last peg loses.

The strategy consists of trying to be the one who removes
the next-to-last peg(s), leaving the loser with the final peg.

To exit the game early, hit ENTER during your turn.
INSTRUCTIONS

echo; echo -n "Hit ENTER to begin game. "; read azx

      echo -e "\033[0m"    # Restore display.
      else tput sgr0; clear
  fi

clear

}


tally_up ()
{
  let "Rows[0] = ${Rows[1]} + ${Rows[2]} + ${Rows[3]} + ${Rows[4]} + \
  ${Rows[5]}"    # Add up how many pegs remaining.
}


display ()
{
  index=1   # Start with top row.
  echo

  while [ "$index" -le "$ROWS" ]
  do
    p=${Rows[index]}
    echo -n "$index:   "          # Show row number.

  # ------------------------------------------------
  # Two concurrent inner loops.

      indent=$index
      while [ "$indent" -gt 0 ]
      do
        echo -n " "               # Staggered rows.
        ((indent--))              # Spacing between pegs.
      done

    while [ "$p" -gt 0 ]
    do
      echo -n "| "
      ((p--))
    done
  # -----------------------------------------------

  echo
  ((index++))
  done  

  tally_up

  rp=${Rows[0]}

  if [ "$rp" -eq 1 ]
  then
    peg_msg=peg
    final_msg="Game over."
  else             # Game not yet over . . .
    peg_msg=pegs
    final_msg=""   # . . . So "final message" is blank.
  fi

  echo "      $rp $peg_msg remaining."
  echo "      "$final_msg""


  echo
}

player_move ()
{

  echo "Your move:"

  echo -n "Which row? "
  while read idx
  do                   # Validity check, etc.

    if [ -z "$idx" ]   # Hitting return quits.
    then
        echo "Premature exit."; echo
        tput sgr0      # Restore display.
        exit $QUIT
    fi

    if [ "$idx" -gt "$ROWS" -o "$idx" -lt 1 ]   # Bounds check.
    then
      echo "Invalid row number!"
      echo -n "Which row? "
    else
      break
    fi
    # TODO:
    # Add check for non-numeric input.
    # Also, script crashes on input outside of range of long double.
    # Fix this.

  done

  echo -n "Remove how many? "
  while read num
  do                   # Validity check.

  if [ -z "$num" ]
  then
    echo "Premature exit."; echo
    tput sgr0          # Restore display.
    exit $QUIT
  fi

    if [ "$num" -gt ${Rows[idx]} -o "$num" -lt 1 ]
    then
      echo "Cannot remove $num!"
      echo -n "Remove how many? "
    else
      break
    fi
  done
  # TODO:
  # Add check for non-numeric input.
  # Also, script crashes on input outside of range of long double.
  # Fix this.

  let "Rows[idx] -= $num"

  display
  tally_up

  if [ ${Rows[0]} -eq 1 ]
  then
   echo "      Human wins!"
   echo "      Congratulations!"
   tput sgr0   # Restore display.
   echo
   exit $WON
  fi

  if [ ${Rows[0]} -eq 0 ]
  then          # Snatching defeat from the jaws of victory . . .
    echo "      Fool!"
    echo "      You just removed the last peg!"
    echo "      Bot wins!"
    tput sgr0   # Restore display.
    echo
    exit $LOST
  fi
}


bot_move ()
{

  row_b=0
  while [[ $row_b -eq 0 || ${Rows[row_b]} -eq 0 ]]
  do
    row_b=$RANDOM          # Choose random row.
    let "row_b %= $ROWS"
  done


  num_b=0
  r0=${Rows[row_b]}

  if [ "$r0" -eq 1 ]
  then
    num_b=1
  else
    let "num_b = $r0 - 1"
         #  Leave only a single peg in the row.
  fi     #  Not a very strong strategy,
         #+ but probably a bit better than totally random.

  let "Rows[row_b] -= $num_b"
  echo -n "Bot:  "
  echo "Removing from row $row_b ... "

  if [ "$num_b" -eq 1 ]
  then
    peg_msg=peg
  else
    peg_msg=pegs
  fi

  echo "      $num_b $peg_msg."

  display
  tally_up

  if [ ${Rows[0]} -eq 1 ]
  then
   echo "      Bot wins!"
   tput sgr0   # Restore display.
   exit $WON
  fi

}


# ================================================== #
instructions     # If human player needs them . . .
tput bold        # Bold characters for easier viewing.
display          # Show game board.

while [ true ]   # Main loop.
do               # Alternate human and bot turns.
  player_move
  bot_move
done
# ================================================== #

# Exercise:
# --------
# Improve the bot's strategy.
# There is, in fact, a Nim strategy that can force a win.
# See the Wikipedia article on Nim:  http://en.wikipedia.org/wiki/Nim
# Recode the bot to use this strategy (rather difficult).

#  Curiosities:
#  -----------
#  Nim played a prominent role in Alain Resnais' 1961 New Wave film,
#+ Last Year at Marienbad.
#
#  In 1978, Leo Christopherson wrote an animated version of Nim,
#+ Android Nim, for the TRS-80 Model I.
```

The rows are a list, its first item the total, as in bash; each row prints
in one `print` with `" " * $index` and `"| " * pegs` in place of two
counting loops. The human's answers are canned (with a row that isn't one
and a count too big, to show the checks) and `random` is seeded; `int`
turns an answer into a number, and an answer that isn't one (the
original's TODO) would raise a `ValueError` rather than crash bash's test.
As in bash, `$RANDOM % ROWS` never picks row 5, so the bot only plays rows
1 to 4 (and would loop forever with only row 5 left). `tput` is called
only when the output is a terminal.

```tessh
import sys random
# nim.sh: Game of Nim. Author: Mendel Cooper, 15 July 2008. License: GPL3.

ROWS = 5         # Five rows of pegs (or matchsticks).
WON = 91         # Exit codes to keep track of wins/losses.
LOST = 92        # Possibly useful if running in batch mode.
QUIT = 99
Rows = [0 5 4 3 2 1]   # Pegs in each row; Rows[0] holds the total.

random.seed 3    # (bash's $RANDOM, seeded so a run repeats.)
# (The human player, canned: the answers typed, row then how many.)
typed = iter [9 2 6 4 4 2 3 3]
def read prompt
  ans = str (next $typed "")      # (read, in the original)
  print "$prompt$ans"
  $ans

tty = (sys.stdout.isatty)
def tput what                     # Bold and back, on a terminal.
  if $tty
    \tput $what

def instructions
  print "Welcome to the game of Nim." ; print
  print "Do you need instructions? (y/n) n"   # (The canned player says no.)

def tally_up                      # Add up how many pegs remaining.
  $Rows[0] = sum $Rows[1:]

def display
  print
  for index in (range 1 ($ROWS + 1))
    # Staggered rows: the row number, an indent, then the pegs.
    print "$index:   " (" " * $index) ("| " * $Rows[$index]) sep=""
  tally_up
  rp = $Rows[0]
  [peg_msg final_msg] = if $rp == 1 then [peg "Game over."] else [pegs ""]
  print "      $rp $peg_msg remaining."
  print "      $final_msg"
  print

def end status *lines
  for line in $lines
    print "      $line"
  tput sgr0                       # Restore display.
  print
  exit $status

def player_move
  print "Your move:"
  while True                      # Validity check, etc.
    idx = read "Which row? "
    if not $idx                   # Hitting return quits.
      print "Premature exit." ; print
      tput sgr0
      exit $QUIT
    idx = int $idx
    if 1 <= $idx <= $ROWS
      break
    print "Invalid row number!"   # Bounds check.
  while True
    num = read "Remove how many? "
    if not $num
      print "Premature exit." ; print
      tput sgr0
      exit $QUIT
    num = int $num
    if 1 <= $num <= $Rows[$idx]
      break
    print "Cannot remove $num!"
  $Rows[$idx] -= $num
  display
  if $Rows[0] == 1
    end $WON "Human wins!" "Congratulations!"
  if $Rows[0] == 0                # Snatching defeat from the jaws of victory . . .
    end $LOST "Fool!" "You just removed the last peg!" "Bot wins!"

def bot_move
  row_b = 0
  while $row_b == 0 or $Rows[$row_b] == 0
    row_b = random.randrange $ROWS    # Choose random row.
  in_row = $Rows[$row_b]
  num_b = if $in_row == 1 then 1 else $in_row - 1
  #  Leave only a single peg in the row. Not a very strong strategy,
  #+ but probably a bit better than totally random.
  $Rows[$row_b] -= $num_b
  print "Bot:  Removing from row $row_b ... "
  print "      $num_b $(if $num_b == 1 then 'peg' else 'pegs')."
  display
  if $Rows[0] == 1
    end $WON "Bot wins!"

# ================================================== #
instructions     # If human player needs them . . .
tput bold        # Bold characters for easier viewing.
display          # Show game board.
while True       # Main loop: alternate human and bot turns.
  player_move
  bot_move
# ================================================== #
```

```output
Welcome to the game of Nim.

Do you need instructions? (y/n) n

1:    | | | | | 
2:     | | | | 
3:      | | | 
4:       | | 
5:        | 
      15 pegs remaining.
      

Your move:
Which row? 9
Invalid row number!
Which row? 2
Remove how many? 6
Cannot remove 6!
Remove how many? 4

1:    | | | | | 
2:     
3:      | | | 
4:       | | 
5:        | 
      11 pegs remaining.
      

Bot:  Removing from row 1 ... 
      4 pegs.

1:    | 
2:     
3:      | | | 
4:       | | 
5:        | 
      7 pegs remaining.
      

Your move:
Which row? 4
Remove how many? 2

1:    | 
2:     
3:      | | | 
4:       
5:        | 
      5 pegs remaining.
      

Bot:  Removing from row 1 ... 
      1 peg.

1:    
2:     
3:      | | | 
4:       
5:        | 
      4 pegs remaining.
      

Your move:
Which row? 3
Remove how many? 3

1:    
2:     
3:      
4:       
5:        | 
      1 peg remaining.
      Game over.

      Human wins!
      Congratulations!

```

## A command-line stopwatch
<!-- help: abs-sw sw.sh stopwatch laps splits stty cbreak getkey termios -->

Pádraig Brady's stopwatch: Space starts it and marks laps, `?` shows a
split, Ctrl-C stops it. It reads no keys itself: it uses `stty` to make
Space the "quit" key and `?` the end-of-file key, and times `cat` with
`/usr/bin/time` between them, telling the keys apart by how `cat` ended.

### Example A-43. A command-line stopwatch (`sw.sh`)

```bash no-run
#!/bin/sh
# sw.sh
# A command-line Stopwatch

# Author: Pádraig Brady
#    http://www.pixelbeat.org/scripts/sw
#    (Minor reformatting by ABS Guide author.)
#    Used in ABS Guide with script author's permission.
# Notes:
#    This script starts a few processes per lap, in addition to
#    the shell loop processing, so the assumption is made that
#    this takes an insignificant amount of time compared to
#    the response time of humans (~.1s) (or the keyboard
#    interrupt rate (~.05s)).
#    '?' for splits must be entered twice if characters
#    (erroneously) entered before it (on the same line).
#    '?' since not generating a signal may be slightly delayed
#    on heavily loaded systems.
#    Lap timings on ubuntu may be slightly delayed due to:
#    https://bugs.launchpad.net/bugs/62511
# Changes:
#    V1.0, 23 Aug 2005, Initial release
#    V1.1, 26 Jul 2007, Allow both splits and laps from single invocation.
#                       Only start timer after a key is pressed.
#                       Indicate lap number
#                       Cache programs at startup so there is less error
#                       due to startup delays.
#    V1.2, 01 Aug 2007, Work around `date` commands that don't have
#                       nanoseconds.
#                       Use stty to change interrupt keys to space for
#                       laps etc.
#                       Ignore other input as it causes problems.
#    V1.3, 01 Aug 2007, Testing release.
#    V1.4, 02 Aug 2007, Various tweaks to get working under ubuntu
#                       and Mac OS X.
#    V1.5, 27 Jun 2008, set LANG=C as got vague bug report about it.

export LANG=C

ulimit -c 0   # No coredumps from SIGQUIT.
trap '' TSTP  # Ignore Ctrl-Z just in case.
save_tty=`stty -g` && trap "stty $save_tty" EXIT  # Restore tty on exit.
stty quit ' ' # Space for laps rather than Ctrl-\.
stty eof  '?' # ? for splits rather than Ctrl-D.
stty -echo    # Don't echo input.

cache_progs() {
    stty > /dev/null
    date > /dev/null
    grep . < /dev/null
    (echo "import time" | python) 2> /dev/null
    bc < /dev/null
    sed '' < /dev/null
    printf '1' > /dev/null
    /usr/bin/time false 2> /dev/null
    cat < /dev/null
}
cache_progs   # To minimise startup delay.

date +%s.%N | grep -qF 'N' && use_python=1 # If `date` lacks nanoseconds.
now() {
    if [ "$use_python" ]; then
        echo "import time; print time.time()" 2>/dev/null | python
    else
        printf "%.2f" `date +%s.%N`
    fi
}

fmt_seconds() {
    seconds=$1
    mins=`echo $seconds/60 | bc`
    if [ "$mins" != "0" ]; then
        seconds=`echo "$seconds - ($mins*60)" | bc`
        echo "$mins:$seconds"
    else
        echo "$seconds"
    fi
}

total() {
    end=`now`
    total=`echo "$end - $start" | bc`
    fmt_seconds $total
}

stop() {
    [ "$lapped" ] && lap "$laptime" "display"
    total
    exit
}

lap() {
    laptime=`echo "$1" | sed -n 's/.*real[^0-9.]*\(.*\)/\1/p'`
    [ ! "$laptime" -o "$laptime" = "0.00" ] && return
    # Signals too frequent.
    laptotal=`echo $laptime+0$laptotal | bc`
    if [ "$2" = "display" ]; then
        lapcount=`echo 0$lapcount+1 | bc`
        laptime=`fmt_seconds $laptotal`
        echo $laptime "($lapcount)"
        lapped="true"
        laptotal="0"
    fi
}

echo -n "Space for lap | ? for split | Ctrl-C to stop | Space to start...">&2

while true; do
    trap true INT QUIT  # Set signal handlers.
    laptime=`/usr/bin/time -p 2>&1 cat >/dev/null`
    ret=$?
    trap '' INT QUIT    # Ignore signals within this script.
    if [ $ret -eq 1 -o $ret -eq 2 -o $ret -eq 130 ]; then # SIGINT = stop
        [ ! "$start" ] && { echo >&2; exit; }
        stop
    elif [ $ret -eq 3 -o $ret -eq 131 ]; then             # SIGQUIT = lap
        if [ ! "$start" ]; then
            start=`now` || exit 1
            echo >&2
            continue
        fi
        lap "$laptime" "display"
    else                # eof = split
        [ ! "$start" ] && continue
        total
        lap "$laptime"  # Update laptotal.
    fi
done

exit $?
```

tessh reads the keys directly. The stopwatch is a function given the keys
and a clock, so here it runs on canned keys and a canned clock, and the
arithmetic `bc` did is Python's floats. (Minutes show as `1:09.75`, where
the original's `bc` gives `1:9.75`.)

```tessh
import sys
# sw.sh: a command-line stopwatch.
# Author: Pádraig Brady, http://www.pixelbeat.org/scripts/sw

def fmt_seconds seconds
  mins = int ($seconds // 60)
  if $mins
    "$mins:${$seconds - $mins * 60:05.2f}"
  else
    "${seconds:.2f}"

def stopwatch keys now
  # keys: the keys pressed, one at a time, ending at Ctrl-C;
  # now: a function giving the time in seconds.
  print "Space for lap | ? for split | Ctrl-C to stop | Space to start..." file=$sys.stderr
  start = None
  last = None       # When the last key came.
  laptotal = 0      # The lap so far, before the last split.
  lapcount = 0
  lapped = False
  for key in $keys
    t = (now)
    match $key
      case " "                    # Space = lap
        if $start is None
          start = $t
        else
          lapcount += 1
          print (fmt_seconds ($laptotal + $t - $last)) "($lapcount)"
          lapped = True
          laptotal = 0
      case "?"                    # ? = split
        if $start is None
          continue
        print (fmt_seconds ($t - $start))
        laptotal += $t - $last
    last = $t
  # Ctrl-C = stop
  if $start is None
    return
  t = (now)
  if $lapped
    lapcount += 1
    print (fmt_seconds ($laptotal + $t - $last)) "($lapcount)"
  print (fmt_seconds ($t - $start))

# (Canned: the keys pressed, and the clock at each one and at the end.)
clock = iter [3.0 15.25 20.5 31.0 40.0 100.75 102.5]
stopwatch [" " " " "?" " " "?" " "] (fn [] (next $clock))
```

```output
12.25 (1)
17.50
15.75 (2)
37.00
1:09.75 (3)
1.75 (4)
1:39.50
```

The real thing reads one key at a time from the terminal: `tty.setcbreak`
turns off line editing and echo, as `stty -echo -icanon` would, and
`termios` puts the terminal back afterwards. Ctrl-C arrives as a
`KeyboardInterrupt`, which ends the keys. Run after the definition above:

```tessh no-run
import sys tty termios time
def keys
  fd = (sys.stdin.fileno)
  saved = termios.tcgetattr $fd
  tty.setcbreak $fd               # One key at a time, not echoed.
  try
    while True
      yield (sys.stdin.read 1)
  except KeyboardInterrupt        # Ctrl-C stops.
    pass
  finally
    termios.tcsetattr $fd $termios.TCSADRAIN $saved   # Restore tty on exit.
stopwatch (keys) $time.monotonic
```

## An all-purpose homework solution
<!-- help: abs-homework homework.sh obfuscated bell -->

M. Leo Cooper's answer to every scripting assignment prints a hidden
message one letter a second, ringing the bell with each. It's also, as its
author says, an example of the obfuscated scripts a sysadmin meets.

### Example A-44. An all-purpose shell scripting homework assignment solution (`homework.sh`)

```bash no-run
#!/bin/bash
#  homework.sh: All-purpose homework assignment solution.
#  Author: M. Leo Cooper
#  If you substitute your own name as author, then it is plagiarism,
#+ possibly a lesser sin than cheating on your homework!
#  License: Public Domain

#  This script may be turned in to your instructor
#+ in fulfillment of ALL shell scripting homework assignments.
#  It's sparsely commented, but you, the student, can easily remedy that.
#  The script author repudiates all responsibility!

DLA=1
P1=2
P2=4
P3=7
PP1=0
PP2=8
MAXL=9
E_LZY=99

declare -a L
L[0]="3 4 0 17 29 8 13 18 19 17 20 2 19 14 17 28"
L[1]="8 29 12 14 18 19 29 4 12 15 7 0 19 8 2 0 11 11 24 29 17 4 6 17 4 19"
L[2]="29 19 7 0 19 29 8 29 7 0 21 4 29 13 4 6 11 4 2 19 4 3"
L[3]="19 14 29 2 14 12 15 11 4 19 4 29 19 7 8 18 29"
L[4]="18 2 7 14 14 11 22 14 17 10 29 0 18 18 8 6 13 12 4 13 19 26"
L[5]="15 11 4 0 18 4 29 0 2 2 4 15 19 29 12 24 29 7 20 12 1 11 4 29"
L[6]="4 23 2 20 18 4 29 14 5 29 4 6 17 4 6 8 14 20 18 29"
L[7]="11 0 25 8 13 4 18 18 27"
L[8]="0 13 3 29 6 17 0 3 4 29 12 4 29 0 2 2 14 17 3 8 13 6 11 24 26"
L[9]="19 7 0 13 10 29 24 14 20 26"

declare -a \
alph=( A B C D E F G H I J K L M N O P Q R S T U V W X Y Z . , : ' ' )


pt_lt ()
{
  echo -n "${alph[$1]}"
  echo -n -e "\a"
  sleep $DLA
}

b_r ()
{
 echo -e '\E[31;48m\033[1m'
}

cr ()
{
 echo -e "\a"
 sleep $DLA
}

restore ()
{
  echo -e '\033[0m'            # Bold off.
  tput sgr0                    # Normal.
}


p_l ()
{
  for ltr in $1
  do
    pt_lt "$ltr"
  done
}

# ----------------------
b_r

for i in $(seq 0 $MAXL)
do
  p_l "${L[i]}"
  if [[ "$i" -eq "$P1" || "$i" -eq "$P2" || "$i" -eq "$P3" ]]
  then
    cr
  elif [[ "$i" -eq "$PP1" || "$i" -eq "$PP2" ]]
  then
    cr; cr
  fi
done

restore
# ----------------------

echo

exit $E_LZY

#  A typical example of an obfuscated script that is difficult
#+ to understand, and frustrating to maintain.
#  In your career as a sysadmin, you'll run into these critters
#+ all too often.
```

The letters are indexes into the alphabet; `.split` and `int` read them.
Here the delay is 0 (the original takes three minutes) and the bell and
colors are left out when the output isn't a terminal. The line breaks come
after the lines numbered in `P` and `PP`, lists here where bash had five
variables.

```tessh
import sys time
#  homework.sh: All-purpose homework assignment solution.
#  Author: M. Leo Cooper. License: Public Domain.

DLA = 0          # (1 second a letter in the original: none here.)
P = [2 4 7]      # (P1, P2, P3 in the original)
PP = [0 8]       # (PP1, PP2)
MAXL = 9
E_LZY = 99

L = [
  "3 4 0 17 29 8 13 18 19 17 20 2 19 14 17 28"
  "8 29 12 14 18 19 29 4 12 15 7 0 19 8 2 0 11 11 24 29 17 4 6 17 4 19"
  "29 19 7 0 19 29 8 29 7 0 21 4 29 13 4 6 11 4 2 19 4 3"
  "19 14 29 2 14 12 15 11 4 19 4 29 19 7 8 18 29"
  "18 2 7 14 14 11 22 14 17 10 29 0 18 18 8 6 13 12 4 13 19 26"
  "15 11 4 0 18 4 29 0 2 2 4 15 19 29 12 24 29 7 20 12 1 11 4 29"
  "4 23 2 20 18 4 29 14 5 29 4 6 17 4 6 8 14 20 18 29"
  "11 0 25 8 13 4 18 18 27"
  "0 13 3 29 6 17 0 3 4 29 12 4 29 0 2 2 14 17 3 8 13 6 11 24 26"
  "19 7 0 13 10 29 24 14 20 26"
]

alph = (list "ABCDEFGHIJKLMNOPQRSTUVWXYZ.,: ")

tty = (sys.stdout.isatty)
def esc s = if $tty then $s else ""   # (Bell and colors on a terminal only.)

def pt_lt n
  print $alph[$n] (esc "\a") sep="" end="" flush=True
  time.sleep $DLA

def b_r = print (esc "\033[31;48m\033[1m")
def cr
  print (esc "\a")
  time.sleep $DLA
def restore = print (esc "\033[0m")   # Bold off.
def p_l line
  for ltr in ($line.split)
    pt_lt (int $ltr)

# ----------------------
b_r
for i in (range ($MAXL + 1))
  p_l $L[$i]
  if $i in $P
    cr
  elif $i in $PP
    cr ; cr
restore
# ----------------------
print
exit $E_LZY
```

```output

DEAR INSTRUCTOR:

I MOST EMPHATICALLY REGRET THAT I HAVE NEGLECTED
TO COMPLETE THIS SCHOOLWORK ASSIGNMENT.
PLEASE ACCEPT MY HUMBLE EXCUSE OF EGREGIOUS LAZINESS,
AND GRADE ME ACCORDINGLY.

THANK YOU.

```

## The Knight's Tour
<!-- help: abs-ktour ktour.sh knights-tour warnsdorff chess -->

Mendel Cooper's Knight's Tour moves a knight to every square of the board
once. It uses Warnsdorff's rule, always moving to the square with the
fewest onward moves, or with `stupid` the first free square (which soon
gets stuck). The start square is a number 0-63 or a square such as `h3`.

### Example A-45. The Knight's Tour (`ktour.sh`)

```bash no-run
#!/bin/bash
# ktour.sh

# author: mendel cooper
# reldate: 12 Jan 2009
# license: public domain
# (Not much sense GPLing something that's pretty much in the common
#+ domain anyhow.)

###################################################################
#             The Knight's Tour, a classic problem.               #
#             =====================================               #
#  The knight must move onto every square of the chess board,     #
#  but cannot revisit any square he has already visited.          #
#                                                                 #
#  And just why is Sir Knight unwelcome for a return visit?       #
#  Could it be that he has a habit of partying into the wee hours #
#+ of the morning?                                                #
#  Possibly he leaves pizza crusts in the bed, empty beer bottles #
#+ all over the floor, and clogs the plumbing. . . .              #
#                                                                 #
#  -------------------------------------------------------------  #
#                                                                 #
#  Usage: ktour.sh [start-square] [stupid]                        #
#                                                                 #
#  Note that start-square can be a square number                  #
#+ in the range 0 - 63 ... or                                     #
#  a square designator in conventional chess notation,            #
#  such as a1, f5, h3, etc.                                       #
#                                                                 #
#  If start-square-number not supplied,                           #
#+ then starts on a random square somewhere on the board.         #
#                                                                 #
# "stupid" as second parameter sets the stupid strategy.          #
#                                                                 #
#  Examples:                                                      #
#  ktour.sh 23          starts on square #23 (h3)                 #
#  ktour.sh g6 stupid   starts on square #46,                     #
#                       using "stupid" (non-Warnsdorff) strategy. #
###################################################################

DEBUG=      # Set this to echo debugging info to stdout.
SUCCESS=0
FAIL=99
BADMOVE=-999
FAILURE=1
LINELEN=21  # How many moves to display per line.
# ---------------------------------------- #
# Board array params
ROWS=8   # 8 x 8 board.
COLS=8
let "SQUARES = $ROWS * $COLS"
let "MAX = $SQUARES - 1"
MIN=0
# 64 squares on board, indexed from 0 to 63.

VISITED=1
UNVISITED=-1
UNVSYM="##"
# ---------------------------------------- #
# Global variables.
startpos=    # Starting position (square #, 0 - 63).
currpos=     # Current position.
movenum=     # Move number.
CRITPOS=37   # Have to patch for f5 starting position!

declare -i board
# Use a one-dimensional array to simulate a two-dimensional one.
# This can make life difficult and result in ugly kludges; see below.
declare -i moves  # Offsets from current knight position.


initialize_board ()
{
  local idx

  for idx in {0..63}
  do
    board[$idx]=$UNVISITED
  done
}



print_board ()
{
  local idx

  echo "    _____________________________________"
  for row in {7..0}               #  Reverse order of rows ...
  do                              #+ so it prints in chessboard order.
    let "rownum = $row + 1"       #  Start numbering rows at 1.
    echo -n "$rownum  |"          #  Mark board edge with border and
    for column in {0..7}          #+ "algebraic notation."
    do
      let "idx = $ROWS*$row + $column"
      if [ ${board[idx]} -eq $UNVISITED ]
      then
        echo -n "$UNVSYM   "      ##
      else                        # Mark square with move number.
        printf "%02d " "${board[idx]}"; echo -n "  "
      fi
    done
    echo -e -n "\b\b\b|"  # \b is a backspace.
    echo                  # -e enables echoing escaped chars.
  done

  echo "    -------------------------------------"
  echo "     a    b    c    d    e    f    g    h"
}



failure()
{ # Whine, then bail out.
  echo
  print_board
  echo
  echo    "   Waah!!! Ran out of squares to move to!"
  echo -n "   Knight's Tour attempt ended"
  echo    " on $(to_algebraic $currpos) [square #$currpos]"
  echo    "   after just $movenum moves!"
  echo
  exit $FAIL
}



xlat_coords ()   #  Translate x/y coordinates to board position
{                #+ (board-array element #).
  #  For user input of starting board position as x/y coords.
  #  This function not used in initial release of ktour.sh.
  #  May be used in an updated version, for compatibility with
  #+ standard implementation of the Knight's Tour in C, Python, etc.
  if [ -z "$1" -o -z "$2" ]
  then
    return $FAIL
  fi

  local xc=$1
  local yc=$2

  let "board_index = $xc * $ROWS + yc"

  if [ $board_index -lt $MIN -o $board_index -gt $MAX ]
  then
    return $FAIL    # Strayed off the board!
  else
    return $board_index
  fi
}



to_algebraic ()   #  Translate board position (board-array element #)
{                 #+ to standard algebraic notation used by chess players.
  if [ -z "$1" ]
  then
    return $FAIL
  fi

  local element_no=$1   # Numerical board position.
  local col_arr=( a b c d e f g h )
  local row_arr=( 1 2 3 4 5 6 7 8 )

  let "row_no = $element_no / $ROWS"
  let "col_no = $element_no % $ROWS"
  t1=${col_arr[col_no]}; t2=${row_arr[row_no]}
  local apos=$t1$t2   # Concatenate.
  echo $apos
}



from_algebraic ()   #  Translate standard algebraic chess notation
{                   #+ to numerical board position (board-array element #).
                    #  Or recognize numerical input & return it unchanged.
  if [ -z "$1" ]
  then
    return $FAIL
  fi   # If no command-line arg, then will default to random start pos.

  local ix
  local ix_count=0
  local b_index     # Board index [0-63]
  local alpos="$1"

  arow=${alpos:0:1} # position = 0, length = 1
  acol=${alpos:1:1}

  if [[ $arow =~ [[:digit:]] ]]   #  Numerical input?
  then       #  POSIX char class
    if [[ $acol =~ [[:alpha:]] ]] # Number followed by a letter? Illegal!
      then return $FAIL
    else if [ $alpos -gt $MAX ]   # Off board?
      then return $FAIL
    else return $alpos            #  Return digit(s) unchanged . . .
      fi                          #+ if within range.
    fi
  fi

  if [[ $acol -eq $MIN || $acol -gt $ROWS ]]
  then        # Outside of range 1 - 8?
    return $FAIL
  fi

  for ix in a b c d e f g h
  do  # Convert column letter to column number.
   if [ "$arow" = "$ix" ]
   then
     break
   fi
  ((ix_count++))    # Find index count.
  done

  ((acol--))        # Decrementing converts to zero-based array.
  let "b_index = $ix_count + $acol * $ROWS"

  if [ $b_index -gt $MAX ]   # Off board?
  then
    return $FAIL
  fi
    
  return $b_index

}


generate_moves ()   #  Calculate all valid knight moves,
{                   #+ relative to current position ($1),
                    #+ and store in ${moves} array.
  local kt_hop=1    #  One square  :: short leg of knight move.
  local kt_skip=2   #  Two squares :: long leg  of knight move.
  local valmov=0    #  Valid moves.
  local row_pos; let "row_pos = $1 % $COLS"


  let "move1 = -$kt_skip + $ROWS"           # 2 sideways to-the-left,  1 up
    if [[ `expr $row_pos - $kt_skip` -lt $MIN ]]   # An ugly, ugly kludge!
    then                                           # Can't move off board.
      move1=$BADMOVE                               # Not even temporarily.
    else
      ((valmov++))
    fi
  let "move2 = -$kt_hop + $kt_skip * $ROWS" # 1 sideways to-the-left,  2 up
    if [[ `expr $row_pos - $kt_hop` -lt $MIN ]]    # Kludge continued ...
    then
      move2=$BADMOVE
    else
      ((valmov++))
    fi
  let "move3 =  $kt_hop + $kt_skip * $ROWS" # 1 sideways to-the-right, 2 up
    if [[ `expr $row_pos + $kt_hop` -ge $COLS ]]
    then
      move3=$BADMOVE
    else
      ((valmov++))
    fi
  let "move4 =  $kt_skip + $ROWS"           # 2 sideways to-the-right, 1 up
    if [[ `expr $row_pos + $kt_skip` -ge $COLS ]]
    then
      move4=$BADMOVE
    else
      ((valmov++))
    fi
  let "move5 =  $kt_skip - $ROWS"           # 2 sideways to-the-right, 1 dn
    if [[ `expr $row_pos + $kt_skip` -ge $COLS ]]
    then
      move5=$BADMOVE
    else
      ((valmov++))
    fi
  let "move6 =  $kt_hop - $kt_skip * $ROWS" # 1 sideways to-the-right, 2 dn
    if [[ `expr $row_pos + $kt_hop` -ge $COLS ]]
    then
      move6=$BADMOVE
    else
      ((valmov++))
    fi
  let "move7 = -$kt_hop - $kt_skip * $ROWS" # 1 sideways to-the-left,  2 dn
    if [[ `expr $row_pos - $kt_hop` -lt $MIN ]]
    then
      move7=$BADMOVE
    else
      ((valmov++))
    fi
  let "move8 = -$kt_skip - $ROWS"           # 2 sideways to-the-left,  1 dn
    if [[ `expr $row_pos - $kt_skip` -lt $MIN ]]
    then
      move8=$BADMOVE
    else
      ((valmov++))
    fi   # There must be a better way to do this.

  local m=( $valmov $move1 $move2 $move3 $move4 $move5 $move6 $move7 $move8 )
  # ${moves[0]} = number of valid moves.
  # ${moves[1]} ... ${moves[8]} = possible moves.
  echo "${m[*]}"    # Elements of array to stdout for capture in a var.

}



is_on_board ()  # Is position actually on the board?
{
  if [[ "$1" -lt "$MIN" || "$1" -gt "$MAX" ]]
  then
    return $FAILURE
  else
    return $SUCCESS
  fi
}



do_move ()      # Move the knight!
{
  local valid_moves=0
  local aapos
  currposl="$1"
  lmin=$ROWS
  iex=0
  squarel=
  mpm=
  mov=
  declare -a p_moves

  ########################## DECIDE-MOVE #############################
  if [ $startpos -ne $CRITPOS ]
  then   # CRITPOS = square #37
    decide_move
  else                     # Needs a special patch for startpos=37 !!!
    decide_move_patched    # Why this particular move and no other ???
  fi
  ####################################################################

  (( ++movenum ))          # Increment move count.
  let "square = $currposl + ${moves[iex]}"

  ##################    DEBUG    ###############
  if [ "$DEBUG" ]
    then debug   # Echo debugging information.
  fi
  ##############################################

  if [[ "$square" -gt $MAX || "$square" -lt $MIN ||
        ${board[square]} -ne $UNVISITED ]]
  then
    (( --movenum ))              #  Decrement move count,
    echo "RAN OUT OF SQUARES!!!" #+ since previous one was invalid.
    return $FAIL
  fi

  board[square]=$movenum
  currpos=$square       # Update current position.
  ((valid_moves++));    # moves[0]=$valid_moves
  aapos=$(to_algebraic $square)
  echo -n "$aapos "
  test $(( $Moves % $LINELEN )) -eq 0 && echo
  # Print LINELEN=21 moves per line. A valid tour shows 3 complete lines.
  return $valid_moves   # Found a square to move to!
}



do_move_stupid()   #  Dingbat algorithm,
{                  #+ courtesy of script author, *not* Warnsdorff.
  local valid_moves=0
  local movloc
  local squareloc
  local aapos
  local cposloc="$1"

  for movloc in {1..8}
  do       # Move to first-found unvisited square.
    let "squareloc = $cposloc + ${moves[movloc]}"
    is_on_board $squareloc
    if [ $? -eq $SUCCESS ] && [ ${board[squareloc]} -eq $UNVISITED ]
    then   # Add conditions to above if-test to improve algorithm.
      (( ++movenum ))
      board[squareloc]=$movenum
      currpos=$squareloc     # Update current position.
      ((valid_moves++));     # moves[0]=$valid_moves
      aapos=$(to_algebraic $squareloc)
      echo -n "$aapos "
      test $(( $Moves % $LINELEN )) -eq 0 && echo   # Print 21 moves/line.
      return $valid_moves    # Found a square to move to!
    fi
  done

  return $FAIL
  #  If no square found in all 8 loop iterations,
  #+ then Knight's Tour attempt ends in failure.

  #  Dingbat algorithm will typically fail after about 30 - 40 moves,
  #+ but executes _much_ faster than Warnsdorff's in do_move() function.
}



decide_move ()         #  Which move will we make?
{                      #  But, fails on startpos=37 !!!
  for mov in {1..8}
  do
    let "squarel = $currposl + ${moves[mov]}"
    is_on_board $squarel
    if [[ $? -eq $SUCCESS && ${board[squarel]} -eq $UNVISITED ]]
    then   #  Find accessible square with least possible future moves.
           #  This is Warnsdorff's algorithm.
           #  What happens is that the knight wanders toward the outer edge
           #+ of the board, then pretty much spirals inward.
           #  Given two or more possible moves with same value of
           #+ least-possible-future-moves, this implementation chooses
           #+ the _first_ of those moves.
           #  This means that there is not necessarily a unique solution
           #+ for any given starting position.

      possible_moves $squarel
      mpm=$?
      p_moves[mov]=$mpm
      
      if [ $mpm -lt $lmin ]  # If less than previous minimum ...
      then #     ^^
        lmin=$mpm            # Update minimum.
        iex=$mov             # Save index.
      fi

    fi
  done
}



decide_move_patched ()         #  Decide which move to make,
{  #        ^^^^^^^            #+ but only if startpos=37 !!!
  for mov in {1..8}
  do
    let "squarel = $currposl + ${moves[mov]}"
    is_on_board $squarel
    if [[ $? -eq $SUCCESS && ${board[squarel]} -eq $UNVISITED ]]
    then
      possible_moves $squarel
      mpm=$?
      p_moves[mov]=$mpm
      
      if [ $mpm -le $lmin ]  # If less-than-or equal to prev. minimum!
      then #     ^^
        lmin=$mpm
        iex=$mov
      fi

    fi
  done                       # There has to be a better way to do this.
}



possible_moves ()            #  Calculate number of possible moves,
{                            #+ given the current position.

  if [ -z "$1" ]
  then
    return $FAIL
  fi

  local curr_pos=$1
  local valid_movl=0
  local icx=0
  local movl
  local sq
  declare -a movesloc

  movesloc=( $(generate_moves $curr_pos) )

  for movl in {1..8}
  do
    let "sq = $curr_pos + ${movesloc[movl]}"
    is_on_board $sq
    if [ $? -eq $SUCCESS ] && [ ${board[sq]} -eq $UNVISITED ]
    then
      ((valid_movl++));
    fi
  done

  return $valid_movl         # Found a square to move to!
}


strategy ()
{
  echo

  if [ -n "$STUPID" ]
  then
    for Moves in {1..63}
    do
      cposl=$1
      moves=( $(generate_moves $currpos) )
      do_move_stupid "$currpos"
      if [ $? -eq $FAIL ]
      then
        failure
      fi
      done
  fi

  #  Don't need an "else" clause here,
  #+ because Stupid Strategy will always fail and exit!
  for Moves in {1..63}
  do
    cposl=$1
    moves=( $(generate_moves $currpos) )
    do_move "$currpos"
    if [ $? -eq $FAIL ]
    then
      failure
    fi

  done
        #  Could have condensed above two do-loops into a single one,
  echo  #+ but this would have slowed execution.

  print_board
  echo
  echo "Knight's Tour ends on $(to_algebraic $currpos) [square #$currpos]."
  return $SUCCESS
}

debug ()
{       # Enable this by setting DEBUG=1 near beginning of script.
  local n

  echo "================================="
  echo "  At move number  $movenum:"
  echo " *** possible moves = $mpm ***"
# echo "### square = $square ###"
  echo "lmin = $lmin"
  echo "${moves[@]}"

  for n in {1..8}
  do
    echo -n "($n):${p_moves[n]} "
  done

  echo
  echo "iex = $iex :: moves[iex] = ${moves[iex]}"
  echo "square = $square"
  echo "================================="
  echo
} # Gives pretty complete status after ea. move.



# =============================================================== #
# int main () {
from_algebraic "$1"
startpos=$?
if [ "$startpos" -eq "$FAIL" ]          # Okay even if no $1.
then   #         ^^^^^^^^^^^              Okay even if input -lt 0.
  echo "No starting square specified (or illegal input)."
  let "startpos = $RANDOM % $SQUARES"   # 0 - 63 permissable range.
fi


if [ "$2" = "stupid" ]
then
  STUPID=1
  echo -n "     ### Stupid Strategy ###"
else
  STUPID=''
  echo -n "  *** Warnsdorff's Algorithm ***"
fi


initialize_board

movenum=0
board[startpos]=$movenum   # Mark each board square with move number.
currpos=$startpos
algpos=$(to_algebraic $startpos)

echo; echo "Starting from $algpos [square #$startpos] ..."; echo
echo -n "Moves:"

strategy "$currpos"

echo

exit 0   # return 0;

# }      # End of main() pseudo-function.
# =============================================================== #


# Exercises:
# ---------
#
# 1) Extend this example to a 10 x 10 board or larger.
# 2) Improve the "stupid strategy" by modifying the
#    do_move_stupid function.
#    Hint: Prevent straying into corner squares in early moves
#          (the exact opposite of Warnsdorff's algorithm!).
# 3) This script could stand considerable improvement and
#    streamlining, especially in the poorly-written
#    generate_moves() function
#    and in the DECIDE-MOVE patch in the do_move() function.
#    Must figure out why standard algorithm fails for startpos=37 ...
#+   but _not_ on any other, including symmetrical startpos=26.
#    Possibly, when calculating possible moves, counts the move back
#+   to the originating square. If so, it might be a relatively easy fix.
```

The board is a list of 64, as bash's array is. `generate_moves` is one
comprehension over the eight knight's hops in place of eight `let`s and
`if`s, still with the count of good moves first and `BADMOVE` for a hop
off the side, so the tours are the same as bash's, square for square.
`from_algebraic` is a `match` with a regex for `a1`…`h8`, giving `None`
for no or bad input where bash returned 99 as a status. Here it runs from
h3 (the sample argument), from f5 (the start square that needs the
patched rule), and stupidly from g6, which fails and exits.

```tessh
import random
# ktour.sh: The Knight's Tour, a classic problem.
# author: mendel cooper; reldate: 12 Jan 2009; license: public domain
#  The knight must move onto every square of the chess board,
#  but cannot revisit any square he has already visited.
#  Usage: ktour.sh [start-square] [stupid]
#  start-square: a square number 0 - 63, or a square such as a1, f5, h3;
#  random if not given. "stupid" sets the stupid (non-Warnsdorff) strategy.

DEBUG = False    # Set this to print debugging info.
FAIL = 99
BADMOVE = -999
LINELEN = 21     # How many moves to display per line.
ROWS = 8         # 8 x 8 board.
COLS = 8
SQUARES = $ROWS * $COLS
MAX = $SQUARES - 1
MIN = 0          # 64 squares on board, indexed from 0 to 63.
UNVISITED = -1
UNVSYM = "##"
CRITPOS = 37     # Have to patch for f5 starting position!

# A list of 64 simulates the two-dimensional board, as in bash.
board = [$UNVISITED] * $SQUARES
movenum = 0
startpos = 0
currpos = 0

def print_board
  print "    _____________________________________"
  for row in (range 7 -1 -1)      # Reverse order of rows, as on a chessboard.
    squares = $board[$ROWS * $row : $ROWS * ($row + 1)]
    cells = [(if $v == $UNVISITED then $UNVSYM else "${v:02d}") for v in $squares]
    print "$($row + 1)  |$('   '.join $cells)|"
  print "    -------------------------------------"
  print "     a    b    c    d    e    f    g    h"

def failure
  # Whine, then bail out.
  print ; print_board ; print
  print "   Waah!!! Ran out of squares to move to!"
  print "   Knight's Tour attempt ended on $(to_algebraic $currpos) [square #$currpos]"
  print "   after just $movenum moves!"
  print
  exit $FAIL

def to_algebraic n                # Board position to chess notation.
  "abcdefgh"[$n % $ROWS] + (str ($n // $ROWS + 1))

def from_algebraic alpos          # Chess notation (or a number) to a position.
  match $alpos
    case None | ""
      None                        # No argument: a random start.
    case _ if ($alpos.isdigit)    # Numerical input?
      if (int $alpos) > $MAX then None else (int $alpos)
    case _ if $alpos =~ re'^[a-h][1-8]$'
      ("abcdefgh".index $alpos[0]) + ((int $alpos[1]) - 1) * $ROWS
    case _
      None

def generate_moves pos            # All knight moves from pos; [0] counts the good ones.
  # [sideways, up] for each move; one that would leave the board's sides is BADMOVE.
  col = $pos % $COLS
  hops = [[-2 1] [-1 2] [1 2] [2 1] [2 -1] [1 -2] [-1 -2] [-2 -1]]
  moves = [(if 0 <= $col + $side < $COLS then $side + $up * $ROWS else $BADMOVE) for [side up] in $hops]
  [(len [$m for m in $moves if $m != $BADMOVE])] + $moves

def is_on_board pos = $MIN <= $pos <= $MAX
def free pos = (is_on_board $pos) and $board[$pos] == $UNVISITED

def possible_moves pos            # How many moves from pos.
  len [$m for m in (generate_moves $pos)[1:] if (free ($pos + $m))]

def decide_move moves patched
  # Warnsdorff's algorithm: the accessible square with the fewest
  # onward moves; the first of equals (the last, patched for f5).
  lmin = $ROWS
  iex = 0
  for mov in (range 1 9)
    square = $currpos + $moves[$mov]
    if (free $square)
      mpm = possible_moves $square
      if $mpm < $lmin or ($patched and $mpm == $lmin)
        lmin = $mpm
        iex = $mov
  if $DEBUG
    print "lmin = $lmin  iex = $iex :: moves[iex] = $moves[$iex]"
  $iex

def show_move square Moves
  print (to_algebraic $square) end=" "
  if $Moves % $LINELEN == 0       # Print LINELEN=21 moves per line.
    print

def do_move moves Moves           # Move the knight!
  global movenum currpos
  iex = decide_move $moves ($startpos == $CRITPOS)
  square = $currpos + $moves[$iex]
  if not (free $square)
    print "RAN OUT OF SQUARES!!!"
    return $FAIL
  movenum += 1
  $board[$square] = $movenum
  currpos = $square
  show_move $square $Moves

def do_move_stupid moves Moves    # Dingbat algorithm, *not* Warnsdorff:
  global movenum currpos          # move to the first-found unvisited square.
  for m in $moves[1:]
    if (free ($currpos + $m))
      movenum += 1
      currpos += $m
      $board[$currpos] = $movenum
      show_move $currpos $Moves
      return
  $FAIL

def strategy stupid
  print
  move = if $stupid then $do_move_stupid else $do_move
  for Moves in (range 1 64)
    if ($move (generate_moves $currpos) $Moves) == $FAIL
      failure
  print
  print_board
  print
  print "Knight's Tour ends on $(to_algebraic $currpos) [square #$currpos]."

def main args
  global startpos currpos movenum board
  startpos = from_algebraic (if $args then $args[0] else None)
  if $startpos is None
    print "No starting square specified (or illegal input)."
    startpos = random.randrange $SQUARES
  stupid = $args[1:] == [stupid]
  if $stupid
    print "     ### Stupid Strategy ###"
  else
    print "  *** Warnsdorff's Algorithm ***"
  board = [$UNVISITED] * $SQUARES
  movenum = 0
  $board[$startpos] = $movenum   # Mark each board square with move number.
  currpos = $startpos
  print "Starting from $(to_algebraic $startpos) [square #$startpos] ..." ; print
  print "Moves:" end=""
  strategy $stupid
  print

main ($argv[1:] or ["23"])          # (sample argument, when none is given)
main [f5]                         # (the patched start square)
main [g6 stupid]                  # (fails, and exits)
```

```output
  *** Warnsdorff's Algorithm ***
Starting from h3 [square #23] ...

Moves:
g1 e2 c1 a2 b4 a6 b8 d7 f8 h7 g5 f7 h8 g6 h4 g2 e1 c2 a1 b3 a5 
b7 d8 c6 a7 c8 e7 g8 h6 g4 h2 f3 e5 d3 c5 a4 b2 d1 f2 h1 g3 f1 
d2 b1 a3 c4 b6 a8 c7 e8 g7 h5 f6 e4 d6 f5 e3 d5 f4 e6 d4 b5 c3 

    _____________________________________
8  |48   07   26   23   50   09   28   13|
7  |25   22   49   08   27   12   51   10|
6  |06   47   24   55   60   53   14   29|
5  |21   62   35   58   33   56   11   52|
4  |36   05   46   61   54   59   30   15|
3  |45   20   63   34   57   32   41   00|
2  |04   37   18   43   02   39   16   31|
1  |19   44   03   38   17   42   01   40|
    -------------------------------------
     a    b    c    d    e    f    g    h

Knight's Tour ends on c3 [square #18].

  *** Warnsdorff's Algorithm ***
Starting from f5 [square #37] ...

Moves:
h4 g2 e1 c2 a1 b3 c1 a2 b4 a6 b8 d7 f8 h7 g5 h3 g1 f3 h2 f1 d2 
b1 a3 b5 a7 c8 b6 a8 c7 e8 g7 h5 g3 h1 f2 d1 e3 g4 h6 g8 f6 d5 
e7 g6 h8 f7 d6 e4 c3 e2 f4 d3 b2 a4 c5 e6 d4 c6 e5 c4 a5 b7 d8 

    _____________________________________
8  |28   11   26   63   30   13   40   45|
7  |25   62   29   12   43   46   31   14|
6  |10   27   58   47   56   41   44   39|
5  |61   24   55   42   59   00   15   32|
4  |54   09   60   57   48   51   38   01|
3  |23   06   49   52   37   18   33   16|
2  |08   53   04   21   50   35   02   19|
1  |05   22   07   36   03   20   17   34|
    -------------------------------------
     a    b    c    d    e    f    g    h

Knight's Tour ends on d8 [square #59].

     ### Stupid Strategy ###
Starting from g6 [square #46] ...

Moves:
e7 c8 d6 b7 d8 f7 h8 
    _____________________________________
8  |##   ##   02   05   ##   ##   ##   07|
7  |##   04   ##   ##   01   06   ##   ##|
6  |##   ##   ##   03   ##   ##   00   ##|
5  |##   ##   ##   ##   ##   ##   ##   ##|
4  |##   ##   ##   ##   ##   ##   ##   ##|
3  |##   ##   ##   ##   ##   ##   ##   ##|
2  |##   ##   ##   ##   ##   ##   ##   ##|
1  |##   ##   ##   ##   ##   ##   ##   ##|
    -------------------------------------
     a    b    c    d    e    f    g    h

   Waah!!! Ran out of squares to move to!
   Knight's Tour attempt ended on h8 [square #63]
   after just 7 moves!

```

## Magic squares
<!-- help: abs-msquare msquare.sh magic-square odd-order -->

Mendel Cooper's magic square generator, inspired by a C program by Kwon
Young Shin: every row, column and long diagonal of an odd-order square
adds up to the same number.

### Example A-46. Magic Squares (`msquare.sh`)

```bash no-run
#!/bin/bash
# msquare.sh
# Magic Square generator (odd-order squares only!)

# Author: mendel cooper
# reldate: 19 Jan. 2009
# License: Public Domain
# A C-program by the very talented Kwon Young Shin inspired this script.
#     http://user.chollian.net/~brainstm/MagicSquare.htm

# Definition: A "magic square" is a two-dimensional array
#             of integers in which all the rows, columns,
#             and *long* diagonals add up to the same number.
#             Being "square," the array has the same number
#             of rows and columns. That number is the "order."
# An example of a magic square of order 3 is:
#   8  1  6   
#   3  5  7   
#   4  9  2   
# All the rows, columns, and the two long diagonals add up to 15.


# Globals
EVEN=2
MAXSIZE=31   # 31 rows x 31 cols.
E_usage=90   # Invocation error.
dimension=
declare -i square

usage_message ()
{
  echo "Usage: $0 order"
  echo "   ... where \"order\" (square size) is an ODD integer"
  echo "       in the range 3 - 31."
  #  Actually works for squares up to order 159,
  #+ but large squares will not display pretty-printed in a term window.
  #  Try increasing MAXSIZE, above.
  exit $E_usage
}


calculate ()       # Here's where the actual work gets done.
{
  local row col index dimadj j k cell_val=1
  dimension=$1

  let "dimadj = $dimension * 3"; let "dimadj /= 2"   # x 1.5, then truncate.

  for ((j=0; j < dimension; j++))
  do
    for ((k=0; k < dimension; k++))
    do  # Calculate indices, then convert to 1-dim. array index.
        # Bash doesn't support multidimensional arrays. Pity.
      let "col = $k - $j + $dimadj"; let "col %= $dimension"
      let "row = $j * 2 - $k + $dimension"; let "row %= $dimension"
      let "index = $row*($dimension) + $col"
      square[$index]=cell_val; ((cell_val++))
    done
  done
}     # Plain math, visualization not required.


print_square ()               # Output square, one row at a time.
{
  local row col idx d1
  let "d1 = $dimension - 1"   # Adjust for zero-indexed array.
 
  for row in $(seq 0 $d1)
  do

    for col in $(seq 0 $d1)
    do
      let "idx = $row * $dimension + $col"
      printf "%3d " "${square[idx]}"; echo -n "  "
    done   # Displays up to 13th order neatly in 80-column term window.

    echo   # Newline after each row.
  done
}


#################################################
if [[ -z "$1" ]] || [[ "$1" -gt $MAXSIZE ]]
then
  usage_message
fi

let "test_even = $1 % $EVEN"
if [ $test_even -eq 0 ]
then           # Can't handle even-order squares.
  usage_message
fi

calculate $1
print_square   # echo "${square[@]}"   # DEBUG

exit $?
#################################################


# Exercises:
# ---------
# 1) Add a function to calculate the sum of each row, column,
#    and *long* diagonal. The sums must match.
#    This is the "magic constant" of that particular order square.
# 2) Have the print_square function auto-calculate how much space
#    to allot between square elements for optimized display.
#    This might require parameterizing the "printf" line.
# 3) Add appropriate functions for generating magic squares
#    with an *even* number of rows/columns.
#    This is non-trivial(!).
#    See the URL for Kwon Young Shin, above, for help.
```

bash has no two-dimensional arrays, so the original works out an index
into one long array; a list of lists holds the square here, as
`$square[$row][$col]`. (In bash, `square[$index]=cell_val` works without a
`$` only because `square` is declared `-i`: its values are arithmetic.)
`calculate` gives the square back rather than setting a global. Exercise 1,
the sums, is a few comprehensions: one value in the set means they agree.
Here it builds the sample order 5 and then 3.

```tessh
# msquare.sh: Magic Square generator (odd-order squares only!)
# Author: mendel cooper; reldate: 19 Jan. 2009; License: Public Domain
# A C-program by the very talented Kwon Young Shin inspired this script.
#   A magic square's rows, columns and long diagonals all add up to the
#   same number. Of order 3:   8 1 6 / 3 5 7 / 4 9 2   (all add up to 15)

EVEN = 2
MAXSIZE = 31     # 31 rows x 31 cols.
E_usage = 90     # Invocation error.

def usage_message
  print "Usage: $(Path $argv[0]).name order"
  print "   ... where \"order\" (square size) is an ODD integer"
  print "       in the range 3 - 31."
  exit $E_usage

def calculate dimension           # Here's where the actual work gets done.
  dimadj = $dimension * 3 // 2    # x 1.5, then truncate.
  square = [[0] * $dimension for _ in (range $dimension)]   # Rows of columns.
  cell_val = 1
  for j in (range $dimension)
    for k in (range $dimension)
      col = ($k - $j + $dimadj) % $dimension
      row = ($j * 2 - $k + $dimension) % $dimension
      $square[$row][$col] = $cell_val
      cell_val += 1
  $square                         # Plain math, visualization not required.

def print_square square           # Output square, one row at a time.
  for row in $square
    print ("".join ["${n:3d}   " for n in $row])

def main args
  if not $args or (int $args[0]) > $MAXSIZE or (int $args[0]) % $EVEN == 0
    usage_message                 # Can't handle even-order squares.
  square = calculate (int $args[0])
  print_square $square
  # (Exercise 1: the sums of the rows, columns and long diagonals.)
  n = len $square
  sums = [(sum $r) for r in $square] + [(sum $c) for c in (zip *$square)]
  sums += [(sum [$square[$i][$i] for i in (range $n)]), (sum [$square[$i][$n - 1 - $i] for i in (range $n)])]
  print "Magic constant: $(set $sums)"

main ($argv[1:] or ["5"])         # (sample argument, when none is given)
print
main ["3"]
```

```output
 17    24     1     8    15   
 23     5     7    14    16   
  4     6    13    20    22   
 10    12    19    21     3   
 11    18    25     2     9   
Magic constant: {65}

  8     1     6   
  3     5     7   
  4     9     2   
Magic constant: {15}
```

## Fifteen Puzzle
<!-- help: abs-fifteen fifteen.sh fifteen-puzzle sliding-puzzle -->

Antonio Macchi's Fifteen Puzzle: slide the numbered pieces into the space
until they read 1 to 15 in order.

### Example A-47. Fifteen Puzzle (`fifteen.sh`)

```bash no-run
#!/bin/bash
# fifteen.sh

# Classic "Fifteen Puzzle"
# Author: Antonio Macchi
# Lightly edited and commented by ABS Guide author.
# Used in ABS Guide with permission. (Thanks!)

#  The invention of the Fifteen Puzzle is attributed to either
#+ Sam Loyd or Noyes Palmer Chapman.
#  The puzzle was wildly popular in the late 19th-century.

#  Object: Rearrange the numbers so they read in order,
#+ from 1 - 15:   ________________
#                |  1   2   3   4 |
#                |  5   6   7   8 |
#                |  9  10  11  12 |
#                | 13  14  15     |
#                 ----------------


#######################
# Constants           #
  SQUARES=16          #
  FAIL=70             #
  E_PREMATURE_EXIT=80 #
#######################


########
# Data #
########

Puzzle=( 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 " " )


#############
# Functions #
#############

function swap
{
  local tmp

  tmp=${Puzzle[$1]}
  Puzzle[$1]=${Puzzle[$2]}
  Puzzle[$2]=$tmp
}


function Jumble
{ # Scramble the pieces at beginning of round.
  local i pos1 pos2

  for i in {1..100}
  do
    pos1=$(( $RANDOM % $SQUARES))
    pos2=$(( $RANDOM % $SQUARES ))
    swap $pos1 $pos2
  done
}


function PrintPuzzle
{
  local i1 i2 puzpos
  puzpos=0

  clear
  echo "Enter  quit  to exit."; echo   # Better that than Ctl-C.

  echo ",----.----.----.----."   # Top border.
  for i1 in {1..4}
  do
    for i2 in {1..4} 
    do
      printf "| %2s " "${Puzzle[$puzpos]}"
      (( puzpos++ ))
    done
    echo "|"                     # Right-side border.
    test $i1 = 4 || echo "+----+----+----+----+"
  done
  echo "'----'----'----'----'"   # Bottom border.
}


function GetNum
{ # Test for valid input.
  local puznum garbage

  while true
  do 
	  echo "Moves: $moves" # Also counts invalid moves.
    read -p "Number to move: " puznum garbage
      if [ "$puznum" = "quit" ]; then echo; exit $E_PREMATURE_EXIT; fi
    test -z "$puznum" -o -n "${puznum//[0-9]/}" && continue
    test $puznum -gt 0 -a $puznum -lt $SQUARES && break
  done
  return $puznum
}


function GetPosFromNum
{ # $1 = puzzle-number
  local puzpos

  for puzpos in {0..15}
  do
    test "${Puzzle[$puzpos]}" = "$1" && break
  done
  return $puzpos
}


function Move
{ # $1=Puzzle-pos
  test $1 -gt 3 && test "${Puzzle[$(( $1 - 4 ))]}" = " "\
       && swap $1 $(( $1 - 4 )) && return 0
  test $(( $1%4 )) -ne 3 && test "${Puzzle[$(( $1 + 1 ))]}" = " "\
       && swap $1 $(( $1 + 1 )) && return 0
  test $1 -lt 12 && test "${Puzzle[$(( $1 + 4 ))]}" = " "\
       && swap $1 $(( $1 + 4 )) && return 0
  test $(( $1%4 )) -ne 0 && test "${Puzzle[$(( $1 - 1 ))]}" = " " &&\
       swap $1 $(( $1 - 1 )) && return 0
  return 1
}


function Solved
{
  local pos

  for pos in {0..14}
  do
    test "${Puzzle[$pos]}" = $(( $pos + 1 )) || return $FAIL
    # Check whether number in each square = square number.
  done
  return 0   # Successful solution.
}


################### MAIN () #######################{
moves=0
Jumble

while true   # Loop continuously until puzzle solved.
do
  echo; echo
  PrintPuzzle
  echo
  while true
  do
    GetNum
    puznum=$?
    GetPosFromNum $puznum
    puzpos=$?
    ((moves++))
    Move $puzpos && break
  done
  Solved && break
done

echo;echo
PrintPuzzle
echo; echo "BRAVO!"; echo

exit 0
###################################################}

#  Exercise:
#  --------
#  Rewrite the script to display the letters A - O,
#+ rather than the numbers 1 - 15.
```

The puzzle is a list with `" "` for the space; `swap` is one tuple
assignment, `GetPosFromNum` is `.index`, and `Move` tries the four
neighbours from a list of conditions and positions rather than four
`test … && swap … && return` lines. `GetNum` keeps the first word of the
line, as `read puznum garbage` does. The canned games: one set up three
moves from solved (with a piece that can't move and a line that isn't a
number, which isn't counted), then a real jumble with `random` seeded, two
moves and `quit`. Note that 100 random swaps make an unsolvable puzzle
half the time, in bash and here: only an even number of swaps can be
undone by sliding.

```tessh
import sys random
# fifteen.sh: the classic "Fifteen Puzzle".
# Author: Antonio Macchi. Lightly edited and commented by ABS Guide author.
#  Object: Rearrange the numbers so they read in order, from 1 - 15,
#+ with the space last.

SQUARES = 16
E_PREMATURE_EXIT = 80

Puzzle = [*(range 1 16) " "]

def swap a b
  [$Puzzle[$a] $Puzzle[$b]] = [$Puzzle[$b] $Puzzle[$a]]

def Jumble                        # Scramble the pieces at beginning of round.
  for _ in (range 100)
    swap (random.randrange $SQUARES) (random.randrange $SQUARES)

def clear
  if (sys.stdout.isatty)
    \clear

def PrintPuzzle
  clear
  print "Enter  quit  to exit." ; print   # Better that than Ctl-C.
  print ",----.----.----.----."           # Top border.
  for i in (range 4)
    print (("".join ["| ${p!s:>2} " for p in $Puzzle[4 * $i : 4 * $i + 4]]) + "|")
    if $i != 3
      print "+----+----+----+----+"
  print "'----'----'----'----'"           # Bottom border.

def GetNum
  # Test for valid input.
  while True
    print "Moves: $moves"         # Also counts invalid moves.
    puznum = .split (read "Number to move: ")
    puznum = if $puznum then $puznum[0] else ""   # (The first word; the rest is garbage.)
    if $puznum == quit
      print
      exit $E_PREMATURE_EXIT
    if ($puznum.isdigit) and 0 < (int $puznum) < $SQUARES
      return (int $puznum)

def Move pos
  # Slide the piece at pos into the space next to it, if there is one.
  for [ok other] in [[($pos > 3) ($pos - 4)] [($pos % 4 != 3) ($pos + 1)]
                     [($pos < 12) ($pos + 4)] [($pos % 4 != 0) ($pos - 1)]]
    if $ok and $Puzzle[$other] == " "
      swap $pos $other
      return True
  False

def Solved = $Puzzle[:15] == (list (range 1 16))

def play
  global moves
  moves = 0
  while True                      # Loop continuously until puzzle solved.
    print ; print
    PrintPuzzle
    print
    while True
      puzpos = $Puzzle.index (GetNum)
      moves += 1
      if (Move $puzpos)
        break
    if (Solved)
      break
  print ; print
  PrintPuzzle
  print ; print "BRAVO!" ; print

# (The player, canned: the lines typed.)
typed = iter []
def read prompt
  line = next $typed
  print "$prompt$line"
  $line

# A puzzle a few moves from solved (set by hand, not jumbled):
# a number that won't move, some garbage, then the solution.
Puzzle = [1 2 3 4 5 6 7 8 9 10 " " 11 13 14 15 12]
typed = iter ["2" "x7" "11" "12 thanks"]
play

# A real round: jumbled, with $RANDOM seeded; two moves, then quit.
random.seed 15
Puzzle = [*(range 1 16) " "]
Jumble
typed = iter ["4" "1" "quit"]
play
```

```output


Enter  quit  to exit.

,----.----.----.----.
|  1 |  2 |  3 |  4 |
+----+----+----+----+
|  5 |  6 |  7 |  8 |
+----+----+----+----+
|  9 | 10 |    | 11 |
+----+----+----+----+
| 13 | 14 | 15 | 12 |
'----'----'----'----'

Moves: 0
Number to move: 2
Moves: 1
Number to move: x7
Moves: 1
Number to move: 11


Enter  quit  to exit.

,----.----.----.----.
|  1 |  2 |  3 |  4 |
+----+----+----+----+
|  5 |  6 |  7 |  8 |
+----+----+----+----+
|  9 | 10 | 11 |    |
+----+----+----+----+
| 13 | 14 | 15 | 12 |
'----'----'----'----'

Moves: 2
Number to move: 12 thanks


Enter  quit  to exit.

,----.----.----.----.
|  1 |  2 |  3 |  4 |
+----+----+----+----+
|  5 |  6 |  7 |  8 |
+----+----+----+----+
|  9 | 10 | 11 | 12 |
+----+----+----+----+
| 13 | 14 | 15 |    |
'----'----'----'----'

BRAVO!



Enter  quit  to exit.

,----.----.----.----.
| 10 |    | 14 | 15 |
+----+----+----+----+
|  1 |  4 |  2 | 12 |
+----+----+----+----+
|  9 |  6 |  5 |  8 |
+----+----+----+----+
| 13 |  7 | 11 |  3 |
'----'----'----'----'

Moves: 0
Number to move: 4


Enter  quit  to exit.

,----.----.----.----.
| 10 |  4 | 14 | 15 |
+----+----+----+----+
|  1 |    |  2 | 12 |
+----+----+----+----+
|  9 |  6 |  5 |  8 |
+----+----+----+----+
| 13 |  7 | 11 |  3 |
'----'----'----'----'

Moves: 1
Number to move: 1


Enter  quit  to exit.

,----.----.----.----.
| 10 |  4 | 14 | 15 |
+----+----+----+----+
|    |  1 |  2 | 12 |
+----+----+----+----+
|  9 |  6 |  5 |  8 |
+----+----+----+----+
| 13 |  7 | 11 |  3 |
'----'----'----'----'

Moves: 2
Number to move: quit

```

## The Towers of Hanoi, graphic version
<!-- help: abs-hanoi2 hanoi2.bash towers-of-hanoi recursion eval -->

Amit Singh's Towers of Hanoi (an earlier example in the ABS Guide), drawn
in ASCII with code by Antonio Macchi and edited by the ABS Guide's
author. `dohanoi` is the classic recursion; the rods are drawn after every
move.

### Example A-48. The Towers of Hanoi, graphic version (`hanoi2.bash`)

```bash no-run
#! /bin/bash
# The Towers Of Hanoi
# Original script (hanoi.bash) copyright (C) 2000 Amit Singh.
# All Rights Reserved.
# http://hanoi.kernelthread.com

#  hanoi2.bash
#  Version 2.00: modded for ASCII-graphic display.
#  Version 2.01: fixed no command-line param bug.
#  Uses code contributed by Antonio Macchi,
#+ with heavy editing by ABS Guide author.
#  This variant falls under the original copyright, see above.
#  Used in ABS Guide with Amit Singh's permission (thanks!).


###   Variables && sanity check   ###

E_NOPARAM=86
E_BADPARAM=87            # Illegal no. of disks passed to script.
E_NOEXIT=88

DISKS=${1:-$E_NOPARAM}   # Must specify how many disks.
Moves=0

MWIDTH=7
MARGIN=2
# Arbitrary "magic" constants; work okay for relatively small # of disks.
# BASEWIDTH=51   # Original code.
let "basewidth = $MWIDTH * $DISKS + $MARGIN"       # "Base" beneath rods.
# Above "algorithm" could likely stand improvement.

###   Display variables   ###
let "disks1 = $DISKS - 1"
let "spaces1 = $DISKS" 
let "spaces2 = 2 * $DISKS" 

let "lastmove_t = $DISKS - 1"                      # Final move?


declare -a Rod1 Rod2 Rod3

###   #########################   ###


function repeat  {  # $1=char $2=number of repetitions
  local n           # Repeat-print a character.
  
  for (( n=0; n<$2; n++ )); do
    echo -n "$1"
  done
}

function FromRod  {
  local rod summit weight sequence

  while true; do
    rod=$1
    test ${rod/[^123]/} || continue

    sequence=$(echo $(seq 0 $disks1 | tac))
    for summit in $sequence; do
      eval weight=\${Rod${rod}[$summit]}
      test $weight -ne 0 &&
           { echo "$rod $summit $weight"; return; }
    done
  done
}


function ToRod  { # $1=previous (FromRod) weight
  local rod firstfree weight sequence
  
  while true; do
    rod=$2
    test ${rod/[^123]} || continue

    sequence=$(echo $(seq 0 $disks1 | tac))
    for firstfree in $sequence; do
      eval weight=\${Rod${rod}[$firstfree]}
      test $weight -gt 0 && { (( firstfree++ )); break; }
    done
    test $weight -gt $1 -o $firstfree = 0 &&
         { echo "$rod $firstfree"; return; }
  done
}


function PrintRods  {
  local disk rod empty fill sp sequence


  repeat " " $spaces1
  echo -n "|"
  repeat " " $spaces2
  echo -n "|"
  repeat " " $spaces2
  echo "|"

  sequence=$(echo $(seq 0 $disks1 | tac))
  for disk in $sequence; do
    for rod in {1..3}; do
      eval empty=$(( $DISKS - (Rod${rod}[$disk] / 2) ))
      eval fill=\${Rod${rod}[$disk]}
      repeat " " $empty
      test $fill -gt 0 && repeat "*" $fill || echo -n "|"
      repeat " " $empty
    done
    echo
  done
  repeat "=" $basewidth   # Print "base" beneath rods.
  echo
}


display ()
{
  echo
  PrintRods

  # Get rod-number, summit and weight
  first=( `FromRod $1` )
  eval Rod${first[0]}[${first[1]}]=0

  # Get rod-number and first-free position
  second=( `ToRod ${first[2]} $2` )
  eval Rod${second[0]}[${second[1]}]=${first[2]}


echo; echo; echo
if [ "${Rod3[lastmove_t]}" = 1 ]
then   # Last move? If yes, then display final position.
    echo "+  Final Position: $Moves moves"; echo
    PrintRods
  fi
}


# From here down, almost the same as original (hanoi.bash) script.

dohanoi() {   # Recursive function.
    case $1 in
    0)
        ;;
    *)
        dohanoi "$(($1-1))" $2 $4 $3
	if [ "$Moves" -ne 0 ]
        then
	  echo "+  Position after move $Moves"
        fi
        ((Moves++))
        echo -n "   Next move will be:  "
        echo $2 "-->" $3
          display $2 $3
        dohanoi "$(($1-1))" $4 $3 $2
        ;;
    esac
}


setup_arrays ()
{
  local dim n elem

  let "dim1 = $1 - 1"
  elem=$dim1

  for n in $(seq 0 $dim1)
  do
   let "Rod1[$elem] = 2 * $n + 1"
   Rod2[$n]=0
   Rod3[$n]=0
   ((elem--))
  done
}


###   Main   ###

setup_arrays $DISKS
echo; echo "+  Start Position"

case $# in
    1) case $(($1>0)) in     # Must have at least one disk.
       1)
           disks=$1
           dohanoi $1 1 3 2
#          Total moves = 2^n - 1, where n = number of disks.
	   echo
           exit 0;
           ;;
       *)
           echo "$0: Illegal value for number of disks";
           exit $E_BADPARAM;
           ;;
       esac
    ;;
    *)
       clear
       echo "usage: $0 N"
       echo "       Where \"N\" is the number of disks."
       exit $E_NOPARAM;
       ;;
esac

exit $E_NOEXIT   # Shouldn't exit here.

# Note:
# Redirect script output to a file, otherwise it scrolls off display.
```

bash gets at "rod number `$rod`" with `eval weight=\${Rod${rod}[$summit]}`;
here the three rods are lists in a dict, `$Rod[$rod][$summit]`, no `eval`
needed. `FromRod` and `ToRod` give back lists rather than echoing words
for the caller to split. A row of the drawing is built with `" " * n` and
`"*" * n` instead of a `repeat` function. The output is bash's, line for
line, for the sample 3 disks.

```tessh
import sys
# The Towers Of Hanoi
# Original script (hanoi.bash) copyright (C) 2000 Amit Singh.
# All Rights Reserved. http://hanoi.kernelthread.com
#  hanoi2.bash, version 2.01: modded for ASCII-graphic display.
#  Uses code contributed by Antonio Macchi,
#+ with heavy editing by ABS Guide author.

E_NOPARAM = 86
E_BADPARAM = 87  # Illegal no. of disks passed to script.
E_NOEXIT = 88
MWIDTH = 7
MARGIN = 2       # Arbitrary "magic" constants.

args = $argv[1:] or ["3"]         # (sample argument, when none is given)
if (len $args) != 1
  print "usage: $(Path $argv[0]).name N"
  print "       Where \"N\" is the number of disks."
  exit $E_NOPARAM
DISKS = int $args[0]
if $DISKS <= 0                    # Must have at least one disk.
  print "$argv[0]: Illegal value for number of disks"
  exit $E_BADPARAM

basewidth = $MWIDTH * $DISKS + $MARGIN   # "Base" beneath rods.
Moves = 0

# Each rod is a list, bottom first, of the disks' widths (0: no disk).
# A dict holds the three, so Rod[1] does what bash's eval of Rod${rod} did.
Rod = {
  1: [(2 * $n + 1) for n in (range ($DISKS - 1) -1 -1)]
  2: [0] * $DISKS
  3: [0] * $DISKS
}

def FromRod rod                   # The top disk: rod, height, width.
  for summit in (range ($DISKS - 1) -1 -1)
    if $Rod[$rod][$summit] != 0
      return [$rod $summit $Rod[$rod][$summit]]

def ToRod weight rod              # Where a disk of that weight goes: rod, height.
  firstfree = 0
  for h in (range ($DISKS - 1) -1 -1)
    if $Rod[$rod][$h] > 0
      firstfree = $h + 1
      break
  assert ($firstfree == 0 or $Rod[$rod][$firstfree - 1] > $weight)
  [$rod $firstfree]

def PrintRods
  print (" " * $DISKS) "|" (" " * (2 * $DISKS)) "|" (" " * (2 * $DISKS)) "|" sep=""
  for disk in (range ($DISKS - 1) -1 -1)
    line = ""
    for rod in [1 2 3]
      fill = $Rod[$rod][$disk]
      empty = " " * ($DISKS - $fill // 2)
      line += $empty + (if $fill > 0 then "*" * $fill else "|") + $empty
    print $line
  print ("=" * $basewidth)        # Print "base" beneath rods.

def display src dst
  print
  PrintRods
  [rod summit weight] = FromRod $src
  $Rod[$rod][$summit] = 0
  [rod2 free] = ToRod $weight $dst
  $Rod[$rod2][$free] = $weight
  print ; print ; print
  if $Rod[3][$DISKS - 1] == 1     # Last move? If yes, then display final position.
    print "+  Final Position: $Moves moves" ; print
    PrintRods

# From here down, almost the same as original (hanoi.bash) script.
def dohanoi n src dst via         # Recursive function.
  global Moves
  if $n == 0
    return
  dohanoi ($n - 1) $src $via $dst
  if $Moves != 0
    print "+  Position after move $Moves"
  Moves += 1
  print "   Next move will be:  $src --> $dst"
  display $src $dst
  dohanoi ($n - 1) $via $dst $src

print ; print "+  Start Position"
dohanoi $DISKS 1 3 2              # Total moves = 2^n - 1, where n = number of disks.
print
```

```output

+  Start Position
   Next move will be:  1 --> 3

   |      |      |
   *      |      |   
  ***     |      |   
 *****    |      |   
=======================



+  Position after move 1
   Next move will be:  1 --> 2

   |      |      |
   |      |      |   
  ***     |      |   
 *****    |      *   
=======================



+  Position after move 2
   Next move will be:  3 --> 2

   |      |      |
   |      |      |   
   |      |      |   
 *****   ***     *   
=======================



+  Position after move 3
   Next move will be:  1 --> 3

   |      |      |
   |      |      |   
   |      *      |   
 *****   ***     |   
=======================



+  Position after move 4
   Next move will be:  2 --> 1

   |      |      |
   |      |      |   
   |      *      |   
   |     ***   ***** 
=======================



+  Position after move 5
   Next move will be:  2 --> 3

   |      |      |
   |      |      |   
   |      |      |   
   *     ***   ***** 
=======================



+  Position after move 6
   Next move will be:  1 --> 3

   |      |      |
   |      |      |   
   |      |     ***  
   *      |    ***** 
=======================



+  Final Position: 7 moves

   |      |      |
   |      |      *   
   |      |     ***  
   |      |    ***** 
=======================

```

## The Towers of Hanoi, alternate graphic version
<!-- help: abs-hanoi2a hanoi2a.bash tput cup civis cursor animation -->

The same, but drawn in place: `tput cup` moves the cursor back to the top
for each move, `tput civis` hides it (and a `trap` on exit shows it
again), and a two-second `sleep` makes it an animation.

### Example A-49. The Towers of Hanoi, alternate graphic version (`hanoi2a.bash`)

```bash no-run
#! /bin/bash
# The Towers Of Hanoi
# Original script (hanoi.bash) copyright (C) 2000 Amit Singh.
# All Rights Reserved.
# http://hanoi.kernelthread.com

#  hanoi2.bash
#  Version 2: modded for ASCII-graphic display.
#  Uses code contributed by Antonio Macchi,
#+ with heavy editing by ABS Guide author.
#  This variant also falls under the original copyright, see above.
#  Used in ABS Guide with Amit Singh's permission (thanks!).


#   Variables   #
E_NOPARAM=86
E_BADPARAM=87   # Illegal no. of disks passed to script.
E_NOEXIT=88
DELAY=2         # Interval, in seconds, between moves. Change, if desired.
DISKS=$1
Moves=0

MWIDTH=7
MARGIN=2
# Arbitrary "magic" constants, work okay for relatively small # of disks.
# BASEWIDTH=51   # Original code.
let "basewidth = $MWIDTH * $DISKS + $MARGIN" # "Base" beneath rods.
# Above "algorithm" could likely stand improvement.

# Display variables.
let "disks1 = $DISKS - 1"
let "spaces1 = $DISKS" 
let "spaces2 = 2 * $DISKS" 

let "lastmove_t = $DISKS - 1"                # Final move?


declare -a Rod1 Rod2 Rod3

#################


function repeat  {  # $1=char $2=number of repetitions
  local n           # Repeat-print a character.
  
  for (( n=0; n<$2; n++ )); do
    echo -n "$1"
  done
}

function FromRod  {
  local rod summit weight sequence

  while true; do
    rod=$1
    test ${rod/[^123]/} || continue

    sequence=$(echo $(seq 0 $disks1 | tac))
    for summit in $sequence; do
      eval weight=\${Rod${rod}[$summit]}
      test $weight -ne 0 &&
           { echo "$rod $summit $weight"; return; }
    done
  done
}


function ToRod  { # $1=previous (FromRod) weight
  local rod firstfree weight sequence
  
  while true; do
    rod=$2
    test ${rod/[^123]} || continue

    sequence=$(echo $(seq 0 $disks1 | tac))
    for firstfree in $sequence; do
      eval weight=\${Rod${rod}[$firstfree]}
      test $weight -gt 0 && { (( firstfree++ )); break; }
    done
    test $weight -gt $1 -o $firstfree = 0 &&
         { echo "$rod $firstfree"; return; }
  done
}


function PrintRods  {
  local disk rod empty fill sp sequence

  tput cup 5 0

  repeat " " $spaces1
  echo -n "|"
  repeat " " $spaces2
  echo -n "|"
  repeat " " $spaces2
  echo "|"

  sequence=$(echo $(seq 0 $disks1 | tac))
  for disk in $sequence; do
    for rod in {1..3}; do
      eval empty=$(( $DISKS - (Rod${rod}[$disk] / 2) ))
      eval fill=\${Rod${rod}[$disk]}
      repeat " " $empty
      test $fill -gt 0 && repeat "*" $fill || echo -n "|"
      repeat " " $empty
    done
    echo
  done
  repeat "=" $basewidth   # Print "base" beneath rods.
  echo
}


display ()
{
  echo
  PrintRods

  # Get rod-number, summit and weight
  first=( `FromRod $1` )
  eval Rod${first[0]}[${first[1]}]=0

  # Get rod-number and first-free position
  second=( `ToRod ${first[2]} $2` )
  eval Rod${second[0]}[${second[1]}]=${first[2]}


  if [ "${Rod3[lastmove_t]}" = 1 ]
  then   # Last move? If yes, then display final position.
    tput cup 0 0
    echo; echo "+  Final Position: $Moves moves"
    PrintRods
  fi

  sleep $DELAY
}

# From here down, almost the same as original (hanoi.bash) script.

dohanoi() {   # Recursive function.
    case $1 in
    0)
        ;;
    *)
        dohanoi "$(($1-1))" $2 $4 $3
	if [ "$Moves" -ne 0 ]
        then
	  tput cup 0 0
	  echo; echo "+  Position after move $Moves"
        fi
        ((Moves++))
        echo -n "   Next move will be:  "
        echo $2 "-->" $3
        display $2 $3
        dohanoi "$(($1-1))" $4 $3 $2
        ;;
    esac
}

setup_arrays ()
{
  local dim n elem

  let "dim1 = $1 - 1"
  elem=$dim1

  for n in $(seq 0 $dim1)
  do
   let "Rod1[$elem] = 2 * $n + 1"
   Rod2[$n]=0
   Rod3[$n]=0
   ((elem--))
  done
}


###   Main   ###

trap "tput cnorm" 0
tput civis
clear

setup_arrays $DISKS

tput cup 0 0
echo; echo "+  Start Position"

case $# in
    1) case $(($1>0)) in     # Must have at least one disk.
       1)
           disks=$1
           dohanoi $1 1 3 2
#          Total moves = 2^n - 1, where n = # of disks.
	   echo
           exit 0;
           ;;
       *)
           echo "$0: Illegal value for number of disks";
           exit $E_BADPARAM;
           ;;
       esac
    ;;
    *)
       echo "usage: $0 N"
       echo "       Where \"N\" is the number of disks."
       exit $E_NOPARAM;
       ;;
esac

exit $E_NOEXIT   # Shouldn't exit here.

#  Exercise:
#  --------
#  There is a minor bug in the script that causes the display of
#+ the next-to-last move to be skipped.
#+ Fix this.
```

`tput` is the same program in tessh; here it's called only when the
output is a terminal, and the delay is 0 when it isn't, so the moves
simply follow one another below. `atexit` does what `trap … 0` did. With
the sample 2 disks:

```tessh
import sys time atexit
# The Towers Of Hanoi
# Original script (hanoi.bash) copyright (C) 2000 Amit Singh.
# All Rights Reserved. http://hanoi.kernelthread.com
#  hanoi2a.bash, version 2: modded for ASCII-graphic display.
#  Uses code contributed by Antonio Macchi,
#+ with heavy editing by ABS Guide author.

E_NOPARAM = 86
E_BADPARAM = 87  # Illegal no. of disks passed to script.
E_NOEXIT = 88
MWIDTH = 7
MARGIN = 2       # Arbitrary "magic" constants.

tty = (sys.stdout.isatty)
DELAY = if $tty then 2 else 0     # Interval, in seconds, between moves.
def tput *what                    # (Cursor moves on a terminal only.)
  if $tty
    \tput *$what

args = $argv[1:] or ["2"]         # (sample argument, when none is given)
if (len $args) != 1
  print "usage: $(Path $argv[0]).name N"
  print "       Where \"N\" is the number of disks."
  exit $E_NOPARAM
DISKS = int $args[0]
if $DISKS <= 0                    # Must have at least one disk.
  print "$argv[0]: Illegal value for number of disks"
  exit $E_BADPARAM

basewidth = $MWIDTH * $DISKS + $MARGIN   # "Base" beneath rods.
Moves = 0

# Each rod is a list, bottom first, of the disks' widths (0: no disk).
# A dict holds the three, so Rod[1] does what bash's eval of Rod${rod} did.
Rod = {
  1: [(2 * $n + 1) for n in (range ($DISKS - 1) -1 -1)]
  2: [0] * $DISKS
  3: [0] * $DISKS
}

def FromRod rod                   # The top disk: rod, height, width.
  for summit in (range ($DISKS - 1) -1 -1)
    if $Rod[$rod][$summit] != 0
      return [$rod $summit $Rod[$rod][$summit]]

def ToRod weight rod              # Where a disk of that weight goes: rod, height.
  firstfree = 0
  for h in (range ($DISKS - 1) -1 -1)
    if $Rod[$rod][$h] > 0
      firstfree = $h + 1
      break
  assert ($firstfree == 0 or $Rod[$rod][$firstfree - 1] > $weight)
  [$rod $firstfree]

def PrintRods
  tput cup 5 0
  print (" " * $DISKS) "|" (" " * (2 * $DISKS)) "|" (" " * (2 * $DISKS)) "|" sep=""
  for disk in (range ($DISKS - 1) -1 -1)
    line = ""
    for rod in [1 2 3]
      fill = $Rod[$rod][$disk]
      empty = " " * ($DISKS - $fill // 2)
      line += $empty + (if $fill > 0 then "*" * $fill else "|") + $empty
    print $line
  print ("=" * $basewidth)        # Print "base" beneath rods.

def display src dst
  print
  PrintRods
  [rod summit weight] = FromRod $src
  $Rod[$rod][$summit] = 0
  [rod2 free] = ToRod $weight $dst
  $Rod[$rod2][$free] = $weight
  if $Rod[3][$DISKS - 1] == 1     # Last move? If yes, then display final position.
    tput cup 0 0
    print ; print "+  Final Position: $Moves moves"
    PrintRods
  time.sleep $DELAY

# From here down, almost the same as original (hanoi.bash) script.
def dohanoi n src dst via         # Recursive function.
  global Moves
  if $n == 0
    return
  dohanoi ($n - 1) $src $via $dst
  if $Moves != 0
    tput cup 0 0
    print ; print "+  Position after move $Moves"
  Moves += 1
  print "   Next move will be:  $src --> $dst"
  display $src $dst
  dohanoi ($n - 1) $via $dst $src

atexit.register (fn [] (tput cnorm))   # trap "tput cnorm" 0: the cursor back.
tput civis                        # Hide the cursor.
tput clear
tput cup 0 0
print ; print "+  Start Position"
dohanoi $DISKS 1 3 2              # Total moves = 2^n - 1, where n = number of disks.
print
```

```output

+  Start Position
   Next move will be:  1 --> 2

  |    |    |
  *    |    |  
 ***   |    |  
================

+  Position after move 1
   Next move will be:  1 --> 3

  |    |    |
  |    |    |  
 ***   *    |  
================

+  Position after move 2
   Next move will be:  2 --> 3

  |    |    |
  |    |    |  
  |    *   *** 
================

+  Final Position: 3 moves
  |    |    |
  |    |    *  
  |    |   *** 
================

```

## Using getopt
<!-- help: abs-usegetopt usegetopt.sh getopt gnu_getopt long-options options -->

Peggy Russell's example of the `getopt` program (not the `getopts`
builtin), which handles long options: it rewrites the arguments with each
option and its argument separate, then `--`, then the rest, and the script
`eval set --`s that and loops over it.

### Example A-50. An alternate version of the `getopt-simple.sh` script (`UseGetOpt.sh`)

```bash
#!/bin/bash
# UseGetOpt.sh

# Author: Peggy Russell <prusselltechgroup@gmail.com>

UseGetOpt () {
  declare inputOptions
  declare -r E_OPTERR=85
  declare -r ScriptName=${0##*/}
  declare -r ShortOpts="adf:hlt"
  declare -r LongOpts="aoption,debug,file:,help,log,test"

DoSomething () {
    echo "The function name is '${FUNCNAME}'"
    #  Recall that $FUNCNAME is an internal variable
    #+ holding the name of the function it is in.
  }

  inputOptions=$(getopt -o "${ShortOpts}" --long \
              "${LongOpts}" --name "${ScriptName}" -- "${@}")

  if [[ ($? -ne 0) || ($# -eq 0) ]]; then
    echo "Usage: ${ScriptName} [-dhlt] {OPTION...}"
    exit $E_OPTERR
  fi

  eval set -- "${inputOptions}"

  # Only for educational purposes. Can be removed.
  #-----------------------------------------------
  echo "++ Test: Number of arguments: [$#]"
  echo '++ Test: Looping through "$@"'
  for a in "$@"; do
    echo "  ++ [$a]"
  done
  #-----------------------------------------------

  while true; do
    case "${1}" in
      --aoption | -a)  # Argument found.
        echo "Option [$1]"
        ;;

      --debug | -d)    # Enable informational messages.
        echo "Option [$1] Debugging enabled"
        ;;

      --file | -f)     #  Check for optional argument.
        case "$2" in   #+ Double colon is optional argument.
          "")          #  Not there.
              echo "Option [$1] Use default"
              shift
              ;;

          *) # Got it
             echo "Option [$1] Using input [$2]"
             shift
             ;;

        esac
        DoSomething
        ;;

      --log | -l) # Enable Logging.
        echo "Option [$1] Logging enabled"
        ;;

      --test | -t) # Enable testing.
        echo "Option [$1] Testing enabled"
        ;;

      --help | -h)
        echo "Option [$1] Display help"
        break
        ;;

      --)   # Done! $# is argument number for "--", $@ is "--"
        echo "Option [$1] Dash Dash"
        break
        ;;

       *)
        echo "Major internal error!"
        exit 8
        ;;

    esac
    echo "Number of arguments: [$#]"
    shift
  done

  shift
  # Only for educational purposes. Can be removed.
  #----------------------------------------------------------------------
  echo "++ Test: Number of arguments after \"--\" is [$#] They are: [$@]"
  echo '++ Test: Looping through "$@"'
  for a in "$@"; do
    echo "  ++ [$a]"
  done
  #----------------------------------------------------------------------
  
}

################################### M A I N ########################
#  If you remove "function UseGetOpt () {" and corresponding "}",
#+ you can uncomment the "exit 0" line below, and invoke this script
#+ with the various options from the command-line.
#-------------------------------------------------------------------
# exit 0

echo "Test 1"
UseGetOpt -f myfile one "two three" four

echo;echo "Test 2"
UseGetOpt -h

echo;echo "Test 3 - Short Options"
UseGetOpt -adltf myfile  anotherfile

echo;echo "Test 4 - Long Options"
UseGetOpt --aoption --debug --log --test --file myfile anotherfile

exit
```

Python's `getopt` module is the same thing: `gnu_getopt` takes the
options from anywhere in the arguments, as the program does, gives back
the options as pairs and the rest as a list, and raises `GetoptError` for
a bad one. The script turns the pairs back into the program's list so the
loop and its test lines print what bash's do; `match` is the `case`, and
`$args.pop 0` the `shift`. (For a script of your own, `argparse` does the
whole job, usage and help included.)

```tessh
import getopt
# UseGetOpt.sh
# Author: Peggy Russell <prusselltechgroup@gmail.com>

def UseGetOpt *argv
  E_OPTERR = 85
  ScriptName = (Path $argv[0]).name
  ShortOpts = "adf:hlt"
  LongOpts = [aoption debug "file=" help log test]   # (file: in getopt's spelling)

  def DoSomething
    print "The function name is '$DoSomething.__name__'"

  # Python's getopt module reads the options as the getopt program does.
  inputOptions = try
    getopt.gnu_getopt $argv[1:] $ShortOpts $LongOpts
  except getopt.GetoptError as e
    print "$ScriptName: $e"
    None
  if $inputOptions is None or not $argv[1:]
    print "Usage: $ScriptName [-dhlt] {OPTION...}"
    exit $E_OPTERR
  [opts rest] = $inputOptions
  # The options, each with its argument if it takes one, then --, then the rest:
  # the list getopt prints, which bash sets as its arguments.
  args = []
  for [o v] in $opts
    $args.extend (if $o in [-f --file] then [$o $v] else [$o])
  args += ["--" *$rest]

  # Only for educational purposes. Can be removed.
  #-----------------------------------------------
  print "++ Test: Number of arguments: [$(len $args)]"
  print '++ Test: Looping through "$@"'
  for a in $args
    print "  ++ [$a]"
  #-----------------------------------------------

  while True
    match $args[0]
      case "--aoption" | "-a"     # Argument found.
        print "Option [$args[0]]"
      case "--debug" | "-d"       # Enable informational messages.
        print "Option [$args[0]] Debugging enabled"
      case "--file" | "-f"        # Check for optional argument.
        if $args[1] == ""         # Not there.
          print "Option [$args[0]] Use default"
        else                      # Got it
          print "Option [$args[0]] Using input [$args[1]]"
        $args.pop 0
        DoSomething
      case "--log" | "-l"         # Enable Logging.
        print "Option [$args[0]] Logging enabled"
      case "--test" | "-t"        # Enable testing.
        print "Option [$args[0]] Testing enabled"
      case "--help" | "-h"
        print "Option [$args[0]] Display help"
        break
      case "--"                   # Done!
        print "Option [$args[0]] Dash Dash"
        break
      case _
        print "Major internal error!"
        exit 8
    print "Number of arguments: [$(len $args)]"
    $args.pop 0
  $args.pop 0

  # Only for educational purposes. Can be removed.
  #----------------------------------------------------------------------
  print "++ Test: Number of arguments after \"--\" is [$(len $args)] They are: [$(' '.join $args)]"
  print '++ Test: Looping through "$@"'
  for a in $args
    print "  ++ [$a]"
  #----------------------------------------------------------------------

################################### M A I N ########################
print "Test 1"
UseGetOpt $argv[0] -f myfile one "two three" four

print ; print "Test 2"
UseGetOpt $argv[0] -h

print ; print "Test 3 - Short Options"
UseGetOpt $argv[0] -adltf myfile  anotherfile

print ; print "Test 4 - Long Options"
UseGetOpt $argv[0] --aoption --debug --log --test --file myfile anotherfile
```

```output
Test 1
++ Test: Number of arguments: [6]
++ Test: Looping through "$@"
  ++ [-f]
  ++ [myfile]
  ++ [--]
  ++ [one]
  ++ [two three]
  ++ [four]
Option [-f] Using input [myfile]
The function name is 'DoSomething'
Number of arguments: [5]
Option [--] Dash Dash
++ Test: Number of arguments after "--" is [3] They are: [one two three four]
++ Test: Looping through "$@"
  ++ [one]
  ++ [two three]
  ++ [four]

Test 2
++ Test: Number of arguments: [2]
++ Test: Looping through "$@"
  ++ [-h]
  ++ [--]
Option [-h] Display help
++ Test: Number of arguments after "--" is [1] They are: [--]
++ Test: Looping through "$@"
  ++ [--]

Test 3 - Short Options
++ Test: Number of arguments: [8]
++ Test: Looping through "$@"
  ++ [-a]
  ++ [-d]
  ++ [-l]
  ++ [-t]
  ++ [-f]
  ++ [myfile]
  ++ [--]
  ++ [anotherfile]
Option [-a]
Number of arguments: [8]
Option [-d] Debugging enabled
Number of arguments: [7]
Option [-l] Logging enabled
Number of arguments: [6]
Option [-t] Testing enabled
Number of arguments: [5]
Option [-f] Using input [myfile]
The function name is 'DoSomething'
Number of arguments: [3]
Option [--] Dash Dash
++ Test: Number of arguments after "--" is [1] They are: [anotherfile]
++ Test: Looping through "$@"
  ++ [anotherfile]

Test 4 - Long Options
++ Test: Number of arguments: [8]
++ Test: Looping through "$@"
  ++ [--aoption]
  ++ [--debug]
  ++ [--log]
  ++ [--test]
  ++ [--file]
  ++ [myfile]
  ++ [--]
  ++ [anotherfile]
Option [--aoption]
Number of arguments: [8]
Option [--debug] Debugging enabled
Number of arguments: [7]
Option [--log] Logging enabled
Number of arguments: [6]
Option [--test] Testing enabled
Number of arguments: [5]
Option [--file] Using input [myfile]
The function name is 'DoSomething'
Number of arguments: [3]
Option [--] Dash Dash
++ Test: Number of arguments after "--" is [1] They are: [anotherfile]
++ Test: Looping through "$@"
  ++ [anotherfile]
```

### Example A-51. The version of the `UseGetOpt.sh` example used in the Tab Expansion appendix (`UseGetOpt-2.sh`)

The same script at the top level, to try with the completion in Appendix
J. Run without arguments it prints its usage, as here.

```bash
#!/bin/bash

#  UseGetOpt-2.sh
#  Modified version of the script for illustrating tab-expansion
#+ of command-line options.
#  See the "Introduction to Tab Expansion" appendix.

#  Possible options: -a -d -f -l -t -h
#+                   --aoption, --debug --file --log --test -- help --

#  Author of original script: Peggy Russell <prusselltechgroup@gmail.com>


# UseGetOpt () {
  declare inputOptions
  declare -r E_OPTERR=85
  declare -r ScriptName=${0##*/}
  declare -r ShortOpts="adf:hlt"
  declare -r LongOpts="aoption,debug,file:,help,log,test"

DoSomething () {
    echo "The function name is '${FUNCNAME}'"
  }

  inputOptions=$(getopt -o "${ShortOpts}" --long \
              "${LongOpts}" --name "${ScriptName}" -- "${@}")

  if [[ ($? -ne 0) || ($# -eq 0) ]]; then
    echo "Usage: ${ScriptName} [-dhlt] {OPTION...}"
    exit $E_OPTERR
  fi

  eval set -- "${inputOptions}"


  while true; do
    case "${1}" in
      --aoption | -a)  # Argument found.
        echo "Option [$1]"
        ;;

      --debug | -d)    # Enable informational messages.
        echo "Option [$1] Debugging enabled"
        ;;

      --file | -f)     #  Check for optional argument.
        case "$2" in   #+ Double colon is optional argument.
          "")          #  Not there.
              echo "Option [$1] Use default"
              shift
              ;;

          *) # Got it
             echo "Option [$1] Using input [$2]"
             shift
             ;;

        esac
        DoSomething
        ;;

      --log | -l) # Enable Logging.
        echo "Option [$1] Logging enabled"
        ;;

      --test | -t) # Enable testing.
        echo "Option [$1] Testing enabled"
        ;;

      --help | -h)
        echo "Option [$1] Display help"
        break
        ;;

      --)   # Done! $# is argument number for "--", $@ is "--"
        echo "Option [$1] Dash Dash"
        break
        ;;

       *)
        echo "Major internal error!"
        exit 8
        ;;

    esac
    echo "Number of arguments: [$#]"
    shift
  done

  shift
  
#  }

exit
```

```output bash
Usage: ex.sh [-dhlt] {OPTION...}
```

The tessh version takes sample options when it's given none.

**Gap:** bash's `complete -W "…" UseGetOpt-2.sh` makes Tab offer the
script's options. tessh's REPL completes names, programs and paths, but
has no way to tell it a command's options.

```tessh
import getopt
#  UseGetOpt-2.sh
#  Modified version of the script for illustrating tab-expansion
#+ of command-line options.
#  Possible options: -a -d -f -l -t -h
#+                   --aoption --debug --file --log --test --help --
#  Author of original script: Peggy Russell <prusselltechgroup@gmail.com>

cmdline = $argv[1:] or [-dt --file notes.txt extra]   # (sample arguments, when none are given)
E_OPTERR = 85
ScriptName = (Path $argv[0]).name
ShortOpts = "adf:hlt"
LongOpts = [aoption debug "file=" help log test]

def DoSomething
  print "The function name is '$DoSomething.__name__'"

inputOptions = try
  getopt.gnu_getopt $cmdline $ShortOpts $LongOpts
except getopt.GetoptError as e
  print "$ScriptName: $e"
  None
if $inputOptions is None or not $cmdline
  print "Usage: $ScriptName [-dhlt] {OPTION...}"
  exit $E_OPTERR

[opts rest] = $inputOptions
args = []
for [o v] in $opts
  $args.extend (if $o in [-f --file] then [$o $v] else [$o])
args += ["--" *$rest]

while True
  match $args[0]
    case "--aoption" | "-a"       # Argument found.
      print "Option [$args[0]]"
    case "--debug" | "-d"         # Enable informational messages.
      print "Option [$args[0]] Debugging enabled"
    case "--file" | "-f"          # Check for optional argument.
      if $args[1] == ""
        print "Option [$args[0]] Use default"
      else
        print "Option [$args[0]] Using input [$args[1]]"
      $args.pop 0
      DoSomething
    case "--log" | "-l"           # Enable Logging.
      print "Option [$args[0]] Logging enabled"
    case "--test" | "-t"          # Enable testing.
      print "Option [$args[0]] Testing enabled"
    case "--help" | "-h"
      print "Option [$args[0]] Display help"
      break
    case "--"                     # Done!
      print "Option [$args[0]] Dash Dash"
      break
    case _
      print "Major internal error!"
      exit 8
  print "Number of arguments: [$(len $args)]"
  $args.pop 0
$args.pop 0
exit
```

```output
Option [-d] Debugging enabled
Number of arguments: [6]
Option [-t] Testing enabled
Number of arguments: [5]
Option [--file] Using input [notes.txt]
The function name is 'DoSomething'
Number of arguments: [3]
Option [--] Dash Dash
```

## All the background colors
<!-- help: abs-showallc show-all-colors.sh 256-colors ansi escape background -->

Chetankumar Phulpagare's script shows each of a terminal's 256 background
colors with ANSI escape sequences: the 16 system colors, the 6x6x6 color
cube and 24 grays.

### Example A-52. Cycling through all the possible color backgrounds (`show-all-colors.sh`)

```bash no-run
#!/bin/bash

# show-all-colors.sh
# Displays all 256 possible background colors, using ANSI escape sequences.
# Author: Chetankumar Phulpagare
# Used in ABS Guide with permission.

T1=8
T2=6
T3=36
offset=0

for num1 in {0..7}
do {
   for num2 in {0,1}
       do {
          shownum=`echo "$offset + $T1 * ${num2} + $num1" | bc`
          echo -en "\E[0;48;5;${shownum}m color ${shownum} \E[0m"
          }
       done
   echo
   }
done

offset=16
for num1 in {0..5}
do {
   for num2 in {0..5}
       do {
          for num3 in {0..5}
              do {
                 shownum=`echo "$offset + $T2 * ${num3} \
                 + $num2 + $T3 * ${num1}" | bc`
                 echo -en "\E[0;48;5;${shownum}m color ${shownum} \E[0m"
                 }
               done
          echo
          }
       done
}
done

offset=232
for num1 in {0..23}
do {
   shownum=`expr $offset + $num1`
   echo -en "\E[0;48;5;${shownum}m ${shownum}\E[0m"
}
done

echo
```

The numbers are plain arithmetic in tessh, no `bc` or `expr` per square.
`"\033[0;48;5;${num}m"` is the same escape sequence (tessh's strings take
`\033`, not bash's `\E`); it's sent only when the output is a terminal, so
here, in a file, just the layout shows. The loops name the cube's axes.

```tessh
import sys
# show-all-colors.sh
# Displays all 256 possible background colors, using ANSI escape sequences.
# Author: Chetankumar Phulpagare

tty = (sys.stdout.isatty)
def show num text                 # A swatch: in its color on a terminal.
  if $tty
    print "\033[0;48;5;${num}m$text\033[0m" end=""
  else
    print $text end=""

# The 16 system colors, two rows of 8.
for row in (range 8)
  for half in [0 1]
    n = 8 * $half + $row
    show $n " color $n "
  print

# The 6 x 6 x 6 color cube, 16 - 231.
for red in (range 6)
  for blue in (range 6)
    for green in (range 6)
      n = 16 + 36 * $red + 6 * $green + $blue
      show $n " color $n "
    print

# The 24 grays, 232 - 255.
for n in (range 232 256)
  show $n " $n"
print
```

```output
 color 0  color 8 
 color 1  color 9 
 color 2  color 10 
 color 3  color 11 
 color 4  color 12 
 color 5  color 13 
 color 6  color 14 
 color 7  color 15 
 color 16  color 22  color 28  color 34  color 40  color 46 
 color 17  color 23  color 29  color 35  color 41  color 47 
 color 18  color 24  color 30  color 36  color 42  color 48 
 color 19  color 25  color 31  color 37  color 43  color 49 
 color 20  color 26  color 32  color 38  color 44  color 50 
 color 21  color 27  color 33  color 39  color 45  color 51 
 color 52  color 58  color 64  color 70  color 76  color 82 
 color 53  color 59  color 65  color 71  color 77  color 83 
 color 54  color 60  color 66  color 72  color 78  color 84 
 color 55  color 61  color 67  color 73  color 79  color 85 
 color 56  color 62  color 68  color 74  color 80  color 86 
 color 57  color 63  color 69  color 75  color 81  color 87 
 color 88  color 94  color 100  color 106  color 112  color 118 
 color 89  color 95  color 101  color 107  color 113  color 119 
 color 90  color 96  color 102  color 108  color 114  color 120 
 color 91  color 97  color 103  color 109  color 115  color 121 
 color 92  color 98  color 104  color 110  color 116  color 122 
 color 93  color 99  color 105  color 111  color 117  color 123 
 color 124  color 130  color 136  color 142  color 148  color 154 
 color 125  color 131  color 137  color 143  color 149  color 155 
 color 126  color 132  color 138  color 144  color 150  color 156 
 color 127  color 133  color 139  color 145  color 151  color 157 
 color 128  color 134  color 140  color 146  color 152  color 158 
 color 129  color 135  color 141  color 147  color 153  color 159 
 color 160  color 166  color 172  color 178  color 184  color 190 
 color 161  color 167  color 173  color 179  color 185  color 191 
 color 162  color 168  color 174  color 180  color 186  color 192 
 color 163  color 169  color 175  color 181  color 187  color 193 
 color 164  color 170  color 176  color 182  color 188  color 194 
 color 165  color 171  color 177  color 183  color 189  color 195 
 color 196  color 202  color 208  color 214  color 220  color 226 
 color 197  color 203  color 209  color 215  color 221  color 227 
 color 198  color 204  color 210  color 216  color 222  color 228 
 color 199  color 205  color 211  color 217  color 223  color 229 
 color 200  color 206  color 212  color 218  color 224  color 230 
 color 201  color 207  color 213  color 219  color 225  color 231 
 232 233 234 235 236 237 238 239 240 241 242 243 244 245 246 247 248 249 250 251 252 253 254 255
```

## Morse code practice
<!-- help: abs-samorse sam.sh morse associative-array dict eval aplay -->

Mendel Cooper's "Still Another Morse" trainer plays each word given to it
as dots and dashes, through `aplay` and two sound files. The code table is
an associative array whose values are commands: `morse[a]="dot; dash"`,
run with `eval`.

### Example A-53. Morse Code Practice (`sam.sh`)

```bash no-run
#!/bin/bash
# sam.sh, v. .01a
# Still Another Morse (code training script)
# With profuse apologies to Sam (F.B.) Morse.
# Author: Mendel Cooper
# License: GPL3
# Reldate: 05/25/11

# Morse code training script.
# Converts arguments to audible dots and dashes.
# Note: lowercase input only at this time.



# Get the wav files from the source tarball:
# http://bash.deta.in/abs-guide-latest.tar.bz2
DOT='soundfiles/dot.wav'
DASH='soundfiles/dash.wav'
# Maybe move soundfiles to /usr/local/sounds?

LETTERSPACE=300000  # Microseconds.
WORDSPACE=980000
# Nice and slow, for beginners. Maybe 5 wpm?

EXIT_MSG="May the Morse be with you!"
E_NOARGS=75         # No command-line args?



declare -A morse    # Associative array!
# ======================================= #
morse[a]="dot; dash"
morse[b]="dash; dot; dot; dot"
morse[c]="dash; dot; dash; dot"
morse[d]="dash; dot; dot"
morse[e]="dot"
morse[f]="dot; dot; dash; dot"
morse[g]="dash; dash; dot"
morse[h]="dot; dot; dot; dot"
morse[i]="dot; dot;"
morse[j]="dot; dash; dash; dash"
morse[k]="dash; dot; dash"
morse[l]="dot; dash; dot; dot"
morse[m]="dash; dash"
morse[n]="dash; dot"
morse[o]="dash; dash; dash"
morse[p]="dot; dash; dash; dot"
morse[q]="dash; dash; dot; dash"
morse[r]="dot; dash; dot"
morse[s]="dot; dot; dot"
morse[t]="dash"
morse[u]="dot; dot; dash"
morse[v]="dot; dot; dot; dash"
morse[w]="dot; dash; dash"
morse[x]="dash; dot; dot; dash"
morse[y]="dash; dot; dash; dash"
morse[z]="dash; dash; dot; dot"
morse[0]="dash; dash; dash; dash; dash"
morse[1]="dot; dash; dash; dash; dash"
morse[2]="dot; dot; dash; dash; dash"
morse[3]="dot; dot; dot; dash; dash"
morse[4]="dot; dot; dot; dot; dash"
morse[5]="dot; dot; dot; dot; dot"
morse[6]="dash; dot; dot; dot; dot"
morse[7]="dash; dash; dot; dot; dot"
morse[8]="dash; dash; dash; dot; dot"
morse[9]="dash; dash; dash; dash; dot"
# The following must be escaped or quoted.
morse[?]="dot; dot; dash; dash; dot; dot"
morse[.]="dot; dash; dot; dash; dot; dash"
morse[,]="dash; dash; dot; dot; dash; dash"
morse[/]="dash; dot; dot; dash; dot"
morse[\@]="dot; dash; dash; dot; dash; dot"
# ======================================= #

play_letter ()
{
  eval ${morse[$1]}   # Play dots, dashes from appropriate sound files.
  # Why is 'eval' necessary here?
  usleep $LETTERSPACE # Pause in between letters.
}

extract_letters ()
{                     # Slice string apart, letter by letter.
  local pos=0         # Starting at left end of string.
  local len=1         # One letter at a time.
  strlen=${#1}

  while [ $pos -lt $strlen ]
  do
    letter=${1:pos:len}
    #      ^^^^^^^^^^^^    See Chapter 10.1.
    play_letter $letter
    echo -n "*"       #    Mark letter just played.
    ((pos++))
  done
}

######### Play the sounds ############
dot()  { aplay "$DOT" 2&>/dev/null;  }
dash() { aplay "$DASH" 2&>/dev/null; }
######################################

no_args ()
{
    declare -a usage
    usage=( $0 word1 word2 ... )

    echo "Usage:"; echo
    echo ${usage[*]}
    for index in 0 1 2 3
    do
      extract_letters ${usage[index]}     
      usleep $WORDSPACE
      echo -n " "     # Print space between words.
    done
#   echo "Usage: $0 word1 word2 ... "
    echo; echo
}


# int main()
# {

clear                 # Clear the terminal screen.
echo "            SAM"
echo "Still Another Morse code trainer"
echo "    Author: Mendel Cooper"
echo; echo;

if [ -z "$1" ]
then
  no_args
  echo; echo; echo "$EXIT_MSG"; echo
  exit $E_NOARGS
fi

echo; echo "$*"       # Print text that will be played.

until [ -z "$1" ]
do
  extract_letters $1
  shift           # On to next word.
  usleep $WORDSPACE
  echo -n " "     # Print space between words.
done

echo; echo; echo "$EXIT_MSG"; echo

exit 0
# }

#  Exercises:
#  ---------
#  1) Have the script accept either lowercase or uppercase words
#+    as arguments. Hint: Use 'tr' . . .
#  2) Have the script optionally accept input from a text file.
```

The table is a dict here, keys and values as in bash (the `?`, `.`, `,`,
`/` and `@` keys just quoted). The original asks why `eval` is needed: the
value is text, and only `eval` makes bash run text as commands. tessh
looks each name up in a dict of functions and calls it, which can't run
anything but `dot` and `dash`. Without the sound files, `dot` and `dash`
print `.` and `-` here (with sounds they'd be `aplay $DOT`), the pauses
are 0, and the sample words are `sos abs`. A word plays letter by letter,
as `${1:pos:len}` sliced it, by looping over the string.

```tessh
import sys time
# sam.sh, v. .01a: Still Another Morse (code training script)
# With profuse apologies to Sam (F.B.) Morse.
# Author: Mendel Cooper. License: GPL3. Reldate: 05/25/11
# Converts arguments to audible dots and dashes.

DOT = soundfiles/dot.wav
DASH = soundfiles/dash.wav
LETTERSPACE = 0       # (0.3 seconds in the original.)
WORDSPACE = 0         # (0.98 seconds.)
EXIT_MSG = "May the Morse be with you!"
E_NOARGS = 75         # No command-line args?

morse = {             # A dict: bash's associative array.
  a: "dot; dash"                 b: "dash; dot; dot; dot"
  c: "dash; dot; dash; dot"      d: "dash; dot; dot"
  e: "dot"                       f: "dot; dot; dash; dot"
  g: "dash; dash; dot"           h: "dot; dot; dot; dot"
  i: "dot; dot;"                 j: "dot; dash; dash; dash"
  k: "dash; dot; dash"           l: "dot; dash; dot; dot"
  m: "dash; dash"                n: "dash; dot"
  o: "dash; dash; dash"          p: "dot; dash; dash; dot"
  q: "dash; dash; dot; dash"     r: "dot; dash; dot"
  s: "dot; dot; dot"             t: "dash"
  u: "dot; dot; dash"            v: "dot; dot; dot; dash"
  w: "dot; dash; dash"           x: "dash; dot; dot; dash"
  y: "dash; dot; dash; dash"     z: "dash; dash; dot; dot"
  "0": "dash; dash; dash; dash; dash"   "1": "dot; dash; dash; dash; dash"
  "2": "dot; dot; dash; dash; dash"     "3": "dot; dot; dot; dash; dash"
  "4": "dot; dot; dot; dot; dash"       "5": "dot; dot; dot; dot; dot"
  "6": "dash; dot; dot; dot; dot"       "7": "dash; dash; dot; dot; dot"
  "8": "dash; dash; dash; dot; dot"     "9": "dash; dash; dash; dash; dot"
  "?": "dot; dot; dash; dash; dot; dot" ".": "dot; dash; dot; dash; dot; dash"
  ",": "dash; dash; dot; dot; dash; dash" "/": "dash; dot; dot; dash; dot"
  "@": "dot; dash; dash; dot; dash; dot"
}

######### Play the sounds ############
# (aplay "$DOT" in the original; here, without the sound files,
#  each sound is shown as a character.)
def dot = print "." end=""
def dash = print "-" end=""
sounds = {dot: $dot dash: $dash}
######################################

def play_letter letter
  # bash runs "dot; dash" with eval; here each name is looked up and called.
  for name in ($morse[$letter].split ";")
    if (.strip $name)
      sound = $sounds[(.strip $name)]
      sound
  time.sleep $LETTERSPACE         # Pause in between letters.

def extract_letters word          # Slice string apart, letter by letter.
  for letter in $word
    play_letter $letter
    print "*" end=""              # Mark letter just played.

def no_args
  usage = [$argv[0] word1 word2 ...]
  print "Usage:" ; print
  print (" ".join $usage)
  for word in $usage[1:]          # (The script's path isn't played here.)
    extract_letters $word
    time.sleep $WORDSPACE
    print " " end=""              # Print space between words.
  print ; print

def clear
  if (sys.stdout.isatty)
    \clear

clear
print "            SAM"
print "Still Another Morse code trainer"
print "    Author: Mendel Cooper"
print ; print

args = $argv[1:] or [sos abs]     # (sample arguments, when none are given)
if not $args
  no_args
  print ; print ; print $EXIT_MSG ; print
  exit $E_NOARGS

print ; print (" ".join $args)   # Print text that will be played.
for word in $args
  extract_letters $word
  time.sleep $WORDSPACE
  print " " end=""                # Print space between words.
print ; print ; print $EXIT_MSG ; print
exit 0
```

```output
            SAM
Still Another Morse code trainer
    Author: Mendel Cooper



sos abs
...*---*...* .-*-...*...* 

May the Morse be with you!

```

## Base64 encoding and decoding
<!-- help: abs-base64 base64.sh base64 bitwise bytes encode decode -->

vladz's Base64 encoder and decoder in pure bash: `xxd` makes a hex dump of
the input, `bc` turns each hex pair into a number, and bitwise operators
regroup each three bytes into four 6-bit numbers (and back with `-d`).

### Example A-54. Base64 encoding/decoding (`base64.sh`)

```bash no-run
#!/bin/bash
# base64.sh: Bash implementation of Base64 encoding and decoding.
#
# Copyright (c) 2011 vladz <vladz@devzero.fr>
# Used in ABSG with permission (thanks!).
#
#  Encode or decode original Base64 (and also Base64url)
#+ from STDIN to STDOUT.
#
#    Usage:
#
#    Encode
#    $ ./base64.sh < binary-file > binary-file.base64
#    Decode
#    $ ./base64.sh -d < binary-file.base64 > binary-file
#
# Reference:
#
#    [1]  RFC4648 - "The Base16, Base32, and Base64 Data Encodings"
#         http://tools.ietf.org/html/rfc4648#section-5


# The base64_charset[] array contains entire base64 charset,
# and additionally the character "=" ...
base64_charset=( {A..Z} {a..z} {0..9} + / = )
                # Nice illustration of brace expansion.

#  Uncomment the ### line below to use base64url encoding instead of
#+ original base64.
### base64_charset=( {A..Z} {a..z} {0..9} - _ = )

#  Output text width when encoding
#+ (64 characters, just like openssl output).
text_width=64

function display_base64_char {
#  Convert a 6-bit number (between 0 and 63) into its corresponding values
#+ in Base64, then display the result with the specified text width.
  printf "${base64_charset[$1]}"; (( width++ ))
  (( width % text_width == 0 )) && printf "\n"
}

function encode_base64 {
# Encode three 8-bit hexadecimal codes into four 6-bit numbers.
  #    We need two local int array variables:
  #    c8[]: to store the codes of the 8-bit characters to encode
  #    c6[]: to store the corresponding encoded values on 6-bit
  declare -a -i c8 c6

  #  Convert hexadecimal to decimal.
  c8=( $(printf "ibase=16; ${1:0:2}\n${1:2:2}\n${1:4:2}\n" | bc) )

  #  Let's play with bitwise operators
  #+ (3x8-bit into 4x6-bits conversion).
  (( c6[0] = c8[0] >> 2 ))
  (( c6[1] = ((c8[0] &  3) << 4) | (c8[1] >> 4) ))

  # The following operations depend on the c8 element number.
  case ${#c8[*]} in 
    3) (( c6[2] = ((c8[1] & 15) << 2) | (c8[2] >> 6) ))
       (( c6[3] = c8[2] & 63 )) ;;
    2) (( c6[2] = (c8[1] & 15) << 2 ))
       (( c6[3] = 64 )) ;;
    1) (( c6[2] = c6[3] = 64 )) ;;
  esac

  for char in ${c6[@]}; do
    display_base64_char ${char}
  done
}

function decode_base64 {
# Decode four base64 characters into three hexadecimal ASCII characters.
  #  c8[]: to store the codes of the 8-bit characters
  #  c6[]: to store the corresponding Base64 values on 6-bit
  declare -a -i c8 c6

  # Find decimal value corresponding to the current base64 character.
  for current_char in ${1:0:1} ${1:1:1} ${1:2:1} ${1:3:1}; do
     [ "${current_char}" = "=" ] && break

     position=0
     while [ "${current_char}" != "${base64_charset[${position}]}" ]; do
        (( position++ ))
     done

     c6=( ${c6[*]} ${position} )
  done

  #  Let's play with bitwise operators
  #+ (4x8-bit into 3x6-bits conversion).
  (( c8[0] = (c6[0] << 2) | (c6[1] >> 4) ))

  # The next operations depends on the c6 elements number.
  case ${#c6[*]} in
    3) (( c8[1] = ( (c6[1] & 15) << 4) | (c6[2] >> 2) ))
       (( c8[2] = (c6[2] & 3) << 6 )); unset c8[2] ;;
    4) (( c8[1] = ( (c6[1] & 15) << 4) | (c6[2] >> 2) ))
       (( c8[2] = ( (c6[2] &  3) << 6) |  c6[3] )) ;;
  esac

  for char in ${c8[*]}; do
     printf "\x$(printf "%x" ${char})"
  done
}


# main ()

if [ "$1" = "-d" ]; then   # decode

  # Reformat STDIN in pseudo 4x6-bit groups.
  content=$(cat - | tr -d "\n" | sed -r "s/(.{4})/\1 /g")

  for chars in ${content}; do decode_base64 ${chars}; done

else
  # Make a hexdump of stdin and reformat in 3-byte groups.
  content=$(cat - | xxd -ps -u | sed -r "s/(\w{6})/\1 /g" |
            tr -d "\n")

  for chars in ${content}; do encode_base64 ${chars}; done

  echo

fi
```

tessh reads bytes as bytes (`sys.stdin.buffer.read`), so there's no hex
dump: indexing a `bytes` gives the numbers, and `bytes $list` turns
numbers back into bytes. The bit operations are the original's. Here
`main` is given made-up data rather than stdin, and the result is checked
against Python's own `base64` module, which is what a tessh script would
use in practice (`base64.b64encode`, `base64.urlsafe_b64encode`).

```tessh
import sys string
# base64.sh: Base64 encoding and decoding.
# Copyright (c) 2011 vladz <vladz@devzero.fr>
#  Encode or decode original Base64 (and also Base64url)
#+ from STDIN to STDOUT.
#    Encode: base64.tsh < binary-file > binary-file.base64
#    Decode: base64.tsh -d < binary-file.base64 > binary-file
#  Reference: RFC4648, http://tools.ietf.org/html/rfc4648#section-5

# The whole base64 charset, and additionally the character "=" ...
base64_charset = $string.ascii_uppercase + $string.ascii_lowercase + $string.digits + "+/="
### base64_charset = $string.ascii_uppercase + $string.ascii_lowercase + $string.digits + "-_="
###   (base64url encoding instead of original base64.)
text_width = 64   # Output text width when encoding (as openssl).

def encode_base64 group
  # Encode three 8-bit bytes into four 6-bit numbers.
  c8 = (list $group) + [0 0]      # (Missing bytes are 0, as in bash.)
  c6 = [($c8[0] >> 2), ((($c8[0] & 3) << 4) | ($c8[1] >> 4)),
        ((($c8[1] & 15) << 2) | ($c8[2] >> 6)), ($c8[2] & 63)]
  # The last group may be short: "=" (64) fills it out.
  match (len $group)
    case 2
      $c6[3] = 64
    case 1
      $c6[2] = 64 ; $c6[3] = 64
  "".join [$base64_charset[$c] for c in $c6]

def decode_base64 chars
  # Decode four base64 characters into up to three bytes.
  c6 = [($base64_charset.index $c) for c in $chars if $c != "="]
  c8 = [((($c6[0] << 2) | ($c6[1] >> 4)) & 255)]
  if (len $c6) >= 3
    $c8.append (((($c6[1] & 15) << 4) | ($c6[2] >> 2)) & 255)
  if (len $c6) == 4
    $c8.append (((($c6[2] & 3) << 6) | $c6[3]) & 255)
  bytes $c8

def encode data                   # Bytes to base64 text, in 3-byte groups.
  "".join [(encode_base64 $data[$i : $i + 3]) for i in (range 0 (len $data) 3)]

def decode text                   # Base64 text to bytes, in 4-character groups.
  text = $text.replace "\n" ""
  b"".join [(decode_base64 $text[$i : $i + 4]) for i in (range 0 (len $text) 4)]

def main args data
  if $args[:1] == [-d]            # decode
    sys.stdout.flush
    sys.stdout.buffer.write (decode (.decode $data))
    sys.stdout.buffer.flush
  else                            # encode, text_width characters a line
    text = encode $data
    for i in (range 0 (len $text) $text_width)
      print $text[$i : $i + $text_width]

# The script proper ends with:   main $argv[1:] (sys.stdin.buffer.read)
# (Here it's given made-up data in place of stdin.)
main [] b"Hello, world!\n"
main [-d] b"SGVsbG8sIHdvcmxkIQo=\n"

# (And every byte value, checked against Python's own base64 module.)
import base64
every = bytes (range 256)
main [] $every[:96]
print ((encode $every) == (.decode (base64.b64encode $every))) ((decode (encode $every)) == $every)
```

```output
SGVsbG8sIHdvcmxkIQo=
Hello, world!
AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4v
MDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5f
True True
```

## Inserting text with sed
<!-- help: abs-sedappend sedappend.sh sed-i insert prepend -->

A short one: `sed -i "1i text"` puts a line at the top of each file whose
name ends in `sample`.

### Example A-55. Inserting text in a file using sed (`sedappend.sh`)

```bash no-run
#!/bin/bash
#  Prepends a string at a specified line
#+ in files with names ending in "sample"
#+ in the current working directory.
#  000000000000000000000000000000000000
#  This script overwrites files!
#  Be careful running it in a directory
#+ where you have important files!!!
#  000000000000000000000000000000000000

#  Create a couple of files to operate on ...
#  01sample
#  02sample
#  ... etc.
#  These files must not be empty, else the prepend will not work.

lineno=1            # Append at line 1 (prepend).
filespec="*sample"  # Filename pattern to operate on.

string=$(whoami)    # Will set your username as string to insert.
                    # It could just as easily be any other string.

for file in $filespec # Specify which files to alter.
do #        ^^^^^^^^^
 sed -i ""$lineno"i "$string"" $file
#    ^^ -i option edits files in-place.
#                 ^ Insert (i) command.
 echo ""$file" altered!"
done

echo "Warning: files possibly clobbered!"

exit 0

# Exercise:
# Add error checking to this script.
# It needs it badly.
```

`sed` is the same program in tessh, and `*sample` written bare is a Glob,
read again when the loop runs. `"${lineno}i $string"` glues the line
number to the `i` command, which the original did with its odd quoting.
Two made-up files to work on, and a fixed string where the original used
your user name.

```tessh
#  Prepends a string at a specified line
#+ in files with names ending in "sample"
#+ in the current working directory.
#  This script overwrites files! Be careful where you run it.

# (Made-up files to try it on. They must not be empty.)
printf "first line\nsecond line\n" >01sample
printf "only line\n" >02sample

lineno = 1             # Append at line 1 (prepend).
filespec = *sample     # Filename pattern to operate on: a Glob.
string = "ABS reader"  # ($(whoami), your user name, in the original.)

for file in $filespec  # Specify which files to alter.
  sed -i "${lineno}i $string" $file    # -i edits in place; i inserts.
  print "$file altered!"

print "Warning: files possibly clobbered!"
head 01sample 02sample # (To see what it did.)
exit 0
```

```output
01sample altered!
02sample altered!
Warning: files possibly clobbered!
==> 01sample <==
ABS reader
first line
second line

==> 02sample <==
ABS reader
only line
```

## The Gronsfeld cipher
<!-- help: abs-gronsfeld gronsfeld.bash cipher vigenere encrypt -->

The Gronsfeld cipher (a GPL3 script from 2011 in the ABS Guide's
collection) is a Vigenère cipher with only ten alphabets: each letter
shifts forward by the matching letter of a ten-letter key, in groups of
five, and the text ends with `29379`, which marks it for decryption.

### Example A-56. The Gronsfeld Cipher (`gronsfeld.bash`)

```bash no-run
#!/bin/bash
# gronsfeld.bash

# License: GPL3
# Reldate 06/23/11

#  This is an implementation of the Gronsfeld Cipher.
#  It's essentially a stripped-down variant of the 
#+ polyalphabetic Vigenère Tableau, but with only 10 alphabets.
#  The classic Gronsfeld has a numeric sequence as the key word,
#+ but here we substitute a letter string, for ease of use.
#  Allegedly, this cipher was invented by the eponymous Count Gronsfeld
#+ in the 17th Century. It was at one time considered to be unbreakable.
#  Note that this is ###not### a secure cipher by modern standards.

#  Global Variables  #
Enc_suffix="29379"   #  Encrypted text output with this 5-digit suffix. 
                     #  This functions as a decryption flag,
                     #+ and when used to generate passwords adds security.
Default_key="gronsfeldk"
                     #  The script uses this if key not entered below
                     #  (at "Keychain").
                     #  Change the above two values frequently
                     #+ for added security.

GROUPLEN=5           #  Output in groups of 5 letters, per tradition.
alpha1=( abcdefghijklmnopqrstuvwxyz )
alpha2=( {A..Z} )    #  Output in all caps, per tradition.
                     #  Use   alpha2=( {a..z} )   for password generator.
wraplen=26           #  Wrap around if past end of alphabet.
dflag=               #  Decrypt flag (set if $Enc_suffix present).
E_NOARGS=76          #  Missing command-line args?
DEBUG=77             #  Debugging flag.
declare -a offsets   #  This array holds the numeric shift values for
                     #+ encryption/decryption.

########Keychain#########
key=  ### Put key here!!!
      # 10 characters!
#########################



# Function
: ()
{ # Encrypt or decrypt, depending on whether $dflag is set.
  # Why ": ()" as a function name? Just to prove that it can be done.

  local idx keydx mlen off1 shft
  local plaintext="$1"
  local mlen=${#plaintext}

for (( idx=0; idx<$mlen; idx++ ))
do
  let "keydx = $idx % $keylen"
  shft=${offsets[keydx]}

  if [ -n "$dflag" ]
  then                  # Decrypt!
    let "off1 = $(expr index "${alpha1[*]}" ${plaintext:idx:1}) - $shft"
    # Shift backward to decrypt.
  else                  # Encrypt!
    let "off1 = $(expr index "${alpha1[*]}" ${plaintext:idx:1}) + $shft"
    # Shift forward to encrypt.
    test $(( $idx % $GROUPLEN)) = 0 && echo -n " "  # Groups of 5 letters.
    #  Comment out above line for output as a string without whitespace,
    #+ for example, if using the script as a password generator.
  fi

  ((off1--))   # Normalize. Why is this necessary?

      if [ $off1 -lt 0 ]
      then     # Catch negative indices.
        let "off1 += $wraplen"
      fi

  ((off1 %= $wraplen))   # Wrap around if past end of alphabet.

  echo -n "${alpha2[off1]}"

done

  if [ -z "$dflag" ]
  then
    echo " $Enc_suffix"
#   echo "$Enc_suffix"  # For password generator.
  else
    echo
  fi
} # End encrypt/decrypt function.



# int main () {

# Check for command-line args.
if [ -z "$1" ]
then
   echo "Usage: $0 TEXT TO ENCODE/DECODE"
   exit $E_NOARGS
fi

if [ ${!#} == "$Enc_suffix" ]
#    ^^^^^ Final command-line arg.
then
  dflag=ON
  echo -n "+"           # Flag decrypted text with a "+" for easy ID.
fi

if [ -z "$key" ]
then
  key="$Default_key"    # "gronsfeldk" per above.
fi

keylen=${#key}

for (( idx=0; idx<$keylen; idx++ ))
do  # Calculate shift values for encryption/decryption.
  offsets[idx]=$(expr index "${alpha1[*]}" ${key:idx:1})   # Normalize.
  ((offsets[idx]--))  #  Necessary because "expr index" starts at 1,
                      #+ whereas array count starts at 0.
  # Generate array of numerical offsets corresponding to the key.
  # There are simpler ways to accomplish this.
done

args=$(echo "$*" | sed -e 's/ //g' | tr A-Z a-z | sed -e 's/[0-9]//g')
# Remove whitespace and digits from command-line args.
# Can modify to also remove punctuation characters, if desired.

         # Debug:
         # echo "$args"; exit $DEBUG

: "$args"               # Call the function named ":".
# : is a null operator, except . . . when it's a function name!

exit $?    # } End-of-script


#   **************************************************************   #
#   This script can function as a  password generator,
#+  with several minor mods, see above.
#   That would allow an easy-to-remember password, even the word
#+ "password" itself, which encrypts to vrgfotvo29379
#+  a fairly secure password not susceptible to a dictionary attack.
#   Or, you could use your own name (surely that's easy to remember!).
#   For example, Bozo Bozeman encrypts to hfnbttdppkt29379.
#   **************************************************************   #
```

bash's function is named `:`, "just to prove that it can be done"; a tessh
function needs a Python name, so it's `crypt`, and it gives its result back
rather than echoing it a letter at a time. `.find` + 1 is `expr index` (0
for a character not in the alphabet), `%` wraps around the alphabet (Python's
`%` is never negative, so the extra check for a negative index isn't
needed), and one regex removes the spaces and digits. With no arguments it
encrypts a sample, decrypts the result and encrypts "password", which
gives `vrgfotvo` as the original's notes say.

```tessh
import re
# gronsfeld.bash: an implementation of the Gronsfeld Cipher.
# License: GPL3. Reldate 06/23/11
#  A stripped-down variant of the polyalphabetic Vigenère Tableau,
#+ but with only 10 alphabets, and a letter string for the key here,
#+ for ease of use. Not a secure cipher by modern standards.

Enc_suffix = "29379"     # Encrypted text output with this 5-digit suffix:
                         # it functions as a decryption flag.
Default_key = gronsfeldk # The script uses this if key not entered below.
GROUPLEN = 5             # Output in groups of 5 letters, per tradition.
alpha1 = abcdefghijklmnopqrstuvwxyz
alpha2 = ABCDEFGHIJKLMNOPQRSTUVWXYZ   # Output in all caps, per tradition.
wraplen = 26             # Wrap around if past end of alphabet.
E_NOARGS = 76

########Keychain#########
key = ""  ### Put key here!!! (10 characters!)
#########################

def crypt plaintext offsets decrypt
  # Encrypt or decrypt, depending on decrypt.
  # (bash names this function ":", just to prove that it can be done;
  #  a tessh name has to be a Python name.)
  out = ""
  for [idx c] in (enumerate $plaintext)
    shft = $offsets[$idx % (len $offsets)]
    pos = ($alpha1.find $c) + 1   # 1-based, 0 if not a letter (as expr index).
    if $decrypt
      off1 = $pos - $shft         # Shift backward to decrypt.
    else
      off1 = $pos + $shft         # Shift forward to encrypt.
      if $idx % $GROUPLEN == 0
        out += " "                # Groups of 5 letters.
    out += $alpha2[($off1 - 1) % $wraplen]   # Normalize, wrap around.
  if $decrypt then $out else "$out $Enc_suffix"

def main args
  if not $args
    print "Usage: $(Path $argv[0]).name TEXT TO ENCODE/DECODE"
    exit $E_NOARGS
  dflag = $args[-1] == $Enc_suffix   # Final command-line arg.
  k = $key or $Default_key
  # The shift values for encryption/decryption, from the key.
  offsets = [($alpha1.index $c) for c in $k]
  # Remove whitespace and digits from command-line args.
  text = re'[\s0-9]'.sub "" (.lower (" ".join $args))
  print (if $dflag then "+" else "") (crypt $text $offsets $dflag) sep=""

# (Sample arguments, when none are given: three runs.)
if $argv[1:]
  main $argv[1:]
else
  main [Hello World]
  main [NVZYG BSCON "29379"]
  main [password]
```

```output
 NVZYG BSCON 29379
+HELLOWORLD
 VRGFO TVO 29379
```

## Bingo
<!-- help: abs-bingo bingo.sh bingo random no-duplicates -->

A bingo caller: each key calls a new number from 1 to 75, never one called
before, until `q`; then it prints which numbers were called, by column.

### Example A-57. Bingo Number Generator (`bingo.sh`)

```bash no-run
#!/bin/bash
# bingo.sh
# Bingo number generator
# Reldate 20Aug12, License: Public Domain

#######################################################################
# This script generates bingo numbers.
# Hitting a key generates a new number.
# Hitting 'q' terminates the script.
# In a given run of the script, there will be no duplicate numbers.
# When the script terminates, it prints a log of the numbers generated.
#######################################################################

MIN=1       # Lowest allowable bingo number.
MAX=75      # Highest allowable bingo number.
COLS=15     # Numbers in each column (B I N G O).
SINGLE_DIGIT_MAX=9

declare -a Numbers
Prefix=(B I N G O)

initialize_Numbers ()
{  # Zero them out to start.
   # They'll be incremented if chosen.
   local index=0
   until [ "$index" -gt $MAX ]
   do
     Numbers[index]=0
     ((index++))
   done

   Numbers[0]=1   # Flag zero, so it won't be selected.
}


generate_number ()
{
   local number

   while [ 1 ]
   do
     let "number = $(expr $RANDOM % $MAX)"
     if [ ${Numbers[number]} -eq 0 ]    # Number not yet called.
     then
       let "Numbers[number]+=1"         # Flag it in the array.
       break                            # And terminate loop.
     fi   # Else if already called, loop and generate another number.
   done
   # Exercise: Rewrite this more elegantly as an until-loop.

   return $number
}


print_numbers_called ()
{   # Print out the called number log in neat columns.
    # echo ${Numbers[@]}

local pre2=0                #  Prefix a zero, so columns will align
                            #+ on single-digit numbers.

echo "Number Stats"

for (( index=1; index<=MAX; index++))
do
  count=${Numbers[index]}
  let "t = $index - 1"      # Normalize, since array begins with index 0.
  let "column = $(expr $t / $COLS)"
  pre=${Prefix[column]}
# echo -n "${Prefix[column]} "

if [ $(expr $t % $COLS) -eq 0 ]
then
  echo   # Newline at end of row.
fi

  if [ "$index" -gt $SINGLE_DIGIT_MAX ]  # Check for single-digit number.
  then
    echo -n "$pre$index#$count "
  else    # Prefix a zero.
    echo -n "$pre$pre2$index#$count "
  fi

done
}



# main () {
RANDOM=$$   # Seed random number generator.

initialize_Numbers   # Zero out the number tracking array.

clear
echo "Bingo Number Caller"; echo

while [[ "$key" != "q" ]]   # Main loop.
do
  read -s -n1 -p "Hit a key for the next number [q to exit] " key
  # Usually 'q' exits, but not always.
  # Can always hit Ctl-C if q fails.
  echo

  generate_number; new_number=$?

  let "column = $(expr $new_number / $COLS)"
  echo -n "${Prefix[column]} "   # B-I-N-G-O

  echo $new_number
done

echo; echo

# Game over ...
print_numbers_called
echo; echo "[#0 = not called . . . #1 = called]"

echo

exit 0
# }


# Certainly, this script could stand some improvement.
#See also the author's Instructable:
#www.instructables.com/id/Binguino-An-Arduino-based-Bingo-Number-Generato/
```

`read -s -n1` reads one key without echoing it; here the keys are canned
(eight, then `q`; as in bash, `q` still calls a number before the loop
ends) and `random` is seeded. For the real keys, see the stopwatch's
`keys` above. The call log is a list of counts, as bash's array is, and
`"${index:02d}"` pads the single digits. Two of the original's slips are
kept: `$RANDOM % 75` never calls 75, and `number / 15` puts 15, 30, 45 and
60 in the next column (here "O 60"). `random.sample (range 1 76) 75`
would give a whole game's calls, in order, without either.

```tessh
import sys random
# bingo.sh: Bingo number generator
# Reldate 20Aug12, License: Public Domain
# Hitting a key generates a new number; hitting 'q' terminates the script,
# which then prints a log of the numbers generated. No duplicates.

MIN = 1          # Lowest allowable bingo number.
MAX = 75         # Highest allowable bingo number.
COLS = 15        # Numbers in each column (B I N G O).
Prefix = [B I N G O]

# How often each number was called; 0 is flagged, so it won't be selected.
Numbers = [1] + [0] * $MAX

def generate_number
  while True
    number = random.randrange $MAX
    if $Numbers[$number] == 0     # Number not yet called.
      $Numbers[$number] += 1      # Flag it.
      return $number

def print_numbers_called
  # Print out the called number log in neat columns.
  print "Number Stats"
  for index in (range 1 ($MAX + 1))
    t = $index - 1
    if $t % $COLS == 0
      print                       # Newline at end of row.
    print "$Prefix[$t // $COLS]${index:02d}#$Numbers[$index]" end=" "

random.seed 2012                  # (RANDOM=$$ in the original.)
# (The keys hit, canned: eight keys, then q.)
keys = iter "        q"
def clear
  if (sys.stdout.isatty)
    \clear

clear
print "Bingo Number Caller" ; print
key = ""
while $key != q                   # Main loop.
  print "Hit a key for the next number [q to exit] " end=""
  key = next $keys                # (read -s -n1, in the original)
  print
  new_number = (generate_number)
  print $Prefix[$new_number // $COLS] $new_number   # B-I-N-G-O
print ; print
# Game over ...
print_numbers_called
print ; print "[#0 = not called . . . #1 = called]"
print
exit 0
```

```output
Bingo Number Caller

Hit a key for the next number [q to exit] 
B 12
Hit a key for the next number [q to exit] 
O 60
Hit a key for the next number [q to exit] 
G 46
Hit a key for the next number [q to exit] 
I 24
Hit a key for the next number [q to exit] 
N 42
Hit a key for the next number [q to exit] 
O 74
Hit a key for the next number [q to exit] 
B 1
Hit a key for the next number [q to exit] 
O 65
Hit a key for the next number [q to exit] 
N 39


Number Stats

B01#1 B02#0 B03#0 B04#0 B05#0 B06#0 B07#0 B08#0 B09#0 B10#0 B11#0 B12#1 B13#0 B14#0 B15#0 
I16#0 I17#0 I18#0 I19#0 I20#0 I21#0 I22#0 I23#0 I24#1 I25#0 I26#0 I27#0 I28#0 I29#0 I30#0 
N31#0 N32#0 N33#0 N34#0 N35#0 N36#0 N37#0 N38#0 N39#1 N40#0 N41#0 N42#1 N43#0 N44#0 N45#0 
G46#1 G47#0 G48#0 G49#0 G50#0 G51#0 G52#0 G53#0 G54#0 G55#0 G56#0 G57#0 G58#0 G59#0 G60#1 
O61#0 O62#0 O63#0 O64#0 O65#1 O66#0 O67#0 O68#0 O69#0 O70#0 O71#0 O72#0 O73#0 O74#1 O75#0 
[#0 = not called . . . #1 = called]

```

## Basics reviewed
<!-- help: abs-basicsreviewed basics-reviewed.bash review arrays parameter-expansion ifs sparse -->

To end its contributed scripts, the ABS Guide reviews the basics: Michael
Zick's tour of bash variables, arrays, quoting, functions and parameter
expansion (edited by the ABS Guide's author, with a section on arrays and
IFS by Cliff Bamford). Its output is long and partly bash's quirks; it
prints the script's path near the end, and makes one intentional "command
not found" error.

### Example A-58. Basics Reviewed (`basics-reviewed.bash`)

```bash no-run
#!/bin/bash
# basics-reviewed.bash

# File extension == *.bash == specific to Bash

#   Copyright (c) Michael S. Zick, 2003; All rights reserved.
#   License: Use in any form, for any purpose.
#   Revision: $ID$
#
#              Edited for layout by M.C.
#   (author of the "Advanced Bash Scripting Guide")
#   Fixes and updates (04/08) by Cliff Bamford.


#  This script tested under Bash versions 2.04, 2.05a and 2.05b.
#  It may not work with earlier versions.
#  This demonstration script generates one --intentional--
#+ "command not found" error message. See line 436.

#  The current Bash maintainer, Chet Ramey, has fixed the items noted
#+ for later versions of Bash.



        ###-------------------------------------------###
        ###  Pipe the output of this script to 'more' ###
        ###+ else it will scroll off the page.        ###
        ###                                           ###
        ###  You may also redirect its output         ###
        ###+ to a file for examination.               ###  
        ###-------------------------------------------###



#  Most of the following points are described at length in
#+ the text of the foregoing "Advanced Bash Scripting Guide."
#  This demonstration script is mostly just a reorganized presentation.
#      -- msz

# Variables are not typed unless otherwise specified.

#  Variables are named. Names must contain a non-digit.
#  File descriptor names (as in, for example: 2>&1)
#+ contain ONLY digits.

# Parameters and Bash array elements are numbered.
# (Parameters are very similar to Bash arrays.)

# A variable name may be undefined (null reference).
unset VarNull

# A variable name may be defined but empty (null contents).
VarEmpty=''                         # Two, adjacent, single quotes.

# A variable name may be defined and non-empty.
VarSomething='Literal'

# A variable may contain:
#   * A whole number as a signed 32-bit (or larger) integer
#   * A string
# A variable may also be an array.

#  A string may contain embedded blanks and may be treated
#+ as if it where a function name with optional arguments.

#  The names of variables and the names of functions
#+ are in different namespaces.


#  A variable may be defined as a Bash array either explicitly or
#+ implicitly by the syntax of the assignment statement.
#  Explicit:
declare -a ArrayVar



# The echo command is a builtin.
echo $VarSomething

# The printf command is a builtin.
# Translate %s as: String-Format
printf %s $VarSomething         # No linebreak specified, none output.
echo                            # Default, only linebreak output.




# The Bash parser word breaks on whitespace.
# Whitespace, or the lack of it is significant.
# (This holds true in general; there are, of course, exceptions.)




# Translate the DOLLAR_SIGN character as: Content-Of.

# Extended-Syntax way of writing Content-Of:
echo ${VarSomething}

#  The ${ ... } Extended-Syntax allows more than just the variable
#+ name to be specified.
#  In general, $VarSomething can always be written as: ${VarSomething}.

# Call this script with arguments to see the following in action.



#  Outside of double-quotes, the special characters @ and *
#+ specify identical behavior.
#  May be pronounced as: All-Elements-Of.

#  Without specification of a name, they refer to the
#+ pre-defined parameter Bash-Array.



# Glob-Pattern references
echo $*                         # All parameters to script or function
echo ${*}                       # Same

# Bash disables filename expansion for Glob-Patterns.
# Only character matching is active.


# All-Elements-Of references
echo $@                         # Same as above
echo ${@}                       # Same as above




#  Within double-quotes, the behavior of Glob-Pattern references
#+ depends on the setting of IFS (Input Field Separator).
#  Within double-quotes, All-Elements-Of references behave the same.


#  Specifying only the name of a variable holding a string refers
#+ to all elements (characters) of a string.


#  To specify an element (character) of a string,
#+ the Extended-Syntax reference notation (see below) MAY be used.




#  Specifying only the name of a Bash array references
#+ the subscript zero element,
#+ NOT the FIRST DEFINED nor the FIRST WITH CONTENTS element.

#  Additional qualification is needed to reference other elements,
#+ which means that the reference MUST be written in Extended-Syntax.
#  The general form is: ${name[subscript]}.

#  The string forms may also be used: ${name:subscript}
#+ for Bash-Arrays when referencing the subscript zero element.


# Bash-Arrays are implemented internally as linked lists,
#+ not as a fixed area of storage as in some programming languages.


#   Characteristics of Bash arrays (Bash-Arrays):
#   --------------------------------------------

#   If not otherwise specified, Bash-Array subscripts begin with
#+  subscript number zero. Literally: [0]
#   This is called zero-based indexing.
###
#   If not otherwise specified, Bash-Arrays are subscript packed
#+  (sequential subscripts without subscript gaps).
###
#   Negative subscripts are not allowed.
###
#   Elements of a Bash-Array need not all be of the same type.
###
#   Elements of a Bash-Array may be undefined (null reference).
#       That is, a Bash-Array may be "subscript sparse."
###
#   Elements of a Bash-Array may be defined and empty (null contents).
###
#   Elements of a Bash-Array may contain:
#     * A whole number as a signed 32-bit (or larger) integer
#     * A string
#     * A string formated so that it appears to be a function name
#     + with optional arguments
###
#   Defined elements of a Bash-Array may be undefined (unset).
#       That is, a subscript packed Bash-Array may be changed
#   +   into a subscript sparse Bash-Array.
###
#   Elements may be added to a Bash-Array by defining an element
#+  not previously defined.
###
# For these reasons, I have been calling them "Bash-Arrays".
# I'll return to the generic term "array" from now on.
#     -- msz


echo "========================================================="

#  Lines 202 - 334 supplied by Cliff Bamford. (Thanks!)
#  Demo --- Interaction with Arrays, quoting, IFS, echo, * and @   ---  
#+ all affect how things work

ArrayVar[0]='zero'                    # 0 normal
ArrayVar[1]=one                       # 1 unquoted literal
ArrayVar[2]='two'                     # 2 normal
ArrayVar[3]='three'                   # 3 normal
ArrayVar[4]='I am four'               # 4 normal with spaces
ArrayVar[5]='five'                    # 5 normal
unset ArrayVar[6]                     # 6 undefined
ArrayValue[7]='seven'                 # 7 normal
ArrayValue[8]=''                      # 8 defined but empty
ArrayValue[9]='nine'                  # 9 normal


echo '--- Here is the array we are using for this test'
echo
echo "ArrayVar[0]='zero'             # 0 normal"
echo "ArrayVar[1]=one                # 1 unquoted literal"
echo "ArrayVar[2]='two'              # 2 normal"
echo "ArrayVar[3]='three'            # 3 normal"
echo "ArrayVar[4]='I am four'        # 4 normal with spaces"
echo "ArrayVar[5]='five'             # 5 normal"
echo "unset ArrayVar[6]              # 6 undefined"
echo "ArrayValue[7]='seven'          # 7 normal"
echo "ArrayValue[8]=''               # 8 defined but empty"
echo "ArrayValue[9]='nine'           # 9 normal"
echo


echo
echo '---Case0: No double-quotes, Default IFS of space,tab,newline ---'
IFS=$'\x20'$'\x09'$'\x0A'            # In exactly this order.
echo 'Here is: printf %q {${ArrayVar[*]}'
printf %q ${ArrayVar[*]}
echo
echo 'Here is: printf %q {${ArrayVar[@]}'
printf %q ${ArrayVar[@]}
echo
echo 'Here is: echo ${ArrayVar[*]}'
echo  ${ArrayVar[@]}
echo 'Here is: echo {${ArrayVar[@]}'
echo ${ArrayVar[@]}

echo
echo '---Case1: Within double-quotes - Default IFS of space-tab- 
newline ---'
IFS=$'\x20'$'\x09'$'\x0A'	    #  These three bytes,
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---Case2: Within double-quotes - IFS is q'
IFS='q'
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---Case3: Within double-quotes - IFS is ^'
IFS='^'
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---Case4: Within double-quotes - IFS is ^ followed by  
space,tab,newline'
IFS=$'^'$'\x20'$'\x09'$'\x0A'       # ^ + space tab newline
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---Case6: Within double-quotes - IFS set and empty '
IFS=''
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---Case7: Within double-quotes - IFS is unset'
unset IFS
echo 'Here is: printf %q "{${ArrayVar[*]}"'
printf %q "${ArrayVar[*]}"
echo
echo 'Here is: printf %q "{${ArrayVar[@]}"'
printf %q "${ArrayVar[@]}"
echo
echo 'Here is: echo "${ArrayVar[*]}"'
echo  "${ArrayVar[@]}"
echo 'Here is: echo "{${ArrayVar[@]}"'
echo "${ArrayVar[@]}"

echo
echo '---End of Cases---'
echo "========================================================="; echo



# Put IFS back to the default.
# Default is exactly these three bytes.
IFS=$'\x20'$'\x09'$'\x0A'           # In exactly this order.

# Interpretation of the above outputs:
#   A Glob-Pattern is I/O; the setting of IFS matters.
###
#   An All-Elements-Of does not consider IFS settings.
###
#   Note the different output using the echo command and the
#+  quoted format operator of the printf command.


#  Recall:
#   Parameters are similar to arrays and have the similar behaviors.
###
#  The above examples demonstrate the possible variations.
#  To retain the shape of a sparse array, additional script
#+ programming is required.
###
#  The source code of Bash has a routine to output the
#+ [subscript]=value   array assignment format.
#  As of version 2.05b, that routine is not used,
#+ but that might change in future releases.



# The length of a string, measured in non-null elements (characters):
echo
echo '- - Non-quoted references - -'
echo 'Non-Null character count: '${#VarSomething}' characters.'

# test='Lit'$'\x00''eral'           # $'\x00' is a null character.
# echo ${#test}                     # See that?



#  The length of an array, measured in defined elements,
#+ including null content elements.
echo
echo 'Defined content count: '${#ArrayVar[@]}' elements.'
# That is NOT the maximum subscript (4).
# That is NOT the range of the subscripts (1 . . 4 inclusive).
# It IS the length of the linked list.
###
#  Both the maximum subscript and the range of the subscripts may
#+ be found with additional script programming.

# The length of a string, measured in non-null elements (characters):
echo
echo '- - Quoted, Glob-Pattern references - -'
echo 'Non-Null character count: '"${#VarSomething}"' characters.'

#  The length of an array, measured in defined elements,
#+ including null-content elements.
echo
echo 'Defined element count: '"${#ArrayVar[*]}"' elements.'

#  Interpretation: Substitution does not effect the ${# ... } operation.
#  Suggestion:
#  Always use the All-Elements-Of character
#+ if that is what is intended (independence from IFS).



#  Define a simple function.
#  I include an underscore in the name
#+ to make it distinctive in the examples below.
###
#  Bash separates variable names and function names
#+ in different namespaces.
#  The Mark-One eyeball isn't that advanced.
###
_simple() {
    echo -n 'SimpleFunc'$@          #  Newlines are swallowed in
}                                   #+ result returned in any case.


# The ( ... ) notation invokes a command or function.
# The $( ... ) notation is pronounced: Result-Of.


# Invoke the function _simple
echo
echo '- - Output of function _simple - -'
_simple                             # Try passing arguments.
echo
# or
(_simple)                           # Try passing arguments.
echo

echo '- Is there a variable of that name? -'
echo $_simple not defined           # No variable by that name.

# Invoke the result of function _simple (Error msg intended)

###
$(_simple)                          # Gives an error message:
#                          line 436: SimpleFunc: command not found
#                          ---------------------------------------

echo
###

#  The first word of the result of function _simple
#+ is neither a valid Bash command nor the name of a defined function.
###
# This demonstrates that the output of _simple is subject to evaluation.
###
# Interpretation:
#   A function can be used to generate in-line Bash commands.


# A simple function where the first word of result IS a bash command:
###
_print() {
    echo -n 'printf %q '$@
}

echo '- - Outputs of function _print - -'
_print parm1 parm2                  # An Output NOT A Command.
echo

$(_print parm1 parm2)               #  Executes: printf %q parm1 parm2
                                    #  See above IFS examples for the
                                    #+ various possibilities.
echo

$(_print $VarSomething)             # The predictable result.
echo



# Function variables
# ------------------

echo
echo '- - Function variables - -'
# A variable may represent a signed integer, a string or an array.
# A string may be used like a function name with optional arguments.

# set -vx                           #  Enable if desired
declare -f funcVar                  #+ in namespace of functions

funcVar=_print                      # Contains name of function.
$funcVar parm1                      # Same as _print at this point.
echo

funcVar=$(_print )                  # Contains result of function.
$funcVar                            # No input, No output.
$funcVar $VarSomething              # The predictable result.
echo

funcVar=$(_print $VarSomething)     #  $VarSomething replaced HERE.
$funcVar                            #  The expansion is part of the
echo                                #+ variable contents.

funcVar="$(_print $VarSomething)"   #  $VarSomething replaced HERE.
$funcVar                            #  The expansion is part of the
echo                                #+ variable contents.

#  The difference between the unquoted and the double-quoted versions
#+ above can be seen in the "protect_literal.sh" example.
#  The first case above is processed as two, unquoted, Bash-Words.
#  The second case above is processed as one, quoted, Bash-Word.




# Delayed replacement
# -------------------

echo
echo '- - Delayed replacement - -'
funcVar="$(_print '$VarSomething')" # No replacement, single Bash-Word.
eval $funcVar                       # $VarSomething replaced HERE.
echo

VarSomething='NewThing'
eval $funcVar                       # $VarSomething replaced HERE.
echo

# Restore the original setting trashed above.
VarSomething=Literal

#  There are a pair of functions demonstrated in the
#+ "protect_literal.sh" and "unprotect_literal.sh" examples.
#  These are general purpose functions for delayed replacement literals
#+ containing variables.





# REVIEW:
# ------

#  A string can be considered a Classic-Array of elements (characters).
#  A string operation applies to all elements (characters) of the string
#+ (in concept, anyway).
###
#  The notation: ${array_name[@]} represents all elements of the
#+ Bash-Array: array_name.
###
#  The Extended-Syntax string operations can be applied to all
#+ elements of an array.
###
#  This may be thought of as a For-Each operation on a vector of strings.
###
#  Parameters are similar to an array.
#  The initialization of a parameter array for a script
#+ and a parameter array for a function only differ
#+ in the initialization of ${0}, which never changes its setting.
###
#  Subscript zero of the script's parameter array contains
#+ the name of the script.
###
#  Subscript zero of a function's parameter array DOES NOT contain
#+ the name of the function.
#  The name of the current function is accessed by the $FUNCNAME variable.
###
#  A quick, review list follows (quick, not short).

echo
echo '- - Test (but not change) - -'
echo '- null reference -'
echo -n ${VarNull-'NotSet'}' '          # NotSet
echo ${VarNull}                         # NewLine only
echo -n ${VarNull:-'NotSet'}' '         # NotSet
echo ${VarNull}                         # Newline only

echo '- null contents -'
echo -n ${VarEmpty-'Empty'}' '          # Only the space
echo ${VarEmpty}                        # Newline only
echo -n ${VarEmpty:-'Empty'}' '         # Empty
echo ${VarEmpty}                        # Newline only

echo '- contents -'
echo ${VarSomething-'Content'}          # Literal
echo ${VarSomething:-'Content'}         # Literal

echo '- Sparse Array -'
echo ${ArrayVar[@]-'not set'}

# ASCII-Art time
# State     Y==yes, N==no
#           -       :-
# Unset     Y       Y       ${# ... } == 0
# Empty     N       Y       ${# ... } == 0
# Contents  N       N       ${# ... } > 0

#  Either the first and/or the second part of the tests
#+ may be a command or a function invocation string.
echo
echo '- - Test 1 for undefined - -'
declare -i t
_decT() {
    t=$t-1
}

# Null reference, set: t == -1
t=${#VarNull}                           # Results in zero.
${VarNull- _decT }                      # Function executes, t now -1.
echo $t

# Null contents, set: t == 0
t=${#VarEmpty}                          # Results in zero.
${VarEmpty- _decT }                     # _decT function NOT executed.
echo $t

# Contents, set: t == number of non-null characters
VarSomething='_simple'                  # Set to valid function name.
t=${#VarSomething}                      # non-zero length
${VarSomething- _decT }                 # Function _simple executed.
echo $t                                 # Note the Append-To action.

# Exercise: clean up that example.
unset t
unset _decT
VarSomething=Literal

echo
echo '- - Test and Change - -'
echo '- Assignment if null reference -'
echo -n ${VarNull='NotSet'}' '          # NotSet NotSet
echo ${VarNull}
unset VarNull

echo '- Assignment if null reference -'
echo -n ${VarNull:='NotSet'}' '         # NotSet NotSet
echo ${VarNull}
unset VarNull

echo '- No assignment if null contents -'
echo -n ${VarEmpty='Empty'}' '          # Space only
echo ${VarEmpty}
VarEmpty=''

echo '- Assignment if null contents -'
echo -n ${VarEmpty:='Empty'}' '         # Empty Empty
echo ${VarEmpty}
VarEmpty=''

echo '- No change if already has contents -'
echo ${VarSomething='Content'}          # Literal
echo ${VarSomething:='Content'}         # Literal


# "Subscript sparse" Bash-Arrays
###
#  Bash-Arrays are subscript packed, beginning with
#+ subscript zero unless otherwise specified.
###
#  The initialization of ArrayVar was one way
#+ to "otherwise specify".  Here is the other way:
###
echo
declare -a ArraySparse
ArraySparse=( [1]=one [2]='' [4]='four' )
# [0]=null reference, [2]=null content, [3]=null reference

echo '- - Array-Sparse List - -'
# Within double-quotes, default IFS, Glob-Pattern

IFS=$'\x20'$'\x09'$'\x0A'
printf %q "${ArraySparse[*]}"
echo

#  Note that the output does not distinguish between "null content"
#+ and "null reference".
#  Both print as escaped whitespace.
###
#  Note also that the output does NOT contain escaped whitespace
#+ for the "null reference(s)" prior to the first defined element.
###
# This behavior of 2.04, 2.05a and 2.05b has been reported
#+ and may change in a future version of Bash.

#  To output a sparse array and maintain the [subscript]=value
#+ relationship without change requires a bit of programming.
#  One possible code fragment:
###
# local l=${#ArraySparse[@]}        # Count of defined elements
# local f=0                         # Count of found subscripts
# local i=0                         # Subscript to test
(                                   # Anonymous in-line function
    for (( l=${#ArraySparse[@]}, f = 0, i = 0 ; f < l ; i++ ))
    do
        # 'if defined then...'
        ${ArraySparse[$i]+ eval echo '\ ['$i']='${ArraySparse[$i]} ; (( f++ )) }
    done
)

# The reader coming upon the above code fragment cold
#+ might want to review "command lists" and "multiple commands on a line"
#+ in the text of the foregoing "Advanced Bash Scripting Guide."
###
#  Note:
#  The "read -a array_name" version of the "read" command
#+ begins filling array_name at subscript zero.
#  ArraySparse does not define a value at subscript zero.
###
#  The user needing to read/write a sparse array to either
#+ external storage or a communications socket must invent
#+ a read/write code pair suitable for their purpose.
###
# Exercise: clean it up.

unset ArraySparse

echo
echo '- - Conditional alternate (But not change)- -'
echo '- No alternate if null reference -'
echo -n ${VarNull+'NotSet'}' '
echo ${VarNull}
unset VarNull

echo '- No alternate if null reference -'
echo -n ${VarNull:+'NotSet'}' '
echo ${VarNull}
unset VarNull

echo '- Alternate if null contents -'
echo -n ${VarEmpty+'Empty'}' '              # Empty
echo ${VarEmpty}
VarEmpty=''

echo '- No alternate if null contents -'
echo -n ${VarEmpty:+'Empty'}' '             # Space only
echo ${VarEmpty}
VarEmpty=''

echo '- Alternate if already has contents -'

# Alternate literal
echo -n ${VarSomething+'Content'}' '        # Content Literal
echo ${VarSomething}

# Invoke function
echo -n ${VarSomething:+ $(_simple) }' '    # SimpleFunc Literal
echo ${VarSomething}
echo

echo '- - Sparse Array - -'
echo ${ArrayVar[@]+'Empty'}                 # An array of 'Empty'(ies)
echo

echo '- - Test 2 for undefined - -'

declare -i t
_incT() {
    t=$t+1
}

#  Note:
#  This is the same test used in the sparse array
#+ listing code fragment.

# Null reference, set: t == -1
t=${#VarNull}-1                     # Results in minus-one.
${VarNull+ _incT }                  # Does not execute.
echo $t' Null reference'

# Null contents, set: t == 0
t=${#VarEmpty}-1                    # Results in minus-one.
${VarEmpty+ _incT }                 # Executes.
echo $t'  Null content'

# Contents, set: t == (number of non-null characters)
t=${#VarSomething}-1                # non-null length minus-one
${VarSomething+ _incT }             # Executes.
echo $t'  Contents'

# Exercise: clean up that example.
unset t
unset _incT

# ${name?err_msg} ${name:?err_msg}
#  These follow the same rules but always exit afterwards
#+ if an action is specified following the question mark.
#  The action following the question mark may be a literal
#+ or a function result.
###
#  ${name?} ${name:?} are test-only, the return can be tested.




# Element operations
# ------------------

echo
echo '- - Trailing sub-element selection - -'

#  Strings, Arrays and Positional parameters

#  Call this script with multiple arguments
#+ to see the parameter selections.

echo '- All -'
echo ${VarSomething:0}              # all non-null characters
echo ${ArrayVar[@]:0}               # all elements with content
echo ${@:0}                         # all parameters with content;
                                    # ignoring parameter[0]

echo
echo '- All after -'
echo ${VarSomething:1}              # all non-null after character[0]
echo ${ArrayVar[@]:1}               # all after element[0] with content
echo ${@:2}                         # all after param[1] with content

echo
echo '- Range after -'
echo ${VarSomething:4:3}            # ral
                                    # Three characters after
                                    # character[3]

echo '- Sparse array gotch -'
echo ${ArrayVar[@]:1:2}     #  four - The only element with content.
                            #  Two elements after (if that many exist).
                            #  the FIRST WITH CONTENTS
                            #+ (the FIRST WITH  CONTENTS is being
                            #+ considered as if it
                            #+ were subscript zero).
#  Executed as if Bash considers ONLY array elements with CONTENT
#  printf %q "${ArrayVar[@]:0:3}"    # Try this one

#  In versions 2.04, 2.05a and 2.05b,
#+ Bash does not handle sparse arrays as expected using this notation.
#
#  The current Bash maintainer, Chet Ramey, has corrected this.


echo '- Non-sparse array -'
echo ${@:2:2}               # Two parameters following parameter[1]

# New victims for string vector examples:
stringZ=abcABC123ABCabc
arrayZ=( abcabc ABCABC 123123 ABCABC abcabc )
sparseZ=( [1]='abcabc' [3]='ABCABC' [4]='' [5]='123123' )

echo
echo ' - - Victim string - -'$stringZ'- - '
echo ' - - Victim array - -'${arrayZ[@]}'- - '
echo ' - - Sparse array - -'${sparseZ[@]}'- - '
echo ' - [0]==null ref, [2]==null ref, [4]==null content - '
echo ' - [1]=abcabc [3]=ABCABC [5]=123123 - '
echo ' - non-null-reference count: '${#sparseZ[@]}' elements'

echo
echo '- - Prefix sub-element removal - -'
echo '- - Glob-Pattern match must include the first character. - -'
echo '- - Glob-Pattern may be a literal or a function result. - -'
echo


# Function returning a simple, Literal, Glob-Pattern
_abc() {
    echo -n 'abc'
}

echo '- Shortest prefix -'
echo ${stringZ#123}                 # Unchanged (not a prefix).
echo ${stringZ#$(_abc)}             # ABC123ABCabc
echo ${arrayZ[@]#abc}               # Applied to each element.

# echo ${sparseZ[@]#abc}            # Version-2.05b core dumps.
# Has since been fixed by Chet Ramey.

# The -it would be nice- First-Subscript-Of
# echo ${#sparseZ[@]#*}             # This is NOT valid Bash.

echo
echo '- Longest prefix -'
echo ${stringZ##1*3}                # Unchanged (not a prefix)
echo ${stringZ##a*C}                # abc
echo ${arrayZ[@]##a*c}              # ABCABC 123123 ABCABC

# echo ${sparseZ[@]##a*c}           # Version-2.05b core dumps.
# Has since been fixed by Chet Ramey.

echo
echo '- - Suffix sub-element removal - -'
echo '- - Glob-Pattern match must include the last character. - -'
echo '- - Glob-Pattern may be a literal or a function result. - -'
echo
echo '- Shortest suffix -'
echo ${stringZ%1*3}                 # Unchanged (not a suffix).
echo ${stringZ%$(_abc)}             # abcABC123ABC
echo ${arrayZ[@]%abc}               # Applied to each element.

# echo ${sparseZ[@]%abc}            # Version-2.05b core dumps.
# Has since been fixed by Chet Ramey.

# The -it would be nice- Last-Subscript-Of
# echo ${#sparseZ[@]%*}             # This is NOT valid Bash.

echo
echo '- Longest suffix -'
echo ${stringZ%%1*3}                # Unchanged (not a suffix)
echo ${stringZ%%b*c}                # a
echo ${arrayZ[@]%%b*c}              # a ABCABC 123123 ABCABC a

# echo ${sparseZ[@]%%b*c}           # Version-2.05b core dumps.
# Has since been fixed by Chet Ramey.

echo
echo '- - Sub-element replacement - -'
echo '- - Sub-element at any location in string. - -'
echo '- - First specification is a Glob-Pattern - -'
echo '- - Glob-Pattern may be a literal or Glob-Pattern function result. - -'
echo '- - Second specification may be a literal or function result. - -'
echo '- - Second specification may be unspecified. Pronounce that'
echo '    as: Replace-With-Nothing (Delete) - -'
echo



# Function returning a simple, Literal, Glob-Pattern
_123() {
    echo -n '123'
}

echo '- Replace first occurrence -'
echo ${stringZ/$(_123)/999}         # Changed (123 is a component).
echo ${stringZ/ABC/xyz}             # xyzABC123ABCabc
echo ${arrayZ[@]/ABC/xyz}           # Applied to each element.
echo ${sparseZ[@]/ABC/xyz}          # Works as expected.

echo
echo '- Delete first occurrence -'
echo ${stringZ/$(_123)/}
echo ${stringZ/ABC/}
echo ${arrayZ[@]/ABC/}
echo ${sparseZ[@]/ABC/}

#  The replacement need not be a literal,
#+ since the result of a function invocation is allowed.
#  This is general to all forms of replacement.
echo
echo '- Replace first occurrence with Result-Of -'
echo ${stringZ/$(_123)/$(_simple)}  # Works as expected.
echo ${arrayZ[@]/ca/$(_simple)}     # Applied to each element.
echo ${sparseZ[@]/ca/$(_simple)}    # Works as expected.

echo
echo '- Replace all occurrences -'
echo ${stringZ//[b2]/X}             # X-out b's and 2's
echo ${stringZ//abc/xyz}            # xyzABC123ABCxyz
echo ${arrayZ[@]//abc/xyz}          # Applied to each element.
echo ${sparseZ[@]//abc/xyz}         # Works as expected.

echo
echo '- Delete all occurrences -'
echo ${stringZ//[b2]/}
echo ${stringZ//abc/}
echo ${arrayZ[@]//abc/}
echo ${sparseZ[@]//abc/}

echo
echo '- - Prefix sub-element replacement - -'
echo '- - Match must include the first character. - -'
echo

echo '- Replace prefix occurrences -'
echo ${stringZ/#[b2]/X}             # Unchanged (neither is a prefix).
echo ${stringZ/#$(_abc)/XYZ}        # XYZABC123ABCabc
echo ${arrayZ[@]/#abc/XYZ}          # Applied to each element.
echo ${sparseZ[@]/#abc/XYZ}         # Works as expected.

echo
echo '- Delete prefix occurrences -'
echo ${stringZ/#[b2]/}
echo ${stringZ/#$(_abc)/}
echo ${arrayZ[@]/#abc/}
echo ${sparseZ[@]/#abc/}

echo
echo '- - Suffix sub-element replacement - -'
echo '- - Match must include the last character. - -'
echo

echo '- Replace suffix occurrences -'
echo ${stringZ/%[b2]/X}             # Unchanged (neither is a suffix).
echo ${stringZ/%$(_abc)/XYZ}        # abcABC123ABCXYZ
echo ${arrayZ[@]/%abc/XYZ}          # Applied to each element.
echo ${sparseZ[@]/%abc/XYZ}         # Works as expected.

echo
echo '- Delete suffix occurrences -'
echo ${stringZ/%[b2]/}
echo ${stringZ/%$(_abc)/}
echo ${arrayZ[@]/%abc/}
echo ${sparseZ[@]/%abc/}

echo
echo '- - Special cases of null Glob-Pattern - -'
echo

echo '- Prefix all -'
# null substring pattern means 'prefix'
echo ${stringZ/#/NEW}               # NEWabcABC123ABCabc
echo ${arrayZ[@]/#/NEW}             # Applied to each element.
echo ${sparseZ[@]/#/NEW}            # Applied to null-content also.
                                    # That seems reasonable.

echo
echo '- Suffix all -'
# null substring pattern means 'suffix'
echo ${stringZ/%/NEW}               # abcABC123ABCabcNEW
echo ${arrayZ[@]/%/NEW}             # Applied to each element.
echo ${sparseZ[@]/%/NEW}            # Applied to null-content also.
                                    # That seems reasonable.

echo
echo '- - Special case For-Each Glob-Pattern - -'
echo '- - - - This is a nice-to-have dream - - - -'
echo

_GenFunc() {
    echo -n ${0}                    # Illustration only.
    # Actually, that would be an arbitrary computation.
}

# All occurrences, matching the AnyThing pattern.
# Currently //*/ does not match null-content nor null-reference.
# /#/ and /%/ does match null-content but not null-reference.
echo ${sparseZ[@]//*/$(_GenFunc)}


#  A possible syntax would be to make
#+ the parameter notation used within this construct mean:
#   ${1} - The full element
#   ${2} - The prefix, if any, to the matched sub-element
#   ${3} - The matched sub-element
#   ${4} - The suffix, if any, to the matched sub-element
#
# echo ${sparseZ[@]//*/$(_GenFunc ${3})}   # Same as ${1} here.
# Perhaps it will be implemented in a future version of Bash.


exit 0
```

tessh's answer to each point follows, in the script's order, as short
scripts of its own.

**Variables.** Values have types (Python's); a name that was never set, or
was `del`eted, is an error to read, not an empty string. `None` is the
value for "nothing". Functions and variables are in one namespace, so a
function's name is a variable holding the function: bash's "is there a
variable named `_simple`?" is yes in tessh.

```tessh
# Values have types: str, int, float, list, dict, None...
VarEmpty = ''                     # Defined but empty (null contents).
VarSomething = 'Literal'          # Defined and non-empty.
VarNull = None                    # Defined as "no value" (Python's null).
del VarNull                       # Undefined again (bash's unset).

print $VarSomething
printf %s $VarSomething           # The printf program: no newline asked, none output.
echo
print "${VarSomething}"           # ${…} is for gluing, in a string.
print (type $VarSomething).__name__ (type 42).__name__ (type (2 ** 100)).__name__ (type [1 2]).__name__

# An undefined name is an error (after looking for an environment variable).
try
  print $VarNull
except NameError as e
  print NameError: $e

# Functions and variables share one namespace, Python's.
def _simple = "SimpleFunc"
print (callable $_simple)         # The name is the function.
```

```output
Literal
Literal
Literal
str int int list
NameError: name 'VarNull' is not defined
True
```

**Parameters.** The arguments are the list `$argv` (`$argv[0]` is the
script). `"$*"` is `" ".join`, `$@` is the list itself, and a list given to
a program is one argument per item, never split again. Strings index and
slice as lists do.

```tessh
args = $argv[1:] or [one "two words" three]   # (sample arguments, when none are given)
print (" ".join $args)            # "$*": one string, joined with spaces
print *$args                      # $@: each argument its own word to print
printf "<%s>" $args               # A list gives a program one argument per item,
echo                              #+ never split again, as "$@" does.
print $args[0] $args[-1] (len $args)   # Indexes (negative too), and $#.
s = $args[1]
print $s[0] $s[-5:] (len $s)      # A string's characters, the same way.
```

```output
one two words three
one two words three
<one><two words><three>
one three 3
t words 9
```

**Arrays and IFS.** A list is packed, can hold any types, and takes
negative indexes. Cliff Bamford's cases show how quoting and IFS change
`${ArrayVar[*]}` and `${ArrayVar[@]}`. tessh has no IFS: a list stays a
list until you join it, with the separator you name, and a string stays
one argument until you split it. (The original also sets `ArrayValue[7..9]`
where it means `ArrayVar`, so the array has six elements, there and here.)
`printf %q` is the coreutils program here, which quotes with `'…'` where
bash's builtin uses backslashes.

```tessh
# A list is packed: items 0, 1, 2…, no gaps; indexes may be negative.
ArrayVar = [zero one two three "I am four" five]
# (The original sets ArrayValue[7..9], not ArrayVar: a typo, so its
#  array has 6 elements too.)
print $ArrayVar
print (len $ArrayVar) $ArrayVar[-1] [1 two 3.0 None]   # Any types, mixed.

# There's no IFS: text is joined, and split, where you say so.
print "--- printf %q of each item, as printf %q \"\${ArrayVar[@]}\" (any IFS)"
printf %q $ArrayVar ; echo
print "--- joined with IFS's first character, then quoted: printf %q \"\${ArrayVar[*]}\""
for sep in [" " q "^" ""]
  printf %q ($sep.join $ArrayVar) ; echo
print "--- bash's unquoted \${ArrayVar[*]}: joined, then split again"
printf %q ((" ".join $ArrayVar).split) ; echo
print "--- echo \"\${ArrayVar[@]}\" (any IFS)"
echo $ArrayVar
print (" ".join $ArrayVar)

# Lengths.
VarSomething = Literal
print "Non-Null character count: $(len $VarSomething) characters."
print "Defined content count: $(len $ArrayVar) elements."
```

```output
['zero' 'one' 'two' 'three' 'I am four' 'five']
6 five [1 'two' 3.0 None]
--- printf %q of each item, as printf %q "${ArrayVar[@]}" (any IFS)
zeroonetwothree'I am four'five
--- joined with IFS's first character, then quoted: printf %q "${ArrayVar[*]}"
'zero one two three I am four five'
'zeroqoneqtwoqthreeqI am fourqfive'
'zero^one^two^three^I am four^five'
'zeroonetwothreeI am fourfive'
--- bash's unquoted ${ArrayVar[*]}: joined, then split again
zeroonetwothreeIamfourfive
--- echo "${ArrayVar[@]}" (any IFS)
zero one two three I am four five
zero one two three I am four five
Non-Null character count: 7 characters.
Defined content count: 6 elements.
```

**Functions and their results.** bash's `$(func)` is what the function
prints, and a line that starts with it runs that text as a command. A
tessh function gives back a value, and text becomes a command only on
purpose: `shlex.split` it into words and hand them to `env` (the program,
which runs the program named first). A function is a value, so
`funcVar = $_print` holds the function, and "delayed replacement", bash's
`eval` of a quoted `'$VarSomething'`, is a function (here an `fn`) that
reads the variable when it's called.

**Gap:** tessh can't run a program whose name is in a variable directly
(bash's `$funcVar parm1`): `$p args` calls `$p` as a Python function, and
`*$cmd` alone is an error. `env $p args` and `env *$cmd` do it.

```tessh
import shlex
VarSomething = Literal

# A function's result is what it gives back, not what it prints.
def _simple *args = "SimpleFunc" + (" ".join $args)
print '- - Output of function _simple - -'
print (_simple)
print '- Is there a variable of that name? -'
print $_simple.__name__           # Yes: the function itself.

# bash runs the text $(_simple) prints as a command. tessh runs text as a
# command only when told to: split it into words, and run them with env
# (the program: it runs the program named first).
env *(shlex.split (_simple))   # Gives an error message (on stderr), as intended.

def _print *args = "printf %q " + (" ".join $args)
print '- - Outputs of function _print - -'
print (_print parm1 parm2)        # A result, not a command.
env *(shlex.split (_print parm1 parm2)) ; echo   # Runs: printf %q parm1 parm2
env *(shlex.split (_print $VarSomething)) ; echo

# Function variables: a function is a value like any other.
print '- - Function variables - -'
funcVar = $_print                 # The function itself.
print (funcVar parm1)
funcVar = (_print)                # Its result, a string.
env *(shlex.split $funcVar) ; echo    # printf %q with nothing to quote
env *(shlex.split $funcVar) $VarSomething ; echo
funcVar = _print $VarSomething    # $VarSomething replaced HERE.
env *(shlex.split $funcVar) ; echo

# Delayed replacement: bash keeps '$VarSomething' in the text and evals it
# later. In tessh, a function (here a fn) runs the code later.
print '- - Delayed replacement - -'
later = fn [] (_print $VarSomething)
env *(shlex.split (later)) ; echo     # $VarSomething replaced HERE.
VarSomething = NewThing
env *(shlex.split (later)) ; echo
VarSomething = Literal
```

```output
- - Output of function _simple - -
SimpleFunc
- Is there a variable of that name? -
_simple
- - Outputs of function _print - -
printf %q parm1 parm2
parm1parm2
Literal
- - Function variables - -
printf %q parm1

Literal
Literal
- - Delayed replacement - -
Literal
NewThing
```

**Defaults and alternates.** `${x-default}`, `${x:-default}`,
`${x=default}`, `${x:=default}`, `${x+alt}` and `${x:+alt}` tell an unset
variable from an empty one. In tessh, "unset" is `None` and "empty" is
`''`, `is None` tells them apart and `or` treats both as false. The
"default" can be code, as in bash's tests that run `_decT` and `_incT`;
running the function named in a string means looking it up (`globals`).
`${x?message}` is raising an exception.

```tessh
VarNull = None                    # The null reference.
VarEmpty = ''                     # Null contents.
VarSomething = Literal            # Contents.

# ${x-default}: when it's None.  ${x:-default}: when it's None or empty.
print '- - Test (but not change) - -'
for [x name] in [[$VarNull null] [$VarEmpty empty] [$VarSomething contents]]
  print "- $name -" (if $x is None then 'Default' else $x) ($x or 'Default') sep=" | "

# State     is None   not x   len
# None      True      True    (no len: a TypeError)
# ''        False     True    0
# 'Literal' False     False   7

# The "default" can run code: bash's ${VarNull- _decT } runs _decT.
print '- - Test 1 for undefined - -'
t = 0
def _decT
  global t
  t -= 1
if $VarNull is None
  _decT
print $t                          # -1
t = len $VarEmpty
if $VarEmpty is None
  _decT                           # Not run.
print $t                          # 0
# bash then runs the *contents* of VarSomething='_simple' as a command;
# tessh looks a function up by name only if you ask: globals.
def _simple = print SimpleFunc end=""
VarSomething = _simple
t = len $VarSomething
if $VarSomething is not None
  f = (globals)[$VarSomething]
  f
print $t
VarSomething = Literal

# ${x=default} and ${x:=default}: set it, too.
print '- - Test and Change - -'
if $VarNull is None
  VarNull = NotSet
print $VarNull $VarNull
VarEmpty = $VarEmpty or 'Empty'
print $VarEmpty $VarEmpty
VarSomething = $VarSomething or 'Content'
print $VarSomething
d = {}                            # A dict has it built in:
print ($d.setdefault key NotSet) ($d.get other NotSet) $d

# ${x+alt} and ${x:+alt}: the alternate when it is set (and not empty).
VarNull = None
VarEmpty = ''
print '- - Conditional alternate (But not change)- -'
for [x name] in [[$VarNull null] [$VarEmpty empty] [$VarSomething contents]]
  print "- $name -" (if $x is not None then 'Alt' else '') (if $x then 'Alt' else '') sep=" | "

print '- - Test 2 for undefined - -'
def _incT
  global t
  t += 1
for [x what] in [[$VarNull "Null reference"] [$VarEmpty " Null content"] [$VarSomething " Contents"]]
  t = (len ($x or '')) - 1
  if $x is not None
    _incT
  print "$t $what"

# ${x?message} and ${x:?message}: stop with the message. In tessh, raise.
try
  if $VarNull is None
    raise (ValueError "VarNull: parameter null or not set")
except ValueError as e
  print $e
```

```output
- - Test (but not change) - -
- null - | Default | Default
- empty - |  | Default
- contents - | Literal | Literal
- - Test 1 for undefined - -
-1
0
SimpleFunc7
- - Test and Change - -
NotSet NotSet
Empty Empty
Literal
NotSet NotSet {key: 'NotSet'}
- - Conditional alternate (But not change)- -
- null - |  | 
- empty - | Alt | 
- contents - | Alt | Alt
- - Test 2 for undefined - -
-1 Null reference
0  Null content
7  Contents
VarNull: parameter null or not set
```

**Sparse arrays and slices.** bash arrays may have gaps (`[1]=one
[2]='' [4]=four`), and printing one loses the subscripts; a dict keeps
them, and tells "no element" (`in`, `.get`) from an empty one. Slices
(`${VarSomething:4:3}`, `${ArrayVar[@]:1:2}`, `${@:2}`) are Python's,
`[start:end]`, the same for strings, lists and `$argv`, with no sparse
gotcha.

```tessh
import shlex
# A sparse array is a dict: only the keys that are there are there.
ArraySparse = {1: one 2: '' 4: four}   # [0] and [3]: null references
print '- - Array-Sparse List - -'
print (shlex.quote (" ".join ($ArraySparse.values)))
for [i v] in ($ArraySparse.items)      # [subscript]=value, kept.
  print " [$i]=$v"
print (0 in $ArraySparse) (2 in $ArraySparse) ($ArraySparse.get 3)

# Slices: strings, lists and arguments alike.
VarSomething = Literal
ArrayVar = [zero one two three "I am four" five]
args = $argv[1:] or [a b c d]         # (sample arguments, when none are given)
print '- All -'
print $VarSomething[0:] $ArrayVar[0:] $args[0:]
print '- All after -'
print $VarSomething[1:] $ArrayVar[1:] $args[1:]
print '- Range after -'
print $VarSomething[4:7] $ArrayVar[1:3] $args[1:3]   # ral; no sparse gotcha
```

```output
- - Array-Sparse List - -
'one  four'
 [1]=one
 [2]=
 [4]=four
False True None
- All -
Literal ['zero' 'one' 'two' 'three' 'I am four' 'five'] ['a' 'b' 'c' 'd']
- All after -
iteral ['one' 'two' 'three' 'I am four' 'five'] ['b' 'c' 'd']
- Range after -
ral ['one' 'two'] ['b' 'c']
```

**Taking off a prefix or a suffix.** `#`, `##`, `%` and `%%` remove the
shortest or longest match of a glob from the front or back, of a string
or of each element. Python's `str` has `.removeprefix` and
`.removesuffix` for literal text; for a glob, `fnmatchcase` tried on each
prefix (or suffix) does it, shortest or longest first. `[… for x in
$arrayZ]` applies it to each element. (Note `"123123"`: written bare, it
would be a number. And where a result is empty, joining shows it as an
extra space, which bash's unquoted `echo` drops.)

```tessh
from fnmatch import fnmatchcase
# New victims for string vector examples:
stringZ = abcABC123ABCabc
arrayZ = [abcabc ABCABC "123123" ABCABC abcabc]   # ("123123" quoted: bare, it's a number.)
sparseZ = {1: abcabc 3: ABCABC 4: '' 5: "123123"}

print " - - Victim string - -$stringZ- - "
print " - - Victim array - -$(' '.join $arrayZ)- - "
print " - - Sparse array - -$(' '.join ($sparseZ.values))- - "
print " - non-null-reference count: $(len $sparseZ) elements"

# bash's #, ##, % and %% take a glob off the front or the back, the
# shortest or the longest match. As tessh functions, with fnmatch:
def cut_prefix s pat longest=False
  ends = if $longest then (range (len $s) -1 -1) else (range ((len $s) + 1))
  for i in $ends
    if (fnmatchcase $s[:$i] $pat)
      return $s[$i:]
  $s
def cut_suffix s pat longest=False
  starts = if $longest then (range ((len $s) + 1)) else (range (len $s) -1 -1)
  for i in $starts
    if (fnmatchcase $s[$i:] $pat)
      return $s[:$i]
  $s

def _abc = "abc"                  # A function's result as the pattern.
print '- Shortest prefix -'
print (cut_prefix $stringZ "123")
print (cut_prefix $stringZ (_abc))
print (" ".join [(cut_prefix $x abc) for x in $arrayZ])   # Applied to each element.
print '- Longest prefix -'
print (cut_prefix $stringZ "1*3" longest=True)
print (cut_prefix $stringZ "a*C" longest=True)
print (" ".join [(cut_prefix $x "a*c" longest=True) for x in $arrayZ])   # (Two empty ones.)
print '- Shortest suffix -'
print (cut_suffix $stringZ "1*3")
print (cut_suffix $stringZ (_abc))
print (" ".join [(cut_suffix $x abc) for x in $arrayZ])
print '- Longest suffix -'
print (cut_suffix $stringZ "1*3" longest=True)
print (cut_suffix $stringZ "b*c" longest=True)
print (" ".join [(cut_suffix $x "b*c" longest=True) for x in $arrayZ])
# (For a literal prefix or suffix, str has its own: .removeprefix .removesuffix)
print ($stringZ.removeprefix abc) ($stringZ.removesuffix abc)
```

```output
 - - Victim string - -abcABC123ABCabc- - 
 - - Victim array - -abcabc ABCABC 123123 ABCABC abcabc- - 
 - - Sparse array - -abcabc ABCABC  123123- - 
 - non-null-reference count: 4 elements
- Shortest prefix -
abcABC123ABCabc
ABC123ABCabc
abc ABCABC 123123 ABCABC abc
- Longest prefix -
abcABC123ABCabc
abc
 ABCABC 123123 ABCABC 
- Shortest suffix -
abcABC123ABCabc
abcABC123ABC
abc ABCABC 123123 ABCABC abc
- Longest suffix -
abcABC123ABCabc
a
a ABCABC 123123 ABCABC a
ABC123ABCabc abcABC123ABC
```

**Replacing.** `${s/old/new}` and `${s//old/new}` are `.replace` (with a
count of 1 for the first only); a pattern such as `[b2]`, or one anchored
at the start (`/#`) or the end (`/%`), is a regex: `re'[b2]'.sub`,
`re'^abc'.sub`, `re'abc$'.sub`. The replacement can be any value, a
function's result included. A small `each` applies a function to each
element and joins the results as `echo` would. These match bash's output
line for line.

```tessh
stringZ = abcABC123ABCabc
arrayZ = [abcabc ABCABC "123123" ABCABC abcabc]
sparseZ = {1: abcabc 3: ABCABC 4: '' 5: "123123"}
def _123 = "123"
def _abc = "abc"
def _simple = "SimpleFunc"
# Applied to each element, the results joined as echo shows them
# (bash's unquoted echo drops the empty ones).
def each f xs = " ".join [$r for r in (map $f $xs) if $r]

# ${s/old/new} is .replace with a count of 1; ${s//old/new} is .replace.
# A glob such as [b2] is a regex here: re'[b2]'.sub.
print '- Replace first occurrence -'
print ($stringZ.replace (_123) "999" 1)
print ($stringZ.replace ABC xyz 1)
print (each (fn ($%.replace ABC xyz 1)) $arrayZ)
print (each (fn ($%.replace ABC xyz 1)) ($sparseZ.values))
print '- Delete first occurrence -'
print ($stringZ.replace (_123) "" 1)
print ($stringZ.replace ABC "" 1)
print (each (fn ($%.replace ABC "" 1)) $arrayZ)
print (each (fn ($%.replace ABC "" 1)) ($sparseZ.values))
print '- Replace first occurrence with Result-Of -'
print ($stringZ.replace (_123) (_simple) 1)
print (each (fn ($%.replace ca (_simple) 1)) $arrayZ)
print (each (fn ($%.replace ca (_simple) 1)) ($sparseZ.values))
print '- Replace all occurrences -'
print (re'[b2]'.sub X $stringZ)
print ($stringZ.replace abc xyz)
print (each (fn ($%.replace abc xyz)) $arrayZ)
print (each (fn ($%.replace abc xyz)) ($sparseZ.values))
print '- Delete all occurrences -'
print (re'[b2]'.sub "" $stringZ)
print ($stringZ.replace abc "")
print (each (fn ($%.replace abc "")) $arrayZ)
print (each (fn ($%.replace abc "")) ($sparseZ.values))

# ${s/#old/new} and ${s/%old/new}: only at the start, or the end.
print '- Replace prefix occurrences -'
print (re'^[b2]'.sub X $stringZ)
print (re'^abc'.sub XYZ $stringZ)
print (each (fn (re'^abc'.sub XYZ $%)) $arrayZ)
print (each (fn (re'^abc'.sub XYZ $%)) ($sparseZ.values))
print '- Delete prefix occurrences -'
print (re'^[b2]'.sub "" $stringZ)
print ($stringZ.removeprefix (_abc))
print (each (fn ($%.removeprefix abc)) $arrayZ)
print (each (fn ($%.removeprefix abc)) ($sparseZ.values))
print '- Replace suffix occurrences -'
print (re'[b2]$'.sub X $stringZ)
print (re'abc$'.sub XYZ $stringZ)
print (each (fn (re'abc$'.sub XYZ $%)) $arrayZ)
print (each (fn (re'abc$'.sub XYZ $%)) ($sparseZ.values))
print '- Delete suffix occurrences -'
print (re'[b2]$'.sub "" $stringZ)
print ($stringZ.removesuffix (_abc))
print (each (fn ($%.removesuffix abc)) $arrayZ)
print (each (fn ($%.removesuffix abc)) ($sparseZ.values))

# ${s/#/NEW} and ${s/%/NEW}: an empty pattern, so just glue it on.
print '- Prefix all -'
print ("NEW" + $stringZ)
print (each (fn ("NEW" + $%)) $arrayZ)
print (" ".join [("NEW" + $v) for v in ($sparseZ.values)])   # Null content too.
print '- Suffix all -'
print ($stringZ + "NEW")
print (each (fn ($% + "NEW")) $arrayZ)
print (" ".join [($v + "NEW") for v in ($sparseZ.values)])
```

```output
- Replace first occurrence -
abcABC999ABCabc
abcxyz123ABCabc
abcabc xyzABC 123123 xyzABC abcabc
abcabc xyzABC 123123
- Delete first occurrence -
abcABCABCabc
abc123ABCabc
abcabc ABC 123123 ABC abcabc
abcabc ABC 123123
- Replace first occurrence with Result-Of -
abcABCSimpleFuncABCabc
abSimpleFuncbc ABCABC 123123 ABCABC abSimpleFuncbc
abSimpleFuncbc ABCABC 123123
- Replace all occurrences -
aXcABC1X3ABCaXc
xyzABC123ABCxyz
xyzxyz ABCABC 123123 ABCABC xyzxyz
xyzxyz ABCABC 123123
- Delete all occurrences -
acABC13ABCac
ABC123ABC
ABCABC 123123 ABCABC
ABCABC 123123
- Replace prefix occurrences -
abcABC123ABCabc
XYZABC123ABCabc
XYZabc ABCABC 123123 ABCABC XYZabc
XYZabc ABCABC 123123
- Delete prefix occurrences -
abcABC123ABCabc
ABC123ABCabc
abc ABCABC 123123 ABCABC abc
abc ABCABC 123123
- Replace suffix occurrences -
abcABC123ABCabc
abcABC123ABCXYZ
abcXYZ ABCABC 123123 ABCABC abcXYZ
abcXYZ ABCABC 123123
- Delete suffix occurrences -
abcABC123ABCabc
abcABC123ABC
abc ABCABC 123123 ABCABC abc
abc ABCABC 123123
- Prefix all -
NEWabcABC123ABCabc
NEWabcabc NEWABCABC NEW123123 NEWABCABC NEWabcabc
NEWabcabc NEWABCABC NEW NEW123123
- Suffix all -
abcABC123ABCabcNEW
abcabcNEW ABCABCNEW 123123NEW ABCABCNEW abcabcNEW
abcabcNEW ABCABCNEW NEW 123123NEW
```

**The "nice-to-have dream".** The script ends wishing for a replacement
computed from each match, with the parts of the element around it. That
is Python's `re.sub` with a function as the replacement: it gets each
match, and the match has the whole string and where it starts and ends.

```tessh
sparseZ = {1: abcabc 3: ABCABC 4: '' 5: "123123"}

# The "nice-to-have dream": a replacement computed from each match, given
# the whole element, the part before the match, the match and the part after.
# A function as re.sub's replacement gets each match, and all of that with it.
def _GenFunc m
  whole = $m.string
  start = ($m.start) ; end = ($m.end)
  "<$whole[0:$start]|$($m[0].upper)|$whole[$end:]>"

for v in ($sparseZ.values)
  print (repr (re'b.'.sub $_GenFunc $v))
```

```output
'a<a|BC|abc>a<abca|BC|>'
'ABCABC'
''
'123123'
```
