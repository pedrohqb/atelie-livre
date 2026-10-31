# Ferramentas para medir o Fine Artist original no Wine

Scripts usados para levantar posições, cores e animações rodando o original
(que você precisa ter) num Xvfb. Nenhum arquivo original entra no repositório;
as capturas ficam em `scratch/` (ignorada pelo git).

- `wfa.sh start|click X Y|key K|type T|shot NOME` — controla o original em
  coordenadas lógicas 640×480 (janela centrada em 1024×768, deslocamento 192,144).
- `restart.sh` — reinicia o original e vai direto ao Ateliê (usuário de teste
  na primeira posição da lista), com a ferramenta de adesivos.
- `rec.sh ROLO CASA PASTA` — escolhe o adesivo, coloca em (330,280) (ou
  `CX`/`CY`) e grava `RECT` segundos (padrão 10) a 15 quadros/s.
- `recroll.sh ROLO [MAX]` — grava as casas de um rolo e roda a análise.
- `locate.py CAPTURA PASTA...` — acha, numa captura, onde aparecem as imagens
  extraídas (PNG) de uma ou mais pastas.
- `analyze.py PASTA X Y` — quadro a quadro de uma gravação, quais imagens de
  adesivo de `ARTIST.PIC` aparecem e onde (aceita partes escondidas).
- `steps.py` e `gen_scripts.py` — transformam as análises em
  `crates/engine/src/scenes/sticker_scripts.rs`.

Os scripts esperam o repositório em `/home/claude/atelie-livre`, a extração do
`atelie-tools extract` em `out/` e o prefixo do Wine em `/root/wine-fa`; ajuste
os caminhos para a sua máquina.
