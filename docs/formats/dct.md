# `SPELL.DCT` e `THES.DCT` — dicionários do Creative Writer

Dicionários licenciados que o Creative Writer usa no **Corrigir** (abelha),
no **Trocar** ("Palavras parecidas" = sinônimos) e no **Embaralhar**
("Palavras embaralhadas" = anagramas). O ortográfico é da **INSO
Corporation** (International CorrectSpell, base `SCREWR2.ICS`, "PORTUGUESE",
21/07/1995); o de sinônimos, da **Houghton Mifflin** ("Brazilian Portuguese
ET", "Portuguese Thesaurus", compilação `r144p500`, 25/07/1995). O código
que os lê está no `WRITER.EXE`.

**Estado: decodificados por completo.** Leitor: `crates/formats/src/dct.rs`
(`formats::dct`); API do motor: `crates/engine/src/writer/dict.rs`;
ferramenta: `atelie-tools dct`.

**Licença.** Os dois arquivos trazem aviso de segredo comercial: o port lê a
cópia instalada do usuário e nunca redistribui o conteúdo. Sem ela, o motor
usa dicionários livres com a mesma API (fim deste documento).

Inteiros dos cabeçalhos: **big-endian**. Tabelas da gramática: little-endian.

## Gramática de pares (os dois arquivos)

O texto é um fluxo de **códigos de N bits, do bit mais alto para o mais
baixo**. Cada código é um *terminal* (uma cadeia em claro) ou um *par* de dois
códigos (gramática do tipo Re-Pair), expandido recursivamente, o esquerdo
primeiro. Há duas variantes de tabela:

- **empacotada** (`SPELL.DCT` e índice do `THES.DCT`; decodificador em
  `WRITER.EXE` seg. 146:0x1214): `TAB` com 2 bytes por código e `LEN` com 1.
  Terminal `c` (`c ≤ T`): `LEN[c]` bytes (o byte inteiro, até 255) das
  cadeias a partir de `TAB[c]`. Par: com `lo, hi` os bytes de `TAB[c]`,
  esquerdo = `hi << 4 | lo >> 4`, direito = `(lo & 0x0F) << 8 | LEN[c]`;
- **larga** (arquivo de entradas do `THES.DCT`): `A` e `B`, 2 bytes por código.
  Terminal: `B[c]` bytes a partir de `A[c]`; par: esquerdo `A[c]`, direito `B[c]`.

Nos blocos, o código 0 encerra o bloco.

## `SPELL.DCT` (154 640 bytes)

Cabeçalho (lido em `WRITER.EXE` seg. 164:0x0000 e 0x0C0E):

| Offset | Conteúdo |
|---|---|
| 0x000 | aviso em texto (`C`opyright…), terminado por 0x1A |
| 0x180 | `6` (formato) · 0x182 código de idioma (`12`) · 0x184 versão (`5`) |
| 0x188 | tamanho do bloco (`128`) · 0x18A blocos antes dos dados (`8`, = 0x400) |
| 0x18C / 0x194 / 0x19F / 0x1BF | data `950721`, versão, idioma `PORTUGUESE`, fonte `SCREWR2.ICS` |
| 0x1F6 | número de blocos (`881`) |
| 0x20C | **N**, largura dos códigos (`10`) |
| 0x1EA…0x20A | tamanhos das regiões abaixo |

Regiões depois dos 881 blocos (a partir de `128 × (8 + 881)` = 0x1BC80), na
ordem em que o motor as lê; somam exatamente o fim do arquivo:

| Campo | Offset | Bytes | Conteúdo |
|---|---|---|---|
| 0x1EC | 0x1BC80 | 6144 | tabela auxiliar (não usada para listar) |
| 0x206 | 0x1D480 | 11947 | **cadeias** dos terminais |
| 0x208 | 0x2032B | 2048 | **TAB** (1024 × 2); `TAB[0]` em BE = nº de pares |
| 0x20A | 0x20B2B | 1024 | **LEN** |
| 0x1EA | 0x20F2B | 3524 | índice: 4 primeiras letras de cada bloco |
| 0x200 | 0x21CEF | 13520 | modelo de n-gramas de letras (ordena sugestões) |
| 0x1EE | 0x251BF | 1000 | tabela de hash |
| 0x1FE | 0x255A7 | 1368 | **equivalências fonéticas** |
| 0x204 | 0x25AFF | 273 | tabelas de classes |

**T** (último terminal) = `2^N − TAB[0]_BE − 1` = 1023: com N = 10 todos os
1024 códigos são terminais.

### Registros

Cada bloco expandido é uma lista de palavras em ordem alfabética, com prefixo
compartilhado:

```
[k | caixa<<6] [letras, bytes ≥ 4] (01 | 02 classe) [variantes]
```

- `k` (6 bits) = letras a tirar do fim da palavra anterior; a primeira do
  bloco tem `k = 0`. O bloco termina com um cabeçalho solto, sem letras.
- caixa: bit 7 = inicial maiúscula; bit 6 = as maiúsculas das letras valem
  (sem ele, a parte herdada da palavra anterior volta a minúsculas).
  Resultado: `Brasil`, `EUA`, `TV`, `a.C.`, `A.C.`.
- classe (1–4) segue a terminação: 1 `-mos`, 2 `-s/-r/-z`, 3 vogal,
  4 nasal (`-m`, `-ão`); `01` = sem classe (abreviaturas, nomes próprios,
  futuros e condicionais, `-mente`…). São as classes de ênclise do português,
  mas **o corretor não as usa** para aceitar palavras (veja abaixo).
- variantes: `0x40` = só Brasil (`ação`, `ônibus`, `idéia`), `0x80` = só
  Portugal (`acção`, `facto`, `ideia`), `0xC0` = as duas (uma palavra tem `0xE0`).
  O Writer escolhe a máscara pelo `WIN.INI` (seg. 143:0x0DCA): `[Intl]
  sLanguage=ptb` ou `sCountry=Brazil` → `0x40`, senão `0x80`.

Total: **215 667 formas** (211 462 válidas no Brasil, 209 220 em Portugal).
Todas as flexões são registros explícitos (`abafar`, `abafo`, `abafávamos`,
`abafadíssimo`…). Faltam alguns plurais irregulares (`irmãos`, `pães`,
`órgãos`, `alemães`); como o corretor só consulta a lista, devem aparecer
como desconhecidos no original (não conferido: o teclado do Wine não digitou
acentos).

### Equivalências fonéticas

8 bytes `!`, depois **169 pares** de registros de 4 bytes (texto à esquerda,
completado com espaços) e `FF × 8`: `al→au`, `am→ão`, `ch→x`, `x→s`, `ç→ss`,
`ge→je`, `ss→ç`, `ão→õe`… São as trocas que o corretor tenta primeiro.

### Comportamento do corretor (observado no original, Wine)

- O hífen separa palavras: em `vamo-nos` marca `vamo`; `faz-lo` → marca
  `lo`; `encontrei-o`, `abafam-no` passam.
- Caixa: palavra minúscula aceita também Capitalizada e TODA MAIÚSCULA;
  `Brasil` aceita `Brasil` e `BRASIL`, não `brasil` (1ª sugestão: `Brasil`).
- Sugestões observadas: `gatu` → gata, gato, tatu; `pexe` → pese, peixe, peie,
  pede, pele; `abaf` → aba, abafa, abafe, abafo, abas; `aco` → Co, ao, aço,
  Ac, acho; `vamo` → amo, vamos, vago, varo, vaso; `brasil` → Brasil, brisa,
  brisas; `tubares` → turbares; `pes` → pés, es, P.es, Es, pez.

O port gera as sugestões em camadas — mesma palavra com a caixa registrada;
uma troca fonética; uma edição (tirar, pôr, trocar ou inverter uma letra); a
duas edições se houver poucas — com empate pela ordem do dicionário. Reproduz
`gatu` e os conjuntos acima, mas **não a ordem exata** do original, que usa o
modelo de n-gramas (0x21CEF, seg. 146:0x02E3) para ranquear. A comparação palavra a
palavra está em `docs/telas/creative-writer-abelha.md` ("Ordem das
sugestões").

## `THES.DCT` (277 504 bytes)

Diretório do arquivo: `u16` em 0x316 (= 2) e `u32` em 0x318 (= 0x32A) →
registros de 12 bytes `(tipo, tipo, cabeçalho u32, início dos dados u32)`:

| Tipo | Nome | Cabeçalho | Dados |
|---|---|---|---|
| 1 | `Entry file` | 0x400 | 0x97B6 |
| 2 | `Main index list` | 0x1D800 | 0x242CB |

Cada seção tem no cabeçalho `+0x50` o número de subseções e `+0x52` (u32) o
deslocamento do seu diretório; registros de 16 bytes `(id u16, deslocamento
u32, tamanho do item u32, contagem u16, bytes u32)`, deslocamentos relativos
ao cabeçalho da seção (código: `WRITER.EXE` seg. 149, 0x0F00 e 0x12D4).

### Arquivo de entradas (tipo 1)

Gramática larga, **N = 12** (`+0x76`). Subseções: 10 = 997 `u32` (início de
cada página, em códigos), 20 = `A` (4096 × 2; `A[0]` em LE = 71 pares,
terminais 0…4024), 30 = `B`, 60 = cadeias (16 930 bytes). O fluxo, a partir
de 0x97B6, expande para as entradas, cada uma

```
FE FE FE FE <WD>título FF <PS>nn FF <SY>s1<D1>s2<D1>…</SY> FF
```

**3 983 entradas**, de `a` a `zumbido`; `<PS>` é a classe gramatical (01
substantivo, 11 verbo, 21/25 adjetivo, 31 advérbio, 41 preposição, 44
conjunção; aparecem também 05, 26, 45); o primeiro sinônimo é o próprio título.

### Índice de formas (tipo 2)

Gramática empacotada, N = 10 (`+0x94`), blocos de 512 bytes (`+0x8C`)
a partir de 0x242CB. Subseções: 200 cadeias (contagem = **T** = 1023), 210 TAB,
220 LEN, 230 = 4 primeiras letras de cada bloco (252 blocos), 240–300 tabelas
auxiliares. Registro:

```
[k | caixa<<6] letras 0A 00 80 [delta i24 BE] [PS] letras-do-lema [corte] 00
```

- forma = anterior − `k` letras + letras (bit 7 de `k` = inicial maiúscula);
- `delta = 0x0FFFFF`: forma flexionada (`corri`); o lema é a forma menos
  `corte` letras mais as letras do lema (`correr`);
- senão, **entrada = anterior + delta** (com sinal), com o acumulador zerado
  a cada forma flexionada; entrada 0 = só o lema; `PS` = o `<PS>` da entrada.

**114 581 formas**; todas as que têm entrada apontam uma que as contém.
Exemplos: `a fim de que` → entrada 2 (`a fim de`); `a despeito de` → 605
(`apesar de`); `a Virgem` → 3004 (`Madona`).

### Comportamento

- Sinônimos de uma palavra = as entradas da própria forma (ou, se não houver,
  as do lema), na ordem do índice, sem repetir e sem a palavra. `casa` →
  botoeira, domicílio, edifício, habitação, lar… (o original mostra
  `botoeira` primeiro).
- **Anagramas** saem do índice de formas, não do ortográfico: `ramo` → amor,
  mora, oram, ramo (`armo`, `roam`, que só existem no ortográfico, não aparecem).

## Alternativa livre

`engine::writer::dict::Dictionary` tem a mesma API (`words`, `contains`,
`suggestions`, `synonyms`, `anagrams`) sobre:

- **Hunspell pt-BR** — VERO 3.2 (Raimundo Moura, LGPLv3/MPL), lido pelo
  crate `spellbook` (MPL-2.0); pacote Debian/Ubuntu `hunspell-pt-br`,
  `/usr/share/hunspell/pt_BR.{aff,dic}`;
- **tesauro pt-BR do LibreOffice** (MyThes, MPL-2.0), pacote `mythes-pt-br`,
  `/usr/share/mythes/th_pt_BR.{dat,idx}`.

Os arquivos são lidos do sistema ou da pasta `ATELIE_DICT` (mesmos nomes);
nada é embutido no repositório. Diferenças: `words()` lista só os radicais
do `.dic` (as flexões saem das regras de afixos; o VERO expandido passa de
80 milhões de formas); os anagramas são permutações (até 8 letras) aceitas
pelo Hunspell; o VERO aceita algumas formas que o original recusa (`brasil`,
`pexe`) e reconhece as que faltam no `.DCT` (`irmãos`, `pães`, `abafá-lo`).
`Dictionary::load` usa o original se houver e o livre se não.

## Ferramenta

```
atelie-tools dct info                      # cabeçalhos e contagens
atelie-tools dct palavras [--variante pt]  # lista completa
atelie-tools dct verificar casa brasil     # OK / ERRO + sugestões
atelie-tools dct sugerir pexe
atelie-tools dct sinonimos casa
atelie-tools dct anagramas ramo
atelie-tools dct entradas                  # entradas do thesaurus
atelie-tools dct --livre sinonimos casa    # dicionários livres
```

(`--data` ou `ATELIE_DATA` indica a pasta `MSKIDS`.)
