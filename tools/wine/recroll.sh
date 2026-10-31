#!/bin/bash
# recroll.sh ROLL [MAX] — grava todos os adesivos do rolo até achar uma casa vazia
r=$1; max=${2:-20}
mkdir -p /home/claude/atelie-livre/scratch/stickers
cd /home/claude/atelie-livre
for k in $(seq 0 $((max-1))); do
  /home/claude/rec.sh $r $k /tmp/rec
  python3 scratch/analyze.py /tmp/rec 330 280 > scratch/stickers/r${r}_k${k}.json
  n=$(python3 -c "import json;d=json.load(open('scratch/stickers/r${r}_k${k}.json'));print(sum(1 for f in d if f))")
  echo "r$r k$k frames_with_sticker=$n"
  if [ "$n" = "0" ]; then rm scratch/stickers/r${r}_k${k}.json; break; fi
done
