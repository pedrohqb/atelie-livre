# Creative Writer: salas do prédio

Saguão, Escritório (a sala do andar 2), Oficina de Projetos (3) e Oficina
de Idéias (4) do Creative Writer: a mesma cena `Room` do Fine Artist com os
`RoomSpec` do Writer (`writer_lobby`, `writer_office_room`,
`writer_floors`, kit em `writer_rooms`, animações e personagens em
`writer_gags`). Fontes: tabela de pontos quentes do `WRITER.EXE`
(`cw/report/building-hotspots.txt`), `building.md` §8, `building-salas.md`
§1, §3–5, `ideas.md` §2 e as capturas `cw/shots/building_*`. Folhas de
comparação original × port: `cw/shots/rooms_compare_{lobby,office,projects,ideas,portfolio}.png`
(geradas por `dump_writer_rooms_for_comparison` + `tools/wine/writer_rooms_sheets.py`).

## Medidas

- **Figuras**: posições da tabela de pontos quentes (cor = id anterior ao de
  1 bit; máscara = 1 bit + 1, conferida com `pic_next_id(id, 2)` em
  `writer_rooms_match_real_data`) e confirmadas com `tools/wine/locate.py` nas
  capturas de base.
- **Balões**: o corpo (sem o rabicho) medido por diferença de capturas. Os
  do McZee começam em (129,233) (o rabicho de `room_talk::mczee_tail` sai
  do topo); os do Max (menu `[29]`, chegadas) começam no canto de cima, com o
  rabicho embaixo (o port ainda não desenha o rabicho do Max).
- **"EPA!" da luminária**: (208,179) em todas as salas.
- **Max**: a tabela põe a `7e03` em (−2,282), mas nas capturas ele aparece
  como `7e0f` (de pé) em (9,253) — todas as chegadas — e, na Oficina de
  Idéias, também como `7e03` (encostado na máquina) em (−4,219). Nos
  Projetos só a `7e0f`; nas Idéias uma das duas é sorteada a cada chegada
  (`CharacterSpec::poses`). A `7e0b` (9,283) é a volta dos close-ups e a
  `7dff` (−2,339) o Max sentado das telas da Máquina e do Quadro (dados em
  `writer_gags`, usados por outras telas).

## Saguão

| área | ação |
|---|---|
| mapa: Oficina de Idéias / Projetos / Escritório | `304` e o andar |
| setinha "Biblioteca" do mapa | `304` e a Biblioteca |
| faixa de baixo do mapa (41,128) 53×12 | `304` e o **sótão** |
| mostrador (288,21) | só `233`; fecha o balão aberto no mesmo clique |
| portas (216,93) e botão (480,202) | elevador (`2d4` + `311`) |
| seta "Biblioteca" (94,306) | `2fb` e a Biblioteca |
| skate "LÁ FORA" (409,391) | `2fc` e a rua |
| luminária "Tchau" (147,162) | `2f7` + `2fe`, a figura com fumaça `691b` e o "EPA!" |

Chegada `[1]`; as opções abrem as dicas `[2]`–`[5]` perto do mapa e da
seta.

## Escritório (andar 2)

- **Quadro do último documento** (`68bb`): miniatura do `THMB` do `.MAX`
  (108×144 reduzida a 73×98, como no original) encostada no topo da página,
  centrada em x = 199. O documento é o `FILR 0` do `~MAXUSER.INF`
  (`writer::prefs`); para um usuário novo, "O Ponto de Fuga" do Max.
  - clique: `[28]` "Esta é a última coisa que você salvou…", ou `[0]` "Esse
    documento é do Max…" se o documento não é do usuário; sai da boca do
    McZee (`RoomSpec::object_balloon`);
  - clique duplo: a área de escrita (`SceneId::Writer`), que deve abrir o
    `prefs::last_document`.
- **Plaqueta do quadro** (131,232) 136×17: documento do usuário → campo de
  título (Return troca o `TITL`/`ADD ` do `.MAX` e o título no
  `~MAXUSER.INF`, Esc desiste) com o balão `[18]`; de outra pessoa → `21c` +
  `2fe` e o "EPA!" `0805[30]` "Você não pode mudar esse título" em
  (203,126), que qualquer clique fecha.
- **Prancheta** (folha e plaqueta "Documento Novo"): `[27]`; clique duplo →
  `CwScene::NewDocument`.
- Opções da chegada `[26]`: `[27]`, `[28]`/`[0]`, `[12]` (a pasta) e `[66]`
  (mastro, com "Descendo?" `[68]` e "Tecla de atalho" `[67]`).
- Chegando pelo elevador toca `304`, como no Ateliê.

## Oficina de Projetos (andar 3)

- Alce JORNAL `[36]`, burro CARTÃO `[35]`, rinoceronte FAIXA `[37]` (a placa
  "FAIXA" não reage); "Ignorar" → `[38]`; Ctrl+clique começa direto. A
  primeira bolinha abre o projeto com o assistente do Max
  (`CwScene::Project`, `creative-writer-projetos.md`); a segunda, a pasta
  filtrada pelo projeto (`CwScene::ProjectFolder`), e o documento abre sem
  assistente.
- Chegada `[31]`: as três primeiras opções abrem direto o balão do bicho; a
  quarta, `[69]` (mastro).
- **Varinha nos bichos**: um acessório sorteado por 3 s (som `293`),
  ancorado no canto da figura do bicho (`RoomSpec::wand_props`); depois a
  bolsa se abre. Encaixando as figuras nos dois bichos, as `08e3`–`08e8`
  são do **burro** e as `08d9`–`08de` do **rinoceronte** (o relatório dava o
  contrário, sem confirmar). Alce: `08fc`–`0901`.

## Oficina de Idéias (andar 4)

- Máquina de Idéias `[44]` (Ignorar → `[45]`) e Quadro de Figuras `[49]`
  (→ `[50]`); a primeira bolinha (ou Ctrl+clique) abre
  os close-ups `CwScene::IdeaMachine` / `CwScene::PictureBoard`
  (`writer_ideas`); a segunda, a pasta
  (no original já filtrada pela área; o port ainda não filtra).
- Chegada `[42]`: Máquina, Quadro e `[70]` (mastro, com "Tecla de atalho"
  `[71]`).

## Comum às salas

- **Bolsa Mágica** por sala (`building.md` §8.3); aberta/fechada e a ajuda
  desligada valem entre salas e entre sessões (`RoomKit::persist`,
  `writer::prefs::load_state`/`save_state`, arquivo `predio-writer.txt`).
- **Varinha**: cursor `CURSOR 0x000f` do `WRITER.EXE`; balão `[56]` com a
  "Tecla de atalho" `[57]`, saindo da boca do McZee.
- **Ctrl+O** abre a pasta em qualquer sala (`RoomKit::ctrl_o`, som `2f1`).
- **Pasta** (maleta, Ctrl+O, segunda bolinha dos objetos): `writer_portfolio`,
  o fichário do Fine Artist com os documentos por área ("Escritório",
  "Cartões", "Faixas", "Jornais"), prévia 108×144 em (363,126), "Título" e
  "Autor"; JÁ torna o documento o último (`FILR 0`) e vai à área de escrita.
- **Gags** e **surpresas da varinha**: as listas de `building.md` §8.7–8.8
  (todas decodificam). Os pontos de ancoragem das surpresas (mastro e buraco
  do rato da sala, como no Fine Artist) não foram conferidos contra o
  original.
- **Mastro**: a mesma volta do Fine Artist (saguão → 2 → 3 → 4 → sótão).

## Sons ambíguos (T15)

O rastro `+winmm` do Wine só dá o tamanho do som; ids de mesmo tamanho não
foram separados. Adotados: McZee `281` (= `295` = `2d3`), FEITO/JÁ `239`
(= `28b`/`2e1`/`2ee`), `2eb` (= `310`), skate `2fc` (= `303`), buraco do
rato `2ea` (= `20a`), luminária `2f7` (= `286`), Max `283` (= `226`/`312`).
Conferir com um depurador do Wine (ponto de parada em `sndPlaySound`/
`PlaySound` lendo o id do recurso) fica pendente; os nomes estão em
`writer_rooms::snd`.

## Pendências

- Paleta das salas: o original troca cores do fundo (parede e chão do
  saguão, a parede do Escritório) em tempo de execução, como o saguão do
  Fine Artist; o port desenha as cores cruas do bitmap.
- Rabicho dos balões do Max; o Max de pé alternando o braço.
- O cursor de mão das salas ainda é o do `ARTIST.EXE` (`DEFAULT_CURSOR`).
- A área de escrita ainda não abre o `prefs::last_document` nem o documento
  escolhido na pasta (integração com o Escritório).
- Pasta filtrada pela área (bolinha 2 dos objetos); as unidades de disco.
