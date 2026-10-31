# `WHAT` — balões de ajuda e assistentes

Recurso numerado dos executáveis (`ARTIST.EXE`: 34 blocos, 200 KB;
`WRITER.EXE` tem o equivalente). Cada bloco guarda os balões de uma área do
programa: o texto que o McZee, a Maggie, o Max e o Rex dizem, com as opções
de resposta, as dicas ligadas e as palavras do glossário. É daqui que o
Ateliê Livre tira os textos dos balões quando o original está instalado
(`Assets::dialogs(program, bloco)`), em vez de copiá-los para o código.

Leitor: `crates/formats/src/what.rs`. Inspeção: `atelie-tools what
ARTIST.EXE --block 3a00`.

## Blocos do `ARTIST.EXE`

| bloco | conteúdo |
|-------|----------|
| `0201`–`0213` | ajuda geral e da Bolsa Mágica |
| `1001`–`1010` | ajuda das ferramentas da tela de pintura (`1007` = Ovo desfazer; `100d` = salas: saguão 1, galeria 6, Ateliê 26, Oficina 31, Truques 42, placas 32–37, cano de bombeiro 67–72) |
| `1388`, `1389` | cadastro |
| `3800`, `3803` | Truques básicos (lição do porco Chiquinho) e saída dos assistentes |
| `3a00` | assistente do Desenho Animado (86 balões) |
| `3c00` | assistente do Botão/Adesivo |
| `3e00` | assistente da História em Quadrinhos (169) |
| `4000` | Truques de 3-D (perspectiva, 250 balões) |
| `4200`–`4c00` | galeria, impressão, Imaginópolis |

## Formato binário

Cabeçalho (little-endian): `u16` número de registros `n`, `u16` 1, dois
`u32` zero, depois `n + 1` deslocamentos `u32` a partir do início do bloco
(o último marca o fim).

Registro: `08 '1' 09 <título> 09` e uma sequência de códigos de controle,
cada um seguido do argumento em texto até um espaço, e texto Windows-1252:

| código | argumento | significado |
|--------|-----------|-------------|
| `00` | hex | largura do balão em pixels |
| `01` | hex | altura |
| `02` | hex | estilo (2 nos balões dos personagens) |
| `03` | hex | cor do texto (`F` preto, `9` roxo dos títulos) |
| `04` | `1`/`0` | assinatura ("Maggie") liga/desliga |
| `05` | 8 hex | figura: `id << 16 | parâmetro` |
| `06` | número | recuo (0, 1, 2) |
| `07` | `1`/`0` | nome de tecla liga/desliga |
| `0b` | — | negrito liga/desliga |
| `0c` | — | separa nomes de tecla equivalentes (`Delete` / `Backspace`) |
| `0d` | — | quebra de linha |
| `0e` | — | marca em volta das figuras (ignorada) |
| `0f` | 1 byte | espaçamento da figura (ignorado) |

Figuras com significado próprio: `22c` opção de resposta (parâmetro =
índice da resposta, que não é necessariamente a ordem na tela), `22e` opção
com figura, `21b` dica (parâmetro = registro de destino no mesmo bloco; o
bit 15 aparece ligado em alguns), `21d` dica do Max, `222` "Precisa de
ajuda?", `223` pasta, `229`/`230` Rex, `ffff` palavra do glossário
(parâmetro = registro cujo título é a palavra e cujo texto é a definição).
Ids `1000`–`10ff` são os ícones das ferramentas desenhados no meio do texto.
O `@` no texto é o nome do usuário cadastrado.
