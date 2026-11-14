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
| `00` | hex | largura do balão em pixels (a caixa tem 2 a mais; os avisos, 1) |
| `01` | hex | altura (não é a do balão desenhado; o original calcula pelo texto) |
| `02` | hex | estilo (2 nos balões dos personagens) |
| `03` | hex | cor do texto: numeração do Mac, ver abaixo |
| `04` | `1`/`0` | centraliza o parágrafo (assinaturas "Rex", "Maggie"; cabeçalho "EPA!") |
| `05` | 8 hex | figura: `id << 16 \| parâmetro` |
| `06` | número | recuo: 0, 1 (parágrafo todo a 18 px) ou 2 (pendente: o primeiro token na margem, o resto a 18 px) |
| `07` | `1`/`0` | abre/fecha alternativas de plataforma |
| `0b` | — | negrito liga/desliga |
| `0c` | — | troca entre o trecho do Mac e o do Windows |
| `0d` | — | fim de parágrafo |
| `0e` | — | marca em volta das figuras (ignorada) |
| `0f` | 1 byte | ignorado (1 nas ligações, 2 nas figuras, 3 nas opções, 5 nas bolinhas verdes, 0 nas caixas de texto) |
| `11` | — | símbolo ⌘ do Mac (só no trecho do Mac) |

**Alternativas de plataforma.** Entre `07 1` e `07 0` o texto alterna a cada
`0c` entre o do Mac (primeiro) e o do Windows:
`segure <07>1 ⌘ e aperte Q <0c>Alt e aperte F4<0c><07>0`. Um `0c` antes de
`07 1` marca um trecho só do Windows (a nota das fontes TrueType). O leitor
guarda só o texto do Windows; os códigos de estado (negrito, cor, recuo)
dentro do trecho do Mac continuam valendo. O item `Format` do leitor
registra as mudanças de recuo e de centralização, que valem também para as
figuras seguintes.

**Cores.** O `03` usa a numeração da paleta do Mac (0 branco, `F` preto), que
cai na paleta de 16 cores do Windows (`LOGPALETTE` 4) uma posição adiante:
`9` roxo (147, 44, 163) dos títulos, `7` azul do McZee, `4` rosa da Maggie,
`8` violeta, `B` verde. O `@` (nome do usuário) costuma vir em negrito
roxo.

## Figuras

O id da figura não é um id do `ARTIST.PIC`: é o índice de uma tabela do
segmento de dados do `ARTIST.EXE` — `0x200 + k` e `0x1000 + k`, uma entrada
de três palavras `a, b, c` por figura. A imagem é a do maior id do `.PIC` que
não passa de `a − 1` (o `.PIC` guarda grupos colorida/branca/máscara e o `a`
é o da branca; o pincel, `a = 0x1000`, cai na `0x495`); a máscara é `a + 1`
quando `b ≠ 0` e ela tem o mesmo tamanho; sem máscara, o branco é
transparente, e com `b = 0` (figura solta) a figura é opaca. `a = 0` remete à
tabela dos botões da barra (16 bytes por entrada, índice `b`). O `c` é o
som do clique: as figuras com `c = ffff` (ícones das ferramentas, bichos não
escolhidos) saem "apagadas" — um pixel sim, outro não (os de x + y par a
partir do canto da figura) vira cinza (100, 100, 100) dentro da figura. O mapa
das 157 figuras usadas está em `crates/engine/src/help_icons.rs`.

Figuras com significado próprio: `0` caixa de texto (parâmetro = largura;
16 px de altura), `22c` opção de resposta (bolinha preta `0x3d3` numa vaga de
16×15; parâmetro = índice da resposta, que não é necessariamente a ordem na
tela), `22e` opção com figura (bolinha verde `0x409`, marcada `0x40d`),
`21b` dica (gema `0x3f7`; parâmetro = registro de destino no mesmo bloco, com
o bit 15 às vezes ligado; a que aponta para o próprio registro ou para o 0 é
só enfeite do título), `21c` tecla de atalho (chave `0x3fb`), `21d` dica do
Max, `21e` dica da Maggie (rosto `0x405`), `222` "Precisa de ajuda?" (rosto
do McZee `0x41b`), `223` pasta, `230` cabeçalho "EPA!" do Rex (`0x2a9`),
`237`–`242` os bichos do cadastro, `ffff` palavra do glossário (parâmetro =
registro cujo título é a palavra e cujo texto é a definição; desenhada numa
caixa tracejada de traços de 3 px, 2 px antes e 4 depois do texto). Ids
`1000`–`10b2` são os ícones das ferramentas, das formas e dos pincéis.

## Como o original desenha

Medido nas capturas do Fine Artist no Wine:

- fonte "Pablo Graphos" de 8 pontos; negrito por sobreimpressão deslocada
  1 px, cada caractere 1 px mais largo;
- texto a 5 px da borda de fora (2 de borda + 3), quebra na largura do
  registro menos 8; o espaço não separável (`a0`) também quebra;
- linha de texto de 14 px; uma figura dá à linha a própria altura e fica
  centralizada nela; 6 px entre parágrafos; 4 px sob a última linha;
- no recuo 2, o espaço depois do primeiro token anda e, se não chegou aos
  18 px, pula até eles ("• Clique", "1 Clique"; a bolinha de 16 + espaço
  passa);
- caixa com o fundo pontilhado do padrão 40 do `PAT#` (invertido: amarelo
  onde o bit está ligado) alinhado à tela, borda preta de 2 px, cantos
  arredondados (raio ≈ 5) nos balões de fala e retos nas dicas e nos avisos
  "EPA!" (que também são amarelos);
- rabicho curvo de ~12 px de base até a boca de quem fala;
- botões Volta / Para / Segue (31×14) em três vagas fixas com 10 px entre
  elas, centralizadas, 15 px abaixo do texto; um botão que falta deixa a vaga
  vazia;
- os balões de ajuda da pintura encostam à direita em x = 617, topo em 117.

O exemplo `crates/engine/examples/balloon_shot.rs` desenha um balão do port
sobre uma captura do original para comparar pixel a pixel.
