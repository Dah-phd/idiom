## Includes only major changes (manual updates)
# Version 0.4.9
* added background color to tree
* fixed line render_centered

# Version 0.4.8
* fixed Token::decrement_at - remove before postion (inc is postion after, dec is postion at (not before))

# Version 0.4.7
* popups fixes
* cleanup copy paste going through the terminal
* added editor error logs option to pallet

# Version 0.4.6
* fixed issue with local token modifications
* updates sent to LSP per char
* tokens requested on buffer push

# Version 0.4.4
* Added proper FileWatcher
* Added LocalLSP -> non-lsp functionallity
* imporved rendering and performance
* added command pallet
* added mouse support to popups

# Version 0.3.0
* dropped ratatui (slugishness on some terminal emulators) - time for normal functioning (frame generation) brough down from ~2 milisec to ~200 nanosecs
* added EditorLine trait, makes struct comparable ot String while wrapping all the data related to the line rendered
* added internal caching for the rendered lines - in most cases the editor will not render anything but the current line
* changed modals to be borderless and use accent color to be separated
* fixes to config files (on initial load all configs will be created, but if some of the theming jsons have missing values will not break the config load)
* fixes to file renaming