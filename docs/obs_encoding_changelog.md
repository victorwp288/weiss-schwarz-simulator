# Observation Encoding Changelog

## Version 1
- Added public layout with fixed header + two player blocks.
- Added reason bits (phase/resource/target gating).
- Added reveal history buffer (last 8 revealed card ids for the observing player; see `REVEAL_HISTORY_LEN`).
- Added context bits (priority window, choice active, stack non-empty, encore pending).
