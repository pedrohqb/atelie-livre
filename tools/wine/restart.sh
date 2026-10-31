#!/bin/bash
# reinicia o Fine Artist no Xvfb e vai direto ao ateliê, ferramenta de adesivos
W=/home/claude/wfa.sh
WINEPREFIX=/root/wine-fa wineserver -k 2>/dev/null; sleep 2
pgrep -x Xvfb >/dev/null || { (Xvfb :99 -screen 0 1024x768x24 >/dev/null 2>&1 &); sleep 2; }
$W start 30; $W click 158 206 2; $W click 332 226 15; $W click 78 173 20
$W click 129 30 5
