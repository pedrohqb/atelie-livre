# Objetos, Transformadores, Super-ferramentas e ajuda do McZee — desenho

Subprojeto do Fine Artist: os dois botões da barra de ferramentas que ainda
dizem "ainda não disponível" (Transformadores `1045` e Super-ferramentas de
desenho `1007`), a camada de objetos de que eles dependem, e o balão de
ajuda do McZee (nicho à direita da faixa). Fonte: os balões de ajuda do
original (`WHAT 1006` Transformadores, `100a` Super-ferramentas, `1005`
adesivos, `1009` palavras; `atelie-tools what ARTIST.EXE --block 1006`) e as
faixas de opções medidas no Wine (`scratch/orig_tr1..4.png`,
`orig_sf1.png`).

## 1. Camada de objetos

No original, adesivos e palavras **não são pixels**: ficam por cima da
pintura como objetos que se movem, esticam, giram, grudam, vão para a
frente/trás e podem ser aspirados ("Como eu vou saber se uma coisa é um
adesivo? … o contorno dele pisca"). O pincel e o balde pintam o fundo, por
baixo deles.

- `paint/objects.rs`: `Object { id, kind, x, y, w, h, flip_h, flip_v,
  turns: u8 (giros de 90°), link: Option<u32>, sound: Option<SoundRef> }`,
  `kind = Sticker { image: Rc<Image>, mask: Option<Rc<Image>> } | Words {
  text, font, color, balloon } | Animated { script, cells }`. `Objects` é
  uma lista ordenada (z); `draw(frame, origin)` desenha na ordem, aplicando
  escala (vizinho mais próximo), inversões e giros; `hit(x, y)` acha o
  objeto de cima cujo pixel opaco está sob o ponteiro.
- A tela de pintura passa a colocar adesivos (`stickers.rs`) e palavras
  (`words`) como objetos em vez de `Canvas::paste`; a animação dos adesivos
  animados continua tocando sobre o quadro; o ponteiro sobre um objeto
  faz o contorno piscar (máscara dilatada em preto/branco alternando a
  cada 0,25 s) quando a ferramenta é Transformadores, Super-ferramentas ou
  Aspirador.
- `History` guarda também a lista de objetos (snapshot leve: `Vec<Object>`
  com `Rc` nas imagens).
- Gravar: `ultimo.png`/`desenho-*.png`/`.mag` recebem a composição
  achatada; `ultimo.objetos.json` (e `desenho-*.objetos.json`) guarda os
  objetos (adesivo: id do rolo + casa, ou arquivo PNG do adesivo do
  usuário; palavra: texto e estilo) para reabrir editável. Se o JSON
  faltar, abre achatado.
- Aspirador: o modo "coisa" tira o objeto clicado; "cor"/"área" agem nos
  pixels como hoje; "copiar" copia o objeto.

## 2. Transformadores (`1045`; ajuda `WHAT 1006`)

Faixa de opções com 2 páginas (setas e número à direita, como o pincel):

Os ícones das opções estão no `ARTIST.PIC` em pares (cor = escolhido,
mono = normal), achados com `tools/wine/locate.py` nas faixas capturadas
(`scratch/orig_tr1.png`, `tr2.png`; botões de 32×32 em x = 25 + 37·k, y = 63):

| pág. | k | ícone cor/mono | nome (`WHAT 1006`) | ação |
|------|---|----------------|--------------------|------|
| 1 | 0 | `1470`/`1471` | Mover-modificar | arrastar move; Ctrl+arrastar copia; Shift+arrastar estica; clique abre o cavalete (fase 2) |
| 1 | 1 | `149d`/`14a0` | Dimensionar por arraste | arrastar estica nas duas direções; Shift mantém a proporção |
| 1 | 2 | `14a1`/`14a4` | Dimensionar lateral | arrastar muda só a largura |
| 1 | 3 | `14a5`/`14a8` | Dimensionar vertical | arrastar muda só a altura |
| 1 | 4 | `14b1`/`14b4` | Aumentar | segurar cresce 4 %/quadro até soltar |
| 1 | 5 | `14b5`/`14b8` | Diminuir | segurar encolhe |
| 1 | 6 | `14b9`/`14bc` | Empurrar para trás | clique desce um nível (segurar continua) |
| 1 | 7 | `14bd`/`14c0` | Trazer para a frente | clique sobe um nível |
| 2 | 0 | `1470`/`1471` | Mover-modificar | (repetido no original) |
| 2 | 1 | `14c1`/`14c4` | Girar à esquerda | clique gira 90° anti-horário |
| 2 | 2 | `14c5`/`14c8` | Girar à direita | clique gira 90° horário |
| 2 | 3 | `14c9`/`14cc` | Grudar ou desgrudar | clique em dois objetos liga (corrente entre eles; movem juntos); clique na corrente desliga |
| 2 | 4 | `14cd`/`14d0` | Inverter lateral | clique espelha |
| 2 | 5 | `14d1`/`14d4` | Inverter em cima/embaixo | clique espelha |
| 2 | 6 | `14d9`/`14dc` | Coloca-adesivos | grava o objeto como adesivo do usuário (fase 2) |

Separador de faixa: `14ef` (29×48). Confira as posições exatas com o
`locate.py` antes de fixar as constantes.

## 3. Super-ferramentas de desenho (`1007`; ajuda `WHAT 100a`)

Faixa única (`scratch/orig_sf1.png`; botões em x = 25 + 37·k, com o
separador `14ef` entre o 3.º e o 4.º): Sem quadriculado `1485`/`1488`,
Quadriculado grande `1489`/`148c`, Quadriculado pequeno `148d`/`1490` |
Ponto de fuga `1477`/`1478`, Linha do ponto de fuga `1479`/`147c`, Eixo de
construção `147d`/`1480`, Aproximar do PF `14d5`/`1481`?, Afastar do PF
`14d7`/`1484`?, Apagar linha guia (ímã) `14d8`? | Interruptor mágico
`1491` (22×41, em x 464; `1494`/`1497`/`1498` ligado/desligado). Os `?`
são palpites a confirmar com o `locate.py` (os ícones escolhidos no
momento da captura não casam porque estão realçados).

- As **linhas guia** são overlay (nunca no PNG): `Guides { grid: None |
  Big(48 px) | Small(24 px), horizon: Option<(y, vp_x)>, vp_lines:
  Vec<(x, y)> (ponta solta; a outra é o PF), axes: Vec<((x,y),(x,y))>,
  visible: bool }`, guardadas em `ultimo.guias.json`. Cores: quadriculado
  verde-claro `[0,200,0]` tracejado; horizonte e PF (círculo de 5 px)
  azuis; linhas do PF e eixos verdes de 1 px.
- Ponto de fuga: clique coloca horizonte + PF; arrastar o PF move os dois.
  Linha do PF: clique coloca uma linha da posição ao PF (sem PF: balão
  `100f[5]` "No VP defined"); arrastar a ponta move. Eixo: arrastar do
  início ao fim; arrastar o meio move, a ponta muda.
- Aproximar/Afastar do PF: com o botão apertado sobre um objeto, ele
  desliza em direção ao PF (ou se afasta) a 60 px/s encolhendo/crescendo
  proporcionalmente à distância ao PF (tamanho = tamanho original ×
  distância/distância original); Ctrl faz uma cópia.
- Ímã: clique com a linha entre as pontas (12 px) apaga a linha; apagar o
  horizonte apaga as linhas do PF.
- Interruptor: esconde/mostra todas as guias.

## 4. Ajuda do McZee (nicho `533..580 × 56..104`)

Clique no McZee abre o balão da ferramenta atual, do `WHAT`: Pincel
`1002[1]`, Lata `1003[0]`, Adesivos `1005[0]`, Sons `1004[0]`, Palavra
`1009[0]`, Ovo `1007[0]`, Aspirador `1008[0]`, Transformadores `1006[0]`,
Super-ferramentas `100a[0]`, Imprimir `100b[0]`; e a opção escolhida na
faixa abre o balão dela (Transformadores `1006[1..20]`, Super-ferramentas
`100a[1..17]`, lata `1003[1..13]`, pincel `1002[5..28]`). O balão fica à
direita do quadro (x 445, y 115, largura do registro), fecha com um
clique fora, e as ligações de dica (`21b`/`21c`/`21e`) abrem o registro
ligado no lugar. Na primeira vez que a tela abre, mostra o do Pincel, como
no original. O `Balloon` precisa desenhar as figuras embutidas (`Item::Picture`
com ids `1xxx`: o ícone da ferramenta) e o negrito (sem fonte negrito:
desenhar duas vezes com 1 px de deslocamento).

## Testes

- Objeto colocado e movido 20 px: o PNG achatado muda e o JSON reabre com o
  objeto na posição nova; aspirar "coisa" tira só o objeto.
- Girar à direita 4 vezes volta ao original; inverter duas vezes idem.
- Grudar dois objetos e mover um move os dois.
- Aproximar do PF por 1 s reduz o tamanho e aproxima; o PNG não tem guias.
- Clique no McZee com a lata abre o balão com o título "Lata de tinta" (do
  `WHAT`).
