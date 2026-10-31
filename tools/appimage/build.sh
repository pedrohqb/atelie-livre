#!/bin/bash
# Gera um AppImage do Ateliê Livre com os dados originais embutidos — SÓ PARA
# TESTE LOCAL: o resultado contém os arquivos do Fine Artist/Creative Writer,
# que não são livres, e não pode ser distribuído.
#
# uso: tools/appimage/build.sh PASTA_MSKIDS [SAIDA.AppImage]
#
# Precisa de: cargo, mksquashfs (squashfs-tools), curl (baixa o runtime do
# AppImage de https://github.com/AppImage/type2-runtime na primeira vez).
set -euo pipefail
MSKIDS=${1:?pasta MSKIDS}
OUT=${2:-Atelie_Livre-x86_64.AppImage}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$(mktemp -d)
APPDIR=$WORK/AppDir
trap 'rm -rf "$WORK"' EXIT

echo ">> compilando (release)"
(cd "$ROOT" && cargo build --release -p app -q)

echo ">> montando AppDir"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/atelie-livre" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp "$ROOT/target/release/atelie-livre" "$APPDIR/usr/bin/"
cp -r "$MSKIDS" "$APPDIR/usr/share/atelie-livre/MSKIDS"
cp "$ROOT/tools/appimage/atelie-livre.desktop" "$APPDIR/"
cp "$ROOT/tools/appimage/atelie-livre.desktop" "$APPDIR/usr/share/applications/"
python3 "$ROOT/tools/appimage/icon.py" "$APPDIR/atelie-livre.png"
cp "$APPDIR/atelie-livre.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/"
cat > "$APPDIR/AppRun" <<'R'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
export ATELIE_DATA="$HERE/usr/share/atelie-livre/MSKIDS"
exec "$HERE/usr/bin/atelie-livre" "$@"
R
chmod +x "$APPDIR/AppRun"

RUNTIME=$ROOT/target/appimage-runtime-x86_64
if [ ! -s "$RUNTIME" ]; then
  echo ">> baixando o runtime do AppImage"
  curl -sSL -o "$RUNTIME" https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64
fi

echo ">> empacotando"
mksquashfs "$APPDIR" "$WORK/app.squashfs" -root-owned -noappend -comp gzip -quiet -no-progress
cat "$RUNTIME" "$WORK/app.squashfs" > "$OUT"
chmod +x "$OUT"
echo "pronto: $OUT ($(du -h "$OUT" | cut -f1))"
echo "dados do jogo em execução: ~/.local/share/atelie-livre/ e ~/.config/atelie-livre/"
