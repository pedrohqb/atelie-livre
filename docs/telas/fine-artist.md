# Fine Artist original: telas e fluxo

Mapa das telas do Microsoft Fine Artist (versão Windows 3.1 em português),
levantado rodando o original no Wine. Serve de referência para remontar as
telas no Ateliê Livre. Coordenadas na tela lógica de 640×480; ids são de
`ARTIST.PIC`, salvo indicação. Capturas de tela do original não entram no
repositório.

## Como rodar o original sem tela (para medir)

```sh
Xvfb :99 -screen 0 1024x768x24 &        # 24 bits: com 8 bits as cores saem erradas
export WINEPREFIX=$HOME/wine-fa          # o prefixo precisa pertencer ao usuário
cd $WINEPREFIX/drive_c/MSKIDS            # cópia da pasta MSKIDS instalada
env -i HOME=$HOME PATH=/usr/bin:/bin DISPLAY=:99 WINEPREFIX=$WINEPREFIX \
    WINEDEBUG=-all wine ARTIST.EXE &
```

Sem o `env -i`, o programa reclama de "espaço insuficiente para o ambiente".
A janela de 640×480 fica centrada no Xvfb (deslocamento 192,144). Com
`xdotool mousemove X Y click 1` e `import -window root -crop
640x480+192+144` dá para clicar e capturar; o duplo clique é `click --repeat
2 --delay 120 1`.

## Fluxo

1. **Entrada** (fundo `4139`; McZee de livro na mão `42f9` em (435,112)):
   balão `WHAT 4400[1]` no alto à esquerda; quadro-negro verde em
   (120,170)–(431,370) com a lista de usuários (caixa branca (130,179),
   linhas de 18 px, setas `2db`/`2df` em (169,343) e (220,343)) e os botões
   **Sou eu** (`473`, (280,215)), **Primeira vez** (`477`, (280,253)) e
   **Procurar** (`47b`, (280,288)). "Sou eu" com um nome escolhido esconde
   o quadro-negro e abre `4400[2]` em (64,76), **modal**, com Volta/Para:
   Imaginópolis / mudar o cadastro / história do McZee / direto à tela de
   pintura (sem passar pelo cômodo do Ateliê); Volta devolve a lista sem
   nome escolhido; clique fora toca a vinheta de recusa. Sem nome, o
   aviso `4402[3]`.
   Avisos "EPA!" (`WHAT 4402` e o de saída `1001[2]`): balão branco com a
   arte casinha-do-Rex + "EPA!" + cachorro (`2a9`, 195×48; no `WHAT` é a
   figura `230`) e sem botão — qualquer clique fecha. O de saída ("O que
   você quer fazer? Sair do Fine Artist / Continuar trabalhando", em
   (214,179)) abre pelo Para de qualquer balão da Entrada por cima do que
   estava; só as opções respondem; fora dele, vinheta de recusa.
   Clicar no McZee toca uma fala curta; as setas fazem o clique mesmo com
   a lista parada; Procurar fala, relê a lista e perde a escolha.
   Sons (ids do `ARTIST.SND`, em `entry::snd`): abertura `300` + `2f6` +
   `2fd` em sequência; escolher um nome `230`; Sou eu / Coloque aqui `239`;
   Primeira vez `2fb`; Procurar `26d`; setas `2bc`; McZee `252`; Para
   `2ec` + vinheta `2fe`; aviso "EPA!" `2fe` (depois do som do botão);
   recusa `217`; opção escolhida (bolinha) e "Continuar trabalhando" `254`;
   Segue `2eb`; Volta `2f5`; escolher a pasta `22d`.
   *No port:* a lista vem de `usuarios/` (ver README); "Procurar" relê a
   pasta (não há discos). Os sons em sequência são enfileirados pela cena
   (`Entry::play`) e tocam um depois do outro pela duração do PCM; uma
   ação nova interrompe a sequência.
2. **Cadastro** ("Primeira vez"): balões sucessivos em (112,78) com os
   botões Volta (`40f`) / Para (`417`) / Segue (`413`), 31×14, centrados no
   rodapé do balão: `4400[4]` nome e sobrenome (caixas de 144 px),
   `4400[8]` bicho preferido (12 figuras `45b`, `45d`… em 4×3, bolinha
   `409`/`40d`), `4400[6]` "Onde você quer que eu guarde o seu trabalho?"
   (Volta/Para; o quadro-negro volta com a lista de unidades, o botão
   **Coloque aqui** (`47f`, (282,218)) e Procurar), `4400[5]` apelido
   (campo já preenchido com o primeiro nome e **selecionado**: digitar
   substitui). Avisos do `WHAT 4402` (balões "EPA!", ver acima): `[1]`
   nome vazio, `[2]` apelido inválido (mais de 8 caracteres, espaço ou
   pontuação), `[0]` apelido repetido. Depois `4400[3]` em (70,76) com o
   botão Para: conte mais (história) / direto para Imaginópolis / mudar o
   cadastro; o Volta do nome volta a esse balão quando se chegou por
   "Quero mudar…", senão à lista. Para em qualquer passo abre o balão de
   saída por cima.
   *No port:* no passo da pasta a lista mostra só a pasta de dados (não
   há discos; a dica do disquete `4400[7]` é omitida); o apelido é
   sugerido a partir do nome.
3. **História do McZee** (álbum `4201`, 495×314, em (72,160) sobre o fundo
   da Entrada): uma página por balão do `WHAT 4200` (11), foto de 169×119 na
   moldura da esquerda (`420b`, `4210`, `4211`, `4218`, `423b`, `4273`,
   `4293`, `4299`, `42b3`, `42b9`, `42f3`) e legenda ou foto na da direita
   (`424b`, `4219`, `4223`, `4241`, `426b`, `428b`, `42a1`, `4281`, `42cb`,
   `42e1`). Segue, Enter, → ou um clique no balão ou na página direita do
   álbum passam a página (rótulo "mais", `42c1`, no canto da página direita
   enquanto há seguinte); Volta, ←, PageUp ou um clique na página esquerda
   voltam; Para sai. *No port:* as figuras `422b` (Max) e `4233` (McZee
   voando) não são usadas; o comportamento delas no original não foi
   confirmado.
4. **Imaginópolis** (`6cfd`, 641×480, alinhado à esquerda): a porta "Entre"
   do Museu Mágico, por volta de (302,172), leva à rua. O McZee passa no
   carrinho da montanha-russa (roteiro `ATAG` 0x2bc) logo ao chegar
   (0,2 s depois de a tela aparecer, 3,5 s de duração), uma vez só: em 90 s
   de gravação ele não volta. A Entrada e o título ficam parados no
   original (33 s de gravação da Entrada sem nenhuma mudança).
5. **Rua** (`6d01`): a porta do prédio, por volta de (338,355–385), leva ao
   saguão (resposta com atraso).
6. **Saguão** (parede `6a6b`): elevador (as portas e o botão ao lado levam
   para dentro dele, ver abaixo); quadro de andares com Truques de
   Desenho, Oficina de Projetos, Ateliê de Pintura e Saguão/Galeria (clique
   no nome, troca direta); botões "Tchau" e "Galeria"; o mastro de
   bombeiro sobe ao Ateliê; o skate "LÁ FORA" volta para a rua; a Bolsa
   Mágica e o buraco do rato (abaixo). McZee oferece: começar a desenhar
   por conta própria; fazer história em quadrinhos, botão, adesivo ou
   desenho animado; aprender truques de desenho; copiar, apagar ou limpar
   desenhos.
7. **Ateliê de Pintura** (clique em "Ateliê de Pintura", (150,86) no
   saguão): Maggie pergunta "Pintar alguma coisa nova", "Trabalhar na minha
   última obra", "Trabalhar em um quadro mais antigo", "Outra coisa?". Com
   "Pintar alguma coisa nova", o cavalete mostra "Quadro Novo"; um duplo
   clique no quadro abre a tela de pintura.
8. **Tela de pintura**: descrita abaixo.

## Tela de pintura

Remontada no Ateliê Livre (subprojeto 2b). A geometria completa está na spec
[`2026-10-31-pintura-design.md`](../superpowers/specs/2026-10-31-pintura-design.md),
seção 4. Resumo:

- **Ferramentas** (barra superior, da esquerda para a direita): Pincel, Lata
  de tinta, Seleciona-adesivos, Seleciona-sons, Ferramenta Palavra, Ovo
  desfazer, Aspirador, Transformadores, Super-ferramentas de desenho (3D),
  Imprimir; à direita, o McZee e a porta "Saída". Cada ícone tem uma versão
  ativa; as máscaras são o id seguinte ao da versão monocromática (ex.:
  `1013` cor, `1014` mono, `1015` máscara).
- **Faixa de opções** do pincel: 12 botões e 5 páginas (setas e número à
  direita). Página 1: redondos e quadrados; 2: carimbos (peixe, maçã,
  globo…); 3: efeitos (bolhas, zigue-zague, explosão, rachaduras, teia,
  espiral, trilho, estrada, onda, filme, água, grama); 4: caligrafia; 5:
  formas (reta, eixos, curvas, retângulo, círculo, triângulo, polígono).
- **Adesivos**: rolos na faixa (7 por página); um rolo abre a tira de filme
  vertical sobre a borda esquerda do quadro (x 33..74, y 58..466): no topo o
  rolo, depois 9 casas de 28×32 em `(40, 103 + 39·k)` e as setas `02db` e
  `02df` em y 452. As miniaturas do rolo de bichos são `7743` e `77ec`…`77fa`;
  os quadros dos bichos ficam de `753b` a `773b` (grupos de 8 ids: cor, traço
  em `N+5`, máscara em `N+6`). O adesivo colocado se anima com deslocamento
  (o pássaro sai andando e volta); os roteiros ainda não foram achados — veja
  a spec [2c](../superpowers/specs/2026-10-31-adesivos-design.md).
- **Rolos de clip-art**: os 28 `CARO` de `MAXMAG10.ART`, `MAX100.ART` e
  `MAG100.ART`, na ordem dos números (spec
  [2d](../superpowers/specs/2026-10-31-adesivos-2-design.md)); tira amarela
  pontilhada, 8 casas de 34×34 a cada 41 px. O `CARO` 28 de `C-MAG100.ART`
  é o rolo de cenários (Caverna, Fazenda, Selva…), o segundo da página 5,
  ao lado do rolo de reciclagem. As pontas do rolo fazem parte do primeiro e
  do último rolo da página; a esquerda fica em x≈12 e a direita logo depois
  do último rolo (x≈390 com a página cheia). O Cachorrinho `025d` usa a
  máscara `0263`. Som de abrir um rolo:
  `0233`.
- **Adesivos colocados** (ferramenta Seleciona-adesivos com a mão vazia): o
  contorno do adesivo sob o ponteiro pisca e arrastá-lo o move (uma ação do
  ovo); um clique parado num bicho o anima de novo (balão
  `1005[9]`: "Para animar um adesivo outra vez agora mesmo: Clique nele"),
  terminando no mesmo lugar e com o som que tinha; num adesivo estático o
  leva ao Cavalete de adesivos para modificar (balão `1005[4]`). Balões do `WHAT 1005`:
  `2` ao abrir a tira ("Clique no adesivo…", ao lado da tira), `3` ao pegar,
  `9` depois de colocar um bicho ("Você viu isso?"), `4` depois de colocar
  um adesivo estático.
- **Cavalete de adesivos** ("Fazer o seu", ícone `123f` em (456, 59);
  `scenes/paint/sticker_easel.rs`): mini-quadro de 331×166 em (154, 177)
  com moldura laranja de 5 px (149, 172)–(489, 392); barra própria em
  y 348–387 com pincel `146f`, lata `1427`, ovo `143b`, aspirador `1443`
  (centros x 182, 230, 282, 330), lupa `1403` (x 367–407; apertada
  `1407`, com as setas `151f`/`14f3`/`151b`/`1523` em volta, que rolam o
  cavalete ampliado 2× em passos de 40 px), NÃO `0271` e OK `026d`.
  A faixa de cima vira a do pincel (páginas 1–6), as cores e os padrões
  continuam ativos e a barra de ferramentas principal fica cinza. Balão
  `1005[6]` ao abrir, em (229, 149); `1005[14]` na lupa, em (216, 224).
  NÃO fecha e reabre o balão Seleciona-adesivos. OK corta o desenho ao que
  foi pintado (o branco ligado à borda fica transparente, o de dentro não),
  grava `adesivo-AAAAMMDD-HHMMSS.png` na pasta de desenhos e abre a tira do
  rolo do usuário (último rolo da faixa, que só aparece depois do primeiro
  adesivo feito; miniaturas de até 30 px; ícone = último adesivo feito) na casa nova. Ao modificar um adesivo colocado, OK
  troca a imagem do objeto (mesmo centro; a origem passa a `png`).
- **Cachorrinho** (ícone `025d` em (409, 63); `scenes/paint/find_pictures.rs`):
  diálogo modal verde (120, 111)–(526, 368) com o resto da tela em cinza.
  "Esta Pasta" (setas trocam entre Imaginópolis e fora dela), "Conteúdo"
  (lista de 8 linhas de 17 px em (143, 178), setas `03b3`/`03b7`), botões
  cachorro `025d` (lê a pasta de novo), X `03bb` (cancela, reabre o balão
  Seleciona-adesivos) e JÁ `03bf` (usa a figura); à direita a prévia (1/4) e
  os campos Título e Artista com setas. Em Imaginópolis a lista são os
  desenhos da pasta de desenhos (`gallery::list_drawings`; Artista = o
  apelido do usuário); "Fora de Imaginópolis" pergunta "Isso pode levar
  algum tempo…" e, com "Sim", lista os desenhos de exemplo
  (`SHARED/USERS/*/*.MAG`, com título e autor do `.MAG`). JÁ transforma o
  desenho em adesivo do usuário (`adesivo-<título>.png`, cortado ao que foi
  pintado) e o põe na mão. Balão `1005[12]` ao abrir.
- **Gravar animações do original**: `tools/wine/rec.sh ROLO CASA PASTA` coloca o
  adesivo e grava 10 s com o ffmpeg; `tools/wine/analyze.py` acha as imagens em
  cada quadro. Rolar a tira de filme no Wine derruba o programa.
- **Lata de tinta**: faixa com 10 opções (`1590`, `1591`, `1593` … `15a1`) e,
  à esquerda, uma amostra da tinta atual.
- **Aspirador**: 8 opções (`1557`, `1559`, `1567`, `1561`, `155f`, `155b`,
  `155c`, `1569`); a primeira limpa o quadro.
- **Cores**: 38 caixas na coluna esquerda (índices da `LOGPALETTE` 8).
- **Padrões**: 38 caixas na coluna direita, padrões 0–37 de `PAT#` 0x80,
  desenhados na cor atual; bit 0 = cor, bit 1 = branco.
- **Objetos**: no original, adesivos e palavras colocados não viram pixels:
  ficam por cima da pintura e podem ser movidos, esticados, girados, grudados
  e aspirados; o contorno pisca sob o ponteiro. No Ateliê Livre são a
  `Objects` de `paint/objects.rs`; o PNG gravado é a composição achatada e
  `ultimo.objetos.json` + `ultimo.fundo.png` (e as cópias com data) guardam
  os objetos e o fundo para reabrir editável (spec
  [objetos](../superpowers/specs/2026-10-31-objetos-design.md)).
- **Transformadores** (`1045`; ajuda `WHAT 1006`): faixa de duas páginas
  com tampa verde `14db` em (4, 57), botões de 32×32 em x = 25 + 37·k,
  y = 63 e a ponta `14ef`/`14f1` logo após o último botão. Página 1:
  Mover-modificar `1470`/`1471`, Dimensionar por arraste `149d`/`14a0`,
  lateral `14a1`/`14a4`, vertical `14a5`/`14a8`, Aumentar `14b1`/`14b4`,
  Diminuir `14b5`/`14b8`, Empurrar para trás `14b9`/`14bc`, Trazer para a
  frente `14bd`/`14c0`. Página 2: Mover, Girar à esquerda `14c1`/`14c4`, à
  direita `14c5`/`14c8`, Grudar ou desgrudar `14c9`/`14cc`, Inverter lateral
  `14cd`/`14d0`, em cima/embaixo `14d1`/`14d4`, Coloca-adesivos
  `14d9`/`14da` (cor/mono). Aumentar e Diminuir agem enquanto o botão fica
  apertado; Empurrar/Trazer repetem ao segurar; Grudar liga dois objetos
  com uma corrente (clique na corrente desgruda). Coloca-adesivos (o
  Cavalete de adesivos pelos Transformadores) e Ctrl/Shift ao arrastar
  ainda não existem.
- **Super-ferramentas de desenho** (`1007`; ajuda `WHAT 100a`): faixa única
  em dois grupos. Grupo 1 (k = 0..2): Sem quadriculado `1485`/`1488`,
  Quadriculado pequeno `148d`/`1490`, Quadriculado grande `1489`/`148c`;
  ponta `14ef` em x 134. Grupo 2 (tampa em x 189; k = 5..10): Ponto de fuga
  `1477`/`1478`, Linha do ponto de fuga `1479`/`147c`, Aproximar do PF
  `1481`/`1484`, Afastar do PF `14d5`/`14d6`, Eixo de construção
  `147d`/`1480`, Apagar linha guia (ímã) `14d7`/`14d8`. Interruptor mágico
  `1491` (ligado) / `1497` (desligado), 22×41 em (464, 58). As linhas guia
  são só da tela (quadriculados de 24 e 48 px tracejados, horizonte e PF
  azuis, linhas do PF e eixos verdes), guardadas em `ultimo.guias.json`;
  Aproximar/Afastar deslizam o objeto sob o botão apertado a 60 px/s,
  mudando o tamanho na proporção da distância ao PF.
- **Balões do McZee**: ao escolher uma ferramenta ou uma opção da faixa, e ao
  clicar no McZee (nicho x 533..580, y 56..104), um balão amarelo explica o
  uso — os registros `WHAT` do `ARTIST.EXE` (`1002` pincel, `1003` lata,
  `1004` sons, `1005` adesivos, `1006` Transformadores, `1007` ovo, `1008`
  aspirador, `1009` palavra, `100a` Super-ferramentas, `100b` imprimir). O
  balão fica em (445, 115) com a largura do registro; negrito, cor roxa dos
  títulos, recuo, o ícone da ferramenta (quando o id `1xxx` existe no
  `.PIC`) e as ligações (dica `21b`, tecla `21c`, Maggie `21e`, palavra do
  glossário) são desenhados por `help.rs`; clicar numa ligação abre o
  registro ligado. Na primeira vez que a tela abre aparece o do Pincel. Os
  ícones das ligações e vários ícones das ferramentas citados nos balões
  (`100e`, `1012`, `1036`…) não estão no `.PIC`: as ligações usam ícones
  desenhados à mão e as figuras ausentes são puladas.

## Oficina de Projetos

Os projetos da Maggie são modos da tela de pintura (spec
[`2026-10-31-projetos-design.md`](../superpowers/specs/2026-10-31-projetos-design.md);
geometria reconstruída, não medida — o Wine não chega a mostrar essas telas).
O rótulo da barra inferior vira "Fazendo uma História em Quadrinhos" /
"Fazendo um Botão" / "Fazendo um Adesivo" e a porta "Saída" volta ao andar
dos Projetos. O nicho do McZee (x 533..580 da faixa) mostra o estado do
projeto e, clicado, abre e fecha o painel do projeto na faixa de opções;
qualquer outro clique fecha o painel e devolve a faixa às opções da ferramenta.

- **História em quadrinhos**: ao abrir, a faixa mostra o painel do nicho:
  os dígitos 1 a 4 (o roteiro de ajuda `WHAT` 3e00 diz "até quatro
  painéis"; `3a78`/`3a79` (1), `3a7b`/`3a7c`/`3a7d` (2), `3a7f`… (3) e
  `3a83`… (4), o escolhido em cor), o **fundo pronto** com as setas (rolo
  `CARO 28 "Comic Backgrounds"` de `SHARED/STUFF/C-MAG100.ART`: onze
  desenhos de 271×219 em preto e branco — Caverna, Fazenda, Selva, Pirâmides,
  Telhados, Sala, Navio, Castelo, Lua, Fundo do mar, Parque; clicar na
  miniatura escolhe e o próximo clique num quadrinho pinta o interior, branco
  com o desenho centrado, uma ação do ovo) e os cartões **Título** e
  **Autor** (balões `WHAT` 3e00 [158]/[160] com caixa de texto; o texto vai
  para a barra inferior — "Título — por Autor" — e para a página impressa,
  numa faixa em cima e outra embaixo). Leiautes no quadro de 594×347 com
  margem de 6 px e calha de 8 px: 1 inteiro, 2 lado a lado, 3 colunas,
  4 = 2×2; molduras pretas de 3 px desenhadas por cima do quadro (não entram
  no PNG). Pincel, lata, palavras e adesivos ficam recortados ao quadrinho
  onde o clique começou; na calha nada pinta. Trocar o número não apaga
  nada. Imprimir grava a página com as molduras. O original mostra um painel
  por vez, com botões de navegação e a cópia do painel N para N+1 (3e00
  [60]/[66]/[163]); o port mostra todos de uma vez e não tem cópia.
- **Cavalete de história em quadrinhos** (ferramenta Palavra nos
  quadrinhos; 3e00 [14]–[20]): a faixa da Palavra vira o cavalete, com a
  fonte (Pablo 16 ou 8 pt), seis formas de balão — retângulo com rabicho (o
  "Oi!" da pintura), retângulo, oval com rabicho, oval, pensamento e o
  "espaço em branco" (sem balão) — e a posição das palavras (esquerda,
  centro, direita). O texto é digitado como na pintura e o estilo muda ao
  vivo; com largura definida, o texto quebra nas palavras. Arrastar um balão
  colocado com a Palavra **move**; com **Shift** apertado, muda a largura (e
  o texto quebra de novo); um clique sem mover reabre o texto para mudar.
  Fora dos quadrinhos a Palavra continua com as duas opções do original
  (texto solto e balão "Oi!"). Os balões ovais são geométricos; as silhuetas
  de `B-MAG100.CHK` servem de referência.
- **Dipitidus** (3e00 [22]): nos quadrinhos, a ferramenta de adesivos ganha,
  depois dos rolos de clip-art (e antes do rolo do usuário), o rolo
  "Dipitidus" com os quatro rabinhos de
  balão de `SHARED/STUFF/B-MAG100.CHK` (`PICT` 16–19, ampliados 2×, contorno
  preto e interior branco; o ícone é o `PICT` 20). Mudam de lugar e de
  tamanho pelos Transformadores. O bitmap `10b0` que o roteiro cita não
  existe no `ARTIST.PIC`.
- **Botão ou adesivo**: a faixa mostra os cartões da forma, com a miniatura
  e o rótulo. Botão (roteiro 3c00 [4], "Redondo ou Quadrado"): círculo de
  300 px ou 300×300 com cantos de raio 30, borda preta de 3 px e anel cinza
  de 12 px (a aba de metal). Adesivo (3c00 [26]): **Retangular** (360×260,
  raio 24), **Redondo** (círculo de 300), **Quadrado** (300×300, raio 30),
  **Oval alto** (elipse de 260×318) e **Oval largo** (318×260), só com a
  borda de 3 px — os bitmaps `109d`–`10a1` do roteiro não existem no
  `ARTIST.PIC`, por isso as formas são geométricas. O último cartão troca
  entre botão e adesivo. Tudo overlay; a pintura fica limitada ao interior
  e, ao trocar a forma, o que ficaria fora vira branco (o ovo desfaz). A
  forma fica em `ultimo-<tipo>.projeto.txt` e volta ao reabrir. Imprimir
  grava uma folha de 827×1169 (210×297 mm a 100 ppp) com 2×3 botões ou 3×4
  adesivos, cada cópia reduzida para caber na célula. As amostras do
  original ("Com uma amostra?", `SAMP3/12/13/14/15.MAG`, `WIZA = STIK`) têm
  o conteúdo em chunks `SOBJ`/`SPLE` ainda não interpretados e não são
  oferecidas.
- **Fim guiado** (3c00 [12]–[31], 3e00 [151]–[162]; código em
  `projects/dialogs.rs`): nos três projetos, Imprimir pergunta "Você quer
  imprimir o botão?" e, depois de imprimir (o McZee imprime quantas cópias
  cabem na página — não há escolha do número), abre "D+. Você terminou! O
  que você quer fazer agora?": trabalhar por conta própria (fecha), começar
  um novo (salvar? → "Digite um nome descritivo" → `<tipo>-<nome>.png`, nome
  só com letras, dígitos e hífens; ou "Não, jogar fora" → "imprimir antes
  que vá para o lixo?") ou voltar ao andar dos Projetos. A porta de Saída
  com mudanças faz a mesma pergunta de salvar; sem mudanças sai direto.
  Jogar fora não grava nada e esquece o último trabalho desse tipo. Clicar
  fora do balão cancela. Sem os dados do original os balões usam textos
  reserva iguais aos do `WHAT`.
- **Arquivos**: ao sair, `ultimo-quadrinhos.png` / `ultimo-botao.png` /
  `ultimo-adesivo.png` (reaberto na próxima vez) e, se mudou,
  `quadrinhos-AAAAMMDD-HHMMSS.png` etc., sempre com `.projeto.txt` ao lado
  (ver [`formats/arquivos-do-port.md`](../formats/arquivos-do-port.md)); a
  impressão vai para `impressao-AAAAMMDD-HHMMSS.png` como na pintura.

### Desenho animado

Apresentação de figuras já feitas, como descrevem os roteiros de ajuda do
original (`WHAT` `3a00`); a geometria não foi medida no Wine e é uma
remontagem "usável primeiro". Código em
`crates/engine/src/scenes/projects/flipbook.rs`.

- **Faixa de opções** (y 56..104): TOCAR / PAUSA / PARAR (`3a61`, `3a59`,
  `3a5d`, 70×12, em x 10, y 58/73/88; o botão ativo usa a versão `+3` com
  um contorno vermelho); régua TEMPO (`02f7`, 207×58, desenhada em x 86 com
  os 10 px de parafusos de cima cortados), cursor `3a65` sobre o trilho
  amarelo com 5 paradas a cada 38 px entre a tartaruga (`3a69`) e o coelho
  (`3a6f`): 4000 / 2000 / 1000 / 500 / 250 ms por figura, padrão 1000
  (clique ou arrasto no trilho; a tartaruga e o coelho vão às pontas);
  botões desenhados (cartões brancos, Pablo 8 pt): **Pasta** (300,59),
  **<** / **>** (364 e 388), **Efeito** (414,59, mostra o efeito do espaço
  escolhido e cicla ao clicar), **Som** (300,82), **Legenda** (344,82),
  **Título** (404,82, o desenho de abertura) e **Mais espaços** (452,82, a
  seta de acrescentar espaços do original);
  "N figura(s), M espaços" no canto inferior direito do quadro.
- **Barra de ferramentas**: as ferramentas de pintura ficam escurecidas
  (pontilhado) e não reagem; só **Imprimir** funciona: grava a folha com
  as figuras numeradas e as legendas (`impressao-….png`, e `lp` se houver).
  O **nicho do McZee** abre a apresentação da Maggie (`WHAT 3a00[2]`, com
  as ligações do roteiro) no lugar do painel do projeto.
- **Quadro parado**: tira de espaços numerados numa grade 4×3 (células de
  148×115), de 4 espaços ao começar até 10 (**Mais espaços** acrescenta um,
  só com todos preenchidos; senão o Rex avisa "Primeiro preencha os
  espaços!"); cada espaço com a miniatura a 1/5 (118×69), o dígito colorido
  do original (`3a78`…`3a97` com a máscara; o 10 em texto) no canto, a
  legenda embaixo e uma bolinha amarela quando tem som. Clique num espaço
  cheio escolhe (contorno vermelho); escolhido, clicar noutro troca as duas
  (ou move, se vazio); clicar de novo desescolhe; **arrastar** uma figura
  (a miniatura segue o mouse) a outro espaço troca ou move, e à **Pasta**
  fechada tira a figura; **clique duplo** numa figura abre a tela de
  pintura com ela (cena `FlipbookFigure` por cima; a porta de saída grava e
  volta, e a figura modificada entra no lugar, com efeito, som e legenda);
  **Delete** (ou Backspace) esvazia; ← / → mostram uma figura de cada vez,
  em pausa. Avisos aparecem numa faixa amarela no alto do quadro.
- **Pasta**: lista dentro do quadro, 4×4 miniaturas por página (lidas só
  quando a página aparece), com os desenhos da pasta (`.png` e `.mag`, como
  a galeria) seguidos das amostras de `SHARED/USERS/*`; clique numa figura =
  vai para o primeiro espaço vazio; segurar e arrastar leva a figura ao
  espaço escolhido (a pasta fecha enquanto arrasta); **Fechar**, **<**,
  **>** na base.
- **Som**: duas listas dentro do quadro, categorias de `MAG100.SND` à
  esquerda e os sons ("Sem som" primeiro) à direita; o escolhido toca na
  hora e ao começar a figura.
- **Legenda**: digitada no próprio espaço (Enter confirma, Esc cancela);
  ao tocar, aparece numa faixa branca de 16 px na base da figura.
- **Título** (desenho de abertura): o texto digitado (faixa amarela no alto
  do quadro; Enter confirma, Esc cancela) vira uma figura branca com as
  letras grandes (Pablo 16 pt ampliada até 4×) no **espaço 1**; a figura
  que estava lá vai para o 2, e assim por diante, com efeito, som e
  legenda (a tira ganha um espaço se estava cheia). Com a abertura já no
  espaço 1 (`arquivo` `abertura`), Título só troca o texto. O clique duplo
  leva a abertura à pintura, para enfeitar.
- **Efeitos**: corte, dissolver (pixels espalhados em 12 passos), cortina
  para a esquerda / direita / cima / baixo e dado (sorteia a cada TOCAR).
  A transição dura 0,5 s (no máximo metade do intervalo).
- **Tocar**: as figuras em laço no quadro, cada uma pelo intervalo da régua;
  PAUSA congela, PARAR (ou um clique no quadro) volta aos espaços.
- **Guardar**: ao sair, `animacao-AAAAMMDD-HHMMSS/` na pasta de desenhos e a
  cópia `ultimo-animacao/`, reaberta ao entrar de novo — ver
  [`docs/formats/animacao.md`](../formats/animacao.md).
- Fora, por enquanto: girar as palavras do título, figura "Fim" pronta (dá
  para fazer qualquer figura com a pintura e o clique duplo), Varinha
  Mágica na pausa, nome do desenho animado, dado para o som.

## Objetos das salas e elevador

Medido no original no Wine (gravação a 10 quadros/s; sons pelo rastro do
winmm, ver `sons-das-acoes.md`). Implementado em
`crates/engine/src/scenes/room_objects.rs`, `elevator.rs`, `room.rs` e
`gallery_room.rs`.

**Bolsa Mágica** (fechada `6833`, aberta `6873`). Chega fechada nos
andares e aberta no Ateliê e na Galeria. Um clique a abre (som `2cf`) e
põe no chão a **maleta vermelha** (`685b`), o **livro verde** (`6863`) e a
**Varinha Mágica** (`686b`); outro clique a fecha (`2ce`). Posições (bolsa
fechada / aberta / maleta / livro / varinha):

| sala | fechada | aberta | maleta | livro | varinha |
|---|---|---|---|---|---|
| Saguão | (97,388) | (97,410) | (263,394) | (356,367) | (222,421) |
| Ateliê | (129,392) | (129,414) | (305,330) | (265,383) | (352,410) |
| Oficina | (146,392) | (146,414) | (330,328) | (247,379) | (374,407) |
| Truques | (149,357) | (149,379) | (387,306) | (285,282) | (284,370) |
| Galeria | (95,378)* | (95,400) | (304,391) | (235,360) | (246,422) |

\* estimada (a bolsa da Galeria só foi vista aberta).

- **Maleta**: abre a pasta de trabalhos por cima da sala (som `2f1`). No
  original é o diálogo "Esta Pasta / Conteúdo / Título / Artista" com o
  botão "Já"; *no port* é o painel de miniaturas da Galeria
  (`SceneId::Portfolio`, aberto com `Effect::Push`): clique duplo abre o
  quadro na pintura, "Fechar" ou um clique fora volta à sala.
- **Livro verde**: a História do McZee por cima da sala (sons `2e2`/`265`
  e `2f6`); Para volta à sala (`SceneId::BookStory`).
- **Varinha Mágica**: some do chão, a bolsa se fecha, o cursor vira a
  varinha (`CURSOR` `b7`) e o McZee mostra `WHAT 100d[59]` ("Quando você
  clica no mastro de bombeiro ou no buraco do rato com a Varinha Mágica,
  coisas incríveis acontecem!"; na Galeria, `[57]`), som `307`. O balão
  não segura o clique: o mesmo clique já usa a varinha. Com ela:
  - no **mastro** (ou na abertura dele no chão): um bicho desce ou sobe
    pelo mastro, sem trocar de andar (som `2f3`);
  - no **buraco do rato**: sai uma surpresa (som `20a` ou `2ea`, mesmo
    tamanho no rastro; o port usa `2ea`);
  - na **bolsa**: a varinha volta e a bolsa se abre;
  - em outro lugar: só fecha o balão (a varinha continua na mão).
  Depois da surpresa a bolsa reaparece aberta, com a varinha dentro.
  Durante a surpresa os cliques esperam.
- **Mastro de bombeiro**: sobe um andar (som `2f3`; nos Truques o original
  sobe a um sótão escuro de onde se desce ao saguão, o port vai direto ao
  saguão). **Abertura do mastro** no chão (`682b`): desce um andar (`2f2`).
- **Skate** "LÁ FORA" (`6c33`, saguão): volta para a rua (som `2fc`).
  Skate "ELEVADOR" (`6843`, andares): entra no elevador (`2fc`, `311`).

As surpresas são roteiros `ATAG` gravados em relação a um ponto. O bloco
de cada sala saiu de casar as figuras quadro a quadro nas gravações:

| sala | mastro (ponto) | buraco do rato (ponto) |
|---|---|---|
| Saguão | 0x38b–0x394 (596,0) | 0x3e9–0x3ef (513,346) |
| Ateliê | 0x70d–0x716 (575,−3) | 0x76c–0x773 (466,328) |
| Oficina | 0x4b0–0x4b8 (573,−22) | 0x515–0x519 (479,289) |
| Truques | 0xcd–0xd4 (539,−32) | 0x12d–0x132 (576,273) |
| Galeria | — | 0x258–0x25e (445,348), por eliminação |

**Elevador** (parede `6dd3`). Portas `6e2b` (máscara `6e31`) em (123,47),
escotilha `6e33` em (189,111), mostrador `6e01` em (259,4) com o número
aceso do andar em (270/300/330/361, 9) (`6e0b`, `6e3b`, `6e5b`, `6e6b`),
painel `6e13` em (530,81) com o botão aceso (`6e23` (574,240), `6e53`
(585,203), `6e63` (572,167), `6e73` (567,127)), "Tchau" `681b` em (46,6) e
o McZee com a fantasia da sala em (−30,180), que pergunta
`WHAT 100d[64]` "Subindo? Ou descendo?". Um clique num andar do painel
toca o motor (`2d8`) e a escotilha mostra o roteiro 0x190 (subindo) ou
0x191 (descendo), 1,4 s por andar, gravado em relação às portas; o
mostrador muda a cada andar. Depois as portas se abrem (`2d4`), o vão
mostra a parede do andar por 1 s e a sala aparece (`311`) com o balão de
chegada. Clicar no andar em que se está só abre as portas.

## Conferência

Comparando o quadro gerado pelo Ateliê Livre com a captura do original no
mesmo estado (pincel, página 2, peixe, cor vermelha), 99,8% dos pixels fora
do quadro são idênticos (a diferença é a fonte da barra inferior), e um
traço com o carimbo de peixe sobre o mesmo caminho do mouse dá 99,4% da tela
inteira idêntica. A rua (`6d01`) sai idêntica pixel a pixel.
