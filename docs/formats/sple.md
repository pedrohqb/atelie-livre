# `SPLE` — palavra com forma (Splot) no documento `.MAX`

Chunk `SPLE n` do documento do Creative Writer, com as partes `SPLE` (este
formato) e `TWPB` (contorno para o texto contornar, [`twpb.md`](twpb.md)).
É citado por uma moldura de tipo 2 (`FRAM`, sprm `0x35` = `SPLE n`). Leitor e
gravador: `formats::sple`; desenho e montagem: `engine::writer::splot`.

O original guarda **o desenho pronto**: cada letra (e cada cópia de sombra)
é um conjunto de polígonos coloridos. Para mostrar um Splot basta pintar os
glifos em ordem; o resto do registro só serve para reabrir o cavalete.
Conferido com os 6 Splots dos exemplos (`SAMP7`, `SAMP13`–`SAMP17`, lidos e
regravados byte a byte) e com 29 gravados no original variando uma escolha
por vez (`report/formats/amostras/SPB.MAX`, `SPC.MAX`).

Todos os campos são **little-endian**.

## Disposição

| Offset | Conteúdo |
|---|---|
| `0x000` | cabeçalho (0x48 bytes, abaixo) |
| `0x048` | Pascal: nome da fonte (campo de 257 bytes) |
| `0x149` | Pascal: texto como digitado (campo de 257 bytes; depois do texto pode haver lixo) |
| `0x24a` | registro da fonte: `u16` tamanho, Pascal com o nome, `0`, completado até par (`0a 00 06 "Impact" 00`) |
| … | registro da forma (`+0x06` bytes) |
| … | bloco da sombra (30 `u16`, só com sombra) e um registro de 11 `u16` por cor, mais um `u16 0` |
| … | letras: por caractere `u16 2`, `u8` caractere, `u8` `8 | estilo das letras`, separadas por `u16 0` (6 bytes, o último sem o `0`) |
| `+0x1c` | glifos |

### Cabeçalho

| Offset | Tipo | Conteúdo |
|---|---|---|
| `0x00` | u16 | 1 |
| `0x02` | i16 | −13 |
| `0x04` | u16 | tamanho do registro da fonte |
| `0x06` | u16 | tamanho do registro da forma |
| `0x08` | u16 | 0 sem sombra; 1 (3 em "Girar e sumir") com sombra |
| `0x0a` | u16 | número de cores do conjunto escolhido |
| `0x0c` | u16 | número de letras (= tamanho do texto) |
| `0x0e` | u16 | número de glifos |
| `0x10` | 4 × i16 | caixa do desenho (cima, esquerda, baixo, direita), nas unidades dos glifos |
| `0x18` | u16 | `0x210` |
| `0x1c` | u16 | offset dos glifos |
| `0x20` | u8 | estilos da forma & `0x1f` |
| `0x21` | u8 | `tipo` do `SCRD` da forma (0 no retângulo) |
| `0x38` | 8 bytes | `00 00 01 00 fc ff 00 00` |
| `0x40` | i16 | giro em graus (vertical = 270) |
| `0x42` | u8 | forma: 0 = retângulo, 1–14 = `SCRD` |
| `0x43` | u8 | sombra: índice no `TMEN 2` |
| `0x44` | u8 | padrão: 0 = cheio, 1–13 = `PATS` |
| `0x45` | u8 | conjunto da lista de cores (`CSCH`, 0–23) |
| `0x46` | u8 | estilo das letras: 1 negrito, 2 inclinado, 4 contorno fino (glifos de tipo 3; ligado por padrão, apagado por "Normal") |
| `0x47` | u8 | estilos da forma: 1 vertical, 2 preencher, 4 MAIÚSCULAS, `0x20` esquerda/direita, `0x40` em cima/embaixo |

Os oito bytes `0x40..0x47` são, na mesma ordem, os registros do recurso
`TDRL` (ver abaixo).

### Registro da forma

É o `SCRD` da forma ([`scrd.md`](scrd.md)) sem os dois primeiros `u16`
(`n`, 1, largura, altura, `c0`..`c4`, pontos). No retângulo o original grava
uma forma de 4 pontos: `n` 4, 1, 8000, 8000, 2, 0, 2, 0, 0 e
(0,0), (8000,0), (0,−8000), (8000,−8000).

### Sombra e cores (só para reabrir o cavalete)

Com sombra vêm primeiro 30 `u16` (na "Sombra normal": 12 zeros, `1 1 1 1 0 0
12`, a cor #8c8c8c como `Rgb48` e 8 zeros; as outras sombras mudam o começo).
Depois, para cada cor do conjunto: cor da frente e cor de fundo (`Rgb48`,
3 × `u16` com o byte alto = cor; o fundo é a cor seguinte do conjunto, ou
branco se só há uma), `u16` (`0xffff` com padrão) e os 8 bytes do padrão
(zeros sem padrão). Fecha com `u16 0`.

## Glifos

Um registro por glifo, desenhados **em ordem** (as cópias de sombra antes das
letras):

```
u16  tag        byte baixo = tipo (2, 3 ou 5); byte alto 0x06 (0x16 com padrão)
u16  total      número total de pontos
u16  n          número de contornos
4 × i16         caixa (cima, esquerda, baixo, direita)
3 × u16         cor (Rgb48)
tipo 5:  3 × u16 cor de fundo, u16 0xffff, 8 bytes do padrão (bit 1 = cor da frente)
n × u16         pontos de cada contorno
total × { i16 x, i16 y }
```

- **Tipo 2**: polígonos cheios, regra par-ímpar (os buracos das letras são
  contornos a mais). As sombras são sempre de tipo 2.
- **Tipo 3**: cheios e com o contorno traçado na mesma cor (o normal das
  letras; sem o bit 4 do estilo das letras viram tipo 2).
- **Tipo 5**: cheios com o padrão 8×8 do `PATS` em duas cores.

Os pontos estão nas unidades da forma (8000 = largura da linha nas formas
comuns; a caixa do cabeçalho dá a escala). Para desenhar um Splot do
tamanho da moldura, a caixa vai para o retângulo da moldura (escalas
diferentes em x e y são permitidas).

## `TDRL` — os "preferidos"

No Writer os botões ＋ e − do cavalete não mudam o tamanho (como no Fine
Artist): mostram "os meus preferidos" (balão `0fa0[20]`), combinações
prontas guardadas no `TDRL 1` do executável (13 registros de 12 bytes,
`splot_res::favorite`):

```
ff ff  i16 giro  u8 forma  u8 sombra  u8 padrão  u8 cores  u8 estilo das letras  u8 (4; 0 com estilo 0)  u8 estilos  ff
```

## Gravação pelo port

O port monta o Splot com o cavalete do Fine Artist e grava o desenho
**vetorizado a partir da imagem** (um glifo de tipo 2 por cor, contornos
pelas bordas dos pixels, unidades = pixel × k com o lado maior perto de
8000). Cabeçalho, nomes, fonte, forma e cores seguem o original; o bloco da
sombra é sempre o da "Sombra normal" (o original desenha pelos glifos). Ao
reabrir, as escolhas voltam de `0x40..0x47`, do texto e da fonte.
