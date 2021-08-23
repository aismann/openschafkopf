#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "h7 ho h9 go eu h8 so su g7 gk ga gz sa g8 sk hu ek ea" --hand "eo ha hz g9" --rules "rufspiel gras von 0" --simulate-hands all --branching "9,9" --verbose --prune none --snapcache "9"

# Result confirms that taking the stich is best - but not with EO.
# HZ: 20 26.86 30 20 26.86 30 20 26.86 30 30 30.00 30
# HA: 20 26.86 30 20 26.86 30 20 26.86 30 30 30.00 30
# G9: 20 22.69 30 20 25.94 30 20 25.94 30 30 30.00 30
# EO: 20 22.14 30 20 22.14 30 20 22.14 30 20 29.96 30

