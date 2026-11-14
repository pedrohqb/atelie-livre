# Biblioteca do Creative Writer (T9)

A Galeria do Fine Artist com os dados do Writer: `GalleryRoom` com o
`writer_library::WRITER_LIBRARY` (`GalleryRoom::for_program(Program::Writer, …)`,
`--program writer --scene library`). Pesquisa do original:
`cw/report/building-salas.md` §6; medidas novas abaixo vêm de capturas do
`WRITER.EXE` no Wine (comparações em `cw/shots/library_compare_*.png`).

## Tela

| objeto | figura (cor; máscara = 2 adiante) | área / posição | som | o que faz |
|---|---|---|---|---|
| fundo | `6807` | 640×480 | — | — |
| parede com as 4 molduras | `692b` | (142,0) 274×271 | — | — |
| molduras (`FILR` 1–4) | — | (159,9), (296,9), (296,143), (159,143), 102×124 | `21c` | página do documento no quadrado (1,1) 100×100 em cinza `8c8c8c`, com um traço preto de cada lado; plaqueta (3,108) 96×13 com o título |
| armário | `68e7` | (420,170) | — | só desenho |
| impressora | `6913` | (569,102) 60×74 | `212` | dica `[10]` |
| lixeira | `690f` | (501,351) 92×108 | `2f8` | dica `[11]` |
| seta "Saguão" | `68eb` | (547,217) 91×46 | `2fb` | saguão |
| luminária "Tchau" | `6917` | (496,5) 56×132 | `2f7` + `2fe` | "EPA!" `0805[12]` em (208,179) |
| buraco do rato | `691f` | (552,273) 54×47 | `2ea` | varinha: surpresas `0870`–`0876` |
| interruptor | `6923` desligado (496,151) / `6927` ligado (496,177) | (496,151) 36×102 | `308` | abre/fecha a gaveta `693b` (275,169) e mostra `[25]` |
| bolsa | (137,364)/(137,386); maleta (366,333), livro (281,336), varinha (336,392) | | | como nas salas |

Não há fala de chegada (no original só toca o `304`/`2fb` do saguão).

## Balões (`WHAT 0bb8` do `WRITER.EXE`)

Posições da caixa (sem o rabicho), pela diferença de capturas:

- Chegada `[6]` e "Outra coisa..." `[7]` em (129,234). Opções de `[6]`:
  `[8]` (330,123), `[9]` (250,163), `[11]` (402,211), `[12]` (333,179),
  `[7]`. Opções de `[7]`: Imprimir `[10]` (458,200), Copiar para um
  disquete `[14]` (268,80), Examinar outras unidades `[13]` (288,111),
  Voltar `[6]`. As ligações das dicas (lâmpada) abrem `[20]`, `[21]`,
  `[15]`.
- Clique na impressora, na lixeira e no interruptor: `[10]`, `[11]`,
  `[25]` em (129,233).
- Clique simples numa moldura: `[41]` "Se você quiser trabalhar neste
  documento…" num lugar fixo por moldura: (286,29), (48,29), (48,150),
  (286,150) (as de cima à direita e embaixo à direita ficam à esquerda).
- Menu do McZee `[29]`/`[30]`; varinha `[58]`.

## Avisos do Rex (`WHAT 0805`)

| situação | registro | posição |
|---|---|---|
| documento da criança na lixeira, vindo da parede (3 opções) / da pasta (2) | `[22]` / `[23]` | (192,156) |
| exemplo na lixeira, da parede (troca por outro) / da pasta | `[34]` / `[35]` | (192,156) |
| plaqueta de um exemplo | `[30]` | (202,126) |
| documento numa unidade onde já está | `[31]` | (192,156) |
| unidade só de leitura / erro ao copiar | `[14]` | (192,156) |

No original, soltar um documento na lixeira toca só o `2fe` (o `2f8` é
do clique na lixeira).

## Impressão

Arrastar um documento para a impressora (`21c`, `212`, `2f6`) abre a
prévia **direto no "OK. Já estou pronto para imprimir!"** `4c00[8]`
(Pára / Segue), sem as boas-vindas nem as cópias da máquina do
Escritório. Segue (`2eb`): `4c00[9]` "Eu ainda estou imprimindo…" com o
dinossauro atrás do rato no canto (`4c0b`, `4c0f`, `4c17`); fim:
`4c00[11]` "Terminei…" (Pára, `2ec`); falha: `0805[21]` "Sua impressora
está com algum problema" (`2fe`, o que o Wine mostrou); Esc: `4c02[2]`.
Fechar o aviso volta à Biblioteca.

`WRITER_PREVIEW`: janela (16,32) 575×431 branca com moldura laranja (as
bordas dos lados com o escuro por fora), moldura cinza (66,62) 210×270, a
folha em pé (76,70) 190×254, McZee `4c03` em (545,0), balões com o canto
direito em 581 e o topo em 63, avisos em (209,149).

A folha impressa (`impressao-*.png`, 595×792 ou o tamanho do `DOP`) é,
por enquanto, o texto da 1ª página com a fonte de 16 pontos, nas margens
e com o alinhamento de cada parágrafo (`writer_library::render_first_page`);
a diagramação completa (fontes, figuras, bordas, cartão em quadrantes,
faixa em várias folhas) é de outra tarefa e deve substituir só essa
função.

## Parede e documentos

- Documentos da criança: `*.max` e `texto-*.txt` da pasta de textos, os
  mais recentes primeiro; exemplos: `SHARED/USERS/MAX/*.MAX`.
- Miniatura: o `THMB` do `.MAX` (108×144) reduzido para caber em 100×100
  (75×100) como o `StretchBlt` do original no modo `BLACKONWHITE` (cada
  pixel junta os de origem que caem nele e fica com o mais escuro); um
  `.txt` ganha uma página desenhada com um risquinho por letra.
- Parede: `FILR` 1–4 do `~MAXUSER.INF` da pasta de textos
  (`formats::maxprefs`; a ordem é em cima à esquerda, em cima à direita,
  embaixo à direita, embaixo à esquerda, conferida com o `~MAXUSER.INF` do
  Wine). Sem nenhum `FILR` 1–4 (usuário novo): `SAMP9`, `SAMP16`, `SAMP5`,
  `SAMP6` (a parede da captura `building_81_lib_base`; o original sorteia).
  Escolha do `~MAXUSER.INF` (e não de um `biblioteca.txt`): é o arquivo do
  original, a T14 (preferências do Writer) grava o lugar 0 e o resto no
  mesmo arquivo com as mesmas funções (`writer_library::read_prefs` /
  `write_prefs`, que mantêm os outros chunks), e um `USERS/<nome>` do
  original pode ser aproveitado.
- Clique duplo (ou "Estou pronto para ir!"): `Effect::OpenDocument`, a
  área de escrita com o documento (`Writer::open_document`).

## Gaveta de unidades

O original mostra as unidades do computador (C, Z) em fileiras de 14 px a
cada 21 px a partir de (296,180), com pontas vermelhas e o nome no meio
cinza; as fileiras vazias são pretas. O port mostra duas: **Ateliê
Livre** (a pasta de textos) e **Exemplos do Max** (só de leitura). Clicar
numa unidade abre a pasta; soltar um documento nela o copia (um exemplo
copiado para o Ateliê Livre vira documento da criança). As setas de
rolagem do original (só úteis com mais de cinco unidades) não foram
feitas.
