# Teste de jogo

Testes de integração que jogam o Fine Artist como uma criança, de uma cena
para outra, para achar falhas entre partes feitas separadamente.

## No engine (`crates/engine/tests/`)

- `common/mod.rs` — **diretor de cenas**: a mesma pilha de cenas e o mesmo
  tratamento de efeitos do laço do app (`make` espelha o `make_scene` de
  `crates/app/src/run.rs`; uma cena nova precisa entrar nos dois), com as
  pastas do usuário (`desenhos/`, `usuarios/`, `textos/`) num diretório
  temporário. Gestos: `click` (aperta e solta em quadros seguidos),
  `quick_click` (no mesmo quadro), `double_click`, `drag`, `key`, `text`,
  `idle`, `wait_for`. `dt` é ajustável (simula uma máquina lenta). O rastro
  (`trace`) usa as linhas do `ATELIE_TRACE`, mais os `Log` e os efeitos que
  um `leave` devolve e o app só imprimiria.
- `playtest_monkey.rs` — macaco determinístico: rajadas aleatórias de
  cliques, arrastos, duplos cliques, teclas e texto em todas as cenas do
  Fine Artist, seguindo as trocas de cena; procura pânicos no `update`, no
  `draw` e no `leave`. `PLAYTEST_BURSTS` (padrão 25), `PLAYTEST_SEEDS`
  (padrão 1) e `PLAYTEST_SCENE` (ex. `Paint`) ajustam a rodada.
- `playtest_paint.rs`, `playtest_rooms.rs`, `playtest_files.rs` — percursos
  e regressões (sons e aspirador, Coloca-adesivos, adesivos do usuário ×
  projeto Adesivo, galeria abrindo e jogando fora desenhos com objetos,
  clique duplo em máquina lenta, desenhos de exemplo).
- `playtest_perf.rs` (ignorado por padrão) — tempo de CPU por quadro de
  cada cena; rode em release com `-- --ignored --nocapture`.

Todos precisam de `ATELIE_DATA`; sem ela são pulados.

## No app

`ATELIE_TRACE=1` imprime cada efeito (`TRACE som`, `TRACE cena`, `TRACE
push`, `TRACE back`...) e, desde o teste de jogo, `TRACE quadro-lento N ms`
para cada quadro (atualizar + desenhar) acima de 50 ms. Com
`XDG_DATA_HOME` próprio e `ATELIE_LP` apontando para um `lp` de mentira, um
robô externo (xdotool + capturas) pode jogar o programa inteiro sem tocar
nos desenhos do usuário nem na impressora.

## Clique duplo

As cenas medem o clique duplo (`ui::DoubleClick`, 0,4 s) num relógio
próprio em que cada quadro conta no máximo 0,1 s
(`scene::click_time`): numa máquina lenta os dois cliques chegam em quadros
seguidos com um `dt` grande entre eles, embora a criança tenha clicado
depressa. Nas salas, o segundo clique que chega no mesmo quadro também
completa o clique duplo.
