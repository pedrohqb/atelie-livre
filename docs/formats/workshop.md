# `WORKSHOP.CHK`: dados da Oficina de Idéias (Creative Writer)

`SHARED/STUFF/WORKSHOP.CHK` é um arquivo [`CHNK`](chnk.md) (criador `ICCC`,
little-endian, 35 chunks) com os dados da **Máquina de Idéias** (o "Splot
Machine" da versão em inglês) e do **Quadro de Figuras**. Leitor:
`crates/formats/src/workshop.rs`; gerador de sentenças:
`crates/engine/src/writer/ideas/machine.rs`.

| Chunk | Nome no índice | Conteúdo |
|---|---|---|
| `STTB 0..4` | `phrase1.txt` … `phrase5.txt` | tabelas de frases: 102, 100, 126, 24 e 82 |
| `SMDG 0` | `Las cosas de Max - 2` | gramática da Máquina (uma regra por janela) |
| `PWDG 0` | `Max's cool stuff` | lista do Quadro de Figuras |
| `PICT 0..27` | títulos (`Cara de cão`, …, `O resgate`) | figuras do Max, só a parte `META` |

Todos os inteiros são `u16` little-endian. "Nenhum" é `0xFFFF`.

## `STTB`: tabela de strings

```text
u16  tamanho do chunk − 10
u16  n (número de strings)
i16  −1
u16  bytes extras depois de cada string (2 aqui; 0 no ~MAXUSER.INF)
u16  0
n × u16  offset da string, contado a partir do byte 10
strings  u8 tamanho + texto Windows-1252, seguido dos bytes extras
```

Os dois bytes extras são um número de som (`SND` do `WRITER.SND`) que a
Máquina toca quando a frase aparece: a rotina de sorteio
(`seg68:0x0cc4`) lê o extra do item escolhido e, se não for `0xFFFF`,
toca o `SND` com esse número. No `WORKSHOP.CHK` todos são `FF FF`.

## `SMDG`: gramática da Máquina de Idéias

```text
u16  0
3 × registro (um por janela, até o fim do chunk):
     3 × u16  números dos STTB que preenchem os %s (0xFFFF = não usado)
     u16      F, número de figuras-rebus
     62 bytes formato: string Pascal (u8 tamanho + texto), resto zerado
     F × { u16 0x100 | caractere, u16 som (0xFFFF = nenhum) }
```

O byte alto `01` de cada figura marca "figura" (no estado da janela o
original guarda separados o caractere e esse marcador).

| Janela | Tabelas | Formato | Figuras |
|---|---|---|---|
| 1 (quem) | `STTB 0` + `STTB 1` | `"    %s %s"` (4 espaços na frente) | 63 |
| 2 (o quê) | `STTB 2` | `"%s"` | 17 |
| 3 (onde) | `STTB 3` + `STTB 4` | `"%s %s"` | 28 |

A sentença é janela 1 + espaço + janela 2 + espaço + janela 3, com os
espaços das pontas de cada janela tirados (o texto que vai para o documento
e para o Caderno de Idéias não tem os 4 espaços do formato).

### Sorteio de uma janela

Rotina `seg68:0x0cc4` do `WRITER.EXE` (argumentos: janela e modo). Sejam
`U` o número de entradas do usuário na janela, `F` o de figuras e `T` o
**maior** tamanho entre as tabelas da janela (o carregador do `SMDG`,
`seg68:0x1584`, faz `T = max(T, |STTB|)` para cada tabela, a partir de 0):

1. sorteia `r` uniforme em `0 .. T + F + U` (gerador congruencial de 32 bits
   `x = x·0x5D588B65 + 0x269EC3`, semente `timeGetTime()`; o resultado é
   `(x >> 16)·n >> 16`);
2. `r < U` → a entrada `r` do usuário;
3. `r < U + F` → a figura `r − U`;
4. senão → texto: para cada tabela da janela, uma frase sorteada
   uniformemente e de forma independente, postas no formato (`sprintf`).

Proporção de figuras (sem entradas do usuário): janela 1 63/165 = 38%,
janela 2 17/143 = 12%, janela 3 28/110 = 25%.

Modos (teclas seguradas no clique, `GetKeyState`): **Ctrl** = só as
entradas do usuário (`r` em `0..U`); **Shift** = só as figuras (`r` em
`U..U+F`). Se não houver nenhuma entrada (ou figura), vale o sorteio
completo. Ctrl vence Shift.

Alavanca: durante **1000 ms** (`timeGetTime() + 0x3e8`) as três janelas são
sorteadas de novo a cada volta do laço; fica o último sorteio. Botão redondo
de uma janela: um sorteio só daquela janela. O modo vale nos dois casos.

## `PWDG`: lista do Quadro de Figuras

```text
u16  n (28)
     string Pascal "~MYPICS" (arquivo do usuário com as figuras próprias,
     ~MYPICS.CHK na pasta do usuário)
u16  0 (não interpretado)
n × { u16 número do PICT, u16 som (0xFFFF) }
```

A ordem da lista é a do Quadro (as figuras próprias do usuário vêm antes,
das mais novas para as mais velhas).

## `PICT`: figuras do Max

Uma parte `META` só (sem prévia `PKBM`): `"META"`, `u32 0`, `u16 largura`,
`u16 altura` e um WMF com um único `StretchDIB` de 1 bit, lido por
`formats::wmf::meta_dib`. Tamanhos de 111×225 a 504×210. O título é o nome
do chunk no índice.

## `SUDG` (no `~MAXUSER.INF` do usuário)

`3 × u16`: o número do `STTB` (no mesmo `~MAXUSER.INF`, com 0 bytes
extras) que guarda as entradas próprias da janela 1, 2 e 3; `0xFFFF` =
nenhuma. Ex.: `00 00 01 00 02 00`. O port grava as entradas no seu próprio
formato (`writer/ideas/store.rs`, ver `arquivos-do-port.md`).

## Figuras-rebus

O caractere `c` (1–255) é desenhado com o bitmap `2c − 1` (colorido, 3–6
cores) e a máscara `2c` (1 bit; preto = opaco) do `WRITER.PIC`, ≈30×28. Ex.:
gato `c = 0x25` → `0x0049`/`0x004a`; casa `c = 0x1a` → `0x0033`/`0x0034`.
São as mesmas figuras do Transformador "figuras junto com as palavras" do
editor. Código: `crates/engine/src/writer/ideas/rebus.rs`.
