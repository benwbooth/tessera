# tessh

tessh is [Tessera](../README.md)'s shell language: you write it like a shell,
and underneath it's Python, so every Python library works in it. It runs as
an interactive shell with its own line editor, as scripts (`tessh file.tsh`,
`#!/usr/bin/env tessh`), and as a Jupyter kernel in Tessera's notebooks.

## Install

- **One file, nothing else needed:** `tessh-x86_64-linux`, `tessh-aarch64-linux`,
  `tessh-x86_64-macos` or `tessh-aarch64-macos` from the
  [latest release](https://github.com/benwbooth/tessera/releases/latest)
  (development builds: [tessh-dev](https://github.com/benwbooth/tessera/releases/tag/tessh-dev)).
  Make it executable and put it on your `PATH`. Python comes with it, set up
  on the first run.
- **With Nix:** `nix run github:benwbooth/tessera#tessh`.
- **With Tessera:** it's included.

## Documentation

- **[The tessh guide](doc/guide.md):** the whole language, a part at a time,
  each with an example and what it prints (every example is run by the
  tests). In tessh, `help` shows it a topic at a time (`help pipes`).
- **[The language reference](../docs/TESSH.md):** how tessh reads and runs,
  for the details.
- **[Advanced tessh Scripting Guide](doc/advanced/00-introduction.md):** Mendel
  Cooper's *Advanced Bash-Scripting Guide*, ported: every example in bash and
  in tessh side by side, to see how a bash habit is written in tessh. In
  tessh, `help advanced`, or a chapter with `help abs-loops` and the like.

### Advanced tessh Scripting Guide: chapters

- [Introduction](doc/advanced/00-introduction.md)
- [1. Shell Programming!](doc/advanced/01-why-shell.md)
- [2. Starting Off With a Sha-Bang](doc/advanced/02-sha-bang.md)
- [3. Special Characters](doc/advanced/03-special-chars.md)
- [4. Introduction to Variables and Parameters](doc/advanced/04-variables.md)
- [5. Quoting](doc/advanced/05-quoting.md)
- [6. Exit and Exit Status](doc/advanced/06-exit-status.md)
- [7. Tests](doc/advanced/07-tests.md)
- [8. Operations and Related Topics](doc/advanced/08-operations.md)
- [9. Another Look at Variables](doc/advanced/09-variables2.md)
- [10. Manipulating Variables](doc/advanced/10-manipulatingvars.md)
- [11. Loops and Branches](doc/advanced/11-loops.md)
- [12. Command Substitution](doc/advanced/12-commandsub.md)
- [13. Arithmetic Expansion](doc/advanced/13-arithexp.md)
- [14. Recess Time](doc/advanced/14-recess-time.md)
- [15. Internal Commands and Builtins](doc/advanced/15-internal.md)
- [16. External Filters, Programs and Commands](doc/advanced/16-external.md)
- [16.5 File and Archiving Commands](doc/advanced/16b-external.md)
- [17. System and Administrative Commands](doc/advanced/17-system.md)
- [18. Regular Expressions](doc/advanced/18-regexp.md)
- [19. Here Documents](doc/advanced/19-here-docs.md)
- [20. I/O Redirection](doc/advanced/20-io-redirection.md)
- [21. Subshells](doc/advanced/21-subshells.md)
- [22. Restricted Shells](doc/advanced/22-restricted-sh.md)
- [23. Process Substitution](doc/advanced/23-process-sub.md)
- [24. Functions](doc/advanced/24-functions.md)
- [25. Aliases](doc/advanced/25-aliases.md)
- [26. List Constructs](doc/advanced/26-list-cons.md)
- [27. Arrays](doc/advanced/27-arrays.md)
- [28. Indirect References](doc/advanced/28-ivr.md)
- [29. /dev and /proc](doc/advanced/29-devproc.md)
- [30. Network Programming](doc/advanced/30-networkprogramming.md)
- [31. Of Zeros and Nulls](doc/advanced/31-zeros.md)
- [32. Debugging](doc/advanced/32-debugging.md)
- [33. Options](doc/advanced/33-options.md)
- [34. Gotchas](doc/advanced/34-gotchas.md)
- [35. Scripting With Style](doc/advanced/35-scrstyle.md)
- [36. Miscellany](doc/advanced/36-miscellany.md)
- [37. Bash, versions 2, 3, and 4](doc/advanced/37-bash2.md)
- [38. Endnotes](doc/advanced/38-endnotes.md)
- [Appendix A. Contributed Scripts](doc/advanced/A1-contributed-scripts.md)
- [Spammer identification](doc/advanced/A2-contributed-scripts.md)
- [A PAD file generator](doc/advanced/A3-contributed-scripts.md)
- [Appendix B. Reference Cards](doc/advanced/B-refcards.md)
- [Appendix C. A Sed and Awk Micro-Primer](doc/advanced/C-sedawk.md)
- [Appendix D. Parsing and Managing Pathnames](doc/advanced/D-pathmanagement.md)
- [Appendix E. Exit Codes With Special Meanings](doc/advanced/E-exitcodes.md)
- [Appendix F. A Detailed Introduction to I/O and I/O Redirection](doc/advanced/F-ioredirintro.md)
- [Appendix G. Command-Line Options](doc/advanced/G-command-line-options.md)
- [Appendix H. Important Files](doc/advanced/H-files.md)
- [Appendix I. Important System Directories](doc/advanced/I-systemdirs.md)
- [Appendix J. An Introduction to Programmable Completion](doc/advanced/J-tabexpansion.md)
- [Appendix K. Localization](doc/advanced/K-localization.md)
- [Appendix L. History Commands](doc/advanced/L-histcommands.md)
- [Appendix M. Sample .bashrc and .bash_profile Files](doc/advanced/M-sample-bashrc.md)
- [Appendix N. Converting DOS Batch Files to Shell Scripts](doc/advanced/N-dosbatch.md)
- [Appendix O. Exercises](doc/advanced/O-exercises.md)
- [Appendix T. ASCII Table](doc/advanced/T-asciitable.md)
- [Gaps: what tessh can't do yet](doc/advanced/Z-gaps.md)
