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

## Desenho animado (`Flipbook`) — revisto pelos roteiros do original

Os roteiros de ajuda da Maggie (`WHAT` `3a00` do `ARTIST.EXE`, 86 balões)
descrevem a tela de verdade: o Desenho Animado **não é um flip book
desenhado quadro a quadro**, é uma **apresentação de figuras** já feitas
("você pode ligar as figuras como em um filme e, depois, mostrar para a sua
família"):

- A tela tem **espaços numerados** (a tira de filme `10e7`…`111d`, de 2 a 10
  casas; começa com poucos e uma seta acrescenta mais — só depois de
  preencher os vazios, senão o Rex avisa "Primeiro preencha os espaços!") e
  uma **pasta** (`2230000`, a pasta de desenhos) que abre uma lista de
  figuras do usuário (e as "preferidas" da Maggie em `SHARED/USERS/MAGGIE`).
- **Arrastar** uma figura da pasta para um espaço a coloca; arrastar uma
  figura sobre outra troca as duas de lugar; arrastar para a pasta fechada
  tira a figura. **Clique duplo** numa figura abre a tela de pintura com ela
  (ao sair pela porta a figura volta atualizada ao espaço).
- Cada espaço pode ter um **efeito especial** de transição (lista com
  escolhas: dissolver, cortinas, etc.; e o **dado** = sortear a cada "tocar"),
  um **som** (das categorias de sons do programa) e uma **legenda** (texto
  curto embaixo da figura, 5 a 7 palavras).
- **Desenho de abertura** com título (texto digitado, com o botão de girar
  as palavras) no espaço 1 e figura de "Fim" no último.
- **TOCAR** mostra as figuras em sequência no quadro com a transição, o som e
  a legenda; **PAUSA** dá a Varinha Mágica para tocar sons/adesivos da figura
  parada; **PARAR** volta aos espaços; setas vão uma figura de cada vez; a
  régua **TEMPO** (`02f7`, tartaruga `3a66` ↔ coelho `3a6d`) define a
  velocidade: 5 passos, 4000 / 2000 / 1000 / 500 / 250 ms por figura, padrão
  1000.
- Ao terminar, salva com um **nome** ("Digite um nome descritivo"), que
  aparece depois na pasta.

Versão "usável primeiro" do Ateliê Livre:

- Faixa de opções: botões TOCAR/PAUSA/PARAR (`3a61`/`3a59`/`3a5d`), régua
  TEMPO, botão da pasta (`2230000` ou um cartão "Pasta"), setas ◀ ▶.
- O quadro (594×347) mostra, parado, a tira de espaços: miniaturas 1/4
  (149×87) em grade 3×3 com o número em cima (dígitos `3a78`…) e a legenda
  embaixo; tocando, mostra a figura inteira. A pasta abre uma lista como a
  galeria (miniaturas das figuras de `desenhos/` e das amostras) dentro do
  quadro; clique numa figura da lista = vai para o primeiro espaço vazio
  (sem arrastar, por enquanto); clique num espaço cheio seleciona; com um
  espaço selecionado, clicar noutro troca; a tecla Delete esvazia.
- Efeitos por espaço (botão "Efeito" na faixa, cicla): corte, dissolver
  (pixels aleatórios em 12 passos), cortina para a esquerda/direita/cima/
  baixo, e "dado" (sorteia a cada tocar). Som por espaço: botão "Som" abre a
  lista de sons já existente (`scenes/sounds.rs`). Legenda: botão "Legenda"
  edita um texto (Pablo 8 pt) desenhado numa faixa branca de 16 px na base
  da figura ao tocar.
- Guardar ao sair: pasta `animacao-AAAAMMDD-HHMMSS/` com `animacao.json`
  (`{"figuras": [{"arquivo": "desenho-….png", "efeito": "dissolver",
  "som": "Agudo/3", "legenda": "…"}], "intervalo_ms": 1000}`), uma cópia
  `figura-N.png` de cada figura (para não depender da pasta) e `animacao.gif`
  (GIF animado com as transições renderizadas, paleta de 256 cores do
  programa, sem dependências novas). `ultimo-animacao/` reabre ao entrar.

## Testes

- Quadrinhos: trocar para 4 quadros e pintar perto da calha não ultrapassa a
  moldura; o PNG guardado não contém as linhas das molduras.
- Botão: um traço que cruza a borda do círculo só pinta dentro; o PNG de
  impressão tem 6 cópias.
- Desenho animado: com 3 figuras e TEMPO a 1000 ms, tocar mostra a figura
  2 entre 1,0 e 2,0 s; o efeito "dissolver" muda só parte dos pixels no meio
  da transição; a pasta guardada tem `figura-1.png` … `figura-3.png`,
  `animacao.json` e `animacao.gif`.
