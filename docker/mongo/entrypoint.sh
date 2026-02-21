#!/bin/bash

tc qdisc add dev eth0 root netem delay 50ms 40ms

mongod --bind_ip_all
