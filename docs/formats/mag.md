# MAG — desenhos salvos do Fine Artist

Formato dos desenhos (`.MAG`) e pôsteres (`.PTM`) do Fine Artist: um arquivo
`CHNK` little-endian (ver `chnk.md`) de criador `SPLT`, versão 4/4. Validado
nos 18 documentos de exemplo de `SHARED/USERS/MAGGIE/` (16 `.MAG` e 2
`.PTM`), que o leitor de `crates/formats/src/mag.rs` abre e o gravador
reproduz.

## Chunks

Na ordem em que `SAMP6.MAG` (desenho sem objetos soltos) os grava:

| Chunk | Nº | Parte | Conteúdo |
|---|---|---|---|
| `CTBL` | 0 | `CTBL` | paleta de 256 cores (ver `chnk.md`); a etiqueta interna é `COBJ` |
| `FLOR` | 0 | `FLAG` | 2 bytes: `02 00` |
| `WIZA` | 0 | `FLAG` | 4 bytes: zeros |
| `FWLS` | 0 | `FWLS` | 26 bytes: cabeçalho `01 00 07 00 "FWLS"`, 1, zeros, largura e altura da pintura (offsets 14 e 16), zeros |
| `PKBM` | 0 | `PKBM` | a pintura: 594×347, 8 bits, PackBits, sem paleta própria (ver `pkbm.md`) |
| `SOBJ` | n | `SOBJ` | adesivo solto (posição, retângulos); não interpretado |
| `COBJ` | n | `COBJ` | palavra ou objeto solto; não interpretado |
| `PKBM` | n ≥ 1 | `PKBM` | imagens dos adesivos soltos (8 bits) e suas máscaras (1 bit), aos pares |
| `SND ` | n | `PCM ` | sons presos a adesivos (RIFF/WAVE) |
| `SPLE`, `CMBD`, `ATAG` | n | — | efeitos, carimbos e animações presos a adesivos; não interpretados |
| `FLS ` | 0 | `FLS ` | 166 bytes: cabeçalho `01 00 07 00 "FLS "`, um `FOS ` com dois retângulos `(0, 0, largura, altura)` (offsets 22 e 30) e constantes |
| `THMB` | 0 | `PKBM` | miniatura: 144 de largura, altura na proporção (84 para 594×347; no máximo 144), 8 bits **com paleta embutida** igual à `CTBL` |
| `THMB` | 1 | `PKBM` | cópia da pintura em tamanho real, com a mesma paleta embutida |
| `TITL` | 0 | `TEXT` | título em Windows-1252, sem terminador |
| `AUTH` | 0 | `TEXT` | nome do autor, idem |
| `SDOC` | 0 | `SDOC` | 88 bytes: cabeçalho `01 00 07 00 "SDOC"`, `TLPS`, 1, 1, retângulo da pintura na janela `(23, 109, 617, 456)` (offsets 16 a 22) e constantes |

Os chunks dos adesivos (`SOBJ`, `COBJ`, `PKBM n`, `SND`, `SPLE`, `CMBD`,
`ATAG`) ficam entre `PKBM 0` e `FLS `. O `.PTM` traz ainda um `DOPT 0` antes
do `SDOC`. Documentos com muitos adesivos (`SAMP1.MAG`, 134 registros)
usam o segundo trecho do índice do `CHNK`.

Todas as partes pequenas começam com o mesmo cabeçalho de 8 bytes: `u16`
marca de ordem (1), `u16` 7, 4 bytes de etiqueta.

## Pintura e paleta

A pintura (`PKBM 0`) não traz paleta; vale a `CTBL` do arquivo, que também é
a paleta embutida nos dois `THMB`. Cada documento guarda a sua paleta
(`SAMP2.MAG` tem uma diferente das demais), e ela não é a `LOGPALETTE` 8 do
`ARTIST.EXE`. Quem abre o desenho noutro programa com paleta fixa precisa
aproximar as cores (o Ateliê Livre usa a cor mais próxima em RGB).

Nos `.MAG` de desenho a pintura tem 594×347; nos de pôster e de cartão
(`SAMP3`, `SAMP12`–`SAMP15`, `.PTM`) outros tamanhos (162×162, 216×162,
162×198, 270×216). Os bytes 20/21 do cabeçalho `PKBM` da pintura são `40 04`
ou `08 04`; nas miniaturas, `08 00`.

## Leitura

`mag::read` aproveita `CTBL`, a pintura, `TITL` e `AUTH` e ignora os objetos
soltos. Com adesivos ou palavras soltos o `PKBM 0` guarda **só o fundo**: em
`SAMP1.MAG` ("5 Artistas") ele é quase todo branco (3 554 pixels desenhados),
e a figura completa (26 010) está na cópia em tamanho real `THMB 1`. Por isso
o leitor usa `THMB 1` quando ela tem o tamanho da pintura e cai no `PKBM 0`
quando não há cópia (nos 12 exemplos sem objetos as duas são iguais). Nos
adesivos e cartões (`SAMP12`–`SAMP15`) o `PKBM 0` é todo branco e só a cópia
tem o desenho. Sem `CTBL`, vale a paleta embutida em `THMB 0` ou na própria
pintura.

## Gravação

`mag::write` grava os chunks da tabela sem `SOBJ`/`COBJ`, na ordem de
`SAMP6.MAG`, com as partes fixas (`FLOR`, `WIZA`, `FWLS`, `FLS `, `SDOC`)
copiadas desse arquivo e os tamanhos ajustados ao desenho. A miniatura é
reduzida por vizinho mais próximo. O arquivo resultante ainda não foi
conferido no Fine Artist original (sem Wine neste ambiente); a estrutura
é byte a byte igual à de `SAMP6.MAG` nas partes fixas e nos cabeçalhos
`PKBM`.
