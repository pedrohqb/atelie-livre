# Sons das ações (pendente)

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

| ação | som |
|------|-----|
| escolher Pincel | `0213` |
| escolher Lata de tinta | `0214` |
| usar a Lata (despejar) | `0225` |
| escolher Seleciona-adesivos | `020a` |
| escolher Seleciona-sons | `020b` |
| escolher Palavra | `0209` |
| Ovo desfazer (cada clique) | `020e` |
| escolher Aspirador | `0215` |
| escolher Transformadores | `0210` |
| escolher Super-ferramentas | `0216` |
| Imprimir | `0212` |
| botão de opção da faixa | `021c` |
| seta de página da faixa | `021d` |
| clique no McZee | `02cf` |
| ícone do andar (mapa) | `0252` |
| lupa | `0258` |
| traço do pincel, página 1 opção 1 | `021e` (com repetição) |
| traço do pincel, página 1 opção 2 | `0263` (com repetição) |
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

Os recursos `BTNW` do `ARTIST.EXE` (definições de botões) trazem o som do
botão no `u16` do deslocamento 4 (ex.: `BTNW 1000` → `021c`, `1001` →
`0252`, `1003..1006` → `0215`, `1007` → `0233`, `3a00..3a05` → `0224`);
servem para os botões das faixas e dos projetos.

## Repetição (`loop`) — falta no decodificador

Cada som do `ARTIST.SND` é um RIFF/WAVE (MS-ADPCM) com um bloco `loop` de
10 bytes: `u32` início, `u32` fim (em amostras) e `u16` 60. Com início e
fim diferentes de zero, o original toca do começo até o fim da repetição,
repete o trecho `início..fim` enquanto o botão está apertado (traços do
pincel, aspirador, spray) e, ao soltar, toca o resto (`fim..final`). Sons
com repetição: `021e`, `0229`, `022a`, `022b`, `022c`, `024f`, `0250`,
`0251`, `025e`, `0263`…`0268`, `026c`…`026e`, `02d8`, `0304`, `030e`.
`formats::wave::decode` hoje ignora o bloco `loop`.

## Para fazer

1. `wave::decode` devolver os pontos de repetição; o mixer do `app`
   ganhar "tocar com repetição até parar" (`Effect::PlayLoop(id)` /
   `Effect::StopLoop`, ou equivalente).
2. Medir o som de cada forma do pincel (6 páginas × 12 opções), das
   opções da lata, do aspirador e das palavras: o laço pronto está em
   `/tmp/brushes.sh` (pode ter sido apagado; refazer com `act.sh`).
3. Ligar na pintura: troca de ferramenta, opção, seta, ovo, lata, imprimir,
   McZee, mapa, lupa e os traços (início/repetição/fim no arrasto); depois
   salas (balões, poste, elevador) e projetos com o mesmo método.
