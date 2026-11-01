# Pincel: as 72 modalidades

Auditoria de 01/11/2026: cada botão das 6 páginas do pincel foi usado no
Ateliê Livre e no Fine Artist original (Wine, mesma tela 640×480), com o mesmo
arrasto em diagonal seguido de um em horizontal. O código fica em
`crates/engine/src/paint/brush.rs` (testes em `paint/brush/tests.rs`) e a
geometria em `paint/geom.rs`.

Legenda da coluna "antes": **ok** = igual ao original; **errado** = pinta, mas
não o que o original faz; **nada** = não pinta. Nenhuma opção estava em
"nada": o problema era que 33 delas faziam outra coisa (palpites antigos:
efeitos como carimbos fixos, página 5 com outras formas e página 6 inteira de
sprays).

## Regras medidas no original

- **Carimbos (página 2)**: um carimbo a cada movimento do mouse que fica a 16
  pixels ou mais do último carimbo (não interpola o caminho). Com movimentos de
  11 px saem carimbos a cada 22 px; "quanto mais depressa, menos peixes".
- **Linhas dos efeitos da página 3**: 2 pixels de espessura. As da página 5
  têm 1 pixel.
- **Bolhas**: a cada movimento, um círculo centrado no ponteiro com raio igual
  à distância andada desde o movimento anterior; movimentos curtos não fazem
  bolha.
- **Espiral**: de Arquimedes, voltas a 10 px; nasce com raio 7 no clique e
  cresce a distância de cada movimento, no máximo 10 px por movimento (medido
  com arrastos lentos e rápidos de mesmo comprimento).
- **Rachadura e teia**: elásticas, centradas no ponto do clique, do tamanho da
  distância até o ponteiro; só pintam ao soltar. A teia é hexagonal, com
  vértices a leste e oeste, raio = distância / cos 30°, e anéis mais juntos no
  meio (2 + raio/50 anéis).
- **Trilhas** (ferrovia, rodovia, cano, rio, toca): ladrilhos 21×21 numa
  grade presa ao canto do quadro (medido: as peças caem em múltiplos de 21).
  Cada casa é desenhada quando o ponteiro sai dela, com a peça escolhida pelos
  lados de entrada e saída (reta ou curva); diagonais viram escada (desce
  primeiro). A casa onde o traço termina não é desenhada, como no original.
  **Pegadas** usam a mesma grade, mas andam em 8 direções, com a imagem da
  direção do passo.
- **Página 5**: reta; linha radial no múltiplo de 15° mais próximo; espelhos
  pela vertical e pela horizontal que passam pela âncora; cria-lados (cada
  arrasto liga o fim do lado anterior ao ponto onde soltou, até clicar fora do
  quadro ou escolher de novo o botão); cria-curvas (B-spline quadrática pelos
  pontos do arrasto, a cada 16 px, pontas fixas); quadrado, círculo e
  triângulo equilátero de dentro para fora com o ponteiro no círculo
  circunscrito (meio lado = distância/√2; ápice para cima); retângulo e forma
  oval de canto a canto; triângulo 2 (primeiro arrasto = primeiro lado, o
  segundo arrasto escolhe o terceiro canto e fecha).
- **Página 6**: discos de 8/16/32 (giz), 8/16/32 com borda de 2/4/4 em tom
  claro (lápis de cera) e 4/8/24 (pastel), medidos na altura do traço. Cerca de
  3/4 dos pixels levam a cor, numa textura fixa na tela (passar de novo não tapa
  os buracos); o lápis de cera tem o miolo na cor com falhas no tom claro e a
  borda só no tom claro; o pastel tem riscos horizontais. Cada pixel é pintado
  uma vez por traço.

## Tabela

| Pág. | Botão | Original (balão `WHAT 1002`) | O que o original faz | Antes | Depois |
|---|---|---|---|---|---|
| 1 | 0–3 | redondos 16, 12, 8, 4 | forma carimbada ao longo do caminho | ok | ok |
| 1 | 4–6 | mais, xis, ponto | idem | ok | ok |
| 1 | 7–11 | quadrados 2, 4, 8, 12, 16 | idem | ok | ok |
| 2 | 0 | Peixes (Fish) | carimbo a cada movimento ≥ 16 px | errado (espaçamento interpolado) | ok |
| 2 | 1 | Cerejas | idem | errado (idem) | ok |
| 2 | 2 | Globos | idem | errado (idem) | ok |
| 2 | 3–11 | nuvem, joaninha, flor, montanha, estrela, árvore cheia, árvore seca, floco, carinha | idem | errado (idem) | ok |
| 3 | 0 | Bolhas | círculo do tamanho do movimento | errado (anéis fixos) | ok |
| 3 | 1 | Rabiscos | linha tremida de 2 px | errado (zigue-zague carimbado) | ok (aprox.) |
| 3 | 2 | Tinta spray | pontos num raio de ~10 px enquanto aperta | errado (estrelas carimbadas) | ok (aprox.) |
| 3 | 3 | Rachadura | 6 braços tortos, elástica | errado (carimbo fixo) | ok (aprox.) |
| 3 | 4 | Teia de aranha | teia hexagonal elástica | errado (hexágonos carimbados) | ok |
| 3 | 5 | Espirais malucas | espiral que cresce com o arrasto | errado (círculos carimbados) | ok |
| 3 | 6 | Ferrovia | trilha de ladrilhos com curvas | errado (ladrilho solto repetido) | ok |
| 3 | 7 | Rodovia | idem | errado (idem) | ok |
| 3 | 8 | Pegadas | pegadas em 8 direções na grade | errado (ladrilho do cano) | ok |
| 3 | 9 | Cano | trilha de ladrilhos | errado (ladrilho solto) | ok |
| 3 | 10 | Rio | idem | errado (idem) | ok |
| 3 | 11 | Toca do esquilo | idem | errado (idem) | ok |
| 4 | 0–11 | caligrafia: diagonais, horizontais, verticais | forma carimbada ao longo do caminho | ok | ok |
| 5 | 0 | Linha | reta elástica de 1 px | ok | ok |
| 5 | 1 | Linha radial | reta presa a múltiplos de 15° | errado (reta com base) | ok |
| 5 | 2 | Espelho direita-esquerda | reta + espelho pela vertical da âncora | errado (curva livre) | ok |
| 5 | 3 | Espelho em cima-embaixo | reta + espelho pela horizontal da âncora | errado (curva fechada) | ok |
| 5 | 4 | Cria-lados | poligonal de vários arrastos | errado (poligonal num arrasto) | ok |
| 5 | 5 | Cria-curvas | curva arredondada pelos pontos | errado (serpentina de 3 px) | ok |
| 5 | 6 | Quadrado | de dentro para fora | errado (retângulo cheio) | ok |
| 5 | 7 | Círculo | de dentro para fora | errado (elipse cheia) | ok |
| 5 | 8 | Triângulo | equilátero de dentro para fora | errado (triângulo cheio na caixa) | ok |
| 5 | 9 | Retângulo | de canto a canto | ok | ok |
| 5 | 10 | Forma oval | de canto a canto | ok | ok |
| 5 | 11 | Triângulo 2 | dois arrastos: lado e terceiro canto | errado (triângulo na caixa) | ok |
| 6 | 0–2 | Giz fininho/médio/grosso | disco 8/16/32 com textura | errado (spray) | ok |
| 6 | 3–5 | Lápis de cera fininho/médio/grosso | miolo na cor, borda em tom claro | errado (spray) | ok |
| 6 | 6–8 | Pastel fininho/médio/grosso | disco 4/8/24, textura em riscos | errado (spray) | ok |
| 6 | 9 | Aquarela | mistura a cor com o que está embaixo | errado (spray) | ok (não medido) |
| 6 | 10 | Clarear | puxa para o branco | errado (spray) | ok (não medido) |
| 6 | 11 | Escurecer | puxa para o preto | errado (spray) | ok (não medido) |

"aprox." = mesma ideia e tamanho, mas o desenho exato é sorteado e não
reproduz o gerador do original. "não medido" = no Wine o original não pinta
nada com esses três botões, nem sobre branco nem sobre cor (provavelmente
dependem do modo de paleta de 256 cores); a mistura (metade da cor na
aquarela, 35% para o branco ou o preto) segue o texto do balão.

## Imagens usadas (`ARTIST.PIC`)

| Uso | Ids |
|---|---|
| Carimbos da página 2 | `164b 16a9 16ab 16ad 16af 16b1 16b3 16b5 16b7 16b9 16bb 16bd` |
| Ferrovia: H, V, SL, SO, NO, NL | `11a3 11a5 11a7 11a9 11ab 11ad` |
| Rodovia: H, V, SL, SO, NO, NL | `11b9 11b7 11b1 11af 11b3 11b5` |
| Cano (pares alternados) | H `16bf 16d6`, V `16d7 16d8`, SL `16df 16e0`, SO `16d9 16da`, NO `16db 16dc`, NL `16dd 16de` |
| Rio | H `16e1 16e2 16e9 16ea`, V `16e5 16e6 16ed 16ee`, SL `16ef 16f0`, SO `16e3 16e4`, NO `16e7 16e8`, NL `16eb 16ec` |
| Toca do esquilo | H `16f1 16f2 16f9 16fa`, V `16f5 16f6 16fd 16fe`, SL `16ff 1700`, SO `16f3 16f4`, NO `16f7 16f8`, NL `16fb 16fc` |
| Pegadas L, SE, S, SO, O, NO, N, NE | `112d 1199 11a1 119d 1195 1197 119f 119b` |

As ligações das peças foram tiradas da tinta nas bordas de cada ladrilho; a
ordem da toca segue a do rio.

## Fora desta rodada

- **Shift e Ctrl** durante o arrasto (reta presa no Shift; Ctrl preenche
  quadrado, círculo, triângulo, retângulo, oval, curva e cria-lados; Ctrl faz
  a linha a partir do centro e os espelhos nas 4 direções; Shift troca
  quadrado ↔ retângulo e círculo ↔ oval): o `Input` do mouse ainda não traz o
  estado das teclas.
- Balões de ajuda das opções 3–11 da página 2 (`wt …`) e 9–11 da página 3
  (`cont …`): o leitor de `WHAT` não lista esses registros; essas opções
  mostram o balão geral do pincel.
