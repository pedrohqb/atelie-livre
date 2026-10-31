# Ateliê Livre — Subprojeto 1: ferramentas de formato

Data: 2026-10-31
Status: aguardando revisão

## 1. Contexto e objetivo do projeto

O Ateliê Livre é uma reimplementação livre (GPL-3.0-or-later), em Rust, dos
programas infantis Microsoft Fine Artist e Microsoft Creative Writer (Microsoft
Kids / Turning Point Software, 1993–1995). O programa novo lê os arquivos de
dados originais que o usuário possui; nenhum arquivo original é distribuído.

Objetivos, em ordem de peso:

1. Crianças usarem os dois programas num Linux atual, com o conteúdo e o
   comportamento dos originais.
2. Resolver os problemas que hoje impedem esse uso: resolução (no Wine a tela
   de 640×480 fica minúscula) e som.
3. Preservação: ler os arquivos originais e os documentos salvos pelos usuários
   (`.MAG`, `.MAX`) sem perda, e documentar os formatos abertamente.

Decisões já tomadas:

- Programa avulso em Rust, não engine do ScummVM. Motivo: as regras de uso de
  IA do ScummVM (`AI-GUIDELINES.md`) não admitem código extenso escrito por
  agente de IA, o que inviabilizaria a entrada no repositório oficial.
- Plataforma: SDL3 via crate `sdl3`, isolada num único crate (`platform-sdl`),
  substituível por `sdl3-sys` ou por winit + softbuffer + cpal se os bindings
  se mostrarem instáveis.
- Resolução: primeiro, ampliação da tela original de 640×480 pela
  apresentação lógica da SDL3 (escala inteira ou suavizada, 4:3 com faixas).
  A engine usará coordenadas lógicas de 640×480 e desenhará num quadro de
  tamanho parametrizável, para permitir mais tarde texto vetorial e tela de
  pintura em alta resolução sem reescrita.

## 2. Decomposição

| # | Subprojeto | Depende de |
|---|---|---|
| 1 | Ferramentas de formato (este documento) | — |
| 2 | Engine base comum (telas, cursores, paleta, sons, animações, usuários) | 1 |
| 3 | Fine Artist | 2 |
| 4 | Creative Writer (inclui ortografia via Hunspell) | 2 |

Cada subprojeto terá spec, plano e implementação próprios.

## 3. Fatos já levantados sobre os dados

Versão analisada: Windows 3.1, português, 1995 (pasta `MSKIDS` instalada).

- `ARTIST.EXE` e `WRITER.EXE`: executáveis NE (Win16). `WRITER.EXE` tem 174
  segmentos. Importam apenas KERNEL, USER, GDI, KEYBOARD, COMMDLG, MMSYSTEM,
  TOOLHELP (e WIN87EM no Fine Artist).
- Recursos com tipos por nome no estilo Mac clássico: `STR#`, `STR`, `PAT#`,
  `PATS`, `WHAT`, `SCRD`, `ATAG`, `SWND`, `DSPC`, `BTNW`, `TMEN`, `CSCH`,
  `SSHD`, `TDRL`, `SMNM`, `ACUR`; além dos padrões do Windows (ícones,
  cursores, diálogos, menus, strings, versão). Os dois executáveis usam o mesmo
  conjunto de tipos. Os dados desses recursos estão em little-endian.
- `LOGPALETTE`: dois recursos por executável — 16 cores (id 0x8004) e
  256 cores (id 0x8008), no formato `LOGPALETTE` do Windows (versão 0x300).
- `ARTIST.PIC` / `WRITER.PIC`: assinatura `GLOBGLOBGLOB`, dois `u16` (1, 1),
  contagem `u16`, seguida de entradas de 6 bytes (`u16` id, `u32` offset),
  tudo little-endian. 2.469 entradas no Fine Artist, 2.312 no Creative Writer.
  Cada entrada começa com `00 01 00 00 'PKBM'` e cabeçalho em **big-endian**
  (largura, altura e profundidade visíveis; profundidades 1 e 4 observadas).
- `ARTIST.SND` / `WRITER.SND`: DLLs NE só de recursos; tipo `SND` (131 no
  Fine Artist) e `SNDRANGE`. Cada `SND` é um arquivo RIFF/WAVE completo, formato
  0x0002 (ADPCM da Microsoft), mono, 11.025 Hz.
- Documentos (`.MAG`, `.PTM`, `.MAX`) e `MSKIDS.INF`: assinatura `CHNK`,
  criador de quatro letras (`SPLT`, `INK `), versão `u16` 4 / `u16` 4, e
  blocos com etiquetas de quatro letras (`CTBL`, `FLOR`, `FLAG`, `WIZA`,
  `FWLS`...). Parece antecessor do formato `CHN2` do 3D Movie Maker
  (Kauai, `CHUNK.CPP`), mas o layout do índice é diferente.
- `SPELL.DCT` / `THES.DCT`: dicionários licenciados (INSO / Houghton Mifflin);
  fora do escopo, serão substituídos por Hunspell no subprojeto 4.
- Disquetes de instalação: compressão KWAJ método 3. Fora do escopo — o
  usuário copia os arquivos já instalados.

## 4. Escopo do subprojeto 1

### Entra

1. Leitor de executáveis NE: tabela de recursos (tipos numéricos e por nome),
   strings, versão, ícones e cursores exportados como PNG, paletas.
2. Bitmaps: índice `GLOB` e decodificação `PKBM` para PNG com a paleta
   correta.
3. Sons: recursos `SND` para WAV PCM de 16 bits (decodificação do ADPCM da
   Microsoft).
4. Documentos `CHNK`: cabeçalho, índice, árvore de blocos e extração bruta de
   cada bloco.
5. Recursos de comportamento: decodificação de `STR#`, `STR`, `PAT#`;
   extração bruta com hexdump dos demais.
6. `docs/formats/`: documentação aberta de cada formato, atualizada a cada
   descoberta.

### Fica fora

- Engine, janelas, qualquer renderização interativa.
- Interpretação semântica dos documentos e dos recursos de comportamento.
- Versões Mac e de outros idiomas (o código não deve presumir que só existe a
  versão em português, mas não as testa).
- Dicionários da INSO, `PICTIMP.FLT`, instalação pelos disquetes.

## 5. Arquitetura

Workspace Cargo na raiz do repositório:

```
atelie-livre/
├── Cargo.toml            (workspace)
├── crates/
│   ├── formats/          (biblioteca pura)
│   └── tools/            (binário `atelie-tools`)
└── docs/
    ├── formats/          (documentação dos formatos)
    └── superpowers/specs/
```

Crates futuros (`engine`, `platform-sdl`, aplicativos) entram em `crates/`
nos subprojetos seguintes.

### 5.1 Crate `formats`

Biblioteca sem janela, som ou E/S de rede; depende só de `std` (e de
`thiserror` se ajudar na clareza dos erros). Sem `unsafe`.

| Módulo | Entrada | Saída |
|---|---|---|
| `reader` | `&[u8]` | leitor com limites verificados: `u8`, `u16_le`, `u16_be`, `u32_le`, `u32_be`, `bytes(n)`, `seek`, `tag4` |
| `ne` | bytes de um executável/DLL NE | `NeFile` com lista de `Resource { kind: ResourceKind, id: u16, data: &[u8] }`; `ResourceKind::Numeric(u16)` ou `Named(String)` |
| `palette` | recurso `LOGPALETTE` | `Vec<Rgb>` |
| `glob` | bytes de um `.PIC` | `GlobArchive` com `entries()` → `(id, &[u8])` |
| `pkbm` | dados de uma entrada `GLOB` | `Bitmap { width, height, depth, pixels: Vec<u8> /* índices */, mask: Option<Vec<u8>> }` |
| `adpcm` | um recurso `SND` (RIFF/WAVE) | `Pcm { sample_rate, channels, samples: Vec<i16> }`; aceita também PCM não comprimido |
| `chnk` | bytes de um documento | `ChunkFile { creator: Tag, version: (u16, u16), chunks: Vec<Chunk> }`, `Chunk { tag, id, offset, len, children }` |
| `macres` | `STR#`, `STR`, `PAT#` | `Vec<String>` (Windows-1252 → UTF-8), `String`, padrões 8×8 |

Regras:

- Arquivos são lidos inteiros para memória (o maior tem ~4 MB); sem streaming.
- Toda falha retorna `FormatError { format, offset, kind }`. Nenhuma entrada
  malformada pode causar pânico, laço infinito ou alocação desproporcional
  (tamanhos declarados são confrontados com o tamanho real antes de alocar).
- O sentido dos bytes é sempre explícito na chamada (`_le` / `_be`); não há
  leitor com sentido implícito.

### 5.2 Crate `tools` (binário `atelie-tools`)

Dependências: `formats`, `clap`, `png`, `serde`/`serde_json`.

- `atelie-tools info <arquivo>`: identifica o tipo (NE, GLOB, CHNK, RIFF) e imprime
  o inventário (recursos por tipo com contagem e tamanho; entradas GLOB;
  árvore CHNK).
- `atelie-tools extract <pasta MSKIDS> <pasta de saída>`: extrai tudo:
  - `ARTIST/bitmaps/<id>.png`, `WRITER/bitmaps/<id>.png`
  - `ARTIST/sounds/<id>.wav`, `WRITER/sounds/<id>.wav`
  - `<EXE>/resources/<TIPO>/<id>.bin` e `<id>.hex.txt`; `STR#`/`STR` também
    como `.txt`; ícones e cursores como `.png`
  - `documents/<arquivo>.json` (árvore CHNK) e `documents/<arquivo>/<tag>-<id>.bin`
  - `index.html`: folha de contato com todos os bitmaps (id, tamanho,
    profundidade) e lista de sons com player HTML
  - `report.json`: contagens e itens que falharam, com o erro
- Um item que falha não interrompe a extração: é registrado em
  `report.json` e na saída de erro, e o processo termina com código ≠ 0 se
  houver falhas.

## 6. Incógnitas a investigar

Cada uma vira seção em `docs/formats/` quando resolvida; nada é presumido.

1. Compressão dos dados `PKBM` (suspeita: PackBits, herança do Mac) e o
   significado de todos os campos do cabeçalho.
2. Existência e formato de máscara de transparência nos bitmaps.
3. Qual paleta (16 ou 256 cores) se aplica a cada bitmap; se há paletas
   embutidas no `.PIC`.
4. Layout do índice `CHNK` e relação pai/filho entre blocos.
5. Papel de `SNDRANGE`.

Se a decodificação de algum `PKBM` não puder ser resolvida pela inspeção dos
dados, o recurso é desmontar a rotina correspondente do executável no Ghidra.

## 7. Testes e critérios de sucesso

Testes:

- Unitários com dados sintéticos para cada módulo, inclusive entradas
  truncadas e tamanhos declarados absurdos (devem dar `FormatError`).
- Integração com os arquivos reais, só quando a variável `ATELIE_DATA` aponta
  para uma pasta `MSKIDS`; sem ela, os testes são pulados com aviso.
  Conferem contagens conhecidas (2.469 e 2.312 entradas GLOB; 131 sons no
  Fine Artist; tipos e quantidades de recursos listados na seção 3).

Critérios de sucesso do subprojeto:

1. `atelie-tools info` processa os dois executáveis, os dois `.SND`, os dois `.PIC`,
   `MSKIDS.INF` e todos os documentos de exemplo sem erro.
2. `atelie-tools extract` gera PNG de 100% das entradas dos dois `.PIC`, e a folha de
   contato, conferida visualmente, mostra imagens corretas (sem ruído,
   cores coerentes). Conferência por amostragem com o programa original
   rodando no Wine.
3. Todos os sons são extraídos como WAV tocáveis, com duração coerente com o
   cabeçalho do original.
4. O índice `CHNK` de todos os documentos de exemplo é lido com offsets e
   tamanhos consistentes com o tamanho do arquivo.
5. `docs/formats/` descreve GLOB, PKBM, SND, CHNK e os recursos decodificados
   em nível suficiente para outra pessoa reimplementar o leitor.
6. `cargo test`, `cargo clippy -- -D warnings` e `cargo fmt --check` passam.

## 8. Política de dados e licença

- Código sob GPL-3.0-or-later.
- Arquivos originais da Microsoft nunca entram no repositório, nem em testes,
  nem como amostras. Saídas de `atelie-tools extract` também não.
- Nomes e marcas da Microsoft (Fine Artist, Creative Writer, McZee,
  Imaginopolis) não aparecem no nome do projeto nem do binário; aparecem na
  documentação apenas para identificar com quais dados o programa é compatível.
