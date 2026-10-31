# Ateliê Livre — Subprojeto 3a: Creative Writer, o Escritório (editor, primeira versão)

Data: 2026-10-31
Status: decidido pelo agente (Pedro: "vai indo até o fim do port sem minha intervenção")
Depende de: 2 (Fine Artist) — reaproveita Assets, Frame, Balloon, fonte Pablo, clip-art, cenas

## 1. O original (medido no Wine)

- `WRITER.EXE` + `WRITER/WRITER.PIC` (2.311 imagens) + `WRITER.SND`. Mesma
  entrada (cadastro de usuário) e mesmo prédio; a sala de trabalho é o
  **Escritório**: uma página branca em x 23..615, y 108..456 (mesma moldura
  da pintura: barra de ferramentas em y 5..55, faixa de opções em y 56..104,
  barra inferior em y 461..474 com "Escritório", o título "Documento Novo",
  um ícone de página e "Pág. 1" com setas).
- Barra de ferramentas (`WRITER.PIC`, normal/ativo): Texto `0251` em (17,12);
  Adesivos `01ff` (65,12); Sons `0205` (110,10); Página `0800` (159,16);
  Inseto (pular para / corrigir / trocar / dicionário) `091f` (214,11); Ovo
  `0212` (266,6); Tesoura `08e8` (319,9); Mão `0219` (372,9); Livro (zoom)
  `0804` (417,12); Imprimir `0226` (472,11); McZee `0235` (536,47); Saída
  `023d` (581,16).
- Faixa do Texto: Seleciona-letras `0275` (24,61), Dimensiona-palavras `0811`
  (89,61), Seleciona-cores `027b` (154,61), Seleciona-estilos `0279`
  (219,61), Posicionador `0bc3` (284,61), Carimbo-de-linha? `080c` (349,58),
  Inserir `0831` (398,61), FORMA `0269` (443,57), interruptor `0229` (508,59).
  Cada um abre uma lista suspensa sob o botão: tipos de letra (TrueType do
  Windows, que não temos), tamanhos (Gigante, Enorme, Grande, Médio,
  Pequeno, Minúsculo), cores (lista de faixas coloridas), estilos (Simples,
  Negrito, Inclinado, Sublinhado), alinhamento (Esquerda, Direita, Centro,
  Justificado).
- Adesivos: os mesmos rolos de clip-art (`MAXMAG10.ART`, `MAX100.ART`), com
  tira de filme, como na pintura; o adesivo entra no texto na posição do
  cursor, como um caractere grande.
- Ovo: Desfazer / Refazer. Tesoura: recortar, copiar, colar, apagar, e a
  "explosão" (apaga tudo). Mão: mover/alinhar/virar adesivos; "texto
  flutuante". Livro: menos zoom / mais zoom (tamanhos a a a A A), página
  inteira. Página: fundo (molduras `.BOR`), largura, margens, numeração.

## 2. Decisões (primeira versão)

1. **Entrada**: `atelie-livre --program writer` (e `program = "writer"` no
   config) abre o Creative Writer: abertura própria (`WRITER.PIC` tela de
   título, achar o id maior de 640×480 como no Fine Artist) e vai direto ao
   Escritório. Sem o prédio por enquanto. Sem argumento, continua no Fine
   Artist. (Depois: tela de escolha.)
2. **Editor**: documento = lista de parágrafos; cada parágrafo tem
   alinhamento e uma lista de trechos (texto com estilo: tamanho, cor,
   negrito/inclinado/sublinhado) e adesivos inline (clip-art em tamanho
   real, alinhado pela base). Quebra de linha automática na largura da
   página (margens de 24 px). Páginas: a página tem 349 px de altura visível
   e o documento rola com as setas de "Pág." e as teclas PageUp/PageDown;
   "Pág. N" = página atual (altura fixa por página).
3. **Fontes**: Pablo Graphos 16 pt para Médio; Pequeno = 8 pt; Minúsculo =
   6 pt; Grande = 16 pt ampliada 2×; Enorme = 3×; Gigante = 4× (vizinho mais
   próximo). Negrito = desenha deslocado 1 px para a direita também;
   Inclinado = cisalhamento de 1 px a cada 4 linhas; Sublinhado = linha 1 px
   abaixo da base. Só um tipo de letra (a lista de "Seleciona-letras" mostra
   "Pablo Graphos" e explica que os outros tipos ainda não existem).
4. **Cores**: as 38 da pintura (`COLORS` de `scenes/paint.rs`), na lista
   suspensa de cores, em faixas.
5. **Teclado**: `Input::Text`, Backspace, Enter (novo parágrafo), setas
   (cursor), Home/End; seleção com Shift+setas fica para depois; Ctrl+Q
   sai. Ovo desfaz/refaz uma ação (uma ação = tudo digitado desde a última
   pausa de 1 s ou mudança de parágrafo/estilo).
6. **Guardar**: ao sair, `~/.local/share/atelie-livre/textos/ultimo.txt`
   (texto puro, parágrafos separados por linha vazia, adesivos como
   `[adesivo:ROLO:N]`) e `texto-AAAAMMDD-HHMMSS.txt` se mudou; ao entrar,
   abre `ultimo.txt`. Formato `.MAX` original fica para depois.
7. **Fora do escopo**: sons, inseto (corretor), tesoura (além de apagar
   tudo), mão, zoom, página (fundos/molduras), imprimir, McZee falando.

## 3. Geometria

Igual à pintura (moldura, barra, faixa, barra inferior) com os ids de
`WRITER.PIC` acima; página branca (23,108)–(615,456) com borda preta de 1 px
fora; cursor de texto = linha vertical de 2 px na cor atual, piscando a 0,5 s.
