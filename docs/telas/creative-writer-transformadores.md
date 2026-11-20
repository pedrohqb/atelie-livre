# Creative Writer — Transformadores (W13)

Ferramenta 8 do Escritório, a mão com tinta (`scenes/writer/transformers.rs`;
pesquisa em `office-ferramentas.md` §7, capturas `office_106`..`office_114`;
balões `WHAT 0fa8`, avisos do Rex `0805`). Os botões da faixa agem no clique.

| botão | figura (cor/máscara) | faz | som | balão |
|---|---|---|---|---|
| Mover-modificar | `0923`/`0929` (33,58) | modo padrão ao escolher a ferramenta | `021f` | `0fa8[1]` |
| Dimensionar tudo | `0857`/`0859` (94,54) | modo de esticar por igual | `0233` | `[2]` |
| Dimensionar lateral | `085b`/`085d` (150,54) | só a largura | `0233` | `[3]` |
| Dimensionar vertical | `085f`/`0861` (187,54) | só a altura | `0233` | `[4]` |
| Agente Secreto 009 | `0807`/`0809` (278,59) | abre o painel SENHA | `0233` | `[5]` |
| Coloca-figuras (Maçã) | `0865` (340,61) | põe as figuras (Ctrl: tira) | `0233` | `[12]` ao pôr |
| Flutua-texto | `08c7`/`08cb` (432,56) | abre a lista dos 4 modos | `0281` (= `0295`) | `[7]` |

## Mover-modificar

- Arrastar um adesivo, desenho ou palavra com forma (não as molduras de página
  dos cartões) o move; o arrasto começa depois de 2 pixels (`022e`) e é uma
  ação do Ovo. O objeto não sai da folha: fica sempre um canto de 8 pontos
  dentro (`FloatObject::move_by`).
- Um clique sem arrastar leva o **adesivo ou desenho** para o Cavalete de
  lápis de cera (`scenes/writer_easel.rs`, sons `0224` + `0267`) com a
  figura no tamanho atual; OK troca a figura (`Editor::replace_picture`: um
  `PICT` novo com `META` + `TWPB`, o objeto vira desenho, no mesmo lugar e com
  o mesmo modo de flutuação, som e borda; uma ação do Ovo); NÃO não muda
  nada. A **palavra com forma** iria para o cavalete de palavras com forma
  (W7): por enquanto o clique só fica no registro.
- Ctrl+arrastar estica por igual (o atalho do balão `0fa8[15]`).
- Cursores: mão aberta `0016` sobre um objeto, mão fechada `0018` arrastando;
  fora dos objetos o clique põe o ponto de inserção, como no texto.

## Dimensionar

Arrastar sobre o objeto, com o **centro fixo**; `022a` toca em laço
(`PlayLoop`) enquanto arrasta e para ao soltar. Medidas a partir do clique:

- tudo: a largura e a altura multiplicadas por distância ao centro agora ÷
  distância ao centro no clique (mínimo de 4 px);
- lateral: largura + 2 × (quanto o ponteiro se afastou do centro na
  horizontal); vertical: o mesmo com a altura (pegando acima do centro,
  "arrastar para cima aumenta", como diz o balão `[4]`).

Com Ctrl, lateral e vertical também esticam por igual. Lado mínimo de 8
pontos. O ponteiro de dimensionar (`000e`, `000d`, `000c`) vale em toda a
página até trocar de ferramenta (`0fa8[10]`): clicar fora dos objetos não
mexe no texto.

## Agente Secreto 009

Painel pendurado na faixa (moldura laranja (113,106) 322×63): placa `08d7`
(118,112) com o campo branco (137,128) 170×19 e a etiqueta SENHA; NÃO `0271`
(345,124) e OK `026d` (388,125) no fundo cinza. Enquanto aberto, o teclado vai
para o campo (até 32 caracteres; Backspace apaga, Enter = OK, Esc = NÃO).

- OK com o documento normal: embaralha (`02ee`, balão `[11]` "Para
  decodificar…"). O texto **não muda**: o `.MAX` grava `ADD[5] = 1` e a
  senha com +5 (`docs/formats/max.md`); só a tela troca as letras
  (`writer::cipher`: cada caractere de `0-9A-Za-z` anda 31 casas, a mesma
  troca para qualquer senha; acentuadas, pontuação e espaços ficam) e cobre
  os objetos e as figuras da Maçã com quadrados pretos. A diagramação mede as
  letras trocadas (é o que o original desenha).
- OK com o documento embaralhado: a mesma senha (ou a senha-mestra
  "Detetive McZee para salvar o dia" com Ctrl+Shift apertados) desembaralha
  e apaga a senha; senha errada: Epa! `0805[38]` e o campo esvazia.
- Senha vazia: `0805[43]` (para embaralhar) ou `[37]` (para desembaralhar).
- Com o documento embaralhado, Ctrl+X e Ctrl+C são recusados (`0805[13]`).
  O Jogar tudo fora dos Re-movedores deve recusar com `[42]` (W12).
- Embaralhar e desembaralhar não entram no Ovo (não mudam o documento).

## Coloca-figuras (Maçã)

A tabela palavra → figura vem do `WRITER.PIC` (`formats::maca`,
`docs/formats/maca.md`), lida no primeiro clique. `writer::rebus_apply`
percorre o documento inteiro (com ou sem seleção) e põe `Item::Rebus(n)` logo
depois de cada palavra conhecida (letras, acentuadas e `-`; em minúsculas),
antes da pontuação, pulando as que já têm figura. Uma ação do Ovo; a seleção
acompanha o texto.

- Pôs alguma: balão `0fa8[12]` ("Quer desfazer?").
- Nada novo, mas há palavras conhecidas: Epa! `0805[4]` (`02fe`).
- Nenhuma palavra conhecida: `0805[3]`. Sem a tabela: `0805[6]`.
- Ctrl+clique tira todas as figuras (uma ação do Ovo).

Exemplo (dados reais): "O gato e a bicicleta foram passear no parque." →
figuras `0x25`, `0x0f`, `0xeb`, como no `.MAX` gravado pelo original.

## Flutua-texto

Lista branca (221,98) 217×221 com as figuras `0c2b`, `0c2d`, `0c2f`, `0c31`
(186×42) em x 245, y 105/160/215/270 (separadores em 153/208/263). Escolher
um modo (`022d`) fecha a lista; daí em diante, clicar num objeto aplica o modo
(`0233`, uma ação do Ovo; o cursor é a bolinha `000b`). Clicar fora da lista
só a fecha. Os modos (`object::Wrap`) e o contorno do texto já existem na
diagramação: em volta das bordas (normal), em todas as páginas, quadrado em
branco, acima e abaixo.

## Ainda falta

- Os botões da faixa não mostram qual modo está escolhido (as capturas não
  mostram diferença) e, com o painel ou a lista abertos, o original desenha o
  resto da tela com as figuras de preto e branco.
- Cavalete de palavras com forma ao clicar numa palavra com forma (W7).
- O código do sprm `0x29` (modo de flutuação) continua provisório (W26).
