# Recursos do Splot: `TMEN`, `CSCH`, `PATS`, `SSHD`, `SMNM`, `TDRL`

Recursos do Cavalete de palavras com forma ("Splot") nos executáveis.
**Idênticos byte a byte** no `ARTIST.EXE` e no `WRITER.EXE` (fora o
preenchimento do NE); o Fine Artist tem ainda o `TMEN 4`. As formas estão em
[`scrd.md`](scrd.md). Leitor: `formats::splot_res`; uso:
`engine::paint::shaped` e `engine::scenes::paint::words::shaped_easel`.

## `TMEN` — menus de texto

```
u16   n
u32   0
n × u8   estilo do item, bits do Mac: 1 negrito, 2 inclinado, 4 sublinhado, 8 contorno
n × { u8 tamanho, texto Windows-1252, u8 0 }
```

- `TMEN 1` (9): Normal, **Negrito** (1), Inclinado (2), Contorno (8),
  MAIÚSCULAS, Vertical, Preencher, Em cima/embaixo, Esquerda/direita — botão
  AA. No Wine só o negrito aparece no menu. Os três últimos agem na forma
  (ver `scrd.md`).
- `TMEN 2` (8): Sem sombra, Sombra normal, Sombra do amanhecer, Sombra do
  anoitecer, Girar e sumir, Girar em círculo, Sombra convergente, Efeito
  tridimensional — botão de sombras.
- `TMEN 4` (só no Fine Artist, 3): Normal, Negrito (1), Sublinhado (4).

## `CSCH` — cores

```
u16  16       cores
u16  24       conjuntos
u16  3        máximo de cores num conjunto
16 × { u16 r, u16 g, u16 b }   como no Mac: o byte alto é a cor
24 × { u8 n, n × u8 índice }   conjuntos da lista de cores, em ordem
```

Cores: #ffffff, #ffee4c, #ff8c32, #b96524, #fa4a72, #fa0039, #8888e6,
#6141db, #de38c7, #932ca3, #5ccb53, #478c50, #bebebe, #8c8c8c, #646464,
#000000 (as mesmas do texto do Writer). Os conjuntos são a lista do botão
de cores do cavalete: preto, branco, preto/branco, as cores 1 a 14 e sete
mistas, alternadas por letra: vermelho/verde-escuro, roxo/verde-escuro,
rosa/lilás, verde/rosa/verde-escuro, vermelho/branco/azul, roxo/magenta/rosa,
laranja/preto/vermelho.

## `PATS` — padrões

`u16 13` + 13 × 8 bytes, uma linha de 8 pixels por byte, bit alto à
esquerda, **bit 1 = tinta** (convenção do Mac). A lista do botão de padrões
mostra um quadrado cheio e depois os 13, em ordem (casas, carinhas,
estrelas, …, listras diagonais no 8º, xadrez no 10º, tijolos no 11º).
Diferente do `PAT#` (`recursos.md`), que guarda as linhas em `u16`.

## `SSHD` — sombras (forma bruta)

1536 bytes: `u16 7` (as sombras do `TMEN 2` depois de "Sem sombra"), `u16 1`
e registros de tamanho variável com a cor da sombra (3 × `u16` do Mac:
#8c8c8c, #646464, #bebebe, branco nas cópias de "Girar em círculo"),
deslocamentos (`fa ff fe ff` = (−6, −2)) e matrizes 2×2 em ponto fixo com
**1200 = 1,0**. "Girar em círculo" são 17 cópias giradas de 20° em 20°
(20° … 340°): a de 20° no fim do registro da sombra (contagem `0x11` em
+0x172, matriz 1127, −410, 410, 1127) e 16 registros de 60 bytes que
começam por `b0 04 ff ff 03 00 06 00 03 00`, com a matriz em +44
(`Sshd::spin_matrices`). Os demais campos não foram fechados.

## `SMNM` — geometria do cavalete (forma bruta)

Dois recursos de 20 `u16`. O primeiro par é o tamanho da moldura do
cavalete: **463 × 335** no `SMNM 1` (o do Fine Artist, igual ao medido na
tela) e 418 × 245 no `SMNM 2`. Os demais (49, 47, 75, 96, 15, 661, 68, 36,
2, 150, 20, 1, 150, 20, 60, 12, 48, 16) devem ser botões e prévia.

## `TDRL` — os "preferidos"

`u16 13` + 13 registros de 12 bytes: `ff ff`, `i16` giro, forma, sombra,
padrão, conjunto de cores, estilo das letras, fonte (4; 0 com estilo 0),
estilos, `ff` — as mesmas escolhas, na mesma ordem, dos bytes `+0x40..+0x47`
do `SPLE` ([`sple.md`](sple.md)). No Writer os botões ＋ e − do cavalete
percorrem esses registros (`splot_res::favorite`).
