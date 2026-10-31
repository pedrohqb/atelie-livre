# Oficina de Projetos (andar 3) — desenho

Subprojeto do Fine Artist: os três projetos da Maggie — **História em
quadrinhos**, **Botão ou adesivo** e **Desenho animado**. Os três são
*modos* da tela de pintura (`scenes/paint.rs`): mesmas ferramentas, mesmo
quadro de 594×347 em (23,109), com um rótulo próprio na barra inferior
(`STR#` do `ARTIST.EXE`: "Fazendo uma História em Quadrinhos", "Fazendo um
Botão", "Fazendo um Adesivo", "Fazendo um Desenho Animado") e um painel
extra na faixa de opções.

## Estado da pesquisa

O original, no Wine, aceita a escolha no balão da Maggie (e nas placas da
vaca/porco/cobra) mas não chega a mostrar a tela do projeto: o programa
continua respondendo, troca a paleta e fica no andar. Por isso a geometria
destes três modos **não foi medida**: é reconstruída a partir dos recursos
que o `ARTIST.PIC` guarda para eles (abaixo), do texto da COMPUTE! de 1994
(Maggie ajuda a escolher o número de quadros, faz o balão de fala, mostra
fundos e adesivos; "flip book eletrônico") e do que o resto da tela de
pintura já impõe. Regra do projeto: usável primeiro; quando o Wine (ou o
Pedro, no CachyOS) mostrar a tela real, ajusta-se.

Recursos encontrados (ids do `ARTIST.PIC`, grupos cor / mono / máscara):

- Dígitos **1–9** coloridos e mono, 24×33: `3a78`…`3a99` (1 = `3a78`/`3a79`,
  2 = `3a7b`/`3a7c`/`3a7d`, 3 = `3a7f`…, 4 = `3a83`…, 5 = `3a87`…, 6 = `3a8b`…,
  7 = `3a8f`…, 8 = `3a93`…, 9 = `3a97`…); dado `3a9b`/`3a9c`/`3a9d` (38×38);
  zero/anel `3a73`/`3a74`/`3a75`/`3a77` (39×35).
- Tiras de filme com 2 a 10 casas, 48 px de altura, verde (ativa) e branca:
  `10e7`/`10fe`/`10ff` (85×48, 2 casas), `1101`/`1102`/`1103` (122, 3),
  `1105`… (159, 4), `1109`… (196, 5), `110d`… (233, 6), `1111`… (270, 7),
  `1115`… (307, 8), `1119`… (344, 9), `111d`… (381, 10). Cada casa tem 37 px.
  Casa isolada com a figura "FUNDO": `10df`/`10e0`/`10e1` (120×42); casa
  vazia `10cf`/`10d0`/`10d1` (66×42); casa verde `10d3`…, `10d7`…
- Botões de texto 70×12: PRONTO `3a57`/`3a58`, PAUSA `3a59`/`3a5c`, PARAR
  `3a5d`/`3a60`, TOCAR `3a61`/`3a64`; tartaruga/coelho 18×9 `3a66`…`3a72`;
  régua TEMPO `02f7`/`02f8`/`02f9` (207×58); botão TOCAR 67×35 `02d7`/`02d8`/`02d9`;
  "COLOQUE AQUI" `02f0`/`02f1` (79×35), "CLIQUE AQUI" `02f4`/`02f5` (70×37);
  faixa de título "★ Desenho Animado ★" `3a53`/`3a54`/`3a55` (482×37).
- Opções da Palavra já usadas: "Oi!" balão `141f`/`1420`/`1421` (43×41).
- Arquivos: o original grava desenhos animados em `*.SLD`; não há amostras
  no CD, então o Ateliê Livre usa o seu próprio formato (abaixo).

## Comum aos três

- `SceneId::Project(ProjectKind)` com `ProjectKind { Comics, Button, Sticker,
  Flipbook }`; `Paint::new_project(kind, drawings, assets)`. A porta "Saída"
  volta ao andar dos Projetos (`SceneId::Projects`, `Room` em
  `Floor::Projects`), não ao Ateliê.
- No andar dos Projetos, o balão da Maggie (opções 0, 1, 2) e as placas
  (vaca "QUADRINHOS", porco "DESENHO ANIMADO", cobra "ADESIVO") abrem o
  projeto. A opção 1 ("um botão ou um adesivo") abre um segundo balão:
  "Botão" / "Adesivo".
- Rótulo da barra inferior (no lugar de "Ateliê"): o texto da `STR#`
  correspondente; o nome do quadro vira "Quadrinhos Novos" / "Botão Novo" /
  "Adesivo Novo" / "Desenho Animado Novo".
- Guardar ao sair: na pasta de desenhos, `quadrinhos-AAAAMMDD-HHMMSS.png`,
  `botao-…png`, `adesivo-…png`; o desenho animado numa subpasta
  `animacao-AAAAMMDD-HHMMSS/` com `quadro-1.png` … `quadro-N.png` e
  `animacao.json` (`{"quadros": N, "intervalo_ms": T}`), mais um
  `animacao.gif` (GIF animado, paleta de 256 cores, para ver fora do
  programa). O último de cada tipo vira também `ultimo-quadrinhos.png` etc.
  para "Trabalhar com … que você já criou" reabrir (opção 1 dos balões das
  placas); se não houver, abre um novo.

## História em quadrinhos (`Comics`)

- Ao abrir, a faixa de opções mostra Maggie perguntando o número de quadros:
  dígitos 1 a 6 (`3a78`…`3a8d`), com o escolhido em cor e os demais em mono.
  Leiautes, no quadro 594×347 com margem de 6 px e calhas de 8 px:
  1 = quadro inteiro; 2 = lado a lado; 3 = três colunas; 4 = 2×2;
  5 = 3 em cima, 2 embaixo; 6 = 3×2. As molduras são linhas pretas de 3 px
  desenhadas **sobre** o quadro (não fazem parte do desenho) e o número
  pode ser trocado a qualquer momento sem apagar nada.
- Pintar e colar respeitam a moldura: o traço, o balde e os adesivos são
  recortados ao retângulo do quadrinho onde o movimento começou
  (`Canvas::clip` no estado da pintura).
- A Palavra com a opção balão ("Oi!") é a mesma da pintura; o balão é
  colocado dentro do quadrinho clicado.
- Imprimir: a página toda com as molduras incluídas (`print()` da pintura
  com o overlay das molduras composto).

## Botão ou adesivo (`Button` / `Sticker`)

- Botão: círculo de 300 px de diâmetro centrado no quadro, borda preta de
  3 px e um anel de 12 px para a "aba" de metal (cinza) — tudo overlay, não
  faz parte do desenho. Adesivo: retângulo de 360×260 de cantos
  arredondados (raio 24), centrado, borda preta de 3 px.
- O desenho é recortado à forma: o traço, o balde, o spray e os adesivos
  ficam limitados à máscara (`Canvas::clip_mask`); fora dela o quadro fica
  branco.
- Imprimir: uma folha com o botão repetido 2×3 (botão) ou 3×4 (adesivo),
  cada cópia reduzida para caber em 210×297 mm a 100 ppp, gravada em
  `impressao-…png` como já faz a pintura.
- Faixa de opções: só o botão de trocar entre botão e adesivo (dois cartões
  desenhados com `frame.rect`, rótulo "Botão" / "Adesivo" na Pablo 8 pt).

## Desenho animado (`Flipbook`)

- Faixa de opções: dígitos 2 a 9 para o número de quadros (`3a7b`…`3a99`);
  escolher um número cria ou corta os quadros (ao cortar, pergunta no balão
  "Jogar fora os quadros a mais?"). Ao lado, TOCAR/PAUSA/PARAR (`3a61`,
  `3a59`, `3a5d`) e a régua TEMPO (`02f7`) com o cursor entre a tartaruga
  (`3a66`) e o coelho (`3a6d`): 5 velocidades, 1000 / 500 / 250 / 125 /
  60 ms por quadro; padrão 250.
- A tira de filme (`1105`… conforme N) fica **sobre a borda esquerda do
  quadro, na vertical**, como a tira dos adesivos (x 33..74), uma casa de
  37 px por quadro, com a miniatura do quadro (594×347 reduzida a 1/9 ≈
  66×39, portanto a casa mostra o centro) e a casa do quadro atual em
  verde. Clicar numa casa troca o quadro atual; o quadro anterior fica
  visível em "papel vegetal" (pixels do quadro anterior misturados a 50 %
  com branco) enquanto se desenha — o truque de flip book; um botão na
  faixa ("Vegetal" ligado/desligado) controla isso.
- Cada quadro é um `Canvas` próprio com sua `History`. Tocar mostra os
  quadros em sequência no próprio quadro, em laço, até PARAR/PAUSA ou
  qualquer clique no quadro.
- Guardar: pasta `animacao-…/` descrita acima; `leave()` grava mesmo sem
  mudanças se houver mais de um quadro com algo desenhado.

## Testes

- Quadrinhos: trocar para 4 quadros e pintar perto da calha não ultrapassa a
  moldura; o PNG guardado não contém as linhas das molduras.
- Botão: um traço que cruza a borda do círculo só pinta dentro; o PNG de
  impressão tem 6 cópias.
- Desenho animado: com 3 quadros, desenhar no 2.º e tocar a 250 ms faz o
  quadro mostrado mudar a cada 0,25 s; a pasta guardada tem `quadro-1.png`
  … `quadro-3.png`, `animacao.json` e `animacao.gif`.
