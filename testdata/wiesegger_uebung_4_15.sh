#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "ha sz hu hk ea ek e8 e7 ez su e9" --hand "eo go so eu sa g8" --rules "schell solo von 1" --simulate-hands 100 --branching "9,9" --verbose --prune none --snapcache "9"

# Result confirms that SA is bad, suggests that G8 may be the best choice.
# G8: -210   50.40  210 -210   50.40  210 -210   50.40  210  210  210.00  210
# EU: -210 -126.00  210 -210 -126.00  210 -210 -126.00  210  210  210.00  210
# SO: -210 -126.00  210 -210 -126.00  210 -210 -126.00  210  210  210.00  210
# GO: -210 -151.20  210 -210 -151.20  210 -210 -151.20  210  210  210.00  210
# EO: -210 -151.20  210 -210 -151.20  210 -210 -151.20  210  210  210.00  210
# SA: -240 -223.80 -210 -240 -223.80 -210 -240 -223.80 -210 -210 -210.00 -210

