# ATAG — roteiros de animação

Roteiros das animações de sprites: os bichos e cenas do título e das salas
(recursos `ATAG` de `ARTIST.EXE` e `WRITER.EXE`) e os adesivos animados de
`SHARED/STUFF/A-MAG100.ANI`. Validado nos 162 recursos de `ARTIST.EXE`, nos
155 de `WRITER.EXE` e nos 16 adesivos do `.ANI`; a semântica foi conferida
contra gravações quadro a quadro do programa original (os nove primeiros
adesivos do rolo "bichos": posições exatas, durações dentro de ±1 quadro a
15 fps).

Todos os campos são **little-endian**. O roteiro é uma sequência de blocos
identificados por um código `u16`, sempre na mesma ordem.

## Duas variantes

| Onde | Prefixo | Imagens | Blocos | Fim |
|---|---|---|---|---|
| Recurso `ATAG` dos executáveis | nenhum | ids do `.PIC` | 1, 2, 4 | zeros de preenchimento do NE |
| Parte `ATAG` dos `CHNK` aninhados do `.ANI` | `u16 1`, `u16 tamanho` | números dos chunks `PKBM` do mesmo `CHNK` | 1, 2, 7, 8, 9 | código 9 |

O `tamanho` da variante dos `CHNK` conta do início da parte até o fim do
bloco 9, inclusive; o resto da parte é lixo (restos de imagens).

## Cabeçalho (12 bytes, após o prefixo quando houver)

```
u32  versão      1
u32  flags       0 ou 0x10 nos executáveis; 0x800 nos .ANI (significado desconhecido)
u32  reservado   0 nos executáveis, 1 nos .ANI
```

## Bloco 1 — início

`u16 1`, sem corpo.

## Bloco 2 — células (imagens com duração)

```
u16  2
6    zeros
u16  contagem
contagem × célula (18 bytes):
     u16  3
     u16  0
     u16  imagem
     u16  segunda imagem (ver abaixo)
     i16  dx  (sempre 0)
     i16  dy  (sempre 0)
     u32  duração em milissegundos
     u16  id do som (recurso SND de ARTIST.SND/WRITER.SND), 0 = nenhum
```

A duração é em milissegundos: 100, 200, 300, 400 ms nos adesivos
correspondem a 1–2, 3, 4–5 e 6 quadros de 1/15 s nas gravações. Os sons
(0x20a, 0x217, 0x239, ...) existem como recursos `SND` em `ARTIST.SND`
(35 dos 38 ids usados). Há células com duração 0 (imagem exibida só durante o
movimento interpolado, ver bloco 4).

**Segunda imagem.** Nos `.ANI` é a máscara: a imagem `n` fica no chunk
`PKBM n` e a máscara (1 bit, mesmo tamanho) no `PKBM n+1`; o chunk 0 é a
miniatura do adesivo e o 1 a máscara dela. Nos executáveis o campo vale sempre
`imagem + 3`, mas no `.PIC` da versão em português esse id é outra figura
(em geral a versão colorida do quadro seguinte); a primeira imagem é a versão
de 1 bit e a máscara dela é `imagem + 1`. O campo parece pressupor uma
numeração de `.PIC` diferente da desta versão; o leitor o devolve sem
interpretar.

No `.PIC` as figuras dos roteiros vêm em trincas na ordem dos ids: **versão
colorida, versão de 1 bit, máscara** (ex.: 0x64a7, 0x64a8, 0x64a9 para o
pássaro). Os ids não são contíguos entre trincas, então a versão colorida da
célula é a imagem de id imediatamente anterior ao dela na lista ordenada do
`.PIC` (`Assets::pic_prev_id`), quando tem o mesmo tamanho; vale para todas as
células dos 162 roteiros de `ARTIST.EXE` (`tools/wine/atag_survey.py`). A
máscara `imagem + 1` difere em 1 px de largura ou altura em dois casos
(0x64dd, 0x6719). Algumas "coloridas" têm só 2 cores (0x600f, o buraco do
McZee).

## Bloco 7 — passos (só nos `.ANI`)

```
u16  7
12   zeros
u16  contagem
contagem × passo (30 bytes):
     u16  0x0100  (marca)
     i16  dx      deslocamento até o passo seguinte (0 no último)
     i16  dy
     6    zeros
     u16  célula  índice no bloco 2
     i16  x       posição do canto superior esquerdo (soma dos dx/dy anteriores)
     i16  y
     i16  x + largura da imagem
     i16  y + altura da imagem
     8    zeros
```

Os passos percorrem as células em ordem cíclica (0, 1, 2, ..., 0, 1, ...),
mas o roteiro é livre para repetir ou pular células. Nos adesivos, a posição
é relativa ao canto superior esquerdo da célula 0 colocada com o centro no
ponto do clique: o passo `k` mostra a célula indicada em `clique −
(largura₀/2, altura₀/2) + (x, y)` durante a duração da célula.

## Bloco 4 (executáveis) / 8 (`.ANI`) — caminho

```
u16  4 ou 8
u16  0
u16  tipo       5 = um deslocamento por passo; 9 = movimento interpolado
i16  origem x   posição inicial na tela de 640×480 em que o roteiro foi gravado
i16  origem y
u32  0
u16  contagem
contagem × movimento (8 bytes):
     u8   0
     u8   quadros de interpolação (0 no tipo 5)
     i16  dx
     i16  dy
     u16  0
```

No tipo 5 há um movimento por célula (executáveis) ou por passo (`.ANI`),
repetindo os `dx`/`dy` do bloco 7; o último é (0, 0). Nos executáveis, onde
não há bloco 7, cada célula é um passo e a posição se acumula com esses
deslocamentos. A origem mostra de onde a figura parte: o rinoceronte sai de
x = 642 (fora da tela, à direita) andando −50 por passo; a cobra, de
x = −215 andando para a direita. Nos adesivos ela é só o resto da ferramenta
de autoria, já que a posição vem do clique.

O tipo 9 aparece em quatro roteiros de `ARTIST.EXE` (Pablo entrando e saindo
correndo no título, ids 0x3840, 0x384a, 0x4010, 0x4024): os movimentos não
são por célula, mas trechos de deslizamento, cada um interpolado em `n`
quadros (ex.: 0x3840, uma célula de duração 0 e um movimento de (−487, −52)
em 20 quadros). `WRITER.EXE` 0x934 (tipo 5) tem um movimento a mais que
células, aparentemente sobra de autoria.

## Bloco 9 — fim (só nos `.ANI`)

`u16 9`, `u16 0`.

## Exemplo (`A-MAG100.ANI`, adesivo 0: cobra)

```
01 00 ac 04                      prefixo: 1, tamanho 1196
01 00 00 00  00 08 00 00  01 00 00 00   versão 1, flags 0x800, 1
01 00                            início
02 00  00×6  04 00               4 células
  03 00 00 00  02 00 03 00  00 00 00 00  2c 01 00 00  00 00   PKBM 2 (máscara 3), 300 ms
  03 00 00 00  04 00 05 00  ...  2c 01 00 00  00 00          PKBM 4, 300 ms
  03 00 00 00  06 00 07 00  ...                              PKBM 6
  03 00 00 00  08 00 09 00  ...                              PKBM 8
07 00  00×12  1c 00              28 passos
  00 01  ff ff 05 00  00×6  00 00  00 00 00 00  d8 00 4d 00  00×8
         → célula 0 em (0,0), caixa até (216,77); depois move (−1, 5)
  00 01  38 00 f8 ff  00×6  01 00  ff ff 05 00  17 01 47 00  00×8
         → célula 1 (280×66) em (−1,5); depois move (56, −8)
  ...
08 00 00 00  05 00  29 ff fd 00  00 00 00 00  1c 00   tipo 5, origem (−215, 253), 28
  00 00 ff ff 05 00 00 00   (−1, 5)
  00 00 38 00 f8 ff 00 00   (56, −8)
  ...
09 00 00 00                      fim
```

Gravação do original para esse adesivo (clique em c): 753b (280×66) em
c + (−109, −33) por 4 quadros, 7543 em c + (−53, −41) por 5, 754b em
c + (−41, −35) por 4, célula 0 (que não existe em `ARTIST.PIC`) por 5,
753b em c + (18, −33)... — exatamente c + (−108, −38) + (x, y) de cada passo,
com 300 ms ≈ 4,5 quadros.

## Os 16 adesivos de `A-MAG100.ANI`

O chunk `ATAG n` é o `n`-ésimo bicho do rolo (mesma ordem da tabela
`ANIMALS` do motor): 0 cobra, 1 peixe-espada, 2 rinoceronte, 3 lagarto,
4 polvo, 5 tartaruga, 6 pássaro, 7 dodô, 8 jogador, 9–15 os demais. As
células de cada um têm os tamanhos das imagens coloridas correspondentes de
`ARTIST.PIC` (0x753b, 0x7553, ...), com três ressalvas: a célula 0 da cobra
(216×77) não está no `.PIC` (só suas versões de 1 bit, 0x7538/0x7539); o
peixe-espada tem cinco células (0x756b e 0x7573 além das três da tabela); e
as duas últimas células do rinoceronte têm o mesmo tamanho (101×48).

| Adesivo | Células | ms | Passos | Origem |
|---|---|---|---|---|
| 0 | 4 | 300 | 28 | (−215, 253) |
| 1 | 5 | 200, 200, 200, 200, 400 | 15 | (17, 208) |
| 2 | 3 | 200 | 16 | (642, 329) |
| 3 | 2 | 200 | 32 | (309, 462) |
| 4 | 5 | 200 | 25 | (−126, 238) |
| 5 | 4 | 200 | 31 | (81, 460) |
| 6 | 4 | 100 | 49 | (628, 194) |
| 7 | 6 | 100 | 26 | (−110, 270) |
| 8 | 6 | 200 | 12 | (−132, 177) |
| 9 | 5 | 200 | 25 | (−356, 183) |
| 10 | 4 | 200 | 14 | (615, 189) |
| 11 | 4 | 200 | 28 | (−132, 189) |
| 12 | 6 | 200 | 50 | (619, 285) |
| 13 | 4 | 200, 200, 200, 400 | 16 | (588, 154) |
| 14 | 4 | 200 | 38 | (−225, 159) |
| 15 | 2 | 200 | 25 | (610, 179) |

O pássaro do adesivo 6 é o mesmo roteiro dos recursos 0x67 e 0x325 de
`ARTIST.EXE` (mesmos deslocamentos; 0x325 tem a mesma origem (628, 194) e 0x67
parte de (628, 134)): 0x67 atravessa o título e 0x325 o saguão, em cores
(`docs/telas/animacoes.md`).

## O que falta

- Significado das flags (0x10, 0x800) e do terceiro campo do cabeçalho.
- `dx`/`dy` da célula e os campos zerados dos passos (nunca usados nos dados
  conhecidos).
- A numeração que o campo "segunda imagem" dos executáveis pressupõe.
- Como o programa decide o fim do adesivo: as gravações terminam antes do
  último passo (a figura some e a célula 0 volta ao clique), talvez quando a
  figura sai da página; o roteiro em si não traz condição de parada.
