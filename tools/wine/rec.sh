#!/bin/bash
# rec.sh ROLL CELL OUTDIR — escolhe o adesivo, coloca em (330,280) e grava 6 s
export DISPLAY=:99
r=$1; k=$2; out=$3; mkdir -p $out; rm -f $out/*.png
cx=$((54 + 52*(r-1)))
W=/home/claude/wfa.sh
$W click $cx 79 2
if [ $k -ge 9 ]; then for i in $(seq 1 $((k-8))); do $W click $((cx+9)) 458 1; done; kk=8; else kk=$k; fi
$W click $cx $((119 + 39*kk)) 1.5
xdotool mousemove $((${CX:-330}+192)) $((${CY:-280}+144))
ffmpeg -loglevel error -f x11grab -draw_mouse 0 -video_size 640x480 -framerate 15 -i :99+192,144 -t ${RECT:-10} $out/f%03d.png &
sleep 0.6
xdotool click 1
wait
$W click 279 30 1.5
xdotool mousemove 900 700
import -window root -crop 640x480+192+144 +repage $out/base.png
