# Truques de Desenho (andar 4) — desenho

Subprojeto do Fine Artist: as lições da Maggie. O andar tem duas: **Truques
básicos** (pedestal "Básico", o porco Chiquinho: espaço negativo/positivo,
formas menores, quadriculado) e **3-D** (estátua "3D", a mesa do café da
manhã: sobreposição, detalhe, linha do horizonte, ponto de fuga, linhas
guia). Os textos estão todos no `WHAT` do `ARTIST.EXE`: bloco `3800` (47
balões, básico), `4000` (250, 3-D), `3803` (saídas), `100d[42..51]`
(andar). O Wine não chega a mostrar a tela das lições (trava na
transição), então a geometria é reconstruída.

## Tela da lição

A lição roda **na tela de pintura** (`Paint` com um `Wizard` acoplado):

- Rótulo da barra inferior: "Estamos vendo os Truques Básicos" / "… de
  3-D" (`STR# 3800`), nome do quadro "Sua Área de Desenho".
- **Área de Desenho da Maggie**: painel de 313×272 (as figuras `3827`…
  `3858` têm exatamente esse tamanho) com moldura preta de 2 px e fundo
  branco, desenhado **sobre** o quadro em (300, 180) → canto inferior
  direito do quadro, só quando o passo tem figura. Clicar nele é "clicar
  em mim (Maggie)".
- **Balão do passo**: `Balloon::from_record` do registro do passo, em
  (30, 115) por padrão (ou ao lado do alvo quando o passo aponta para um
  botão da barra), com os botões **Volta** `040f`/`0410`, **Pára**
  `0417`/`0418` e **Segue** `0413`/`0414` (31×14) na base quando o passo
  avança por Segue. Pára abre `3803[4]` ("Quit Drawing Tricks": parar e
  voltar a trabalhar por conta própria / ficar).
- **Alvo**: quando o passo manda clicar numa ferramenta ou opção, o botão
  pisca (moldura amarela/preta alternando a cada 0,3 s) e só esse clique
  avança; os outros cliques são ignorados.
- Clicar na **Maggie** (painel) ou no McZee abre o balão `3803[0]` com as
  três saídas do original quando não há passo esperando por ela.

## `Wizard`

`scenes/wizard.rs`: `Step { record: (u16 bloco, usize índice), maggie:
Option<u16 figura>, your_area: Option<u16 figura de fundo no quadro>,
advance: Advance }` com `Advance = Next (Segue) | ClickTool(Tool) |
ClickOption(Tool, page, k) | ClickMaggie | ClickIn(rect do painel) |
Choice(Vec<(opção, próximo passo)>) | Paint (qualquer traço; Segue libera)`.
`Wizard { steps, current, done }` com `balloon()`, `target()`,
`on_event(ev) -> Option<Effect>`. Roteiro em `scenes/tricks_basic.rs` (e
`tricks_3d.rs` depois) como constantes.

## Roteiro básico (índices do `WHAT 3800`; figuras do `ARTIST.PIC`)

| passo | balão | Maggie | avanço |
|-------|-------|--------|--------|
| 0 | `0` "A garotada sempre me pergunta" | — | escolha: 0 → 1; 1 → sai para a pintura livre; 2 → volta ao andar |
| 1 | `3` Intro | — | Segue |
| 2 | `4` É divertido desenhar | — | Segue |
| 3 | `6` espaço negativo/positivo | `3827` (Chiquinho) | clique no painel dentro do porco |
| 4 | `7` trace o espaço negativo | `382c`; seu quadro recebe `3831` (silhueta clara) centrada como fundo | pintar com o pincel; Segue |
| 5 | `8` olhe as formas menores | `382c` | clique no rosto (metade esquerda do painel) |
| 6 | `9` três formas circulares | `3838` (formas) | 3 cliques no painel; depois Maggie |
| 7 | `10` acabe de desenhar | `3838` | Segue |
| 8 | `11` Oba! truques 1 e 2 | — | Maggie |
| 9 | `13` ferramentas especiais? | — | escolha: 0 → 10; 1 → 16 |
| 10 | `14` clique nas Super-ferramentas | — | `ClickTool(SuperTools)` (sem a ferramenta ainda: Segue) |
| 11 | `15` clique no Quadriculado grande | — | `ClickOption` (idem) |
| 12 | `18` olhe o Chiquinho com os quadrados | `382d` (grade) | Maggie |
| 13 | `20` eu contornei; preencha o resto | `3830`; seu quadro recebe `3831` + grade | pintar; Maggie |
| 14 | `23` Sem quadriculado | — | `ClickOption` ou Segue |
| 15 | `40` Uau, D+ | — | escolha: 0 → 14; 1 → 16 |
| 16 | `41` Tudo pronto | — | escolha: 0 → 3; 1 → volta ao andar |

As figuras exatas de cada passo (há variantes `3829`, `382b`, `3834`,
`3835`, `3839`, `383c`, `3857`, `3858`, `3859`, `3865`) devem ser
escolhidas olhando as PNGs extraídas; registrar a escolha em
`docs/telas/fine-artist.md`.

## Roteiro 3-D (fase 2, depois das Super-ferramentas e objetos)

`WHAT 4000`: 0 (escolha) → 50 Intro → 3 → 4/5 (com/sem profundidade:
figuras `3e0b`/`4008`…) → 6 (ponto de vista: `40bb`…) → 9/11 → 22/23/29/36/42
(sobreposição e detalhe: cliques nas tigelas e caixas, figuras `4053`…
`4088`) → 44 resumo → 47 (ferramentas? ) → 48/49/51/52/53/55 (ponto de fuga)
→ 60–62 (interruptor) → 63–67 (mesa com o pincel Linha) → 70–82 (adesivos
do café: rolo `109c`) → 100–103 (mover o PF) → 126 (arrumar) → 137–148
(linhas guia, Aproximar do PF) → 155–161 (grudar) → 164–170 (cavalete,
lados) → fim. Figuras `40bb`…`4111` (177×148) mostram cada etapa da mesa.

## Testes

- O roteiro básico percorre do passo 0 ao 16 com os eventos certos
  (clique na opção 0, Segue, clique no painel…) e termina em
  `ChangeScene(Projects floor)`.
- Um clique fora do alvo durante `ClickTool` não avança.
- O painel da Maggie aparece só nos passos com figura e o PNG guardado não
  o contém (nem a silhueta de fundo? — a silhueta **é** pintada no quadro,
  como no original "Já comecei para você").
