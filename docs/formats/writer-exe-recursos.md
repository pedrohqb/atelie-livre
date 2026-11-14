# Recursos do `WRITER.EXE`

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

Inventário completo dos recursos do executável do Creative Writer (NE,
Windows 3.1, PT-BR, 20/10/1995), com o que cada tipo contém e onde é usado.
Formatos comuns aos dois programas (NE, `STR#`, `PAT#`, `LOGPALETTE`,
ícones, cursores) estão em `recursos.md`; `ATAG` em `atag.md`; `WHAT` em
`what.md`. Este documento acrescenta os tipos que ainda não estavam
interpretados (`SCRD`, `TMEN`, `CSCH`, `SSHD`, `PATS`, `SMNM`, `TDRL`) e o
mapa de uso de cada recurso no Writer.

Extração bruta: `atelie-tools extract` → `out/resources/WRITER.EXE/<TIPO>/`.

| Tipo | Qtde | Conteúdo | Reaproveitável do Fine Artist? |
|---|---|---|---|
| `ATAG` | 155 | roteiros de animação (bichos que atravessam as salas, gags do McZee, mastro de bombeiro) | leitor `formats::atag` pronto; catálogo em `writer-atag.md` |
| `WHAT` | 32 blocos | balões (ajuda, assistentes, mensagens) | leitor `formats::what` pronto; mapa abaixo |
| `SCRD` | 14 | formas do Splot (curvas) | **idênticas** às do `ARTIST.EXE`; ver `scrd.md` |
| `STR#` | 14 | listas de textos | leitor pronto |
| `STRING` | 1 bloco | mensagens de erro de inicialização | leitor pronto |
| `TMEN` | 2 | menus de texto do Splot (estilos, sombras) | idênticos ao `ARTIST.EXE` |
| `CSCH` | 1 | tabela de 16 cores + combinações | idêntico ao `ARTIST.EXE` |
| `SSHD` | 1 | parâmetros das 7 sombras do Splot | idêntico ao `ARTIST.EXE` |
| `PATS` | 1 | 13 padrões 8×8 (preenchimento do Splot) | idêntico ao `ARTIST.EXE` |
| `PAT#` | 1 | 18 padrões 8×8 | formato em `recursos.md` |
| `SMNM` | 2 | geometria do cavalete do Splot | idênticos ao `ARTIST.EXE` |
| `TDRL` | 1 | 13 registros de 12 bytes (não interpretado) | idêntico ao `ARTIST.EXE` (mesmo início) |
| `MENU` | 1 (0x3e8) | barra de menus escondida (Arquivo, Editar, Ferramentas) | novo |
| `ACCELERATOR` | 1 (0x3e9) | atalhos Ctrl+N/O/S/Q/Z/X/C/V | novo |
| `DIALOG` | 5 | Sobre, Controle de Som, Decodificar, Redefinir Máquina de Idéias, Opções de Impressão | novo |
| `BITMAP` | 6 | 4 dinossauros (64×25…72×32), PARE (40×29), proibido (32×31), 1 bit | novo |
| `CURSOR` / `GROUP_CURSOR` | 35 / 35 | cursores (folha `shots/formats_cursores_bitmaps.png`) | parte comum (setas, mãos) |
| `ICON` / `GROUP_ICON` | 6 / 2 | ícone do programa (1 e 4 bits) | — |
| `LOGPALETTE` | 2 | paletas de 16 (id 4) e 256 cores (id 8) | formato em `recursos.md` |
| `VERSION` | 1 | versão 1.0 | — |

`WRITER.EXE` não tem `SWND`, `DSPC`, `BTNW`, `ACUR` nem `STR` (que o
`ARTIST.EXE` tem).

## Textos

### `STR#`

| Id | Itens | Uso |
|---|---|---|
| 0x200 | 9 | erros de arquivo ("Não consigo achar esse documento", "o disco está cheio"…) |
| 0x201 | 12 | nomes de pastas e arquivos: MSKids, Writer, Shared, Stuff, Fonts, Users, McZee, Maggie, Max, `mskids.prf`, `~user.prf`, `~maxuser.prf` |
| 0x204 | 7 | sons do usuário: `~max.snd`, "Nome do Som >", "Sem Som", "Som sem Nome", "Sons", "Meus Sons", `writer.snd` |
| 0x205 | 2 | "minha caixa de ferramentas", "McZee" |
| 0x206 | 7 | **fontes**: Pablo Graphos, Lucida Sans, Lucida Handwriting, Lucida Blackletter, Impact, Stencil, `pablo` |
| 0x3ac | 17 | nomes das áreas/projetos na biblioteca: Escritório, Lições sobre os Quadriculados, Histórias em Quadrinhos, Lições sobre Perspectiva, Histórias em Quadrinhos, Botões e Adesivos, Cartões, Faixas, Jornais, Máquina de Idéias, Quadro de Figuras, Autor, Esta Pasta, Título, Conteúdo, Fora de Imaginópolis, Documentos Achados pelo Cachorrinho |
| 0x800 | 3 | "Documento Novo", "Lucida Sans" (fonte padrão), "Pág. %d" |
| 0x803 | 1 | "Creative Writer" |
| 0x804 | 10 | menu de sistema ("Central de Controle…", "&Voltar ao Creative Writer") e erros de impressora/disco |
| 0x806 | 6 | erros de inicialização (falta arquivo, memória, fontes TrueType) |
| 0x807 | 18 | estilos: Simples, Negrito, Inclinado, Sublinhado; **efeitos** (índice = valor do sprm `0x0b`): Nada de especial, Pisca-pisca, Formiguinhas, Vapt, Vupt, Brilho, Treme-treme, Maluco; **mixes**: Anotação, Título, Era uma vez, Poesia, Relatório, Super-simples |
| 0x808 | 11 | "Detetive McZee para salvar o dia" (senha-mestra), "SENHA", localizar/ortografia ("Não encontrado", "Impossível trocar", "Correção terminada", "(Sem sugestões)", "Nada"), andares: Saguão, Escritório, Oficina de Projetos, Oficina de Idéias |
| 0x809 | 7 | "Máquina de Idéias", "Lembretes", "Figura Desconhecida", `writer.pic`, `~SplotId.MAX`, `~NotesId.MAX`, `~PicWin.CHK` (arquivos temporários do Splot, das anotações e do Quadro de Figuras) |
| 0x80a | 8 | folhas dos cartões (Capa, Página Interna 2, Página Interna 3, Contracapa), "Sua Figura", "Estamos fazendo um Cartão / uma Faixa / um Jornal" |

### `STRING` (bloco 1)

1 "Erro", 2 "…processador no mínimo 386", 3 "…Windows em modo avançado…",
4 "O computador está sem memória ou um arquivo está com defeito…", 5 "…640x480
ou melhor…", 6 "…Microsoft Mouse…".

## Menu, atalhos e diálogos (novos no Writer)

O Writer tem uma **barra de menus do Windows** (escondida na tela cheia;
"Olhe a barra de menu para ver comandos úteis", diálogo Sobre):

| Menu | Comando (id) | Atalho |
|---|---|---|
| Arquivo | Novo (0x1f41) | Ctrl+N |
| | Abrir… (0x1f42) | Ctrl+O |
| | Salvar Como… (0x1f43) | Ctrl+S |
| | Salvar Como Texto… (0x1f44) | — |
| | Opções de Impressão… (0x1f45) | — |
| | Voltar ao Creative Writer / Sair (0x1f46) | Ctrl+Q |
| | Sobre o Creative Writer… (0x1f5c) | — |
| Editar | Desfazer (0x1f47) | Ctrl+Z |
| | Recortar (0x1f49), Copiar (0x1f4a), Colar (0x1f4b), Limpar (0x1f4c) | Ctrl+X/C/V |
| Ferramentas | Controle de Som… (0x1f4d) | — |
| | Decodificar Documento… (0x1f4f) | — |
| | Redefinir Máquina de Idéias… (0x1f50) | — |

`ACCELERATOR` 0x3e9: `^N 1f41, ^O 1f42, ^S 1f43, ^Q 1f46, ^Z 1f47, ^X 1f49,
^C 1f4a, ^V 1f4b` (ASCII de controle, sem flags).

Diálogos (MS Sans Serif 8, unidades de diálogo):

| Id | Título | Controles |
|---|---|---|
| 0x7d0 | Creative Writer (Sobre) | "Versão 1.0", "20 de outubro de 1995", "® 1993-1995 Microsoft…", "Olhe a barra de menu…", OK |
| 0xbb8 | Controle de Som | texto; "&Ligar sons" (0xbba), "&Desligar sons" (0xbb9) |
| 0x1388 | Decodificar o Documento | "O McZee vai decodificar um documento se você esquecer a senha!" 1. Abra o documento. 2. Escolha o Agente Secreto 3. Digite: "Detetive McZee para salvar o dia" 4. Segure Control+Shift e clique em OK; "&Valeu, McZee" |
| 0x1770 | Redefinir a Máquina de Idéias | "Remove todas as entradas que você colocou." "Cuidado!…"; "&Apagar" (3), "&Deixar como está" (2) |
| 0x1b58 | Opções de Impressão | "Opções locais" (0x1b5a), "Opções globais" (0x1b59), "Deixar como está" |

Decodificador de referência: `tools/win16res.py menu|dialog|accel ARQ.bin`.

## Recursos do Splot (compartilhados com o Fine Artist)

Todos idênticos byte a byte (fora o preenchimento do NE) aos do
`ARTIST.EXE`: o port do Fine Artist (`engine::paint::shaped`) pode passar a
usar estes dados em vez de constantes.

### `TMEN` — menus de texto

```
u16   n
n × u8   um byte por item (0, ou 1/2/8 nos itens Vertical/Preencher/Em cima-embaixo: tipo de item ~)
4 bytes  zeros
n × { u8 tamanho, texto Windows-1252, u8 0 }
```

- `TMEN 1` (9): Normal, Negrito, Inclinado, Contorno, MAIÚSCULAS, Vertical,
  Preencher, Em cima/embaixo, Esquerda/direita — botão de estilos do Splot.
- `TMEN 2` (8): Sem sombra, Sombra normal, Sombra do amanhecer, Sombra do
  anoitecer, Girar e sumir, Girar em círculo, Sombra convergente, Efeito
  tridimensional — botão de sombras (mesma ordem do `Shadow::ALL` do port).

### `CSCH` — tabela de cores

```
u16  16       número de cores
u16  24       número de pares
u16  3
16 × { u16 r, u16 g, u16 b }   estilo Mac: byte alto = cor (ffff ffff ffff = branco)
24 × { u8, u8 }                pares de índices (combinações de cores ~ "Maluco"/alternadas)
```

As 16 cores, em ordem: #ffffff, #ffee4c, #ff8c32, #b96524, #fa4a72,
#fa0039, #8888e6, #6141db, #de38c7, #932ca3, #5ccb53, #478c50, #bebebe,
#8c8c8c, #646464, #000000 — exatamente as do Seleciona-cores, do Iluminador
(`shots/formats_menu_cores.png`) e dos sprms `0x10`/`0x11` do `.MAX`.

### `SSHD` — sombras

1536 bytes: `u16 7` (sombras 1–7 do `TMEN 2`), `u16 1`, depois registros
de ~62 bytes com cor da sombra (3 × `u16` Mac: cinza #8c8c8c, #646464,
#bebebe…), deslocamentos (`fa ff fe ff` = (−6, −2)), fatores de escala
(0x0362, 0x01f4…) e ângulos; a segunda metade tem registros de 60 bytes
(`b0 04 ff ff 03 00 06 00 03 00`…) das sombras giratórias. Campos não
fechados; o port já reproduz as sombras por fórmula.

### `PATS` — padrões do Splot

`u16 13` + 13 × 8 bytes (uma linha de 8 pixels por byte, bit alto à
esquerda). Diferente do `PAT#` (linhas em `u16`). Usado pelo estilo
"Preencher" do Splot.

### `SMNM`

Dois registros de 20 `u16`: (463, 335, 49, 47, 75, 96, 15, 661, 68, 36, 2,
150, 20, 1, 150, 20, 60, 12, 48, 16) e (418, 245, 49, 41, 66, 75, …).
Provável geometria das duas variantes do cavalete do Splot (tamanho da
janela, posição dos botões e da prévia).

### `TDRL`

`u16 13` + 13 registros de 12 bytes (`ff ff`, `u16`, 4 bytes, `04 00`, `u8`,
`ff`). Não interpretado.

### `SCRD` — formas

Ver `scrd.md`.

## `PAT#` (0x80)

18 padrões no formato de `recursos.md` (padrões de preenchimento das
figuras desenhadas).

## `WHAT` — mapa dos blocos

Leitor: `atelie-tools what WRITER.EXE --block XXXX`; dump completo em
`what_writer.txt`. A coluna "tela" diz onde o balão aparece.

| Bloco | Balões | Tela / uso |
|---|---|---|
| 0201 | 70 | piadas, ditados e "livros que deviam ter sido escritos" do McZee (balões aleatórios ao clicar nele; o título é a palavra-chave em inglês) |
| 0202 | 18 | mensagens de sistema: som do usuário, salvar (`idsSaveDoc`, `idsSaveBoth`), assistentes, gravação de som, atalho de salvar, backup do assistente, renomear |
| 0203 | 2 | "now what?", cópia de arquivo |
| 0212 | 1 | aviso "Fora de Imaginópolis" |
| 0213 | 2 | Cachorrinho (Ratdog): procurar documento por nome, parar busca |
| 0801 | 16 | erros do documento: moldura, som, figura vazia, memória, abrir/gravar/ler/renomear, MacPaint ilegível/vazio, máximo de molduras |
| 0805 | 44 | erros e confirmações das ferramentas: localizar/trocar, ortografia concluída, rébus, apagar Splot/figura, sair, cifra (`idsEncryptCutCopy`, `idsEncryptNuke`, `idsNeedPassword`, `idsNeedPassScram`), Máquina de Idéias, balde, faixa, nº de página com som, impressora, apagar arquivo, importar figura/som, documento largo, quadro de avisos (`idsCantDelOnBoard`…), "e agora?" do Splot e da figura |
| 0bb8 | 75 | **prédio**: Saguão (Welcome lobby, Lobby 2–4, ir à biblioteca), **Biblioteca** (documento na parede, abrir, impressora, lixeira, pasta, trocar usuário, copiar para disquete, dicas de arrastar), **Escritório** (Welcome-WrStudio, novo documento, último documento, ajuda liga/desliga, pasta, mastro), **Oficina de Projetos** (cartão, jornal, faixa, trabalhar), **Oficina de Idéias** (Splot, Quadro de Figuras, Máquina de Idéias, caderno de idéias, copiar/recortar notas), varinha mágica, elevador, atalhos |
| 0fa0 | 30 | ferramentas de palavras (Pablo): Seleciona-letras, Dimensiona-palavras, Seleciona-cores, Seleciona-estilos, Posicionador, Aplicador, ponto de inserção, Splot, Iluminador, Efeitos especiais, Mix de estilos, Estilos de parágrafo, Espaçamento, Recuos e dicas |
| 0fa1 | 13 | Seleciona-adesivos: rolos, clique, mão, Cavalete de lápis de cera, redimensionar, associar sons, Cachorrinho, setas |
| 0fa2 | 11 | Seleciona-sons: rolos, volume, cavalete de som, Cachorrinho |
| 0fa3 | 15 | Removedores: Recortar, Copiar, Colar, apagar som/figura, teclado |
| 0fa4 | 14 | Localizar e trocar, Ortografia ("Spell 1"), Sinônimos ("Tip - synonyms"), Liquidificador (Blender 1–3: embaralhar palavras) |
| 0fa5 | 13 | Preparação da página: Fundo, Borda, Colunas, Alto/Largo, Números de página, Margens |
| 0fa6 | 12 | Visualizador de páginas: normal, livro, todas, lado a lado, página inteira, lupa, zoom |
| 0fa7 | 1 | Imprimir |
| 0fa8 | 16 | Transformadores: mão, redimensionadores (todos, largura, altura), Agente secreto (2), Maçã (figura na palavra), Texto flutuante |
| 0fa9 | 2 | Ovo desfazer |
| 0faa | 22 | Pablo liga/desliga, caderno de idéias, varinha, mapa do prédio, renomear documento, esconder trabalho, páginas, Splot, salvar, pasta, Quadro de Figuras, copiar/recortar/colar Splot e figura |
| 3800 | 29 | assistente da **Faixa** |
| 3802, 3803, 3804 | 2, 1, 3 | páginas inválidas/documento em branco; sair do assistente; ajuda só do McZee, não abriu a pasta |
| 3a00, 3a02 | 30, 2 | assistente do **Jornal** |
| 3c00, 3c02 | 110, 2 | assistente do **Cartão** (dobrar, páginas, bordas, adesivos, Splot, impressão) |
| 4000 | 11 | história de exemplo "My story begins…" (Máquina de Idéias ~) |
| 4c00, 4c02 | 13, 3 | **impressão** (assistente do Rex: tipo de impressora, cópias, "ainda imprimindo", cancelar) |
| 4e00, 4e02 | 11, 7 | **Entrada/cadastro** (lista de usuários, opções, nome, apelido, casa, bicho; erros) — mesmo papel dos blocos 1388/1389 do Fine Artist |

Os ícones desenhados dentro dos balões (`05` figura `id<<16|param`) usam a
mesma numeração própria do Fine Artist (`what.md`), mais `20c` (cachorrinho)
e `230` (Rex).

## `ATAG`

Catálogo em `writer-atag.md`.

## `BITMAP` e `CURSOR`

- `BITMAP` 0xa0–0xa3: dinossauro em 4 quadros (64×25 a 72×32, 1 bit) —
  animação de espera; 0xc0 placa PARE; 0xc1 sinal de proibido (balão de
  erro do Rex).
- `CURSOR` 0x01–0x23 (hotspot no nome do PNG extraído): seta, pincel com
  mão (pincel de palavras), documento, pincel, adesivo, tesoura, carimbo de
  som, lixeira, nota musical, canto (corte), bolinha, setas de
  redimensionar, mover, varinha mágica, estrela, despertador (espera), mãos
  (pegar/arrastar), dedo apontando. Folha: `shots/formats_cursores_bitmaps.png`.
