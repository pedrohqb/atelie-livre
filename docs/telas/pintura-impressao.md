# Tela de pintura: máquina de impressão e Aspirador

Complementa [`pintura-menu-mczee.md`](pintura-menu-mczee.md) e a seção
"Tela de pintura" de [`fine-artist.md`](fine-artist.md). Medido nas capturas
do original (Wine, tela lógica de 640×480; varredura `paint-print`,
`paint-vacuum` e `paint-egg`). No Ateliê Livre: `crates/engine/src/scenes/
paint/{print_machine,vacuum}.rs`, `crates/engine/src/paint/print.rs` e
`crates/engine/src/help_icons.rs`.

## Faixa do Imprimir

Escolher o Imprimir (torradeira, x 458..488) mostra dois botões na faixa:
"imprimir depressa" (`02e7` em (131,62)) e "fazer escolhas" (`02f3` em
(338,61)). Com a ferramenta escolhida o quadro não responde, mas as cores e
os padrões das colunas continuam trocando.

## Máquina de impressão (`WHAT 4c00`)

Tela modal por cima da pintura (que fica esmaecida): moldura laranja em
(18,30)–(590,458), faixa cinza no alto com o botão **FUNDO** (`10df`, ilha,
em (35,37)), área branca com a **prévia da folha** em (26,98), 385×297 (a
folha carta deitada, 11 × 8,5 polegadas, com o contorno da figura quando
não há fundo) e o **McZee** (`4403`, máscara
`4c01`) em (540,2). Os balões do McZee ficam com o canto direito em x 590 e
o topo em y 63, com o rabicho para a boca dele; botões Volta / Pára / Segue
(`040f`, `0417`, `0413`). Clique fora dos botões dá beep (`0217`); Esc é
Pára (imprimindo, cancela); Enter é Segue.

| Passo | Registro | Botões | O que faz |
|---|---|---|---|
| Boas-vindas | `4c00[0]` | Pára, Segue | só "fazer escolhas" (sons `0224` + `02f6`) |
| Fundo | `4c00[1]` | Volta, Pára | bolinhas: **É, quero escolher um fundo** (vai aos fundos), **Sem fundo** (tira), **Deixar como está**; escolher já segue |
| Fundos | `4c00[4]` | Volta, Pára, Segue | miniaturas na faixa ao lado do FUNDO (5 por vez, setas); clicar escolhe; clicar no McZee segue |
| Esticar | `4c00[6]` | Volta, Pára, Segue | bolinhas verdes (`0409`/`040d`): tamanho normal, esticar por igual para caber, esticar para ocupar a página, **pôster** |
| Cor | `4c00[8]` | Volta, Pára, Segue | em cores (ou tons de cinza, conforme a impressora) / em branco e preto sem tons de cinza |
| Cópias | `4c00[10]` | Volta, Pára, Segue | caixa com "1" selecionado; de 1 a 10, senão o aviso `4c00[3]` |
| Pronto | `4c00[11]` | Pára, Segue | só "imprimir depressa" (sons `0212` + `02f6`) |
| Imprimindo | `4c00[12]` | — | as folhas são montadas e gravadas numa thread; depois, até o `lp` terminar (no mínimo 1,5 s); Esc cancela (`100f[20]`) |
| Terminei | `4c00[14]` | Pára | volta à pintura |

O botão FUNDO leva aos fundos em qualquer passo de escolha. Pôster com fundo
abre `4c00[7]` ("Eu não consigo imprimir um pôster com fundo!") e o fundo
sai. Se o `lp` não existir ou falhar, o "Epa!" do Rex (`100f[19]`, som
`02fe`) por cima do "Eu ainda estou imprimindo…"; qualquer clique fecha a máquina, e o Imprimir continua escolhido.
Pára fecha a máquina sem imprimir.

### Folhas impressas

Tudo em pontos (72 por polegada) sobre a área de todas as folhas; a margem
de fora é de ¼ de polegada (18 pontos).

- **Normal**: um pixel do quadro (594×347) por ponto, centrado — a prévia do
  original mostra a figura com 288×168 numa folha de 384 px.
- **Por igual**: a maior escala que cabe na área imprimível.
- **Ocupar a página**: a área imprimível inteira, sem manter a proporção.
- **Pôster**: 2 × 2 folhas, a figura por igual sobre as quatro (a prévia
  mostra as quatro folhas com as divisões).
- **Fundo**: esticado na área imprimível, por trás; o branco da figura deixa
  o fundo aparecer.
- **Branco e preto**: limiar no brilho (sem tons de cinza).

Cada folha vira `impressao-AAAAMMDD-HHMMSS.png` (pôster: `-1` a `-4`), na
pasta de desenhos, a 144 pixels por polegada (1584×1224), e todas vão juntas
para `lp -n <cópias> -o fit-to-page <arquivos>`. A variável `ATELIE_LP` troca
o programa (testes).

A prévia de impressão da Galeria (`scenes/gallery_print.rs`, o "imprimir
depressa" do quadro arrastado para a impressora) usa as mesmas constantes
(`paint::print::machine`: registros, McZee em (541,4), folha, balões, Rex em
(209,167), sons `02ec`/`02eb`/`02fe`, 1,5 s mínimos) e a mesma chamada
`paint::print::spawn_lp`.

### Fundos de impressão (`0MAG100.BOR`)

Os fundos são os chunks `IBGF` de `SHARED/STUFF/0MAG100.BOR` (11, de "PD -
Island Fever" a "PD - Paint & Palette"): uma miniatura `PKBM` e uma parte
`IBGF` de 22 bytes — `"PICT"`, `u16` número do `PICT` com a figura inteira
(um metarquivo `META` com um DIB, como os clip-arts), `0xff`, deslocamentos
e, às vezes, `"FRAM"` + número de um chunk `FRAM` (caixas de texto do
fundo, não interpretadas). `0MAX100.BOR` tem os do Creative Writer;
`1MAX100.BOR`/`2MAX100.BOR` têm as molduras `BDR`.

## Aspirador pelo caminho do mouse

No Aspira-tinta, no Aspira-cópias e no Aspira-cores o bocal (3 × 3 px)
segue o arrasto como um pincel: a tinta some enquanto se arrasta (no
Aspira-cores, só a cor escolhida na coluna); no Aspira-cópias o caminho
aparece pontilhado e nada some. **Shift** + arrasto faz o retângulo de
canto a canto (a "tecla de atalho" de `1008[5]` e `[8]`). Ao soltar, o
Aspirador guarda só os pixels por onde o bocal passou (o Lançador cola a
forma do traço) e o McZee abre:

- `1008[16]` "Onde foi parar?" (Aspira-tinta e Aspira-cores);
- `1008[15]` "Iau!" (Aspira-cópias; um clique sem arrastar num adesivo
  copia o adesivo);
- `1008[14]` "Chocante!" (Aspira-coisas: o adesivo aspirado também vai para
  o Aspirador).

A ligação "Fazer um adesivo" é uma dica (`21b`, registro 17): "Lance o
conteúdo do aspirador no Cavalete de adesivos".

## Ícones dos balões

As figuras `1000`–`10ff` dos registros `WHAT` não são ids do `ARTIST.PIC`
(ver [`what.md`](../formats/what.md)); `help_icons.rs` traduz as conhecidas:

| `WHAT` | Figura | |
|---|---|---|
| `1000`–`1005` | `0495`, `1003`, `1007`, `1031`, `1045`, `104d` (com as máscaras da barra) | pincel, lata, super-ferramentas, palavra, transformadores, imprimir |
| `1036`–`1039` | `100b`, `101b`, `1029`, `103d` | aspirador, adesivos, sons, ovo |
| `1049`–`104d`, `1050`–`1052` | `1557`, `1559`, `155b`, `155c`, `155f`, `1561`, `1567`, `1569` | tinta, coisas, cores, sons, cópias, área, Lançador, tudo fora |
| `1071`, `1072` | `02e7`, `02f3` | imprimir depressa, fazer escolhas |

A dica (`21b`) é uma pedra roxa e os balões do McZee na posição de ajuda
(445,115) têm um rabicho até o nicho (528,104).
