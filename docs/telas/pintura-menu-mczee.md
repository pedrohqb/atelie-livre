# Tela de pintura: menu do McZee, lente, porta e nome do quadro

Complementa a seção "Tela de pintura" de [`fine-artist.md`](fine-artist.md).
Geometria medida no original (Wine, tela lógica de 640×480); ids de
`ARTIST.PIC` e sons do `ARTIST` em hexadecimal. No Ateliê Livre o código
fica em `crates/engine/src/scenes/paint/{menu,zoom,exit_dialog,rename,front}.rs`.

## Menu do McZee

Clicar no McZee do nicho (x 533..580, y 56..104; som `02cf`) abre e fecha uma
coluna branca à direita do quadro: x 538..585, y 105..455 (cobre a borda
laranja de cima), borda preta de 1 px à esquerda e um separador preto entre
os botões, cada um com 44 px de altura e a figura centrada. De cima para
baixo:

| Botão | Figura | O que faz |
|---|---|---|
| Cara do McZee dormindo | `041b` | Balão `WHAT 100e[1]` "OK, vou embora se você quiser mesmo que eu vá." (som `022d`). "É, por favor dê o fora" (`0252`) tira o McZee do nicho e desliga os balões automáticos das ferramentas; o nicho vazio chama ele de volta com `100e[0]` ("Sim, por favor!" religa). |
| NOVO (estrela) | `15e7` | Diálogo "Epa!" (abaixo) sem abrir a porta; depois, quadro em branco. |
| Pasta vermelha | `044f` | Guarda o quadro e abre a pasta de trabalhos por cima (`Push(Portfolio)`, som `02f1`). No original é o diálogo "Abrir" (título, artista, lista da pasta). |
| Cofrinho SALVAR | `15e3` | O rótulo da barra de baixo vira campo de nome (som `0232`); Return guarda. |
| Cartola | `15a3` | A porta abre com a Maggie na soleira (`6d87`/`6d89` em (580,18)) e o balão `WHAT 4600[1]` "Oi! Você quer que eu te mostre alguns truques de desenho?" (som `02f6`). "É, me mostre…" vai ao andar dos Truques; "Não, obrigado!" fecha a porta (`0254`+`02d7`). |
| Varinha Mágica | `0457` | Balão `100e[2]` (som `0307`); o próximo clique num adesivo com som toca o som. |
| Cara de sol | `03f3` | Uma piada ou ditado do bloco `WHAT 0201` (58 registros, ao acaso), num balão de 200 px em (218,197), sem opções: qualquer clique fecha (som `0226`). |
| Álbum | `0453` | História do McZee por cima (`Push(BookStory)`, som `02f6`). |

Escolher um botão fecha o menu; cliques fora da coluna seguem normais com o
menu aberto. Com a coluna aberta, o pedaço do quadro embaixo dela não pinta.

## Diálogo "Epa!"

Porta de saída (x 580..635, y 18..105) e NOVO abrem o diálogo `WHAT 0203[0]`
("now what?"): caixa amarela pontilhada em (210,170)–(430,310) com a figura
`02a9` (casinha do Rex, EPA!, cachorro) em (223,174), o texto "O que você
quer fazer agora?" e três bolinhas: **Salvar este trabalho**, **Jogar fora —
reciclar**, **Ops! Ainda não terminei!** Sons ao abrir: `02d6` e `02fe`. Pela
porta, a porta fica aberta (batente preto `105d`/`105f`).

O diálogo é modal: só as bolinhas respondem; clique em qualquer outro lugar
dá um beep (`0217`) e Esc não fecha.

- **Salvar**: abre o campo de nome; Return guarda com o nome e então sai
  para o Ateliê (porta) ou começa um quadro em branco (NOVO).
- **Jogar fora**: descarta o quadro (fica em branco) e sai (porta) ou
  recomeça (NOVO; o ovo traz o anterior de volta).
- **Ops!**: fecha o diálogo e a porta (`0254`+`02d7`).

Nos projetos da Oficina e nas lições dos Truques a porta sai direto, como
antes (os diálogos deles são outros: `0202[9]`, `0202[16]`, `3a00[76]`…).

## Nome do quadro

O rótulo "Quadro Novo" na barra de baixo (x 152..486, y 461..475) vira campo
de texto ao ser clicado (som `0230`), pelo SALVAR do menu (`0232`) ou pelo
"Salvar" do "Epa!": o nome fica selecionado (fundo azul `(48,150,250)`,
texto branco, à esquerda do campo), digitar substitui, Backspace apaga,
Return/Enter guarda e **Esc não cancela**; um clique fora do campo cancela
sem guardar. Abre também o balão `100e[4]` ("Digite um nome para o
documento…"). Nome vazio volta a "Quadro Novo".

Ao gravar com nome, além de `ultimo.png` (e `ultimo.objetos.json`,
`ultimo.fundo.png`), a cópia vai para `<nome>.png` e `<nome>.mag` (com o
nome como título do `.MAG`) em vez de `desenho-AAAAMMDD-HHMMSS.*`; barras e
outros caracteres impróprios viram `_`. O nome fica em `ultimo.nome.txt`
para o rótulo voltar quando a tela reabre; sem nome, o arquivo é apagado.
A pasta de trabalhos mostra o nome do arquivo (ou o título do `.MAG`).

## Lente de aumento

A lupa da barra de baixo (x 522..546) amplia o quadro 3× (som `0309`),
centrado no último ponto do quadro tocado, e abre o balão `100e[6]` ("Lente
de aumento"). Aparecem quatro setas numa caixa branca da barra de baixo
(x 558..626): ◀ `14f3` em x 565, ▶ `151b` em 579, ▲ `151f` em 593, ▼ `1523`
em 607 (y 462, 12×12). Cada seta rola a vista meia tela (sons `02bc`…`02bf`;
sem som quando já está na borda). Pintar, despejar, aspirar e arrastar
objetos funcionam na lente (os pontos da tela são convertidos em pixels do
quadro). O segundo clique na lupa volta ao normal (som `030a`).

## Outros

- A caixa "Ateliê" da barra de baixo (x 44..140) abre o mapa do prédio,
  como a cabeça vermelha ao lado.
- **Imprimir** é uma ferramenta: o ícone fica ativo e a faixa mostra
  "imprimir depressa" (`02e7` em (131,62)) e "fazer escolhas" (`02f3` em
  (338,61)); enquanto está escolhida o quadro não responde (as cores e os
  padrões, sim). Os dois abrem a máquina de impressão do McZee, descrita em
  [`pintura-impressao.md`](pintura-impressao.md). Nos projetos e lições, o
  ícone imprime na hora.
- O botão direito do mouse faz o mesmo que o esquerdo em toda a tela.
- Áreas de clique: a coluna de padrões responde até x ≈ 630 e o ícone
  Imprimir até x ≈ 494 (no original 488).
