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

## Documentos do Creative Writer (`<usuarios>/<apelido>/*.MAX`)

O Escritório grava os documentos no formato do original (`.MAX`, ver
`max.md`), na pasta do usuário — a mesma do cadastro (`perfil.toml`), como
o `C:\MSKIDS\USERS\<APELIDO>\` do original. Um `.MAX` do port abre no
Creative Writer original (conferido no Wine, com a miniatura na pasta) e
vice-versa. Código: `crates/engine/src/writer/{convert,store}.rs`.

| Arquivo | Quando |
|---|---|
| `<NOME8>.MAX` | salvar (Saída → "Salvar este trabalho", SALVAR do McZee, Ctrl+S, clique no título). O nome sai do título: maiúsculas, sem acentos, `_` no lugar de espaços e pontuação, até 8 caracteres ("Gato Teste" → `GATO_TES.MAX`; "!" do começo, que esconde o documento, fica de fora). Se o nome já existe e é de outro arquivo, os últimos caracteres viram um número a partir de 11 (`GATO_T11.MAX`). "Salvar a versão antiga e a nova" grava num nome novo e deixa o antigo; "Salvar só a nova" substitui o arquivo do documento |
| `KIDSnnnn.MAX` | o documento mudou e a cena foi fechada sem passar pelo "Epa!" (janela fechada): gravado no arquivo do documento, ou num `KIDSnnnn.MAX` novo, como o original depois de uma queda |
| `<NOME8>.TXT` | "Salvar como texto" (`Editor::save_text`): texto puro UTF-8, um parágrafo por linha, sem as figuras do Maçã |

Ao gravar, o port regera a miniatura (`THMB`: a folha 1 reduzida com o lado
maior de 144 pixels, 4 bits, paleta `THUMB_PALETTE`), a tabela de folhas
(`PGD `, da diagramação) e a seleção (`SELN`); o resto do que veio no
arquivo aberto (cópias dos objetos, sons em palavras, impressora, `ADD `,
molduras que o port ainda não representa) é regravado como veio
(`convert::MaxExtra`). Um `.MAX` de fora da pasta do usuário (os exemplos do
Max em `SHARED/USERS/MAX`) abre como cópia: salvar grava na pasta do usuário.
Os arquivos internos do original (`~NOTESID.MAX` etc.) não aparecem na
lista de documentos.

## Creative Writer: preferências e documentos (`<usuarios>/<apelido>/`)

Gravados por `crates/engine/src/writer/prefs.rs`.

| Arquivo | Conteúdo |
|---|---|
| `~MAXUSER.INF` | o mesmo formato do original (`maxprefs.md`): `FILR 0` = último documento (quadro do Escritório), `FILR 1..4` = molduras da Biblioteca; `STTB`/`SUDG`/`FNTL` guardados como vieram. Sem o arquivo valem os exemplos do Max (`SAMP18`, `SAMP9`, `SAMP16`, `SAMP5`, `SAMP6`). |
| `*.MAX` | documentos do usuário; no `FILR` ficam como `USERS\<APELIDO>\ARQUIVO.MAX` (os outros caminhos são relativos à pasta `MSKIDS` dos dados) |
| `predio-writer.txt` | estado das salas: `bolsa_aberta=0/1` e `ajuda_desligada=0/1` (o original também guarda, em bytes não localizados) |

Biblioteca (`scenes/writer_library.rs`): lista os `*.MAX` da pasta do
usuário (`store::list`) e os exemplos do Max; a parede vai nos `FILR` 1–4
pelas mesmas funções (`prefs::library`, `wall_doc`, `set_slot`, `save`) e
abrir um documento grava o `FILR 0` (`prefs::remember_last`). A prévia de
impressão grava a folha mandada ao `lp` como
`impressao-AAAAMMDD-HHMMSS.png` na pasta do usuário.
