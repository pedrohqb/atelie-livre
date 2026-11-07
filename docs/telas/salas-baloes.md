# Salas do prédio: balões, objetos e mastro

Comportamento medido na varredura de cliques contra o original (relatórios
`studio`, `projects`, `tricks` e `attic`). Textos do bloco `WHAT 0x100d` de
`ARTIST.EXE` (`atelie-tools what …/ARTIST.EXE --block 0x100d`). Código:
`crates/engine/src/scenes/room.rs` (áreas e fluxo), `room_talk.rs`
(balões), `attic.rs` (sótão), `studio_wall.rs` (plaqueta da parede) e
`paint/fresh.rs` (quadro novo pelo cavalete).

## Regras gerais

- **Balão de chegada**: as opções só apontam. No saguão levam ao andar; nos
  andares abrem a dica ou o balão do objeto (som `254` na bolinha).
- **Objetos com balão** (cavalete, quadro da parede, vaca, cobra, porco,
  estátuas, moldura, luminária): o clique toca `21c`. Com outro balão
  aberto, clicar num deles troca direto de balão; clicar em outro lugar só
  fecha.
- **Ligações** dentro de um balão (lâmpada `21B`, tecla de atalho `21C`,
  palavra do glossário `FFFF`): abrem a dica por cima do balão; o próximo
  clique fecha só a dica. Som `2dd` para "Ignorar isto"/"Descendo?",
  `2e0` para glossário e "Tecla de atalho". A tecla de atalho (`21C`) não
  é ligação no `Balloon` comum; nas salas ela é tratada como lâmpada.
- **Ctrl** (`Input::Ctrl`, enviado pela plataforma ao apertar/soltar):
  Ctrl+clique num objeto da Oficina ou dos Truques entra direto; no mastro
  e na abertura anda dois andares.
- **Luminária e "Tchau"** (todos os andares): "EPA!" modal do
  `WHAT 1001[2]` (o mesmo da Entrada, com a casinha do Rex), sons `2f7` +
  `2fe`; só as bolinhas respondem ("Sair do Fine Artist" sai, "Continuar
  trabalhando" fecha, som `254`); fora delas, a vinheta de recusa `217`. O
  original também solta uma baforada de fumaça da luminária; a figura não
  foi achada no `ARTIST.PIC`, então o port não a mostra.
- **McZee / Maggie**: clique toca `252` (McZee) ou `226` (Maggie) e abre
  `[29]` "Você quer me ver menos?" (Sim / Diga de novo / Deixa para lá).
  "Sim" desliga a ajuda: os andares chegam sem balão e o próximo clique no
  personagem abre `[30]` "Que bom que você me chamou de novo" (Ajudar /
  Não); "Ajudar" religa e mostra o balão de chegada. (Que a ajuda desligada
  valha para os outros andares é suposição do port.)
- **Buraco do rato** sem a varinha: só o som `20a`. Depois de uma surpresa
  da varinha, o balão de chegada volta.

## Saguão (andar S)

- Opções do balão de chegada: dicas `[2]` (34,111), `[3]` (30,65), `[4]`
  (25,45) e `[5]` (95,182), apontando os andares do quadro e a seta.
- Quadro de andares: os nomes e também a figura do prédio (faixas y 18–53
  Truques, 53–75 Projetos, 75–96 Ateliê, 96–118 térreo com a setinha
  "Galeria") levam ao andar ou à Galeria, com a fala `304`. A seta grande
  "Galeria" toca `2fb`.
- Porta do elevador também no topo do arco (y 64–130).
- Aro do mastro no chão (`682b` em (552,389), por baixo do skate): desce ao
  sótão (Ctrl: Truques).
- Menu do McZee em (119,221).

## Elevador

- Números S/2/3/4 do mostrador no alto são botões de andar, como os da
  seta.
- Porta e escotilha (100,40)-(500,480): abrem a porta no andar em que se
  está (`2d4`, depois `311`).
- Luminária inteira e placa "Tchau": o "EPA!".
- McZee: o menu `[29]`/`[30]` em (117,102); com a ajuda desligada, o
  "Subindo? Ou descendo?" não aparece.

## Ateliê (andar 2)

| área | clique simples | clique duplo |
|---|---|---|
| cavalete (tela, plaqueta, poste) | `[27]` "Para criar um quadro novo • Dê um clique duplo na tela em branco." em (130,233) | pintura com quadro em branco "Quadro Novo" (`SceneId::PaintNew`) |
| quadro da parede (283,63)-(440,182) | `[28]` "Esta é a última coisa que você salvou…" em (62,110) | pintura com o último quadro (`SceneId::Paint`) |
| plaqueta da parede (283,186)-(440,206) | campo de nome (texto selecionado) e `[18]`; Return ou clique fora troca o nome, Esc desiste | — |

O quadro da parede é `ultimo.png` reduzido a 1/4 em (287,65), com o nome de
`ultimo.nome.txt` (ou "Quadro Novo") centrado em x=363. Trocar o nome
renomeia a cópia `<nome>.png/.mag/.objetos.json/.fundo.png` e grava o
título novo no `.mag`. O aviso do Rex "Você não pode mudar esse nome" (para
quadros que não são do usuário) não existe no port: o último quadro é
sempre uma cópia na pasta do usuário.

Na pintura, um quadro em branco, sem objetos e sem mudanças não é guardado
por cima do último ao sair (`Paint::keeps_last`): abrir um quadro novo e
sair sem desenhar, ou jogá-lo fora, deixa a parede como estava (no original
a parede só muda quando se salva).

Opções do balão de chegada: 1 → `[27]`; 2 → `[28]`; 3 → `[66]` (a pasta,
em (266,178)); 4 → `[67]` (mastro, em (340,82), com "Tecla de atalho" →
`[68]` e "Descendo?" → `[69]`). A fala `304` toca ao chegar pelo saguão ou
pelo elevador (não pelo mastro).

## Oficina de Projetos (andar 3) e Truques (andar 4)

| objeto | balão | "Ignorar isto" |
|---|---|---|
| vaca (QUADRINHOS) | `[35]` em (8,116) | `[38]` |
| cobra (ADESIVO) | `[36]` em (183,239) | `[38]` |
| porco (DESENHO ANIMADO) | `[37]` em (232,82) | `[38]` |
| estátua do porco (Básico) | `[44]` em (140,193) | `[45]` |
| O Pensador (3D) | `[49]` em (323,215) | `[50]` |

O "Ignorar isto" vem depois das duas opções, como no original: no port ele
é a terceira linha do balão (`room_talk::IGNORE`), com bolinha em vez da
lâmpada.

Bolinha 1 ("criar novo") entra no projeto ou na lição (a cobra ainda
pergunta "Um botão / Um adesivo", porque no port são dois projetos);
bolinha 2 ("trabalhar com um que você já criou") abre a pasta (`02f1`).
Sons do objeto: `21c` + `2f4`.

Balão de chegada da Oficina: 1–3 abrem `[35]`/`[36]`/`[37]`, 4 abre `[70]`.
Dos Truques: 1–2 abrem `[44]`/`[49]`, 3 abre `[71]` (com "Tecla de
atalho" → `[72]`); a palavra "tridimensional" da opção 2 abre o glossário
`[51]` sem fechar o balão. A moldura dos Truques troca a obra-prima a cada
clique (sorteio entre as 8, som `2d2`).

## Mastro e sótão

Ciclo saguão → Ateliê → Projetos → Truques → sótão → saguão. Mastro +1,
abertura −1, Ctrl ±2. O saguão tem a abertura do mastro à direita do skate
((560,390)-(640,426)). O sótão não tem personagem, bolsa nem balão; o aro
foi montado com a abertura `682b` virada e um pedaço do mastro `6823`
(a figura do original não foi achada).
