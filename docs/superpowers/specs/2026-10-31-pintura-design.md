# Ateliê Livre — Subprojeto 2b: tela de pintura do Fine Artist

Data: 2026-10-31
Status: decidido pelo agente (Pedro pediu execução autônoma até o fim do dia)
Depende de: 2a (plataforma), concluído

## 1. Contexto e objetivo

O 2a abre o programa e mostra as telas ampliadas. O 2b torna o Ateliê Livre
**usável por uma criança**: entrar na rua, clicar na porta e pintar. A tela
escolhida é a do quadro do Ateliê de Pintura (a que o original abre depois de
"Pintar alguma coisa nova" e do duplo clique no quadro), porque é o coração do
Fine Artist. Saguão, elevador, Ateliê com o McZee e as demais salas ficam para
um subprojeto seguinte (2c).

A referência é o original rodando no Wine (veja `docs/telas/fine-artist.md`):
posições, cores e formas foram medidas em capturas dele, que **não** entram
no repositório.

## 2. Decisões (rulings)

1. **Navegação mínima**: Abertura → Rua → (clique na porta do prédio) →
   Pintura. A porta "Saída" da pintura volta à Rua. O fluxo completo do
   original (Imaginópolis, saguão, Ateliê) fica para o 2c.
2. **Ferramentas do 2b**: Pincel (páginas 1, 2 e 4 das formas), Lata de tinta
   (preenchimento), Ovo desfazer, e Aspirador só com a opção 1 ("limpar o
   quadro"). As demais ferramentas (adesivos, sons, palavra, transformadores,
   super-ferramentas, imprimir) aparecem na barra, mas clicar nelas só
   registra um `Log` ("ferramenta ainda não disponível"); não mudam a
   ferramenta ativa. As páginas 3 (efeitos) e 5 (formas geométricas) do pincel
   aparecem com botões vazios e inertes.
3. **Cores**: a coluna esquerda tem as 38 cores do original, que são entradas
   da `LOGPALETTE` 8 (256 cores) de `ARTIST.EXE`, nesta ordem de índices:
   `0, 2, 18, 19, 20, 21, 22, 5, 6, 25, 26, 27, 28, 9, 10, 31, 32, 33, 7, 8,
   36, 37, 38, 11, 12, 41, 42, 43, 3, 4, 46, 47, 48, 14, 13, 15, 52, 1`.
   Cor inicial: preto (índice 1, a última).
4. **Padrões**: a coluna direita mostra os padrões 0 a 37 de `PAT#` 0x80 de
   `ARTIST.EXE`, desenhados na cor atual. **Regra de tinta** (medida no
   original): no pixel de tela `(x, y)`, o bit `(linha y % 8, coluna x % 8,
   bit mais alto à esquerda)` do padrão em **0 pinta a cor atual** e em **1
   pinta branco**. O padrão 0 (todo zero) é a cor sólida. Padrão inicial: 0.
5. **Quadro**: 594×347 pixels, ocupando `x 23..=616, y 109..=455` da tela
   lógica. Guarda índices da paleta de 256 cores (`u8`), começa branco
   (índice 0). Tudo que se pinta é recortado ao quadro.
6. **Formas do pincel** (todas medidas 1:1 nos botões do original):
   - Página 1: redondos de 16, 12, 8 e 4 pixels; "mais" 3×3; "xis" 3×3;
     ponto 1×1; quadrados de 2, 4, 8, 12 e 16 pixels. Inicial: redondo 16.
   - Página 4: traços de caligrafia: diagonais "\\" de 16, 12 e 8, diagonais
     "/" de 8, 12 e 16 (duas colunas de pixels por linha, como no original),
     horizontais de 8, 12 e 16 e verticais de 16, 12 e 8.
   - Página 2: carimbos de 16×16 de `ARTIST.PIC` `0x164b` (peixe) e
     `0x16a9, 0x16ab, …, 0x16bd` (de dois em dois). Pintam com a tinta onde o
     bitmap é preto e de branco no interior; o exterior (brancos ligados à
     borda por vizinhança de 4) é transparente.
7. **Traço**: ao apertar o botão esquerdo no quadro, carimba a forma
   centrada no ponteiro; ao arrastar, formas das páginas 1 e 4 são carimbadas
   em cada pixel da reta entre a posição anterior e a atual (Bresenham);
   carimbos da página 2 só quando o ponteiro se afasta 16 pixels do último
   carimbo. Soltar o botão termina o traço.
8. **Lata de tinta**: clique no quadro preenche a região de vizinhança 4 com
   o mesmo índice de cor do pixel clicado, com a tinta atual (cor + padrão).
   As 10 opções da faixa da lata aparecem; todas fazem o preenchimento
   simples no 2b.
9. **Ovo desfazer**: desfaz a última ação (traço, preenchimento, limpeza); um
   novo clique refaz; e assim por diante — só um nível, como no original.
   Clicar no ovo não muda a ferramenta ativa.
10. **Guardar os desenhos**: ao sair da pintura (porta Saída, Ctrl+Q ou fechar
    a janela), o quadro é gravado como PNG em
    `$XDG_DATA_HOME/atelie-livre/desenhos/` (padrão
    `~/.local/share/atelie-livre/desenhos/`): sempre como `ultimo.png` e, se
    algo foi pintado nesta visita, também como `desenho-AAAAMMDD-HHMMSS.png`
    (hora UTC). Ao entrar, a pintura abre `ultimo.png` se existir e tiver
    594×347; senão, quadro branco. Cores que não estão na paleta viram a mais
    próxima. Falhas de gravação viram `Log`, nunca travam.
11. **Moldura**: barra de ferramentas, colunas, barra inferior e bordas são
    desenhadas por código com as cores e posições medidas; os ícones das
    ferramentas, a faixa verde, as setas de página, o McZee e a porta Saída
    vêm de `ARTIST.PIC` (tabela na seção 4). Balões de ajuda do McZee e sons
    das ferramentas ficam para depois.

## 3. Mudanças na plataforma

- `Input::MouseUp { button, x, y }` (novo); `Input::Click` continua sendo o
  aperto do botão. O app passa a emitir `MouseUp`.
- `SceneId::Paint` (novo).
- `trait Scene` ganha `fn leave(&mut self) -> Vec<Effect> { Vec::new() }`,
  chamado pelo app antes de trocar ou descartar a cena e ao encerrar o laço.
  Os efeitos `Log` devolvidos são impressos.
- `Assets::palettes(Program) -> &Palettes` e
  `Assets::patterns(Program) -> Vec<[u8; 8]>` (vazio se faltar `PAT#`).
- O app calcula a pasta de desenhos (`XDG_DATA_HOME` ou
  `~/.local/share`) e a passa para `Paint::new`.

## 4. Geometria da tela de pintura (coordenadas lógicas)

| Elemento | Posição | Fonte |
|---|---|---|
| Ferramentas (normal / ativa) | Pincel (10,10) `0495`/`1013`; Lata (57,10) `1003`/`1017`; Adesivos (113,13) `101b`/`1025`; Sons (157,10) `1029`/`102d`; Palavra (211,13) `1031`/`1061`; Ovo (257,8) `103d`; Aspirador (306,13) `100b`; Transformadores (360,9) `1045`/`1065`; 3D (412,12) `1007`/`1069`; Imprimir (458,13) `104d`/`106d` | `ARTIST.PIC` |
| Área de clique de cada ferramenta | retângulo de 47×50 a partir de `(x_icone − 5, 5)` | medido |
| Faixa de opções | `112a` em (4,56) | `ARTIST.PIC` |
| Botões de opção | 12 botões 32×32 em `(25 + 37·k, 63)`, interior branco 26×26 em `(+3, +3)` | código |
| Seta acima / abaixo | `03b3` em (504,58) / `03b7` em (504,90); número da página entre elas, em (504,74) | `ARTIST.PIC` |
| McZee / porta Saída | `1055` em (537,49) / `1059` em (580,18); clique da Saída: (580..635, 18..105) | `ARTIST.PIC` |
| Cores | 38 caixas; a `k`-ésima tem interior 7×6 em `(7, 111 + 9·k)` com borda preta de 1 pixel; a ativa é desenhada maior (14×10) | medido |
| Padrões | 38 caixas; a `k`-ésima tem interior 11×6 em `(621, 111 + 9·k)` | medido |
| Quadro | (23,109), 594×347 | medido |
| Barra inferior | y 460..=474: "Ateliê" à esquerda, "Quadro Novo" no centro | código |

A troca de página do pincel (setas) circula 1→2→3→4→5→1 (abaixo) e ao
contrário (acima).

## 4a. Ferramenta Palavra (acrescentado depois do 2b)

Primeira versão, sem medir o original: a faixa tem dois botões em
`option_rect(25, 0..2)` com as imagens `1797` (texto simples) e `1799` (balão
"Oi!"). Clique no quadro fixa o canto superior esquerdo da linha e abre a
edição; o texto digitado chega como `Input::Text` (só caracteres com código
Windows-1252), Backspace apaga, Enter grava; qualquer outro clique também grava
(e, se for no quadro, começa outra linha); trocar de ferramenta e sair gravam.
Enquanto edita, o texto aparece sobre o quadro com um cursor que pisca a cada
0,5 s. A gravação usa a cor atual sólida (padrão ignorado), a "Pablo Graphos"
de 16 pontos (8 pontos ou a 8×8 reserva se faltar), é uma única ação de
desfazer e não pinta nada se o texto estiver vazio. O balão é o retângulo do
texto com 6 px de folga, branco com contorno preto de 1 px e um rabicho de 7 px
apontando para baixo e para a esquerda. Aproximações: a posição do texto em
relação ao clique, o formato do balão e a faixa de opções (sem a imagem de
fundo da faixa) não foram conferidos no original.

## 5. Fora do escopo

Adesivos, sons, transformadores, super-ferramentas, impressão,
efeitos da página 3, formas da página 5, opções 2+ do aspirador e da lata,
balões e falas do McZee, sons das ferramentas, galeria de desenhos, fluxo
completo de navegação e o Creative Writer.

## 6. Testes

- Unidades puras: regra de tinta com padrão; formas (contagem de pixels igual
  à medida: redondo 16 → 208, 12 → 112, 8 → 52, 4 → 12; quadrado 16 → 256;
  diagonal 16 → 31); Bresenham; preenchimento (região limitada, não vaza
  pela diagonal); máscara dos carimbos; desfazer/refazer de um nível;
  gravação e leitura PNG (ida e volta, tamanho errado ignorado).
- Cena: clique na porta da rua troca para `Paint`; Saída volta; clique em
  cor/padrão/forma/página muda o estado; traço por `Click`/`MouseMove`/
  `MouseUp` pinta o quadro; ferramenta inativa só registra `Log`.
- Dados reais (pulados sem `ATELIE_DATA`): quadro desenhado na posição certa
  e cor da coluna k igual à medida no original.
