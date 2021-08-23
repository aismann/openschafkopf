#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "ea ez e7 e8 gk hk g7 gz" --hand "go ho eu h7 sk s7" --rules "rufspiel eichel von 1" --simulate-hands 100 --branching "9,9" --verbose --prune none --snapcache "2,-7"

# Example solution I got from the above command:
# S7: -20  9.10 80 -20 31.70 100 -20 31.70 100 20 39.60 100
# SK: -20  7.50 80 -20 31.30 100 -20 31.30 100 20 39.60 100
# HO: -50  6.70 90 -20 31.00 100 -20 31.00 100 30 40.30 100
# GO: -50  6.70 90 -20 31.00 100 -20 31.00 100 30 40.30 100
# H7: -80 -8.10 80 -20 27.60 100 -20 27.60 100 30 40.40 100
# EU: -50 -0.90 90 -20 25.10 100 -20 25.10 100 30 40.30 100
# Indicating that - even without knowing if player 0 has Schelln or not, S7/SK seems advised

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "ea ez e7 e8 gk hk g7 gz" --hand "go ho eu h7 sk s7" --rules "rufspiel eichel von 1" --simulate-hands 100 --constrain-hands "!s(0)" --branching "9,9" --verbose --prune none --snapcache "2,-7"

# Assuming that 0 has no Schelln, SK/S7 is advised, too.
# SK: -50 18.60  90  20 50.50 110  20 50.50 110 30 52.70 110
# S7: -50 19.90  90  20 49.90 110  20 49.90 110 30 52.70 110
# HO: -50 19.50 110  20 49.60 110  20 49.60 110 30 52.70 110
# GO: -50 19.50 110  20 49.60 110  20 49.60 110 30 52.70 110
# EU: -50  9.90 100 -20 48.00 110 -20 48.00 110 30 52.70 110
# H7: -80 -9.30  90 -20 47.70 110 -20 47.70 110 30 52.70 110
