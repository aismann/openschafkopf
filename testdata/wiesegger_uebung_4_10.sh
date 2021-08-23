#!/bin/sh

cargo run --release -j16 --bin openschafkopf -- suggest-card --cards-on-table "e8 ea hu su sa hk ez s7 e9 eu e7 gu sk sz eo" --hand "go so ek ga gz" --rules "eichel solo von 0" --simulate-hands all --branching "9,9" --verbose --prune none --snapcache "9"

# Result confirms that one should throw away EK or SO.
# EK: -150 136.69 150 -150 136.69 150 -150 136.69 150 150 150.00 150
# SO: -150 136.69 150 -150 136.69 150 -150 136.69 150 150 150.00 150
# GZ: -150  70.20 150 -150  70.20 150 -150  70.20 150 150 150.00 150
# GA: -150  70.20 150 -150  70.20 150 -150  70.20 150 150 150.00 150
# GO: -150 -31.63 150 -150 -31.63 150 -150 -31.63 150 150 150.00 150
