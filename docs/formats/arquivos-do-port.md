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

Abrir um desenho pela Galeria ou pela pasta (`store::open_as_last`) grava
`ultimo.png` e copia `<stem>.objetos.json` e `<stem>.fundo.png` para
`ultimo.*` (ou apaga os do último desenho anterior), para que os adesivos e
as palavras continuem editáveis. Jogar um desenho no lixo da Galeria apaga
o `.png`, o `.mag` gêmeo, o `.objetos.json`, o `.fundo.png` e o
`.projeto.txt`.

Adesivos do usuário (Cavalete de adesivos, Cachorrinho, lata que muda um
adesivo, Coloca-adesivos dos Transformadores): `meu-adesivo-<...>.png`, o
carimbo cortado ao que foi pintado, com o branco ligado à borda
transparente. Ficam no rolo "Meus adesivos" e não aparecem na Galeria nem no
Cachorrinho. Os `adesivo-*.png` de versões anteriores ainda são lidos como
adesivos do usuário, menos os que têm `.projeto.txt` ao lado: esses são
trabalhos do projeto Adesivo (`adesivo-<nome>.png`, página inteira).

Os `.MAG` (desenhos de exemplo e os gravados pelo port) abrem pelo tamanho:
594×347 ocupa o quadro; uma figura menor (os botões e adesivos de exemplo,
162×162, 216×162, 162×198) fica no meio de um quadro branco; maior é
recusada.

## `<stem>.objetos.json`

JSON de uma linha por objeto, gravado por `crates/engine/src/paint/objects.rs`
(`to_json`/`parse_json`; o leitor tolera linhas inválidas e campos ausentes):

```json
{"quadro": "<impressão digital FNV-1a do PNG achatado, 16 hex>",
"objetos": [
  {"id": 1, "x": 10, "y": 20, "w": 48, "h": 36, "inv_h": false, "inv_v": false, "giros": 0, "rolo": 0, "casa": 5},
  {"id": 2, "x": 60, "y": 20, "w": 30, "h": 30, "inv_h": false, "inv_v": false, "giros": 1, "corrente": 1, "png": "meu-adesivo-20261031-120000-2.png"},
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
  `cresce`, `encolhe`, `zigue-zague`, `triangulo`, `seta`, `arco-iris`,
  `esse`, `coracao`, `carro`, `peixe`, `cabeca` — o retângulo e os `SCRD`
  1–14; os nomes antigos `triangulo-baixo`, `arco` e `circulo` são lidos
  como `triangulo`, `arco-iris` e `arco-iris`), `estilos` (lista separada
  por vírgulas de `negrito`, `inclinado`, `contorno`, `maiusculas`,
  `vertical`, `preencher`, `em-cima-embaixo`, `esquerda-direita`), `cores`
  (índices da paleta separados por vírgula; mais de um = alternados por
  letra), `padrao` (índice na lista de padrões do cavalete: 0 = cheio, 1–13
  = `PATS`; antes era o padrão da pintura), `sombra` (`nenhuma`, `normal`,
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

## Ideias do Creative Writer (`<usuarios>/<apelido>/ideias/`)

Gravadas por `crates/engine/src/writer/ideas/store.rs` (texto UTF-8; os
leitores ignoram linhas inválidas e arquivos ausentes; cada arquivo é
gravado num `.tmp` e renomeado). Correspondem aos arquivos `~MAXUSER.INF`
(`STTB`/`SUDG`), `~MYPICS.CHK`, `~SPLOTID.MAX`, `~NOTESID.MAX` e
`~PICWIN.CHK` do original (ver `workshop.md`).

| Arquivo | Conteúdo |
|---|---|
| `maquina.txt` | entradas próprias da Máquina de Idéias, uma por linha: `1=O pirata Zeca,` (janela 1, 2 ou 3 e o texto, numa linha só) |
| `figuras.txt` + `figuras/NNN.png` | figuras próprias do Quadro de Figuras, das mais novas para as mais velhas: uma linha `NNN.png=Título` por figura; PNG RGBA |
| `caderno/maquina.txt` | Caderno de Idéias, aba Máquina: uma sentença por linha; figura-rebus como `[rebus:N]` (N = caractere, 1–255, decimal) |
| `caderno/lembretes.txt` | Caderno, aba Lembretes: texto livre |
| `caderno/figuras.txt` + `caderno/figuras/NNN.png` | Caderno, aba Figuras: uma página por figura, na ordem das páginas, no mesmo formato de `figuras.txt` |

Nas listas de figuras só valem nomes `NNN.png` (dígitos), sem caminho. Ao
gravar, figuras novas ou modificadas recebem o próximo número livre e os
PNGs que saíram da lista são apagados.

## Textos do Creative Writer (`~/.local/share/atelie-livre/textos/`)

| Arquivo | Conteúdo |
|---|---|
| `ultimo.txt`, `texto-AAAAMMDD-HHMMSS.txt` | o documento de trabalho e as cópias datadas do Escritório (`writer/store.rs`) |
| `*.max` | documentos no formato do original (`max.md`); a Biblioteca os mostra com a miniatura `THMB` |
| `~MAXUSER.INF` | preferências do usuário, como no original (`maxprefs.md`); a Biblioteca grava a parede nos `FILR` 1–4 e mantém o resto (lugar 0 = Escritório, fontes, Máquina de Idéias) |
| `impressao-AAAAMMDD-HHMMSS.png` | folha mandada ao `lp` pela prévia de impressão da Biblioteca |

Nos `FILR` gravados pelo port, os exemplos do Max vão como no original
(pastas `SHARED`, `USERS`, `MAX` a partir de `MSKIDS`); um documento da
criança vai **sem pastas**, só com o nome do arquivo, e é procurado na
pasta de textos. A Biblioteca lista como documentos da criança os `*.max`
e os `texto-*.txt` (não o `ultimo.*`).
