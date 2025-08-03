## Includes only major changes (manual updates)
# Version 0.5.5
- diagnostics moved to italic style
- scroll on modals (autocomplete/info)
- modal cleanup on context switching
- modal mouse support
- added set lsp to pallet
- added fisual effect (arrow) on tree scroll
- moved popupchoice buffer from string to text_field
- added session support

# Version 0.5.4
- dep version bump

# Version 0.5.3
- fixed cursor showing after embeds

# Version 0.5.2
- fix paste of multiline clip

# Version 0.5.1
- split off tui componenets into idiom_tui crate
- fixed EditType::Multi apply and apply_rev
- fixed snippet parsing of the Type { data: ${1:()} }
- fix mass replace on find and replace popup
- added on mouse clip drop for multiple popups (find, repalce, go to line, etc...)

# Version 0.5.0
* git integration
* embeded tui apps
* imporved terminal integration

# Version 0.4.9
* added context menus (right click) on editor/tree
* idiom mark linked to file tree
* editor mode linked to file tree
* added background color to tree instead of borders
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