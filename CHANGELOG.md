## Includes only major changes

# Version 0.3.0
* dropped ratatui (slugishness on some terminal emulators) - time for normal functioning (frame generation) brough down from ~2 milisec to ~200 nanosecs
* added EditorLine trait, makes struct comparable ot String while wrapping all the data related to the line rendered
* added internal caching for the rendered lines - in most cases the editor will not render anything but the current line
* changed modals to be borderless and use accent color to be separated
* fixes to config files (on initial load all configs will be created, but if some of the theming jsons have missing values will not break the config load)
* fixes to file renaming