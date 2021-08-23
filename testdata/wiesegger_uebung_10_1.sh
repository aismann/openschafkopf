#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "eo eu hu ha h9 hz gu ho hk go so s8 ek e8 e7 ea sa sz s9 e9" --hand "h8 h7 gz" --rules "rufspiel eichel von 0" --simulate-hands all --branching "9,9" --verbose --prune none --snapcache "9"

# Result confirmst that one should play trumpf.
# H7: 50 57.65 70 50 61.10 70 50 61.10 70 60 65.00 70
# H8: 50 57.65 70 50 61.10 70 50 61.10 70 60 65.00 70
# GZ: 50 56.85 70 50 58.62 70 50 58.62 70 50 64.53 70
