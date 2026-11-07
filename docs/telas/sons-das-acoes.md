# Sons das ações

Relato do Pedro (31/10): o Ateliê Livre funciona, mas as ações não fazem
som. Causa: a tela de pintura só toca som na ferramenta Sons e nos adesivos
animados; os sons das ferramentas, opções e traços do original nunca foram
ligados.

## O que já foi medido no original (Wine, `WINEDEBUG=+winmm`)

Método: cada `waveOutWrite` registra `dwBufferLength`, que é o tamanho em
bytes do som decodificado (PCM 16 bits, mono). Comparando com os tamanhos
dos sons do `ARTIST.SND` (e com os pontos de repetição, abaixo) acha-se o
id. Scripts usados (fora do repositório, em `/home/claude`): `wfa98t.sh`
(original no Xvfb :98 com o rastro do winmm, com `drag`), `act.sh ROTULO
COMANDOS` (executa e lista os sons) e `match.py` (tamanhos → ids, usando
`/tmp/sndinfo.json`).

Medições completas (1/11). Ids ambíguos por tamanho (sons de mesmo tamanho)
foram resolvidos encurtando, numa cópia do `ARTIST.SND` no prefixo do Wine,
o bloco `data` de um candidato: o som encurtado deixa de tocar, e o que some
é o id certo (a cópia foi restaurada depois).

### Ferramentas e botões

| ação | som |
|------|-----|
| escolher Pincel | `0213` |
| escolher Lata de tinta | `0214` |
| escolher Seleciona-adesivos | `020a` |
| escolher Seleciona-sons | `020b` |
| escolher Palavra | `0209` |
| Ovo desfazer (cada clique) | `020e` |
| escolher Aspirador | `0215` |
| escolher Transformadores | `0210` |
| escolher Super-ferramentas | `0216` |
| Imprimir | `0212` |
| botão de opção da faixa (pincel, lata, transformadores 2–8, super 6–11) | `021c` |
| seta de página da faixa (pincel, sons, adesivos) | `021d` |
| opção do Aspirador (qualquer uma) | `0252` |
| Transformadores, opção 1 (mão: mover) | `021f` |
| Super-ferramentas: sem grade | `0221` |
| Super-ferramentas: grade pequena / grande | `0233` |
| clique no McZee | `02cf` |
| ícone do andar (mapa) | `02d3` |
| lupa | `0309` |
| porta Saída (abre o "Epa! O que você quer fazer agora?") | `02fe` + `02d6` |
| escolher cor / padrão | (sem som) |
| salas: abrir / fechar a Bolsa Mágica | `02cf` / `02ce` |
| salas: maleta (pasta) | `02f1` |
| salas: livro verde (história) | `02e2` ou `0265`, e `02f6` |
| salas: pegar a Varinha Mágica | `0307` |
| salas: varinha no buraco do rato | `020a` ou `02ea` (mesmo tamanho) |
| salas: mastro (subir, ou varinha no mastro) | `02f3` |
| salas: abertura do mastro (descer) | `02f2` |
| salas: skate | `02fc` (e `025e`/`0269`/`0303`, mesmo tamanho) |
| elevador: motor, portas, chegada | `02d8` (com repetição), `02d4`, `0311` |

### Traços do pincel (com repetição: começo no aperto, laço no arrasto, fim ao soltar)

| página | opções | som |
|--------|--------|-----|
| 1 (linhas) | todas | `021e` |
| 2 (carimbos) | todas | `0263` |
| 3 (efeitos) | 1–12 | `0266` `0267` `0268` `0269` `0301` `026b` `026c` `026d` `02c6` `0255` `026e` `024f` |
| 4 | todas | `021e` |
| 5 (formas elásticas) | 1, 2, 5, 12 | `022a` |
| 5 | 3, 4, 6 | `022b` |
| 5 | 7–11 | `022c` |
| 6 (spray) | todas | `0250` |

Na página 3, `0269`, `0301`, `026b`, `02c6` e `0255` não têm laço: tocam uma
vez por traço. `0269` (mesmo tamanho de `025e`, `02fc`, `0303`) e `0301`
(mesmo tamanho de `021f`) foram confirmados pelo método do `data` encurtado.

### Lata de tinta (despejo)

| opção | som |
|-------|-----|
| 1 | `0225` |
| 2, 3 | `0227` |
| 4 | `0258` |
| 5–10 | `0223` (a 5 num quadro vazio abre o aviso "Epa!", som `02fe`) |

### Aspirador (uso no quadro)

| opção | som |
|-------|-----|
| 1 aspira tinta | `0229` (laço, enquanto arrasta) |
| 2 aspira adesivos | (não medido: sem adesivo no quadro) |
| 3 cola | `0228` |
| 4 aspira-área | `025f` |
| 5 aspira-cópias | `025e` (laço) |
| 6 aspira-cores | `0229` (laço) |
| 7 remove-sons | (sem som sem sons no quadro) |
| 8 jogar tudo fora | `025f` |

### Palavra, adesivos, sons, transformadores, super-ferramentas

| ação | som |
|------|-----|
| Palavra: opção (no original abre o Cavalete de palavras) | `0224` + `0209` |
| Palavra: digitar | (sem som) |
| Palavra: OK (a palavra vai para o quadro) | `02ee` |
| Palavra: clique numa palavra do quadro | `0228` + `022e` + `0209` |
| Adesivos: clique num rolo | `0259` |
| Adesivos: escolher adesivo no filme | `022d` |
| Adesivos: colocar no quadro | `0228` |
| Sons: clique numa categoria | `0233` |
| Sons: colocar | o próprio som escolhido |
| Transformadores: uso (arrastar adesivo) | (sem som) |
| Super-ferramentas: traçar guia (PF, linha do PF, eixo, grade) | `022c` (laço) |

Os recursos `BTNW` do `ARTIST.EXE` (definições de botões) trazem o som do
botão no `u16` do deslocamento 4 (ex.: `BTNW 1000` → `021c`, `1001` →
`0252`, `1003..1006` → `0215`, `1007` → `0233`, `3a00..3a05` → `0224`);
servem para os botões das faixas e dos projetos.

## Repetição (`loop`)

Cada som do `ARTIST.SND` é um RIFF/WAVE (MS-ADPCM) com um bloco `loop` de
10 bytes: `u32` início, `u32` fim (em amostras) e `u16` 60 (ver
`docs/formats/sons.md`). Com fim maior que início, o original toca do começo
até o fim da repetição, repete o trecho `início..fim` enquanto o botão está
apertado (traços do pincel, aspirador, spray, guias) e, ao soltar, toca o
resto (`fim..final`). Sons com repetição: `021e`, `0229`, `022a`, `022b`,
`022c`, `024f`, `0250`, `0251`, `025e`, `0263`…`0268`, `026c`…`026e`,
`02d8`, `0304`, `030e`.

No Ateliê Livre: `formats::wave::decode` devolve o trecho em
`Pcm::loop_points`; as cenas pedem `Effect::PlayLoop(som)` no aperto e
`Effect::StopLoop` ao soltar; o `Mixer` do `app` (com a lógica de amostras
em `Looper`, testada sem SDL) mantém só ~1/8 s na fila, para o fim do som
vir logo depois de soltar. Um laço por vez: um `PlayLoop` novo solta o
anterior. Trocar de cena também solta o laço.

## Ligado na tela de pintura

`crates/engine/src/scenes/paint_sfx.rs` guarda as tabelas acima; `paint.rs`
toca: ferramentas da barra, opções e setas das faixas, McZee, mapa, lupa,
despejo da lata, traços do pincel (laço com o som da forma), aspirador
(laço nos modos de arrastar), adesivos (rolo, escolher, colocar), categorias
de sons, Palavra (opção, começar, terminar) e guias das Super-ferramentas.

## Animação dos ícones da barra

Gravação do original (15 quadros/s) ao clicar em cada ferramenta: nenhum
dos 162 roteiros `ATAG` usa as imagens da barra (`0x1000`–`0x10ff`,
`0x1470`–`0x14f8`); a "animação" é troca de imagens. Por um quadro o ícone
pisca a versão de 1 bit (a imagem logo antes da máscara) sobre o fundo aceso
e passa à versão ativa. Só dois ícones fazem mais:

- Ovo: o pintinho (`103d`) vira o ovo inteiro (`1041`, máscara `1043`) por
  ~0,27 s e volta (o ovo não tem versão ativa).
- Imprimir: a impressora vira a torradeira soltando fumaça (`106d`/`106f`)
  enquanto imprime (no Ateliê Livre, 1 s).

O Aspirador ativo é o saco cheio (`100f`/`1011`), não o ícone normal.
`paint_sfx::State` guarda a animação; `draw_tools` desenha a imagem dela
centrada no ícone.

## Entrada

Medidos pela varredura de cliques (`sweep`, 6/11) e ligados em
`scenes/entry.rs` (`entry::snd`); a lista completa está em
`docs/telas/fine-artist.md`, item 1. Os sons em sequência (abertura,
botão + vinheta do "EPA!") tocam um depois do outro: a cena guarda a
fila e espera a duração do PCM anterior.

## Para fazer

1. Porta Saída: o original abre o aviso "Epa!" (`02fe` + `02d6`); o Ateliê
   Livre vai direto para o andar, sem aviso nem som.
2. Aspirador de adesivos e ímã das Super-ferramentas: som não confirmado.
3. Salas (balões, poste, elevador) e projetos, com o mesmo método.
