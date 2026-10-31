#!/bin/bash
# wfa.sh start | click X Y | key K | type TEXT | shot NAME  — coordenadas lógicas 640x480 do Fine Artist original
export DISPLAY=:99
OX=192; OY=144
case "$1" in
  start) WINEPREFIX=/root/wine-fa wineserver -k 2>/dev/null; sleep 1
         cd /root/wine-fa/drive_c/MSKIDS && (env -i HOME=/root PATH=/usr/bin:/bin DISPLAY=:99 WINEPREFIX=/root/wine-fa WINEDEBUG=-all wine ARTIST.EXE >/tmp/artist.log 2>&1 &); sleep ${2:-18};;
  click) xdotool mousemove $(( $2 + OX )) $(( $3 + OY )) click 1; sleep ${4:-3};;
  key)   xdotool key "$2"; sleep ${3:-2};;
  type)  xdotool type --delay 80 "$2"; sleep 1;;
  shot)  import -window root -crop 640x480+$OX+$OY +repage /home/claude/atelie-livre/scratch/orig_$2.png;;
esac
