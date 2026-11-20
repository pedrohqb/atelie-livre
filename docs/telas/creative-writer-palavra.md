# Creative Writer: ferramenta Palavra

Pesquisa: `cw/report/office-palavra.md` e `office-complemento.md` §2–3.
Código: `crates/engine/src/scenes/writer/{word_strip,word_lists,effects_draw}.rs`
(cena) e `crates/engine/src/writer/{brush,mix,effects,new_style}.rs`
(lógica, testada sem tela).

## Faixa

Duas páginas, trocadas pelo interruptor "Mais escolhas" (`0229`/`022b` em
(508,59), som `021d`; nas capturas a figura é a mesma nas duas páginas).

| página | botões (balão `0fa0[n]`) |
|---|---|
| 1 | Seleciona-letras `[3]`, Dimensiona-palavras `[4]`, Seleciona-cores `[5]`, Seleciona-estilos `[7]`, Posicionador `[8]`, Aplicador `[9]`, Ponto de inserção `[10]`, FORMA `[11]` (W7, ainda "não disponível") |
| 2 | Iluminador `[12]`, Efeitos especiais `[13]`, Mix de estilos `[14]`, Estilos de parágrafo `[15]`, Espaçamento `[16]`, Recuos `[17]`, Aplicador, Ponto de inserção |

Ids e posições: tabela "Faixas" de `creative-writer.md`.

## Listas

Abrem no clique do botão (som `0233` + balão), fecham ao escolher (som
`022d`), com Esc ou com um clique fora. Geometria medida nas capturas: borda
preta de 1 px a partir de y 94, separadores de 1 px, ✓ `029d` (14×12) a
3 px da borda no item atual.

| lista | x (bordas) | itens (linhas horizontais) | rótulos |
|---|---|---|---|
| Letras | 24..175 | 5 visíveis, passo 21, setas ▲ `02db` (62,201) ▼ `02df` (115,201) até y 214 | nome na própria fonte |
| Tamanhos | 89..240 | 94, 159, 208, 245, 275, 307, 333, 357 | `0c1d`..`0c29` (de 2 em 2), no meio à direita do ✓ |
| Cores / Iluminador | 154..256 / 24..126 | 16, passo 21, até 430 | faixa de 14 px com moldura preta; sem ✓ (no Iluminador o branco tira a cor) |
| Estilos | 219..310 | 4, passo 21 | texto no próprio estilo |
| Posicionador | 284..411 | 4, passo 49 | `0bc9`..`0bcf` |
| Efeitos | 89..197 | 7, passo 21 | "Nada de especial", Pisca-pisca, Formiguinhas, Brilho, Treme-treme, Vapt, Vupt |
| Mix | 154..262 | 7, passo 21 | texto, sem ✓ (não é um estado) |
| Estilos de parágrafo | 219..359 | 4, passo 51 | "Nada de especial", `0bd9`, `0bdb`, `0bdd` |
| Espaçamento | 215..402 | 4, passo 52 | "Nada de especial", `0bdf`, `0be1`, `0be3` |
| Recuos | 324..458 | 4, passo 49 | `0bd1`..`0bd7` |

O ✓ marca o item do pincel carregado, se for daquela lista; senão o estado
do texto no ponto de inserção. Os textos usam a Pablo Graphos.

## Pincel

Escolher um item carrega o pincel (cursor `0004` sobre a página; o ponto de
inserção some). Nada muda só por escolher.

- Apertar o pincel na página: som `021c` (`0285` no Dimensiona-palavras);
  soltar: `021e`. A passada inteira é uma ação do Ovo (nenhuma, se nada
  mudou).
- A parte de **caractere** vale para as **palavras inteiras** tocadas pelo
  arrasto (`Document::expand_to_words`); **um clique não basta**.
- A parte de **parágrafo** (Posicionador, estilos de parágrafo,
  espaçamento, recuos e a dos Mix) vale para os parágrafos tocados; basta
  clicar.
- Durante o arrasto as posições vêm da diagramação de antes da passada
  (as palavras que crescem não fogem do pincel), e cada movimento
  reaplica o pincel sobre o documento de antes (voltar o mouse desfaz).
- **Aplicador** (`0220`): passa o pincel no documento inteiro, inclusive
  nos `\r`. Sem pincel, só o balão `[9]`. Com **Ctrl**, o pincel vai para o
  estilo dos documentos novos (`estilo-writer.txt`, ver
  `docs/formats/arquivos-do-port.md`) e o documento atual não muda (balão
  `[2]`).
- **Ponto de inserção** (`0224`, balão `[10]`): larga o pincel.
- Trocar de ferramenta também larga o pincel.

## Mix de estilos

Valores gravados pelo original (`office-complemento.md` §3):

| Mix | caractere (palavras tocadas) | parágrafo |
|---|---|---|
| Maluco | Brilho (`0x0b`=5), 36 pt, violeta #932ca3 — fixo | — |
| Anotação | sublinhado, vermelho #fa0039, iluminador #ffee4c | — |
| Título | negrito, 24 pt | centro |
| Era uma vez | — | capitular de 3 linhas, espaço depois |
| Poesia | — | esquerda e direita, recuo de todas as linhas (36 pt dos dois lados) |
| Relatório | — | recuo da 1ª linha, espaço depois |
| Super-simples | limpa todos os parágrafos tocados (caracteres, `\r` e parágrafo) | |

Não existe efeito especial "Maluco" (o antigo `Effect::Crazy = 7` foi
removido).

## Capitular

A letra é ampliada até as maiúsculas irem do alto das maiúsculas da 1ª
linha à base da 3ª (`Measure::cap_height`; ~32×40 px na fonte padrão). As
linhas ao lado começam depois da letra, e os parágrafos seguintes também
enquanto a capitular não acaba. A letra fica no começo do primeiro trecho
livre na altura dela: com um adesivo ou desenho encostado na margem, ao
lado dele (antes ficava por baixo do objeto, e só o texto fugia dele).

## Efeitos especiais animados

Relógio único (`Editor::effects_ms`), ciclo de 2,7 s: 1,1 s normal e 8
passos de 0,2 s. Cada trecho seguido com o mesmo efeito numa linha é um
retângulo (do 1º ao último caractere, sem os espaços das pontas; altura da
caixa da linha). Só a vista da página anima; o Visor e a impressão mostram
o estado normal.

| efeito | passos 1..8 |
|---|---|
| Pisca-pisca | vídeo inverso |
| Formiguinhas | moldura de 1 px tracejada (4 acesos, 4 apagados) que anda 1 px por passo, em coordenadas da tela |
| Brilho | 2 a 4 faíscas 3×3 sorteadas a cada passo (7 formas, 6 cores puras) |
| Treme-treme | passos ímpares: linhas de pixels ímpares +1 px, pares −1 px |
| Vapt | apaga `2k` linhas de pixels de cima (tudo no passo 8) |
| Vupt | apaga `⌈w·k/8⌉` colunas da esquerda |
