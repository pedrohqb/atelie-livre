# `SCRD` — formas do Splot (palavras com forma)

Recurso dos executáveis (`ARTIST.EXE` e `WRITER.EXE`, ids 1 a 14, **dados
idênticos** nos dois; só o preenchimento do NE muda). Cada `SCRD` é uma
forma da lista FORMA do Cavalete de palavras com forma ("Splot"): duas ou
três curvas (de cima, do meio, de baixo) dadas por pontos. Leitor:
`formats::scrd`; uso: `engine::paint::shaped`.

Todos os campos são **little-endian**.

## Formato

```
u16  tipo         0 = três curvas, 1 = metade direita, 2 = metade esquerda, 3 = duas curvas
u16  1 nas de tipo 3, 0 nas demais
u16  n            número total de pontos
u16  1
u16  largura      largura do texto, em unidades da forma (8000 na maioria; 8192, 9424, 11468, 12288)
u16  altura       altura de uma linha de texto, em unidades da forma (1365 a 8000)
u16  c0           pontos de cada curva
u16  c1           0, c0 ou 2·c0 (não usado na leitura)
u16  c2           início da curva de cima (sempre 0)
u16  c3           início da curva do meio (= c2 ou c4 quando não há meio próprio)
u16  c4           início da curva de baixo
n × { i16 x, i16 y }   centrados em (0, 0), y para baixo
```

| Id | tipo | n | c0..c4 | Forma (lista FORMA) | nome no port |
|---|---|---|---|---|---|
| — | — | — | — | retângulo (1º item, sem `SCRD`) | `retangulo` |
| 1 | 3 | 322 | 161, 0, 0, 0, 161 | arco embaixo (cima reta, baixo em arco) | `arco-embaixo` |
| 2 | 3 | 60 | 30, 30, 0, 30, 30 | onda | `onda` |
| 3 | 0 | 603 | 201, 201, 0, 201, 402 | inflado (losango com lados retos) | `inflado` |
| 4 | 2 | 303 | 101, 101, 0, 101, 202 | cresce para a direita | `cresce` |
| 5 | 1 | 303 | 101, 101, 0, 101, 202 | encolhe para a direita | `encolhe` |
| 6 | 3 | 12 | 6, 6, 0, 6, 6 | zigue-zague | `zigue-zague` |
| 7 | 3 | 6 | 3, 3, 0, 3, 3 | triângulo (ponta para cima) | `triangulo` |
| 8 | 3 | 1503 | 501, 1002, 0, 1002, 501 | seta | `seta` |
| 9 | 0 | 387 | 129, 129, 0, 129, 258 | arco-íris (três arcos concêntricos) | `arco-iris` |
| 10 | 0 | 579 | 193, 193, 0, 193, 386 | S (três curvas paralelas) | `esse` |
| 11 | 3 | 78 | 39, 0, 0, 39, 39 | coração | `coracao` |
| 12 | 3 | 1088 | 544, 0, 0, 544, 544 | carro | `carro` |
| 13 | 3 | 100 | 50, 0, 0, 50, 50 | peixe | `peixe` |
| 14 | 3 | 628 | 314, 0, 0, 314, 314 | cabeça de perfil | `cabeca` |

Cada curva tem `c0` pontos e começa no índice `c2` (cima), `c3` (meio) e
`c4` (baixo). Só os `SCRD` 3, 4, 5, 8, 9 e 10 têm curva do meio própria; nos
outros `c3` repete `c2` ou `c4`. O meio fica sempre (quase) na média de cima
e de baixo.

## Como o original usa (conferido no Wine, nov/2026)

Capturas (fora do repositório, derivadas dos dados originais):
`/home/claude/sweep/splot_res/shots/o_*`; folhas comparativas original × port
`folha_formas.png` e `folha_menus.png` na mesma pasta.

- **Ordem da lista**: retângulo e depois os `SCRD` 1 a 14, por id. A lista
  mostra 9 casas e, ao abrir, rola para deixar a forma atual em cima (no
  máximo até a 6ª, para as 9 últimas caberem).
- **Distribuição pelo índice**: a fração `u` da linha (0 = começo do
  texto) cai no ponto `u·(c0−1)` das curvas, não numa posição x. Por isso o
  `SCRD 4`, com pontos mais juntos à esquerda, deixa "Palavr" pequeno e
  "as" grande (efeito de perspectiva), e o arco-íris e o S, cujas curvas não
  são funções de x, enrolam o texto. A letra fica na reta entre o ponto de
  cima e o de baixo (passando pelo do meio), por isso as hastes inclinam no
  zigue-zague e no arco.
- **Tamanho**: a largura do texto corresponde a `largura` unidades e a
  altura das letras (da tinta: o alto do "P" encosta na curva de cima) a
  `altura` unidades — escalas diferentes em x e y. O arco (8000 × 2000,
  curvas de −2000 a 4000) sai com a largura do texto e 3 vezes a altura das
  letras nas pontas. Os **contornos fechados** (coração, carro, peixe,
  cabeça: `c1 = 0` e `c3 = c4`, a curva de baixo é o caminho de volta)
  guardam a proporção da própria forma (mesma escala em x e y): medidas no
  original, largura/altura 1,11, 1,54, 1,82 e 0,90, contra 1,15, 1,59, 1,87
  e 0,92 das curvas.
- **Prévia**: o cavalete sempre aumenta ou diminui a imagem para encher a
  caixa da prévia.
- **Estilos do `TMEN 1` que mexem na forma**: "Em cima/embaixo" espelha a
  forma na vertical e "Esquerda/direita" na horizontal (as letras continuam
  de pé); "Preencher" faz as letras ocuparem um pouco mais a forma (no
  original a diferença é pequena; o port tira também a folga dos lados do
  texto — interpretação provável, não confirmada). Os três ligam e desligam
  a cada clique e não mudam nada no retângulo.
- O original transforma os vértices do contorno das letras (TrueType), o
  que deixa as bordas retas entre vértices; o port amostra a máscara
  pixel a pixel (bordas curvas) — mesma forma, contorno mais liso.
