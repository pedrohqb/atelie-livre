# Preferências do usuário: `~MAXUSER.INF` (e `~MAGUSER.INF`)

Cada usuário tem, na sua pasta `USERS/<NOME>/`, um `~MAXUSER.INF` (Creative
Writer) e um `~MAGUSER.INF` (Fine Artist). O programa lê o arquivo ao entrar
e o regrava ao sair. Leitor e gravador: `formats::maxprefs` (`read`,
`write`). Nenhum desses arquivos vem nos dados do produto; os analisados
foram gravados pelo original no Wine.

## Estrutura

É um arquivo [`CHNK`](chnk.md) little-endian, versão 4/4, criador `INK `
(o mesmo dos documentos `.MAX`), com o índice no primeiro bloco e os dados a
partir de 0x200. O original grava sempre 4608 bytes (9 blocos), sem
encurtar o arquivo: depois do fim lógico pode haver restos de uma gravação
anterior.

| Chunk | Conteúdo |
|---|---|
| `FILR` 0–4 | documentos pendurados nos 5 lugares da parede da Biblioteca |
| `FNTL` 0 | cache com os nomes das fontes do sistema |
| `STTB` n | entradas que o usuário pôs na Máquina de Idéias (uma tabela por coluna; só existe quando há entradas) |
| `SUDG` 0 | 6 bytes (`00 00 01 00 02 00` no exemplo: três `u16` 0, 1, 2), junto com as `STTB`; não interpretado |

Ordem observada no índice: `FILR` 0–4, `FNTL`, `STTB 0`, `SUDG`, `STTB 1`,
`STTB 2`. O gravador do port põe as `STTB` juntas e os outros chunks no fim
(a ordem não importa para a leitura).

### `FILR` — documento na parede

```
"kwMAX"\0        tipo ("kwMAG" no ~MAGUSER.INF do Fine Artist)
título\0         o mesmo do chunk TITL do documento
pasta\0 …        pastas a partir de MSKIDS, ex. SHARED\0 USERS\0 MAX\0
\0               fim da lista de pastas
ARQUIVO.MAX      nome 8.3, sem terminador (vai até o fim do chunk)
```

Exemplo (50 bytes): `kwMAX\0O Ponto de Fuga\0SHARED\0USERS\0MAX\0\0SAMP18.MAX`.
Nos sete arquivos vistos (um usuário só, em sessões diferentes) a parede
tem só exemplos de `SHARED/USERS/MAX`: `SAMP18.MAX` "O Ponto de Fuga"
sempre no lugar 0 e, nos outros quatro, exemplos que variam de uma gravação
para outra.

### `FNTL` — cache de fontes

```
u16   n
u16   offsets[n + 1]    a partir do início do chunk; o último é o fim
n × { u8 tamanho, nome (Windows-1252), u8 0 }
```

Uma entrada por face enumerada, com repetições (no Wine: 196 entradas, 35
nomes distintos, "Liberation Sans" 8 vezes). O port não precisa deste cache
(lista as fontes com `engine::paint::ttf`); ao gravar pode deixá-lo vazio
ou com as fontes que achar.

### `STTB` — tabela de strings

A mesma estrutura das listas da Máquina de Idéias:

```
u16   tamanho do resto (offsets + entradas)
u16   n
i16   −1
u16   bytes extras depois de cada texto (0 aqui; 2 nas listas da Máquina, com ff ff)
u16   0
u16   offsets[n]        a partir do byte 10
n × { u8 tamanho, texto, extras }
```

No exemplo, `STTB 0` = "O pirata Zeca,", `STTB 1` = "dan", `STTB 2` = "na
lua." — entradas digitadas na Máquina de Idéias, uma por coluna. O diálogo
"Redefinir a Máquina de Idéias" (`DIALOG 0x1770`, ver `winres-ui.md`) apaga
essas entradas.

## Variante sem índice

Os arquivos de várias sessões do Wine encerradas à força têm os primeiros
512 bytes zerados (sem o cabeçalho `CHNK` nem o índice), os registros `FILR`
em sequência a partir de 0x200 e o `FNTL` logo depois; nenhum tinha `STTB`.
O índice parece ser gravado por último. O leitor aceita essa forma: lê os
registros enquanto começarem por `kw`, toma o nome do arquivo como 8.3 (até
3 caracteres depois do ponto) e lê o `FNTL` em seguida. O gravador sempre
grava a forma com índice.
