# Appendix K. Localization
<!-- help: abs-localization abs-k localization gettext translation i18n locale -->

*This appendix is by Stéphane Chazelas, with changes by Alfredo Pironti
and Bruno Haible, maintainer of GNU gettext.*

A localized script prints its messages in the language of the user's
locale: German in Berlin, Germany, English in Berlin, Maryland. In bash
that's GNU `gettext`: each message goes through `gettext` (or
`eval_gettext`, when it holds a variable), `xgettext` collects the
messages into a `.po` file, a translator fills in the translations, and
`msgfmt` compiles it into the `.mo` file the script finds through
`$TEXTDOMAIN` and `$TEXTDOMAINDIR`.

```bash no-run
#!/bin/bash
# localized.sh
#  Script by Stéphane Chazelas,
#+ modified by Bruno Haible, bugfixed by Alfredo Pironti.

. gettext.sh

E_CDERROR=65

error()
{
printf "$@" >&2
exit $E_CDERROR
}

cd $var || error "`eval_gettext \"Can\'t cd to \\\$var.\"`"
#  The triple backslashes (escapes) in front of $var needed
#+ "because eval_gettext expects a string
#+ where the variable values have not yet been substituted."
#    -- per Bruno Haible
read -p "`gettext \"Enter the value: \"`" var
#  ...
```

(The original goes on with Alfredo Pironti's notes on `xgettext`'s quirks:
it takes only the first word after `gettext`, so `gettext -s "…"` and
`gettext -- "…"` extract the wrong string, and a message that starts with
`-` needs a `\0` in front.)

A translation, `fr.po`:

```text
#: a:6
msgid "Can't cd to $var."
msgstr "Impossible de se positionner dans le repertoire $var."

#: a:7
msgid "Enter the value: "
msgstr "Entrez la valeur : "
```

`msgfmt -o localized.sh.mo fr.po` compiles it, and the `.mo` goes in
`/usr/local/share/locale/fr/LC_MESSAGES`, with `TEXTDOMAINDIR` and
`TEXTDOMAIN` set in the script.

tessh uses Python's `gettext` module, which reads the same `.mo` files.
`_ = translation.gettext` is the usual short name, and a message with a
value in it is translated first and filled in after, with `.format` (or
`%`), so the translator sees `{var}`, not its value. Here a French
catalog is made in a local folder, and the script runs once in French and
once in a language that has none, which falls back to the English text.

```tessh
import gettext sys
# (A French catalog, compiled into ./locale, to try it on.)
mkdir -p locale/fr/LC_MESSAGES
cat >fr.po <<'END'
msgid "Can't cd to {var}."
msgstr "Impossible de se positionner dans le repertoire {var}."

msgid "Enter the value: "
msgstr "Entrez la valeur : "
END
msgfmt -o locale/fr/LC_MESSAGES/localized.mo fr.po

E_CDERROR = 65

def run_in language
  t = gettext.translation "localized" "locale" languages=([$language]) fallback=True   # (in parentheses: a bug, for now)
  _ = $t.gettext
  var = /no/such/folder
  cd $var 2>/dev/null || print ((_ "Can't cd to {var}.").format var=$var)
  print (_ "Enter the value: ")

run_in fr
run_in de
```

```output
Impossible de se positionner dans le repertoire /no/such/folder.
Entrez la valeur : 
Can't cd to /no/such/folder.
Enter the value: 
```

On a real system the language comes from the locale (`$LANG`,
`$LC_MESSAGES`) when `languages` isn't given, and the catalogs from
`/usr/share/locale`. To collect a tessh script's messages, `xgettext
--language=Shell -k_ -o messages.pot script.tsh` reads `_ "…"` as a
command `_` with its argument (its Python mode wants `_("…")`, which tessh
doesn't write).
