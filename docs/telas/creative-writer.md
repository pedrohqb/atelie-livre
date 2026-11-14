# Creative Writer: Escritório (editor de texto)

Geometria medida no original (Wine, tela lógica de 640×480); ids de
`WRITER.PIC`, sons de `WRITER.SND` e cursores de `WRITER.EXE` em
hexadecimal. A pesquisa completa está nos relatórios `office.md`,
`office-palavra.md`, `office-ferramentas.md` e `office-arquivos.md` da
varredura do Creative Writer; este documento descreve **como o código está
organizado** e o contrato para implementar cada ferramenta.

## Organização do código

O Escritório fica em `crates/engine/src/scenes/writer/`:

| arquivo | conteúdo |
|---|---|
| `mod.rs` | `Writer` (a cena): escolhe a ferramenta, repassa os eventos, desenha na ordem certa; `Tools` (uma instância de cada ferramenta) |
| `tools.rs` | `ToolId` (as 10 ferramentas), tabela `TOOLS` (figuras inativa/ativa, som, balão), trait `ToolStrip`, `Cx`, `Pic`, `StripButton` e ajudantes |
| `editor.rs` | `Editor`: documento, página, digitação, desfazer, adesivos dentro do texto, gravação |
| `chrome.rs` | moldura, barra de ferramentas (aba da ativa), barra de baixo |
| `mczee.rs` | balões automáticos, menu de 8 botões, ligar/desligar a ajuda, Varinha |
| `cursors.rs` | nomes dos `CURSOR` 0001–0023 e o `Pointer` (emite `SetCursor` só quando muda) |
| `exit.rs` | porta Saída |
| `word_strip.rs` | Palavra |
| `stickers.rs` | Seleciona-adesivos |
| `sounds.rs` | Seleciona-sons |
| `page_strip.rs` | Preparação da página |
| `speller.rs` | Corretor e troca-palavras (abelha) |
| `egg.rs` | Ovo desfazer |
| `removers.rs` | Re-movedores |
| `transformers.rs` | Transformadores |
| `viewer.rs` | Visor de páginas |
| `print.rs` | Imprimir |

O modelo de documento rico (`crates/engine/src/writer/{doc,object,layout,
selection,undo}.rs`) é outro trabalho; quando entrar, muda o `editor.rs`, e as
ferramentas continuam recebendo `&mut Editor`.

## Contrato de uma ferramenta (`ToolStrip`)

Cada arquivo de ferramenta define uma `struct` com `Default` e implementa
`ToolStrip`. Todos os ganchos têm versão vazia:

| gancho | quando |
|---|---|
| `id()` | qual `ToolId` é |
| `activate(ed, cx, again)` | escolhida na barra (`again` = já era a ativa). O som da ferramenta, a aba e o balão já foram tratados |
| `deactivate(ed, cx)` | outra ferramenta foi escolhida: feche listas, painéis, tiras |
| `modal()` | `true` com lista, painel ou cavalete aberto: a ferramenta recebe todos os cliques antes da barra |
| `press(ed, cx, x, y) -> bool` | botão apertado (faixa ou página); `true` = tratado |
| `motion(ed, cx, x, y)` | mouse movido (para arrastos: guarde o estado no `press`) |
| `release(ed, cx, x, y)` | botão solto |
| `key(ed, cx, key, ctrl) -> bool` | tecla; `false` deixa a tecla ir para o editor de texto |
| `tick(ed, dt)` | animação |
| `draw_strip(ed, f, a)` | faixa de opções (y 56..104), sobre o fundo cinza-claro |
| `draw_front(ed, f, a)` | por cima da página (tiras de filme, listas, painéis) |
| `cursor(ed, x, y)` | cursor do mouse; `None` = padrão (texto na página, mão no resto) |

Ordem de um clique no `Writer`: balões e menu do McZee → ferramenta modal →
barra de ferramentas → nicho do McZee → porta → setas da barra de baixo →
`press` da ferramenta ativa → clique na página põe o ponto de inserção.
Teclas: `key` da ferramenta, senão o editor (sem Ctrl). Desenho: moldura,
barra, McZee, porta, `draw_strip`, página, barra de baixo, `draw_front`,
balões e menu do McZee.

`Cx` é a saída de um evento:

- `cx.sound(id)`: som do `WRITER.SND`;
- `cx.help((bloco, registro))`: pede o balão do McZee (só aparece com a ajuda
  ligada); `cx.help_even_off(..)` aparece mesmo desligada (Re-movedores nas
  três primeiras vezes); `cx.close_help()` fecha o balão;
- `cx.log(..)`, `cx.fx.push(Effect::..)`: registro e outros efeitos;
- `cx.ctrl`, `cx.shift`: modificadores (Ctrl+clique no Aplicador, Coelho…).

Ajudantes de `tools.rs`: `Pic` (figura com máscara num lugar fixo),
`StripButton` + `strip_button(id, máscara, retângulo, balão, nome)`,
`press_static` / `unavailable` (botão que ainda não faz nada: som `0233`,
balão e registro "ainda não disponível"), `SWITCH` (interruptor "Mais
escolhas" em (508,59)), `snd`, `pic`, `cursor`.

Para implementar uma ferramenta: mexa só no arquivo dela (e, se precisar de
estado novo do documento, no `editor.rs`/modelo). Ponha os testes no próprio
arquivo ou no `mod.rs` (`tool(ToolId::X)` clica na ferramenta).

## Figuras do `WRITER.PIC`

Cada figura colorida vem seguida, **na ordem do arquivo**, da versão em
preto e branco (para vídeo de 16 cores) e, se não for retangular, da
máscara: `020d` cor, `020e` preto e branco, `020f` máscara. A máscara fica,
portanto, duas figuras adiante, e a figura seguinte não é máscara (o port
antigo usava as de preto e branco como ícones). Os ids não são sempre
consecutivos: Flutua-texto é `08c7` (cor), `08ca`, `08cb` (máscara); os
botões brancos da Palavra (`0811`, `0279`, `0bc3`…) são de 1 bit e
retangulares, sem máscara.

## Barra de ferramentas

Aba da ferramenta ativa cinza-claro (190), barra cinza-escuro (100); som ao
escolher `0209`..`0212` (na ordem); escolher de novo a ativa toca `02fc` e
reabre o balão.

| ferramenta | clique x | inativa | ativa | balão |
|---|---|---|---|---|
| Palavra | 12..59 | `020d`/`020f` (17,12) | `0251`/`0253` (17,12) | `0fa0[1]` |
| Seleciona-adesivos | 60..103 | `01ff`/`0201` (65,12) | `0203` (65,12) | `0fa1[0]` |
| Seleciona-sons | 104..154 | `0205`/`0207` (110,10) | `0209`/`020b` (106,9) | `0fa2[0]` |
| Preparação da página | 155..208 | `0495`/`0801` (159,16) | `0907`/`0909` (159,9) | `0fa5[0]` |
| Corretor | 209..260 | `091f`/`0921` (214,11) | `0937`/`0939` (214,21) | `0fa4[0]` |
| Ovo | 261..314 | `0211`/`0213` (266,6) | `0215`/`0217` (272,6) | `0fa9[0]` |
| Re-movedores | 315..366 | `08e7`/`08e9` (319,9) | a mesma | `0fa3[0]` |
| Transformadores | 367..412 | `0219`/`0223` (372,9) | `02ad`/`02af` (372,9) | `0fa8[0]` |
| Visor de páginas | 413..466 | `0803`/`0805` (417,12) | `090b`/`090d` (417,8) | `0fa6[0]` |
| Imprimir | 467..518 | `0225`/`0227` (472,11) | `02bb`/`02bd` (472,11) | `0fa7[0]` |

McZee no nicho `0235`/`0237` (536,47); porta `023d`/`023f` (581,16).

## Faixas (figuras e balões)

Estado atual: a Palavra (listas de tamanho, cor, estilo e alinhamento),
os Adesivos (rolos e tira de filme, adesivo dentro do texto), o Ovo (um
nível, ícone e DESFAZER/REFAZER alternam) e a explosão dos Re-movedores
funcionam; as outras faixas são só desenhadas, com o interruptor trocando de
página na Preparação da página e no Visor. Botão sem função: som `0233`,
balão da opção e "ainda não disponível" no registro.

| ferramenta | botões (figura/máscara em (x,y) → balão) |
|---|---|
| Palavra pág. 1 | Seleciona-letras `0275`/`0277` (24,61) → `0fa0[3]`; Dimensiona-palavras `0811` (89,61) → `[4]`; Seleciona-cores `027b` (154,61) → `[5]`; Seleciona-estilos `0279` (219,61) → `[7]`; Posicionador `0bc3` (284,61) → `[8]`; Aplicador `080b`/`080d` (349,58) → `[9]`; Ponto de inserção `0831`/`0833` (398,61) → `[10]`; FORMA `0269`/`026b` (443,57) → `[11]`; interruptor `0229`/`022b` (508,59) → `[25]` |
| Adesivos | tampas `0245`, `0239`; rolos `0249`/`024b`; Cachorrinho `025d`/`0263` (414,62) → `0fa1[11]`; Cavalete `0881`/`0883` (459,57) → `[6]`; setas `03b3` (511,56), `03b7` (511,86) sobre `08d9`/`08dd` (510,56); rolo → `[2]` |
| Sons | teatros `024d`/`024f` (20,59), (133,59), (246,59) → `0fa2[2]`; Cachorrinho `025d` (405,62) → `[9]`; Gravador `0265`/`0267` (454,57) → `[5]`; setas |
| Página pág. 1 | Fundo `0821`/`0823` (38,57) → `0fa5[2]`; Cria-colunas `086b` (188,61) → `[4]`; Alto ou largo `08c3` (267,61) → `[6]`; Margens `0869` (346,61) → `[8]`; Nº de página `0875` (425,61) → `[9]`; interruptor → `[1]` |
| Página pág. 2 | Borda `0953`/`0955` (104,57), `0957`/`0959` (212,57), `0825`/`0827` (306,57) → `0fa5[3]` |
| Corretor | PULAR PARA `08cd`/`08cf` (59,59) → `0fa4[1]`; CORRIGIR `082d`/`082f` (204,59) → `[4]`; TROCAR `0835`/`0837` (344,57) → `[5]`; Embaralhador `08b6`/`08bb` (449,56) → `[9]` |
| Ovo | DESFAZER `0889`/`088b` (31,69); REFAZER `088d`/`088f` (427,69); ovos `08e3`/`08e5` (chocado) e `08df`/`08e1` (inteiro) a partir de (144,68), passo 13 |
| Re-movedores | Recortar `0839`/`083b` (58,54) → `0fa3[1]`; Copiar `083d`/`083f` (114,54) → `[2]`; Colar `0841`/`0843` (151,54) → `[3]`; Remove-sons `0849`/`084b` (195,54) → `[6]`; Remove-figuras `0851`/`0853` (240,54) → `[7]`; Ponto de inserção `0831` (356,61) → `[13]`; Jogar tudo fora `0845`/`0847` (435,55) → `[9]` (som `025f`) |
| Transformadores | Mover-modificar `0923`/`0929` (33,58) → `0fa8[1]`; Dimensionar tudo `0857`/`0859` (94,54) → `[2]`; lateral `085b`/`085d` (150,54) → `[3]`; vertical `085f`/`0861` (187,54) → `[4]`; Agente Secreto `0807`/`0809` (278,59) → `[5]`; Coloca-figuras `0865` (340,61) → `[6]`; Flutua-texto `08c7`/`08cb` (432,56) → `[7]` |
| Visor pág. 1 | Página inteira `08a9` (35,58) → `0fa6[5]`; Aumentar visão `08ad` (122,58) → `[6]`; MENOS ZOOM `0891`/`0893` (209,64) → `[7]`; régua `0885` (278,73) com o bichinho `092c`/`092d`; MAIS ZOOM `0895`/`0897` (434,63) → `[8]` |
| Visor pág. 2 | Visão normal `0899`/`089b` (55,58) → `[1]`; Duas páginas `089d` (168,58) → `[2]`; Todas as páginas `08a1` (281,58) → `[3]`; Lado a lado `08a5` (394,58) → `[4]` |
| Imprimir | depressa `02ef`/`02f1` (133,60); fazer escolhas `02f3`/`02f5` (337,59) → `0fa7[0]` |

## Barra de baixo

Três caixas brancas em y 461..474: x 29..168 (cabeça vermelha `03c3`/`03c9`
em (33,464) e "Escritório"), 194..445 (nome do documento, "Documento Novo"
sem nome) e 471..610 (ícone de páginas `0c37`/`0c39` em (475,463), "Pág. N",
setas ▲ `02db` (543,463), ▼ `02df` (560,463), ◀ `0819` (577,463), ▶ `0871`
(594,463)). Hoje ▲▼ trocam de página (sons `02bc`/`02bd`; a primeira seta usada
abre `0faa[11]`); no original rolam a vista (tarefa do Visor).

## McZee

- **Entrada**: som `0304` e balão `0fa0[0]` ("Oi! Aqui é o McZee!").
- **Balões automáticos**: escolher uma ferramenta abre o balão dela (tabela
  acima); os botões das faixas pedem o seu. Balões encostados à direita (borda
  em x 629), topo em y 106, rabicho para o McZee. As figuras embutidas no texto
  dos balões do Writer usam numeração própria do `WRITER.EXE` (`200`
  Palavra, `211` Seleciona-letras, `802` Dimensiona-palavras, `83d` seta ▲…);
  a tabela está em `help_icons::writer_icon` (ids sem entrada não são
  desenhados). Ligações `[21b:n]`, `[21c:n]`, `[21d:n]` e glossário funcionam
  como na pintura (sub-balão ao lado).
- **Menu** (clique no nicho, som `02ce`): o `paint::menu::Menu` genérico com
  `menu::WRITER`: coluna x 533..580, y 103..455, botões de 44 px — cara do
  McZee `041b` (desligar: `0faa[1]`), NOVO `084d`, maleta `044f`, SALVAR
  `08af`, Caderno de Idéias `0c01`, Varinha `0457`, cara de sol `03f3` (piada do
  `WHAT 0201` do Writer), livro verde `0453` (história). Escolher toca `022d`.
  NOVO, maleta, SALVAR e Caderno ainda só registram "ainda não disponível".
- **Ligar/desligar**: "É, por favor dê o fora" desliga os balões automáticos;
  com a ajuda desligada o nicho pergunta `0faa[0]` ("Sim, por favor!" religa).
- **Varinha**: cursor de varinha (`000f`) até escolher uma ferramenta; clique
  na página ainda não toca nada (sons das palavras/adesivos).

## Cursores

`CURSOR` do `WRITER.EXE` (`cursors.rs`): seta `0001`, texto (barra com mão)
`0002`, documento `0003`, pincel `0004`, bomba `0005`, tesoura `0006`, câmera
`0007`, cola `0008`, nota `0009`, canto `000a`, bolinha `000b`, setas de
redimensionar `000c`/`000d`/`000e`, varinha `000f`, estrela `0010`,
despertador `0011`, mãos `0015`–`0018`, documento com figura `0019`, rolo
`001d`, dedo `0023`. Hoje: texto sobre a página (sem lista aberta), varinha
com a Varinha na mão, mão padrão no resto; cada ferramenta pode pedir o seu
em `ToolStrip::cursor`.
