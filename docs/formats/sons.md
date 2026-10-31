# Sons

## DLLs de som (`ARTIST.SND`, `WRITER.SND`)

São DLLs NE só com recursos (ver `recursos.md`):

| Arquivo | `SND` | `SNDRANGE` |
|---|---|---|
| `ARTIST/ARTIST.SND` | 131 | 1 |
| `WRITER/WRITER.SND` | 122 | 1 |

Cada recurso `SND` é um arquivo WAV completo (RIFF/WAVE). Como todo recurso
NE, ele vem com preenchimento no fim; o tamanho real é o do cabeçalho RIFF.

`SNDRANGE` (16 bytes) traz dois `u16` little-endian: o primeiro e o último id
dos sons (`09 02 12 03` = 0x209 a 0x312, nos dois arquivos), seguidos de zeros.
Os ids não são contínuos dentro dessa faixa.

## Formato dos WAV

Todos os 253 sons: formato 0x0002 (ADPCM da Microsoft), mono, 4 bits por
amostra, blocos de 256 bytes com 500 amostras, os 7 coeficientes padrão.
Taxas de amostragem:

| Taxa | ARTIST.SND | WRITER.SND |
|---|---|---|
| 11.025 Hz | 102 | 98 |
| 11.127 Hz | 23 | 18 |
| 11.128 Hz | 5 | 5 |
| 44.100 Hz | 1 | 1 |

11.127 Hz é a taxa clássica dos Macintosh; esses sons provavelmente vieram da
versão Mac sem reamostragem. Um tocador atual precisa reamostrar para a taxa
da placa de som; a extração grava cada WAV na taxa original.

Exemplo de cabeçalho `fmt ` (ARTIST.SND, id 0x209):

```
02 00  01 00  11 2b 00 00  22 16 00 00  00 01  04 00
ADPCM  mono   11.025 Hz    bytes/s      bloco 256  4 bits
20 00  f4 01  07 00
extra 32  500 amostras/bloco  7 coeficientes
00 01 00 00 | 00 02 00 ff | 00 00 00 00 | c0 00 40 00 | f0 00 00 00 | cc 01 30 ff | 88 01 18 ff
(256,0)       (512,−256)    (0,0)         (192,64)      (240,0)       (460,−208)    (392,−232)
```

## Decodificação do ADPCM da Microsoft

Cada bloco começa, por canal, com: índice do coeficiente (`u8`), delta inicial
(`i16`), amostra 1 (`i16`) e amostra 2 (`i16`). As duas primeiras saídas são
amostra 2 e amostra 1. Depois vêm nibbles, primeiro o alto:

```
previsão = (amostra1 × coef1 + amostra2 × coef2) / 256   divisão inteira, truncando em direção a zero
nova     = satura_i16(previsão + nibble_com_sinal × delta)
delta    = max(16, (adapt[nibble] × delta) >> 8)
adapt    = 230 230 230 230 307 409 512 614 768 614 512 409 307 230 230 230
```

A divisão truncada importa: usar deslocamento aritmético (`>> 8`) arredonda
valores negativos para baixo e o erro se acumula. Com a divisão, a saída é
idêntica, amostra por amostra, à do decodificador `adpcm_ms` do FFmpeg em
todos os 253 sons.

## Bibliotecas de som em `CHNK`

`SHARED/STUFF/MAG100.SND` e `MAX100.SND` têm o mesmo nome de extensão, mas
são arquivos `CHNK` (ver `chnk.md`), com sons RIFF dentro dos blocos.
