# Truques de Desenho — a lição na tela de pintura

Complementa a spec `docs/superpowers/specs/2026-10-31-truques-design.md`
com o comportamento do assistente (`scenes/wizard.rs`) e dos roteiros
(`scenes/tricks_basic.rs`, `scenes/tricks_3d.rs`) depois da varredura de
cliques contra o original (`WHAT 3800` e `4000`).

## Balão do passo

- Texto, **dicas** (lâmpada + título, ligações `21b` do registro) e
  **opções** (bolinhas), nessa ordem. Clicar numa dica abre o registro
  ligado como balão de ajuda (`HelpBalloon`) à direita do balão da lição;
  um clique fora fecha a dica (o clique não vai para a pintura).
- Palavras do glossário (`ffff`) viram o título do registro ligado (o
  parâmetro pode vir com o bit 15 ligado: `& 0x7fff`).
- Botões Volta / Pára / Segue sob o balão. Segue aparece nos passos
  `Next`, `PaintThenNext` e `Pictures`.
- Passo `TypeName` ("Digite um nome…"): o balão mostra `> nome_`; as teclas
  vão para a lição (Backspace apaga, Enter grava). O nome vira
  `<nome>.png` e `<nome>.mag` (com o nome como título) na pasta de
  desenhos, pelo mesmo `store::name_stem` dos quadros com nome (sem
  barras, dois-pontos etc.; sem sobrescrever: `-2`, `-3`…); o último
  desenho (`ultimo.*`) não muda. Depois de gravar com nome, sair não grava
  a cópia `truque-AAAAMMDD-HHMMSS.png`. "Não, jogar fora" também não grava.
- Lâmpada de título: nos registros de dica a primeira linha traz uma
  lâmpada que aponta para o registro 0 (`21B0000`) ou para o próprio
  registro; ela é só enfeite do título, que fica no texto. As outras
  lâmpadas viram ligações.

## Maggie e McZee

- Passos que esperam a Maggie (`Maggie`, `FindShapes`, `Click(Panel…)`):
  clique no painel ou no McZee do nicho.
- Em qualquer outro passo, clicar na Maggie (painel) ou no McZee abre
  `3803[0]` ("O que você quer que eu faça agora?"):
  1. *Deixar você trabalhar por conta própria* — a lição **pausa**: sem
     balão, painel nem alvo; a ajuda normal das ferramentas volta a
     funcionar; clicar no McZee retoma o passo.
  2. *Parar de trabalhar aqui* — volta ao andar dos Truques.
  3. *Ignorar isso* — fecha.
- Pára abre `3803[4]` como antes.

## Alvos

| alvo | avança com | destaque piscante |
|------|-----------|-------------------|
| `Tool(i)` | clique na ferramenta `i` | a ferramenta |
| `Option { tool, page, k }` | clique no botão `k` **com a faixa de `tool` aberta** (e na página `page`, se houver) | o botão; com outra faixa aberta, a ferramenta; na página errada, a seta de página |
| `Page(tool, page)` | a página aparecer (setas) | a seta de página |
| `Switch` | clique no Interruptor mágico (Super-ferramentas abertas) | o interruptor (ou a ferramenta) |
| `Panel` / `PanelArea` | clique no painel (na área) | o painel |

Assim "Clique na ferramenta Ponto de fuga" não avança (nem abre nada) com
a faixa dos adesivos aberta, e o pincel Linha exige a página 5 do pincel.

## Passo das 3 formas (`FindShapes`)

As "3 formas circulares meio achatadas" do Chiquinho são elipses no painel
(`PIG_CIRCLES`: cabeça, corpo, focinho). Cada uma conta uma vez; a terceira
avança. Clicar no McZee ("se você ficar encalacrado, clique em mim") mostra
as elipses piscando; as já achadas ficam verdes.

## Botões de rádio (`Pictures`)

Nos passos "Com profundidade / Sem profundidade" e "Bem no meio / Um pouco
mais para cima / Bem lá em cima" as opções do balão trocam a figura do
painel (`3e0b`/`4008`; `403b`/`4053`/`4059`); Segue continua.

## Animações do painel (`PanelAnim`)

Ao avançar por um clique no painel, uma figura recortada do `.PIC` (com a
máscara em `id + 8`) desliza ou aparece de cima para baixo e fica até o
passo seguinte acabar:

- `4000[29]` tigela da frente (`4069`, de (27, 218) para (0, 150)): "veja
  que ela se move para a frente do leite";
- `4000[42]` embalagem de leite com detalhes (`40a3` em (20, 73), de cima
  para baixo): "veja o que acontece à medida que são colocados mais
  detalhes".

## Fim guiado

Básico: `40` Uau → `24` salvar? → (`25` nome → `27` imprimir?) ou (`26`
imprimir antes do lixo?) → `29` "Chocante! O que você quer fazer em
seguida?": nova figura (volta à Intro) / **figura existente** (abre a pasta
de trabalhos por cima; o clique duplo abre o desenho na pintura livre) /
voltar ao andar.

3-D, antes dos lados: `148` cópia de trás (Maggie) → `155` "Vamos juntar
os dois adesivos" → `156` Transformadores → `157` seta até a página 2 →
`159` Grudar ou Desgrudar → `161` clique nos dois adesivos (a corrente
mágica) e na Maggie.

3-D: `168` lados (Maggie) → `188` Uau (terminar sozinho = pintura livre /
usar os passos do leite como guia = volta aos adesivos / próximo passo) →
`187` sem linhas guia? (→ `189` Super-ferramentas → `190` Interruptor) →
`191` cores (Maggie) → `194` salvar? → `196`/`198` ou `199` → `202` "Oba!
O que você quer fazer em seguida?" (nova / tornar 3-D um desenho existente
= pasta / andar). "Não, obrigado" às ferramentas especiais vai a `230`
"Tudo pronto!".

## O que ficou de fora (e por quê)

- Sub-roteiro do **Cavalete de adesivos** (`3800[5,16,17,19,21]`,
  `4000[164]–[167]`, `[173]`): o cavalete (Coloca-adesivos) não existe no
  port; no 3-D os lados da embalagem (`[168]`) são desenhados com o pincel
  Linha direto no quadro.
- Verificar o Ctrl+arrasto do `[148]` e a corrente do `[161]`: os dois
  passos avançam pela Maggie ("clique em mim quando terminar").
- Recapitulações "Truques básicos/3-D #1/#2" ao reabrir uma figura
  existente (`3800[34]–[39]`, `4000[205]–[229]`): a pasta abre a pintura
  livre, sem a Maggie.
- Verificação dos adesivos colocados (`[73]`–`[82]`): só Segue/Maggie.
