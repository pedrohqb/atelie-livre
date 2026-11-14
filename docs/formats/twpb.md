# `TWPB` — contorno de objeto para o texto contornar

Parte dos chunks `PICT` e `SPLE` do documento `.MAX` do Creative Writer
(ver `max.md` do relatório do Writer): o contorno que o texto corrido
contorna. Leitor, gravador e cálculo: `formats::twpb`.

```
u16  tipo       8 nas figuras e na maioria dos Splots; 16 em alguns Splots; 4 numa figura de SAMP13
u16  largura    do objeto como aparece na página (pode diferir do bitmap, se redimensionado)
u16  altura
u16  nl         pontos da borda esquerda
u16  nr         pontos da borda direita
nl × { i16 x, i16 y }   margem esquerda: pixels vazios desde a borda esquerda até a tinta
nr × { i16 x, i16 y }   margem direita: pixels vazios desde a borda direita até a tinta
```

`y` cresce para baixo; linhas vazias no alto e embaixo ficam de fora e o
último ponto vai até a borda de baixo da última linha (até `altura`). Entre
dois pontos a margem é a reta que os liga: o original guarda só os vértices.

## Cálculo (`twpb::from_mask`)

O Writer calcula o contorno ao inserir a figura. O port reproduz: a tinta
(pixels não brancos) é **engordada 2 px na horizontal e 1 px na vertical**,
mede-se a margem de cada linha e a poligonal é reduzida por Douglas–Peucker
com tolerância de **0,5 px**. Nos 58 `PICT` dos exemplos
(`SHARED/USERS/MAX/SAMP*.MAX`), 95% das linhas ficam a até 2 px do contorno
gravado e o número de vértices fica na mesma ordem do original
(`tests/real_twpb.rs`). Sem a folga, só 58% das linhas ficavam a 2 px.
