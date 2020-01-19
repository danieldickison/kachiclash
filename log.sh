#!/bin/sh

journalctl -u kachiclash -f --lines=1000 -o cat | less -S
