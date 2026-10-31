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

1. **Entrada** (fundo `4139`): "Sou eu", "Primeira vez", "Procurar".
2. **Cadastro** ("Primeira vez"): nome e sobrenome (botões Volta / Para /
   Segue, Segue em (302,222)), bicho favorito, escolha do disco ("Coloque
   aqui"), apelido (Segue em (320,174)); "Não precisa, vamos direto para
   Imaginópolis" em (82,151).
3. **Imaginópolis** (`6cfd`, 641×480): a porta em (302,172) leva à rua.
4. **Rua** (`6d01`): a porta do prédio, por volta de (338,355–385), leva ao
   saguão (resposta com atraso).
5. **Saguão** (parede `6a6b`): elevador; quadro de andares com Truques de
   Desenho, Oficina de Projetos, Ateliê de Pintura e Saguão/Galeria (clique
   no nome); botões "Tchau" e "Galeria". McZee oferece: começar a desenhar
   por conta própria; fazer história em quadrinhos, botão, adesivo ou
   desenho animado; aprender truques de desenho; copiar, apagar ou limpar
   desenhos.
6. **Ateliê de Pintura** (clique em "Ateliê de Pintura", (150,86) no
   saguão): Maggie pergunta "Pintar alguma coisa nova", "Trabalhar na minha
   última obra", "Trabalhar em um quadro mais antigo", "Outra coisa?". Com
   "Pintar alguma coisa nova", o cavalete mostra "Quadro Novo"; um duplo
   clique no quadro abre a tela de pintura.
7. **Tela de pintura**: descrita abaixo.

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
- **Balões do McZee**: ao escolher uma ferramenta, um balão amarelo explica o
  uso (ex.: "Pincel — Para pintar o quadro: segure o botão do mouse e
  arraste"). Ainda não reproduzidos.

## Oficina de Projetos

Os projetos da Maggie são modos da tela de pintura (mesmo quadro de 594×347
em (23,109), mesma barra de ferramentas), com um rótulo próprio na barra
inferior e um painel extra na faixa de opções. Desenho em
[`docs/superpowers/specs/2026-10-31-projetos-design.md`](../superpowers/specs/2026-10-31-projetos-design.md).

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
