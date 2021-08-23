#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "eo su h7 so go ho h8 ga eu g7 h9 gz s9 s8" --hand "gu hu hk gk g9" --rules "rufspiel eichel von 0" --simulate-hands all --constrain-hands "ea(3)" --branching "9,9" --verbose --prune none --snapcache "9"

# Result confirms that HK will actually lead to win:
# HK:  20  20.00 20  20  20.00 20  20  20.00 20 20 20.00 20
# G9: -20   6.29 20 -20   9.39 20 -20   9.39 20 20 20.00 20
# GK: -20   6.29 20 -20   9.39 20 -20   9.39 20 20 20.00 20
# HU: -20 -19.67 20 -20 -10.42 20 -20 -10.42 20 20 20.00 20
# GU: -20 -19.67 20 -20 -10.42 20 -20 -10.42 20 20 20.00 20
