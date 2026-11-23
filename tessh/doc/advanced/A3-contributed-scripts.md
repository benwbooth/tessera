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
