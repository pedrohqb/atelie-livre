# MAX — documentos do Creative Writer

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

Formato dos documentos salvos pelo Creative Writer (`USERS/<usuário>/*.MAX` e
os exemplos de `SHARED/USERS/MAX/SAMP*.MAX`): um arquivo `CHNK`
little-endian, versão 4/4, de criador **`INK `** (ver `chnk.md`). Por dentro,
o modelo é o de um processador de texto da família do Word/Write: **texto
corrido** num chunk `TEXT`, **páginas FKP** de 512 bytes com as propriedades
de caractere e de parágrafo, **tabelas PLC** que ligam posições do texto a
páginas FKP, folhas e sons, e **objetos soltos** (figuras, Splots, bordas)
descritos numa lista de molduras (`FRAM`).

Validado nos 21 exemplos de `SHARED/USERS/MAX/` e em 6 documentos criados no
original (Wine) para isolar cada campo: `amostras/TESTE1.MAX` (só "abc"),
`F1.MAX` (troca de fonte), `S2.MAX` (estilos, cores, tamanhos, efeitos,
alinhamento, recuos, espaçamento, capitular, marcadores), `S3.MAX` (+ quebra de
página e um adesivo), `S4.MAX` (+ 3 colunas, "Largo", margens de 3,8 cm,
números de página, borda de patinhas), `S5.MAX` (+ Agente secreto, senha
"abc"). Leitor de referência: `tools/maxdoc.py` (Python, só leitura).

No port: leitor e gravador em `formats::max` (`crates/formats/src/max.rs`;
ver [No port](#no-port)) e `atelie-tools max2txt ARQ.MAX` (texto, estilos e
objetos) / `atelie-tools info ARQ.MAX` (resumo).

Todas as posições abaixo são **little-endian**. "cp" = posição de caractere
no `TEXT` (0 = primeiro byte); "fc" = offset absoluto no arquivo.

## Chunks

Na ordem em que o original grava (`TESTE1.MAX`, `SAMP18.MAX`):

| Chunk | Nº | Parte(s) | Conteúdo | Ver |
|---|---|---|---|---|
| `TEXT` | 0 | `TEXT` | texto corrido, Windows-1252, parágrafos terminados por `\r` (0x0D) | [Texto](#texto) |
| `FDPP` | 0.. | `FDPP` | páginas FKP (512 bytes) de propriedades de **parágrafo** | [FKP](#páginas-fkp-fdpc-fdpp) |
| `FDPC` | 0.. | `FDPC` | páginas FKP (512 bytes) de propriedades de **caractere** | idem |
| `BDR ` | n | `WBDR` + `PKBM` | borda de página/adesivo, cópia da biblioteca `?MAX100.BOR` | [Objetos](#objetos-pict-sple-bdr--snd) |
| `PICT` | n | `META` + [`PKBM`] + `TWPB` | figura (adesivo, figura importada, rébus) | idem |
| `SPLE` | 0 | `SPLE` + `TWPB` | "Splot" (palavras com forma) | idem |
| `SND ` | n | `PCM ` | som (RIFF/WAVE) ligado a figura, a palavra ou a página | idem |
| `DOP ` | 0 | `DOP ` | propriedades do documento (página, margens, colunas, borda, nº de página) | [DOP](#dop--propriedades-do-documento) |
| `FRAM` | 0 | `FRAM` | lista de molduras: posição/tamanho de cada objeto solto | [FRAM](#fram--molduras) |
| `BTEP` | 0 | `PLC ` | fc → página `FDPP` | [PLC](#tabelas-plc) |
| `BTEC` | 0 | `PLC ` | fc → página `FDPC` | idem |
| `PGD ` | 0 | `PLC ` | cp → folhas (quebras de página) | idem |
| `FONT` | 0 | `FONT` | tabela de fontes | [FONT](#font--tabela-de-fontes) |
| `SNDT` | 0 | `PLC ` | sons ligados a trechos do texto (só quando há) | [PLC](#tabelas-plc) |
| `PRNT` | 0 | `WNPR` | configuração da impressora usada | [Demais](#demais-chunks) |
| `SELN` | 0 | `SELN` | seleção/cursor ao salvar | idem |
| `THMB` | 0 | `PKBM` | miniatura da página (lado maior 144 px), 4 ou 8 bits, paleta embutida | idem |
| `ADD ` | 0 | `ADD ` | descritor de 128 bytes (projeto, andar, senha, título, autor) | idem |
| `AUTH` | 0 | `TEXT` | autor | idem |
| `TITL` | 0 | `TEXT` | título (nome dado ao salvar) | idem |
| `FLOR` | 0 | `FLAG` | `u16` andar do prédio em que foi feito (2 = Escritório, 3 = Oficina de Projetos) | idem |
| `WIZA` | 0 | `LONG` | tipo de projeto: `card`, `banr`, `news` ou 0 | idem |
| `INK ` | 0 | `INK ` | `04 00 04 00` (versão do formato) | idem |

As três primeiras partes ficam em offsets múltiplos de 512: `TEXT` em 0x200
e as páginas FKP alinhadas a 512 depois dele (`TESTE1.MAX`: TEXT 0x200,
FDPP 0x400, FDPC 0x600). O resto vem em sequência, sem alinhamento, e o
arquivo é completado com zeros até múltiplo de 512 (como em `chnk.md`).

## Texto

Bytes Windows-1252. `\r` (0x0D) fecha parágrafo; não há `\n`. O último
parágrafo também termina com `\r`. Nos documentos salvos no Wine há um `\r`
a mais no fim (a marca do parágrafo final, como no Word): "abc" digitado é
gravado `61 62 63 0d 0d`; os exemplos da instalação terminam com um só `\r`
e os cartões e faixas vazios são só `0d`. Uma página nova não é um
caractere: é a propriedade de parágrafo `0x1f` ou a quebra natural do texto
(ver `PGD `).

Objetos dentro da linha (figuras do "Maçã"/rébus, ver `WHAT 0fa8[6]`) ficam
no texto como **um caractere especial** marcado com o sprm de caractere
`0x03 = 1`; o valor do byte não é texto (aparecem `O`, `‹`, `d`, `Â`, `:`,
`Z`, `Ú`, `\x12` em `SAMP18.MAX`). Quem lê o texto para exibir/exportar deve
pular esses caracteres. A ligação exata byte → `PICT n` ainda não foi
confirmada (8 âncoras em `SAMP18`, 4 `PICT` + 2 `SND`).

"Salvar Como Texto" (menu Arquivo, `0x1f44`) grava só esse texto, com
`\r\n`.

## sprms

`FDPC`, `FDPP`, `DOP ` e `FRAM` usam **um único espaço de códigos** de
propriedades, à moda dos "sprm" do Word: um byte de código seguido de um
argumento de tamanho fixo por código. Tabela levantada (✔ = confirmado por
documento criado no original; ~ = deduzido dos exemplos):

### Caractere (`FDPC`)

| Código | Arg. | Significado | Valores |
|---|---|---|---|
| `0x03` | 1 | caractere especial (âncora de objeto na linha) | 1 ~ |
| `0x04` | 1 | negrito | `ff` ✔ |
| `0x05` | 1 | inclinado | `ff` ✔ |
| `0x06` | 1 | sublinhado | `ff` ✔ |
| `0x0b` | 1 | efeito especial | 1 Pisca-pisca, 2 Formiguinhas, 3 Vapt ✔, 4 Vupt, 5 Brilho ✔, 6 Treme-treme, 7 Maluco (ordem da `STR#` 0x807 itens 4–11) |
| `0x0c` | 1 | cor do texto definida (usa `0x10`; sem `0x10` = preto) | `ff` ✔ |
| `0x0d` | 1 | marca-texto ligado (usa `0x11`) | `ff` ✔ |
| `0x0e` | 2 | fonte: `ftc` da tabela `FONT` | ✔ |
| `0x0f` | 2 | tamanho em pontos | 48 Gigante, 36 Enorme, 24 Grande, 18 Média, 14 Pequena, (12 Mínima = padrão, sem sprm), 9 Micro ✔ |
| `0x10` | 6 | cor do texto: 3 × `u16` (r, g, b) no estilo Mac, **byte alto = cor** | ✔ (`fa fa 00 00 39 39` = #fa0039) |
| `0x11` | 6 | cor do marca-texto, mesmo formato | ✔ |

As 16 cores que o Seleciona-cores e o Iluminador oferecem são as da tabela do
recurso `CSCH` (ver `writer-exe-recursos.md`), na mesma ordem: branco
#ffffff, amarelo #ffee4c, laranja #ff8c32, marrom #b96524, rosa #fa4a72,
vermelho #fa0039, lavanda #8888e6, roxo #6141db, magenta #de38c7, violeta
#932ca3, verde #5ccb53, verde-escuro #478c50, cinza-claro #bebebe, cinza
#8c8c8c, cinza-escuro #646464, preto #000000.

Os "Mix de estilos" (Maluco, Anotação, Título, Era uma vez, Poesia,
Relatório, Super-simples) não têm sprm próprio: aplicam combinações dos
sprms acima (Título = negrito + 24 pt + parágrafo centralizado, ✔).

### Parágrafo (`FDPP`)

| Código | Arg. | Significado | Valores |
|---|---|---|---|
| `0x19` | 1 | alinhamento | 0 (ou ausente) esquerda, 1 direita ✔, 2 centro ✔, 3 esquerda e direita (justificado) ✔ |
| `0x1a` | 1 | recuo | 1 só a 1ª linha ✔, 2 todas menos a 1ª ~, 3 todas as linhas ✔ |
| `0x1b` | 1 | espaço extra entre as linhas | `ff` ✔ |
| `0x1c` | 1 | espaço extra depois do parágrafo | `ff` ✔ |
| `0x1d` | 1 | letra capitular (grande e caída) | 3 ✔ (provável nº de linhas) |
| `0x1e` | 1 | letra destacada (grande e saltada) | `ff` ✔ |
| `0x1f` | 1 | começa uma nova página | `ff` ✔ |
| `0x20` | 1 | marcadores (lista com bolinha) | `ff` ✔ |

### Moldura (`FRAM`)

| Código | Arg. | Significado |
|---|---|---|
| `0x27` | 2 | página (0 = primeira; ausente = 0; `ffff` = todas, nas molduras copiadas de um fundo) |
| `0x28` | 1 | tipo: 2 Splot, 3 figura/adesivo, 4 moldura de página (cartões), 6 obstáculo de um fundo (`bor.md`) |
| `0x29` | 1 | modo de contorno do texto ("Texto flutuante"): 2, 3 (a mapear); nos obstáculos dos fundos 1 = texto só à esquerda, 2 = só à direita, 3 = nada ao lado |
| `0x2a` | 1 | `ff` (sempre) |
| `0x2b` | 1 | `ff` nas molduras copiadas de um fundo (`FUNDO_SC.MAX`; sentido desconhecido) |
| `0x2c` / `0x2d` | 2 | x / y em pontos da página |
| `0x2e` / `0x2f` | 2 | largura / altura atuais |
| `0x30` / `0x31` | 2 | largura / altura originais (diferem após esticar) |
| `0x35` | 6 | objeto: etiqueta de 4 letras + `u16` número do chunk (`PICT 2`, `SPLE 0`) |
| `0x37` | 6 | som preso ao objeto: `SND ` + número |

### Documento (`DOP `)

| Código | Arg. | Significado | Valores observados |
|---|---|---|---|
| `0x3e` / `0x3f` | 2 | largura / altura da página em pontos | 595 × 792 "Alto" ✔; 792 × 595 "Largo" ✔; 542 × 767 nas faixas |
| `0x40`–`0x43` | 2 | margens (topo, esquerda, base, direita ~) | 36, 54, **72 (padrão)**, 90, 108 = 1,3 / 2 / 2,5 / 3,2 / 3,8 cm ✔ |
| `0x45` | 2 | espaço entre colunas ~ | 12; 42 nos cartões |
| `0x47` / `0x48` | 2 | borda: espessura + 1 e faixa (1,5 × espessura) + 1 | 33/49 patinhas (32) ✔; 25/37 (24) ✔; 17/25 (16) |
| `0x49` | 2 | 36 (constante) | |
| `0x4a` | 2 | colunas − 1 | 2 = três colunas ✔ |
| `0x4c` | 1 | `ff` em jornal (a mapear) | |
| `0x4d` | 1 | estilo do nº de página: 1 = "Pág. N" ✔ (2 = "N" ~) | |
| `0x51`–`0x54` | 2 | retângulo do nº de página (topo, esq., base, dir.) | (580, 364, 595, 428) em "Largo" ✔ |
| `0x56` | 6 | fundo da página: `IBGF` + número do chunk (que aponta `PICT` e `FRAM`, ver `bor.md`) | `FUNDO_SC.MAX` ✔ |
| `0x58`, `0x5a` | 6 | borda: `BDR ` + número do chunk (as duas iguais) | ✔ |
| `0x5b` | 1 | `ff` nas faixas (orientação de faixa ~) | |

O primeiro byte de `DOP ` é o tamanho do resto (grpprl). Os sprms de
caractere, parágrafo e moldura vêm em ordem crescente de código; os do `DOP`
na ordem `3e`–`43`, `45`, `49`, `4a`, `4c`, `4d`, `51`–`54`, `48`, `58`,
`47`, `5a`, `5b` (a borda no fim, em pares largura + `BDR`). Sem DOP extra o
documento novo tem `18 3e 53 02 3f 18 03 40 48 00 41 48 00 42 48 00 43 48 00
45 0c 00 49 24 00`.

## Páginas FKP (`FDPC`, `FDPP`)

Cada parte tem exatamente 512 bytes:

```
u16  crun          número de trechos
u16  1
(crun+1) × u32     rgfc: fc (offset no arquivo) do início de cada trecho e o fim
                   do último; o primeiro vale 0 (= início do TEXT)
crun × u16         offset, dentro da página, do grpprl do trecho (0 = sem propriedades)
...                livre
grpprl             no fim da página, de trás para frente: u8 tamanho, sprms
```

Exemplo (`TESTE1.MAX`, "abc\r\r" em 0x200):

```
FDPP: 02 00 01 00 | 00000000 04020000 05020000 | 0000 0000        2 parágrafos sem propriedades
FDPC: 01 00 01 00 | 00000000 05020000 | 0000                        1 trecho sem propriedades
```

`F1.MAX` (2ª linha em outra fonte): `FDPC` com trechos `[0x200,0x207) [0x207,0x208)
[0x208,0x209) [0x209,0x212) [0x212,0x213)` e offsets `0, 0x1fc, 0, 0x1fc, 0`;
no byte 0x1fc da página: `03 0e 01 00` = 3 bytes, sprm `0x0e` ftc 1.

Os trechos cobrem o texto inteiro, sem buracos, inclusive os `\r`. Em
`FDPP` há **um trecho por parágrafo**, mesmo com propriedades iguais.

Montagem da página (confirmada regravando os 27 documentos byte a byte):

- grpprls iguais na mesma página são gravados uma vez; eles ocupam o fim da
  página, **do fim para o começo, na ordem do primeiro uso** (o primeiro
  grpprl termina no byte 511), sem alinhamento;
- só o primeiro fc da **primeira** página vale 0; nas seguintes é o fc real;
- a página recebe trechos enquanto couberem os 6 bytes do novo trecho (fc +
  offset) mais o grpprl, se for novo na página; senão começa outra página
  FKP (`SAMP18.MAX` tem `FDPC 0`, com 38 trechos e 2 bytes livres, e
  `FDPC 1`) e o `BTEC` passa a ter duas entradas (`[0, 0x4f6, 0xf14]`);
- o espaço livre e o enchimento entre as partes do arquivo são lixo de
  memória no original (não são zerados).

## Tabelas PLC

Partes `PLC ` (de `BTEP`, `BTEC`, `PGD `, `SNDT`):

```
u16  n             número de elementos
u16  cb            tamanho de cada elemento (4 em BTE, 2 em PGD, 10 em SNDT)
u16  extra         0 em BTE; 0x00ff em PGD; 0xff00 em SNDT
(n+1) × u32        posições (fc em BTE; cp em PGD e SNDT)
n × cb             elementos
```

- **`BTEP`/`BTEC`**: elemento `u32` = fc da página FKP correspondente.
  `TESTE1`: `01 00 04 00 00 00 | 00000000 05020000 | 00040000` → [0, 0x205) na
  página em 0x400.
- **`PGD `** (folhas): posições cp do início de cada folha e o fim; elemento
  `u16` de flags (0 nos documentos livres; 1, 4, 0, 0 nas 4 folhas dos
  cartões — capa, páginas internas, contracapa, `STR#` 0x80a). `S3.MAX`
  (quebra antes de "GGGGGG"): `02 00 02 00 ff 00 | 0 42 86 | 0000 0000`.
- **`SNDT`** (sons em palavras): elemento de 10 bytes = 4 bytes (sem uso
  aparente; lixo de memória num exemplo) + `SND ` + `u16` número do chunk.
  `SAMP6`: cps 0, 134, 140 com `SND 62` e `SND 803` (o 2º cobre "Mães.\r").
  A semântica exata do 1º elemento (cp 0) ainda não foi conferida no original.

## FONT — tabela de fontes

```
u16  tamanho dos dados que seguem o cabeçalho
u16  n             número de fontes
i16  -1
u16  4
u16  0
n × u16            offset de cada entrada, a partir do byte 10
entradas: u8 tamanho + nome (Pascal, sem terminador), u16 ftc, u16 família
```

A família é a do `LOGFONT` (`lfPitchAndFamily >> 4`: 1 romana, 2 sem serifa,
4 script, 5 decorativa). O nome é o que o GDI devolveu ao criar a fonte —
no Wine sem as TrueType do produto, "Liberation Sans" (`F1.MAX`); por isso
o port deve casar o nome com a lista `STR#` 0x206 (Pablo Graphos, Lucida
Sans, Lucida Handwriting, Lucida Blackletter, Impact, Stencil) e cair num
substituto quando ele não existir. O ftc 0 é sempre a fonte padrão
"Lucida Sans" (`STR#` 0x800 [1]).

Exemplo (`SAMP18.MAX`): Lucida Sans (ftc 0, fam. 2), Stencil (1, 5), Lucida
Blackletter (2, 4), Impact (3, 2).

`SAMP1.MAX` tem Lucida Sans (0) e Times (4) mas usa `0x0e = 1`: documentos
gravados no Mac ou com fontes ausentes podem citar ftcs que não estão na
tabela; o leitor deve cair na fonte padrão.

## FRAM — molduras

```
u16  n
n × { u8 tamanho, grpprl com os sprms 0x27..0x37 }
```

`S3.MAX` (adesivo bicicleta): `01 00 | 1d 28 03 2a ff 2c fe 00 2d 3a 02 2e 95 00
2f 96 00 30 95 00 31 96 00 35 50 49 43 54 02 00` = figura (`PICT 2`) em
(254, 570) pt, 149 × 150. Os cartões (`WIZA card`) começam com uma moldura
tipo 4 na página 3 (`27 03 00 28 04 29 02 2e 02 00 2f 02 00`), de 2 × 2 pt.

## Objetos (`PICT`, `SPLE`, `BDR `, `SND `)

- **`PICT n`**: cópia **byte a byte** do `PICT` da biblioteca de clip-art, com
  o mesmo número (`S3.MAX` `PICT 2` = `MAXMAG10.ART` `PICT 2`, `META` e
  `PKBM`). `META` = "META" + 4 × `u16` (x, y, largura, altura) + WMF com um
  único `StretchDIB` (60 de 60 figuras dos exemplos; algumas com
  CreatePalette/SelectPalette antes) — o `formats::wmf::meta_dib` atual já
  lê. `PKBM` é a prévia de 1 bit (falta nas figuras desenhadas no "Quadro de
  Figuras", como `SAMP1 PICT 0`).
- **`TWPB`** (contorno para o texto contornar): `u16` 8, `u16` largura,
  `u16` altura, `u16` nº de pontos da borda esquerda, `u16` nº da direita,
  depois os pontos `(i16 x, i16 y)` da esquerda e da direita, com `y`
  crescente de 0 à altura (64 de 64 figuras validadas). O Writer calcula ao
  inserir; o port pode recalcular a partir da máscara. No `SPLE` o cabeçalho
  vale `0x10` e a lista é outra (a interpretar).
- **`BDR n`**: cópia do `BDR` de `0MAX100.BOR`/`1MAX100.BOR`/`2MAX100.BOR`
  (mesmo número e nome, p.ex. `BDR 0 "Paw"`), referenciada pelo `DOP`
  (`0x58`/`0x5a`) ou por uma moldura.
- **`SND n`**: WAVE completo; o número é o da biblioteca (`MAX100.SND`, ou
  `WRITER.SND` ≥ 0x209 ~); referenciado por `FRAM 0x37` ou `SNDT`.
- **`SPLE 0`** (Splot, parcialmente interpretado): cabeçalho de 0x48 bytes
  (`u16 1`, `i16 −13`, `+0x04` tamanho/estilo, `+0x0a` provável forma
  `SCRD`, `+0x0c` nº de letras do texto, `+0x10..+0x17` caixa em coordenadas
  SCRD, `+0x20` provável sombra 0–7 do `TMEN` 2, `+0x40..0x47` estilos e
  cores), nome da fonte (Pascal em `+0x48`, 32 bytes), texto (Pascal em
  `+0x149`), e o restante (≈6–15 KB) com a figura já montada. Os campos
  devem ser fechados com documentos criados no original (tarefa F-7 do plano).

## Demais chunks

- **`SELN`** (48 bytes): `u16 2`, `u16 0x00ff`, `u32` cp início, `u32` cp fim
  da seleção, `i16 -1` em +0x10, zeros.
- **`PRNT`/`WNPR`** (22–24 bytes): `"prr1"`, `u16` dpi x, `u16` dpi y, `u16`
  largura e `u16` altura da página em pixels da impressora, margens
  imprimíveis. Sem impressora (Wine): 72 × 72, 595 × 792.
- **`THMB`**: miniatura `PKBM` da página inteira com paleta embutida: o
  lado maior tem 144 px e o outro é proporcional, arredondado para baixo
  (108 × 144 em "Alto", 144 × 108 em "Largo", 101 × 144 nas faixas de
  542 × 767). 8 bits nos exemplos, 4 bits (16 cores: branco, preto e as
  cores da `CSCH` com #d016de) nos salvos no Wine. O PackBits **recomeça a
  cada linha**: com um fluxo contínuo (como nas miniaturas do `.MAG`) a
  prévia da pasta do Writer mostra lixo. Os 2 bytes após `u16 1` no
  cabeçalho são lixo (variam entre gravações).
- **`ADD `** (128 bytes), também o descritor que a biblioteca usa:

  | Offset | Conteúdo |
  |---|---|
  | 0 | 4 bytes = `WIZA` (`card`, `banr`, `news`, 0) |
  | 5 | `u8` 1 = **documento cifrado pelo Agente secreto** ✔ |
  | 10 | `u8` andar (= `FLOR`) |
  | 11 | `u8` 1–3 (modo de visualização ~) |
  | 12 | `i16` (−2, −4, 0x19, 0x21, 0x32, 0x43, 0x64; a mapear) |
  | 14 | `u16` 1 |
  | 0x1c | 32 bytes: **senha**, cada letra **+5** ("abc" → "fgh"), completada com espaços ✔ |
  | 0x3c | Pascal: título (≤ 33) |
  | 0x5e | Pascal: autor (nome completo do usuário) |

- **`AUTH`**, **`TITL`**: texto sem terminador. `TITL` é o nome digitado
  na barra de status ao salvar; o arquivo recebe o mesmo nome (8.3).
- **`FLOR`**: `u16` 2 (Escritório) ou 3 (Oficina de Projetos). O Fine Artist
  grava 2 nos `.MAG`.
- **`WIZA`**: 4 bytes (`card`, `banr`, `news` ou zeros).

## Agente secreto (cifra)

"Embaralhar" o documento **não altera o `TEXT`**: grava `ADD[5] = 1` e a
senha deslocada; o embaralhamento é só na exibição (as letras são trocadas
por outras e as figuras viram retângulos pretos — captura
`shots/formats_agente_secreto.png`). Para decodificar no original é preciso
a senha ou a senha-mestra "Detetive McZee para salvar o dia" com
Control+Shift + OK (diálogo 0x1388, `STR#` 0x808 [0]). O port pode ler o
documento cifrado normalmente e só exibir o texto embaralhado enquanto
`ADD[5] = 1`.

## Gravação

- Primeiro Ctrl+S: o nome é digitado na barra de status (balão "Digite um
  nome para o seu documento…", `WHAT 0202`). Com o documento já nomeado, o
  Rex pergunta "Salvar a versão antiga e a nova? / Salvar só a nova? / Ops!"
  (`idsSaveBoth`); "antiga e nova" pede outro nome.
- "Salvar só a nova" renomeia o arquivo antigo para um nome temporário
  (`S1.MAX` → `S147.MAX` observado) antes de gravar; no Wine a gravação nova
  não chegou a acontecer nesse caminho (registrado como possível problema do
  Wine, não do formato).
- `~MAXUSER.INF` (preferências do usuário no Writer, não-CHNK): 512 bytes de
  zeros, depois registros `"kwMAX\0" título\0 SHARED\0 USERS\0 MAX\0 \0
  ARQUIVO.MAX` dos documentos pendurados na parede da biblioteca, uma
  tabela de `u16` offsets e um cache com os nomes de todas as fontes do
  sistema (Pascal). Só é regravado ao sair.

## No port

`formats::max` lê e grava o formato inteiro:

- `max::read(&[u8]) -> Result<Document>` e `max::write(&Document) -> Vec<u8>`;
  `Document::from_text` cria um documento novo como o do original.
- Propriedades tipadas: `CharProps` (`FDPC`), `ParaProps` (`FDPP`), `Frame`
  (`FRAM`), `DopProps` (`DOP `), cada uma com um campo por sprm e `extra`
  para o que não conhece (regravado como veio); `Add` (`ADD ` + `TITL`,
  `AUTH`, `FLOR`, `WIZA`) com a senha já decodificada e os bytes não
  interpretados preservados; `Font`, `Page` (`PGD `), `WordSound` (`SNDT`).
- Texto decodificado do Windows-1252 sem perdas; posições em caracteres
  (= bytes do `TEXT`).
- Objetos (`PICT`, `SPLE`, `BDR `, `SND ` e chunks desconhecidos) guardados
  como vieram e regravados entre as páginas FKP e o `DOP `, citados por
  `ObjRef`.

Conferência: os 21 exemplos e as 6 amostras regravados por
`write(read(x))` saem com cabeçalho, índice e todas as partes iguais byte a
byte ao original (só o enchimento difere). Três documentos gravados pelo port
abriram no `WRITER.EXE` (Wine) pela pasta, com prévia, texto e acentos,
negrito/inclinado/sublinhado, fonte e tamanho, cor, marca-texto, efeito
Brilho, centralizado/justificado/direita, recuo, entrelinha, marcadores,
borda de patinhas (`DOP` → `BDR 0`), figura solta (`FRAM` → `PICT 2`),
quebra de folha (`PGD ` + `0x1f`) e Agente secreto (texto embaralhado,
decodificado com a senha). Rolar a página para baixo derruba o `winevdm` também
com os documentos do próprio original (problema do Wine).

Para um `PICT` novo, o port precisa gravar também o contorno `TWPB`
(calculado pela máscara; ver a tarefa de `TWPB`), como o Writer faz ao
inserir a figura.
