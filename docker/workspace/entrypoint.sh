#!/bin/sh

tc qdisc add dev eth0 root netem delay 50ms 40ms

tail -f /dev/null
