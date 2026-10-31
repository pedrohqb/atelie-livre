# Ateliê Livre — Subprojeto 2d: todos os rolos de adesivos e roteiros gravados

Data: 2026-10-31
Status: decidido pelo agente (Pedro: "vai em frente sem me questionar")
Depende de: 2c

## 1. Descobertas

- **Rolos**: o Fine Artist mostra 29 rolos, 7 por página (setas e número à
  direita, como as páginas do pincel). O primeiro é o de bichos animados
  (`ARTIST.PIC`); os outros 28 são os chunks `CARO` das bibliotecas
  `MAXMAG10.ART`, `MAX100.ART` e `MAG100.ART`. Os números dos `CARO` formam
  uma sequência única de 0 a 27 entre as três bibliotecas, e é nessa ordem
  que os rolos aparecem (conferido no original: rolo 2 = `CARO` 0 "Hobby",
  skate; 3 = 1 "Animals (Kirk)"; 4 = 2; 5 = 3 "Dinos"; 6 = 4; 7 = 5 "Fairy").
- **Clip-art**: cada `PICT` tem um `META` (metarquivo do Windows com um único
  `StretchDIB` de um bitmap de 1 bit, documentado em `formats::wmf`) e um
  `PKBM` com a miniatura, que é a usada na tira de filme. O adesivo é colado
  em tamanho real (o skate tem 128×46 no bitmap e na tela), centrado no
  clique, sem animação.
- **Tira amarela** (rolos de clip-art): x = 33 + 52·casa, y 99..466, fundo
  pontilhado amarelo (255,238,76) e branco; 8 casas de 34×34 a cada 41 px a
  partir de y 103. A verde (bichos) tem 9 casas de 28×32 a cada 39 px.
- **Molduras dos rolos**: `10d3` (verde, 52 px), `10a7` (amarela, 52 px), com
  as tampas de `10cf` (14 px à esquerda) e `10ab` (12 px à direita).
- **Roteiros dos bichos**: gravados do original com `scratch/rec.sh` (ffmpeg a
  15 quadros/s) e `scratch/analyze.py`, que acha cada imagem e sua posição em
  cada quadro, inclusive parcialmente escondida pelo balão do McZee. Viram
  `crates/engine/src/scenes/sticker_scripts.rs` (gerado por
  `scratch/gen_scripts.py`). A cobra se repete andando para a direita até
  sair do quadro e não fica colada; os demais terminam numa imagem de
  repouso. Rolar a tira no original (Wine) derruba o programa, então os bichos
  10 a 16 não foram gravados e continuam com a animação aproximada.

## 2. Decisões

1. A ferramenta de adesivos passa a ter os 29 rolos e as 5 páginas.
2. Clip-art: preto pinta preto, branco interno pinta branco, branco ligado à
   borda é transparente (mesma regra dos carimbos do pincel). Colocar é uma
   ação do ovo.
3. Bichos 1 a 9: roteiro gravado. Bichos 10 a 16: animação aproximada (2c).
4. "Adesivos animados" e "faça o seu" aparecem na faixa, sem ação.
5. O código de adesivos sai de `paint.rs` para `scenes/stickers.rs`.
