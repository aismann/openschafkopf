#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "g8 gk hz ga ez e9 ha ea gz sz" --hand "go so ek e8 e7 sa" --rules "rufspiel eichel von 3" --simulate-hands 100 --branching "9,9" --verbose --prune none --snapcache "9"

# Result somewhat confirms that one should not take the stich. Surprisingly, SA seems to be a very good choice here.
# SA:  20 30.80 80  20 31.70 90  20 31.70 90 20 35.00 90
# EK: -20 26.70 80 -20 31.00 90 -20 31.00 90 20 34.90 90
# E7: -20 26.40 80 -20 30.90 90 -20 30.90 90 20 34.90 90
# E8: -20 26.40 80 -20 30.90 90 -20 30.90 90 20 34.90 90
# GO: -20 28.40 80 -20 30.30 90 -20 30.30 90 30 34.80 90
# SO: -20 27.90 80 -20 29.90 90 -20 29.90 90 30 34.80 90

