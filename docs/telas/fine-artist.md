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
   **Procurar** (`47b`, (280,288)). "Sou eu" com um nome escolhido abre
   `4400[2]` (Imaginópolis / mudar o cadastro / história do McZee / Ateliê
   de Pintura); sem nome, o aviso `4402[3]`.
   *No port:* a lista vem de `usuarios/` (ver README); "Procurar" só
   registra um aviso (não há discos).
2. **Cadastro** ("Primeira vez"): balões sucessivos em (112,78) com os
   botões Volta (`40f`) / Para (`417`) / Segue (`413`), 31×14, centrados no
   rodapé do balão: `4400[4]` nome e sobrenome (caixas de 144 px),
   `4400[8]` bicho preferido (12 figuras `45b`, `45d`… em 4×3, bolinha
   `409`/`40d`), `4400[5]` apelido. Avisos do `WHAT 4402`: `[1]` nome
   vazio, `[2]` apelido inválido (mais de 8 caracteres, espaço ou
   pontuação), `[0]` apelido repetido. Depois `4400[3]`: conte mais
   (história) / direto para Imaginópolis / mudar o cadastro.
   *No port:* a escolha do disco (`4400[6]`) não existe; o apelido é
   sugerido a partir do nome.
3. **História do McZee** (álbum `4201`, 495×314, em (72,160) sobre o fundo
   da Entrada): uma página por balão do `WHAT 4200` (11), foto de 169×119 na
   moldura da esquerda (`420b`, `4210`, `4211`, `4218`, `423b`, `4273`,
   `4293`, `4299`, `42b3`, `42b9`, `42f3`) e legenda ou foto na da direita
   (`424b`, `4219`, `4223`, `4241`, `426b`, `428b`, `42a1`, `4281`, `42cb`,
   `42e1`). Segue, Enter ou um clique no balão passam a página; Para sai.
4. **Imaginópolis** (`6cfd`, 641×480, alinhado à esquerda): a porta "Entre"
   do Museu Mágico, por volta de (302,172), leva à rua; o McZee no
   carrinho da montanha-russa (roteiro `ATAG` 0x2bc) atravessa de tempos em
   tempos.
5. **Rua** (`6d01`): a porta do prédio, por volta de (338,355–385), leva ao
   saguão (resposta com atraso).
6. **Saguão** (parede `6a6b`): elevador; quadro de andares com Truques de
   Desenho, Oficina de Projetos, Ateliê de Pintura e Saguão/Galeria (clique
   no nome); botões "Tchau" e "Galeria". McZee oferece: começar a desenhar
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
  pontilhada, 8 casas de 34×34 a cada 41 px.
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
  com uma corrente (clique na corrente desgruda). Coloca-adesivos (cavalete
  de adesivos) e Ctrl/Shift ao arrastar ainda não existem.
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

- **História em quadrinhos**: ao abrir, a faixa pergunta quantos quadrinhos,
  1 a 4 (o roteiro de ajuda `WHAT` 3e00 diz "até quatro painéis"), com os
  dígitos `3a78`/`3a79` (1), `3a7b`/`3a7c`/`3a7d` (2), `3a7f`… (3) e
  `3a83`… (4), o escolhido em cor. Leiautes no quadro de 594×347 com margem
  de 6 px e calha de 8 px: 1 inteiro, 2 lado a lado, 3 colunas, 4 = 2×2;
  molduras pretas de 3 px desenhadas por cima do quadro (não entram no PNG).
  Pincel, lata, palavras e adesivos ficam recortados ao quadrinho onde o
  clique começou; na calha nada pinta. Trocar o número não apaga nada.
  Imprimir grava a página com as molduras.
- **Botão ou adesivo**: a faixa mostra três cartões — "Botão redondo"
  (círculo de 300 px), "Botão quadrado" (300×300, cantos de raio 30) e
  "Adesivo" (360×260, raio 24) —, como no roteiro 3c00 ("Redondo ou
  Quadrado"). Borda preta de 3 px e, nos botões, anel cinza de 12 px (a aba
  de metal), tudo overlay; a pintura fica limitada ao interior e, ao trocar
  a forma, o que ficaria fora vira branco (o ovo desfaz). Imprimir grava uma
  folha de 827×1169 (210×297 mm a 100 ppp) com 2×3 botões ou 3×4 adesivos,
  cada cópia reduzida para caber na célula.
- **Arquivos**: ao sair, `ultimo-quadrinhos.png` / `ultimo-botao.png` /
  `ultimo-adesivo.png` (reaberto na próxima vez) e, se mudou,
  `quadrinhos-AAAAMMDD-HHMMSS.png` etc.; a impressão vai para
  `impressao-AAAAMMDD-HHMMSS.png` como na pintura.

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
  (`3a6f`): 4000 / 2000 / 1000 / 500 / 250 ms por figura, padrão 1000;
  botões desenhados (cartões brancos, Pablo 8 pt): **Pasta** (300,59),
  **<** / **>** (364 e 388), **Efeito** (414,59, mostra o efeito do espaço
  escolhido e cicla ao clicar), **Som** (300,82) e **Legenda** (364,82);
  "N figura(s)" em x 440.
- **Quadro parado**: grade 3×3 de espaços numerados (células de 198×115),
  cada um com a miniatura a 1/4 (148×86), o dígito colorido do original
  (`3a78`…`3a97` com a máscara) no canto, a legenda embaixo e uma bolinha
  amarela quando tem som. Clique num espaço cheio escolhe (contorno
  vermelho); escolhido, clicar noutro troca as duas (ou move, se vazio);
  clicar de novo desescolhe; **Delete** (ou Backspace) esvazia; ← / →
  mostram uma figura de cada vez, em pausa. Avisos ("Primeiro preencha os
  espaços!") aparecem numa faixa amarela no alto do quadro.
- **Pasta**: lista dentro do quadro, 4×3 miniaturas por página, com os
  desenhos da pasta (`.png` e `.mag`, como a galeria) seguidos das amostras
  de `SHARED/USERS/*`; clique numa figura = vai para o primeiro espaço
  vazio; **Fechar**, **<**, **>** na base.
- **Som**: duas listas dentro do quadro, categorias de `MAG100.SND` à
  esquerda e os sons ("Sem som" primeiro) à direita; o escolhido toca na
  hora e ao começar a figura.
- **Legenda**: digitada no próprio espaço (Enter confirma, Esc cancela);
  ao tocar, aparece numa faixa branca de 16 px na base da figura.
- **Efeitos**: corte, dissolver (pixels espalhados em 12 passos), cortina
  para a esquerda / direita / cima / baixo e dado (sorteia a cada TOCAR).
  A transição dura 0,5 s (no máximo metade do intervalo).
- **Tocar**: as figuras em laço no quadro, cada uma pelo intervalo da régua;
  PAUSA congela, PARAR (ou um clique no quadro) volta aos espaços.
- **Guardar**: ao sair, `animacao-AAAAMMDD-HHMMSS/` na pasta de desenhos e a
  cópia `ultimo-animacao/`, reaberta ao entrar de novo — ver
  [`docs/formats/animacao.md`](../formats/animacao.md).
- Fora, por enquanto: arrastar figuras, clique duplo para editar a figura na
  pintura, desenho de abertura com título girado, figura "Fim", Varinha
  Mágica na pausa, nome do desenho animado e seta de acrescentar espaços.

## Conferência

Comparando o quadro gerado pelo Ateliê Livre com a captura do original no
mesmo estado (pincel, página 2, peixe, cor vermelha), 99,8% dos pixels fora
do quadro são idênticos (a diferença é a fonte da barra inferior), e um
traço com o carimbo de peixe sobre o mesmo caminho do mouse dá 99,4% da tela
inteira idêntica. A rua (`6d01`) sai idêntica pixel a pixel.
