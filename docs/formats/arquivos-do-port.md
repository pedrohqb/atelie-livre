# Arquivos do Ateliê Livre ao lado dos desenhos

Os desenhos guardados na pasta de desenhos (`~/.local/share/atelie-livre/desenhos/`)
são PNGs achatados (`ultimo.png`, `desenho-AAAAMMDD-HHMMSS.png`,
`ultimo-quadrinhos.png`, `botao-<nome>.png`...). Para reabrir editável, cada
PNG pode ter arquivos ao lado, com o mesmo radical (`<stem>`):

| Arquivo | Conteúdo |
|---|---|
| `<stem>.objetos.json` | adesivos e palavras por cima da pintura (ver abaixo) |
| `<stem>.fundo.png` | a pintura sem os objetos |
| `<stem>.projeto.txt` | metadados do projeto da Oficina (ver abaixo) |
| `sons/<nome>.wav` | sons gravados pelo Gravador do Seleciona-sons (PCM 16 bits, mono, 22 050 Hz) |

Sem o `.objetos.json` o desenho abre achatado; sem o `.projeto.txt` o projeto
abre com os valores iniciais.

## `<stem>.objetos.json`

JSON de uma linha por objeto, gravado por `crates/engine/src/paint/objects.rs`
(`to_json`/`parse_json`; o leitor tolera linhas inválidas e campos ausentes):

```json
{"quadro": "<impressão digital FNV-1a do PNG achatado, 16 hex>",
"objetos": [
  {"id": 1, "x": 10, "y": 20, "w": 48, "h": 36, "inv_h": false, "inv_v": false, "giros": 0, "rolo": 0, "casa": 5},
  {"id": 2, "x": 60, "y": 20, "w": 30, "h": 30, "inv_h": false, "inv_v": false, "giros": 1, "corrente": 1, "png": "adesivo-20261031-120000-2.png"},
  {"id": 3, "x": 100, "y": 100, "w": 80, "h": 40, "inv_h": false, "inv_v": false, "giros": 0, "texto": "Oi!", "balao": true, "cor": 1, "forma": "oval-rabicho", "alinha": "centro", "largura": 60, "fonte": "pequena"},
  {"id": 4, "x": 150, "y": 90, "w": 300, "h": 140, "inv_h": false, "inv_v": false, "giros": 0, "palavra_forma": "Ana", "tipo": "Liberation Sans", "molde": "onda", "estilos": "negrito,contorno", "cores": "1,25", "padrao": 0, "sombra": "normal", "tamanho": 128, "giro": 15},
  {"id": 5, "x": 200, "y": 160, "w": 40, "h": 28, "inv_h": false, "inv_v": false, "giros": 0, "dipitidu": 2}
]}
```

- `id`, `x`, `y` (canto, coordenadas do quadro), `w`, `h` (tamanho mostrado),
  `inv_h`, `inv_v`, `giros` (0–3, de 90° horários); `corrente` (id do objeto
  grudado); `som` e `som_indice` (som preso: biblioteca e índice).
- Origem, uma de: `rolo` + `casa` (adesivo de um rolo; rolo 0 = bichos, 1–29
  clip-art (29 = cenários de `C-MAG100.ART`), 30 = rabinhos de balão nos
  quadrinhos; o rolo do usuário, sempre o último, não aparece aqui: os
  adesivos dele são gravados como `png`); `png` (adesivo do usuário,
  arquivo na pasta de desenhos); `palavra_forma` (palavra com forma);
  `dipitidu` (casa do rolo de dipitidus da Palavra: rabinhos `PICT` 16–19 de
  `B-MAG100.CHK`); `texto` (palavra ou balão).
- Palavra com forma (Cavalete de palavras com forma, `paint/shaped.rs`): o
  texto em `palavra_forma`; `tipo` (família instalada; vazio = "Pablo
  Graphos"; se a família não existir na máquina, a imagem sai na do
  programa), `molde` (`retangulo`, `arco-embaixo`, `onda`, `inflado`,
  `cresce`, `encolhe`, `zigue-zague`, `triangulo`, `triangulo-baixo`,
  `arco`, `circulo`), `estilos` (lista separada por vírgulas de `negrito`,
  `inclinado`, `contorno`, `maiusculas`, `vertical`), `cores` (índices da
  paleta separados por vírgula; mais de um = alternados por letra), `padrao`
  (índice do padrão da pintura, 0 = cheio), `sombra` (`nenhuma`, `normal`,
  `amanhecer`, `anoitecer`, `girar-sumir`, `girar-circulo`, `convergente`,
  `tridimensional`), `tamanho` (altura da linha em pixels) e `giro` (graus,
  anti-horário). A imagem é refeita ao abrir.
- Palavra: `balao` (bool, sempre gravado) e `cor` (índice da paleta). Os
  campos do Cavalete de balões só aparecem quando fogem do texto solto ou do
  balão "Oi!": `forma` (`nenhum`, `retangulo-rabicho`, `retangulo`,
  `oval-rabicho`, `oval`, `pensamento`), `alinha` (`esquerda`, `centro`,
  `direita`), `largura` (pixels em que o texto quebra; ausente = uma linha) e
  `fonte` (`pequena` = Pablo 8 pt; ausente = 16 pt). Sem `forma`, `balao: true`
  é `retangulo-rabicho` e `false` é `nenhum`.
- Som preso (Seleciona-sons), um de: `som` + `som_indice` (biblioteca de
  `SHARED/STUFF`, como `MAG100.SND` ou `MAX100.SND`, e o índice do som nela) ou
  `som_arquivo` (caminho de um `.wav` do usuário, achado pelo Cachorrinho ou
  gravado pelo Gravador em `sons/<nome>.wav` dentro da pasta de desenhos). Um
  `som_arquivo` vazio ou que não seja texto é ignorado; o objeto fica sem som.

## `<stem>.projeto.txt`

Linhas `chave=valor` (UTF-8, sem caracteres de controle), gravadas por
`crates/engine/src/scenes/projects/mod.rs` (`save_meta`/`load_meta`):

| Chave | Projeto | Valor |
|---|---|---|
| `forma` | botão/adesivo | `botao-redondo`, `botao-quadrado`, `retangular`, `redondo`, `quadrado`, `oval-alto`, `oval-largo` |
| `quadrinhos` | história em quadrinhos | número de quadrinhos, 1 a 4 |
| `titulo` | história em quadrinhos | título (vai para a barra inferior e para a página impressa) |
| `autor` | história em quadrinhos | nome do autor |

Um projeto sem metadados (lista vazia) não grava o arquivo (e apaga o que
houvesse). Ao jogar fora um trabalho no fim guiado, `ultimo-<tipo>.png`,
`.objetos.json`, `.fundo.png` e `.projeto.txt` são apagados.
