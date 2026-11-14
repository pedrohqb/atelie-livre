# Maçã (Coloca-figuras) — tabela palavra → figura no `WRITER.PIC`

A ferramenta **Coloca-figuras** (a maçã da faixa
dos Transformadores, `0865` em (340,61), balão `0fa8[6]`) põe uma figurinha
**dentro da linha, logo depois de cada palavra que ela conhece**. A lista de
palavras e as figuras ficam no `WRITER.PIC`, numa região que o índice `GLOB`
não descreve: entre o fim do índice (0x3642) e a primeira imagem (0xAC82).

Leitor do port: `formats::maca` (`Maca::read(&writer_pic)`, `lookup`,
`lookup_all`, `color_id`, `mono_id`; teste com o arquivo real em
`crates/formats/tests/real_maca.rs`). A ferramenta fica em
`crates/engine/src/scenes/writer/transformers/` e a varredura do documento em
`crates/engine/src/writer/rebus_apply.rs`. Os exemplos abaixo são do leitor
de referência em Python usado na pesquisa (`maca.py`).

```
$ maca.py WRITER.PIC info
região em 0x3642; N=1866 baldes/registros; 1797 palavras distintas;
253 figuras usadas (de 1..255; sem palavras: [126, 193])
baldes vazios: 689; maior balde: 6
$ maca.py WRITER.PIC lookup gato sono pique-nique abelha
gato -> n=0x25 (cor 0x0049, 1 bit 0x004a), n=0xc6 (cor 0x018b, 1 bit 0x018c)
sono -> n=0x0c (cor 0x0017, …), n=0x61 (…), n=0x7b (…)
pique-nique -> n=0x99 (cor 0x0131, 1 bit 0x0132)
abelha -> -
```

## 1. Onde fica

`R` = fim do índice `GLOB` = `18 + 6 × entradas` (= 0x3642 no `WRITER.PIC`
PT-BR, 2.312 entradas). A região vai até o menor offset de imagem do índice
(0xAC82, a imagem de id 0). O `ARTIST.PIC` não tem tabela da Maçã: o espaço
entre o seu índice (0x39f0) e a primeira imagem (0x3e28) é só um `PKBM` 4 bits
112×37 sem entrada no índice (sobra de montagem; o leitor `GLOB` o ignora).

## 2. Layout (little-endian, como o índice)

```
R+0          u16  N                     1866: número de baldes E de registros
R+2          N × { u16 off, u16 cnt }   tabela de hash (baldes)
R+2+4N       N × { u16 fig, u16 str }   registros, na ordem dos baldes
R+2+8N       N × palavra\0              palavras em Windows-1252/latin1, minúsculas
...          zeros até R+… = 0xAC82     (11 bytes de enchimento)
```

- **Balde** `i` (0 ≤ i < N): `cnt` registros a partir do deslocamento `off`
  contado de `R`. Os registros dos baldes são contíguos e na mesma ordem dos
  baldes (a soma dos `cnt` é N). Exceção: o primeiro balde não vazio (nº 1)
  grava `off = 0` em vez de 0x1d2a — por isso o leitor percorre os registros
  **pelas contagens**, não pelos `off`.
- **Registro**: `fig` é sempre par (2..510); `n = fig / 2` é o número da
  figura (1..255). `str` é o deslocamento da palavra a partir de `R`.
- **Palavras**: 1.866 strings, todas referenciadas exatamente uma vez; ficam
  agrupadas por figura na ordem do arquivo-fonte (aeronave, aeronaves, jato,
  jatos… → n=1; relógio, relógios, alarme… → n=2; …; escritor, escritora no
  fim). Caracteres usados: `a–z`, `-` e `á â ã ç é ê í ó ô õ ú ü`. Não há
  maiúsculas nem espaços. Há 66 palavras com mais de um registro (ex.: gato →
  0x25 e 0xc6; sono → 0x0c, 0x61, 0x7b; troféu → 0xf3 duas vezes).
- Dentro de cada balde os registros estão em ordem crescente de `str` (ordem
  do arquivo-fonte).

## 3. Hash

```
h = 0
para cada byte c da palavra (latin1):  h = rol16(h, 5) + c   (mod 2^16)
balde = h mod N            (rol16(h,5) = ((h << 5) | (h >> 11)) & 0xFFFF)
```

Conferido nas 1.866 entradas (todas caem no seu balde). A palavra é hasheada
como está na tabela (minúsculas, com acentos).

## 4. Figura n → imagens

| | id no `WRITER.PIC` | formato |
|---|---|---|
| colorida | `2n − 1` (**exceção: n = 1 → id 0**, pois o id 1 não existe) | PKBM 4 bits, ~30×28 (29..32 × 27..34) |
| versão 1 bit | `2n` | PKBM 1 bit, mesmo tamanho: a mesma figura em preto e branco (não é silhueta/máscara) |

Os ids 0x0000–0x01fe são todos figuras-rébus; 0x01ff/0x0200/0x0201 (32×34,
um quadro emoldurado) estão no mesmo bloco mas nenhum registro aponta para eles.
O mesmo `n` é o **código do caractere** guardado no `TEXT` do `.MAX` (ver
`max.md`): "gato" + figura vira `gato%` (0x25), bicicleta `\x0f`, parque
`\xeb` — conferido em `work/max/GATO_TES.MAX`. É também o mesmo número das
figuras-rébus da Máquina de Idéias (`SMDG`, ideas.md §8).

**Correção para o port**: `crates/engine/src/writer/ideas/rebus.rs::image_ids`
devolve `2c−1` também para c = 1 (id 1 não existe; deve ser 0). No original a
palavra "aeronave" recebeu o avião (id 0) — captura `w26_08_maca_sel.png`.

## 5. Comportamento medido no original (Wine, `w26_01..08`)

Documento de teste: "gato Gato GATO aeronave sono gatos Bicicleta, parque. cama
bzzz" / "pique-nique tic-tac abelha maç café papai".

| caso | resultado |
|---|---|
| clique na maçã (som `0233`, 13356 B) | percorre **o documento inteiro** (com ou sem seleção) e insere a figura depois de cada palavra conhecida |
| maiúsculas | "Gato" e "GATO" recebem o gato: a palavra é comparada em minúsculas |
| palavra com várias figuras | usa a **primeira** do balde (gato → 0x25, sono → 0x0c) |
| plural | só se estiver na lista ("gatos" está; não há radicalização) |
| hífen | faz parte da palavra ("pique-nique", "tic-tac" acharam) |
| acentos | "café" achou (é); palavra incompleta ("maç") não acha |
| pontuação | a figura entra **entre a palavra e a pontuação**: `Bicicleta[🚲], parque[🏞].` |
| palavra já com figura | não recebe outra; se nada novo foi posto: "Epa!" `0805[4]` idsAllMatched ("Já coloquei figuras junto com as palavras…", som `02fe`) |
| nenhuma palavra conhecida | `0805[3]` idsNoMatches (pela lista de balões; não provocado) |
| **Ctrl+clique** na maçã | tira **todas** as figuras-rébus do documento (som `0233`); `0fa8[12]` "Quer jogar fora depois? Segure Ctrl e clique em [maçã]" |
| desfazer | pelo Ovo (balão `0fa8[6]` → `[21b:c]` Desfazer) |
| erros | `0805[5]` idsRebusOOM, `0805[6]` idsRebusFile |

### Desenho na linha

- A figura é um **caractere** do texto (1 byte = `n`, com o sprm de caractere
  `0x03 = 1` no `FDPC`); move-se, apaga-se e seleciona-se como uma letra
  (selecionada, sai em vídeo inverso junto com o texto).
- A base da figura fica na **linha de base** do texto (gato 27 px: y 184..210
  com a base do texto em 210; bicicleta 28 px: 219..246, base 246).
- Avanço horizontal ≈ **4 + largura + 4** px (4 px de folga de cada lado: "gato"
  termina em x 113, o gato vai de 118 a 147; a vírgula depois da bicicleta
  começa 4 px depois da figura).
- A linha cresce para caber a figura (passo de linha 15 px vira 36 px na linha
  com o avião de 31 px).
- Na tela a figura é opaca (fundo branco do bitmap). O port já resolve fundos
  não brancos com a máscara calculada de `rebus.rs::masked`.

## 6. No port

- `formats::maca::Maca::read` lê a tabela (verifica as contagens, a paridade
  das figuras e o balde de cada palavra); `lookup` compara em minúsculas e
  devolve a primeira figura; caractere fora do latin1 não tem figura.
- `writer::rebus_apply::place` percorre o documento inteiro: palavra =
  sequência máxima de letras (inclusive as acentuadas) e `-`; a figura entra
  logo depois da palavra (antes da pontuação), se ali ainda não houver uma.
  `remove` tira todas (Ctrl+clique). Cada clique é uma ação do Ovo.
- Sem novidade: "Epa!" `0805[4]` (som `02fe`); nenhuma palavra conhecida
  no documento: `0805[3]`.
