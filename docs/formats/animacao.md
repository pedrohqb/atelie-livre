# Desenho animado gravado (`animacao-…/`)

Formato próprio do Ateliê Livre para o projeto **Desenho animado** da
Oficina de Projetos. O original grava `*.SLD`, de que o CD não traz
amostras; o Ateliê Livre grava uma pasta aberta, legível sem o programa.

Ao sair do projeto (porta "Saída", Ctrl+Q ou fechando a janela), se houver
alguma figura e algo mudou, o programa grava na pasta de desenhos
(`~/.local/share/atelie-livre/desenhos/`) duas pastas iguais:

- `animacao-AAAAMMDD-HHMMSS/` (data e hora em UTC), uma por sessão;
- `ultimo-animacao/`, reaberta ao entrar de novo no Desenho animado.

## Conteúdo da pasta

| Arquivo | Conteúdo |
|---|---|
| `figura-1.png` … `figura-N.png` | Cada figura, 594×347, PNG RGB de 8 bits (as cores vêm da paleta de 256 cores do programa). Numeradas na ordem dos espaços cheios, sem buracos. |
| `animacao.json` | Lista das figuras com os ajustes e o intervalo (abaixo). |
| `animacao.gif` | GIF animado com tudo renderizado: transições, legendas e o intervalo; para ver fora do programa. |

Ao regravar `ultimo-animacao/`, as `figura-*.png` antigas são apagadas antes.

## `animacao.json`

```json
{
  "figuras": [
    {"arquivo": "desenho-20261031-153012.png", "efeito": "corte", "som": "", "legenda": "Era uma vez"},
    {"arquivo": "Praia.mag", "efeito": "dissolver", "som": "Agudo/3", "legenda": ""},
    {"arquivo": "desenho-20261031-153400.png", "efeito": "dado", "som": "AnimalDeFazenda/0", "legenda": "Fim"}
  ],
  "intervalo_ms": 1000,
  "espacos": 4
}
```

- `arquivo`: nome do arquivo de origem (só informativo; a figura em si é a
  `figura-N.png` de mesma posição).
- `efeito`: `corte`, `dissolver`, `cortina-esquerda`, `cortina-direita`,
  `cortina-cima`, `cortina-baixo` ou `dado` (sorteia um dos outros a cada
  "tocar"; no GIF, um sorteio fixo).
- `som`: `Categoria/índice` dentro de `MAG100.SND` (as categorias da
  ferramenta de sons, ex.: `Agudo`, `Dinossauro`), ou vazio.
- `legenda`: texto curto mostrado numa faixa branca de 16 px na base da
  figura ao tocar; `"` e `\` vêm escapados com `\`.
- `intervalo_ms`: tempo por figura: 4000, 2000, 1000, 500 ou 250 (as cinco
  paradas da régua TEMPO).
- `espacos`: quantos espaços numerados a tira tinha (4 a 10; a seta "Mais
  espaços" acrescenta um de cada vez). Ao reabrir, a tira tem pelo menos
  esse tanto, e nunca menos que as figuras.

O leitor é tolerante: cada figura é um objeto numa linha; uma figura sem o
seu PNG é pulada; um efeito desconhecido vira `corte`; um intervalo fora da
lista mantém o padrão (1000 ms); sem `espacos`, a tira tem 4 espaços ou o
tanto de figuras.

Enquanto uma figura está aberta na tela de pintura (clique duplo), ela fica
em `ultimo-animacao/figura-em-edicao.png`; a porta de saída da pintura
regrava o arquivo e, de volta ao desenho animado, ele entra no lugar da
figura e é apagado.

## `animacao.gif`

GIF89a de 594×347 gerado por `crates/formats/src/gif.rs` (codificador LZW
próprio, sem dependências):

- tabela global de 256 cores = a paleta de 256 cores do `ARTIST.EXE`;
- extensão `NETSCAPE2.0` com laço infinito;
- para cada figura: se há efeito (e uma figura anterior), 12 quadros de
  transição de ~4 centésimos cada (0,5 s no total, ou metade do intervalo,
  se menor) seguidos do quadro parado pelo resto do intervalo; com `corte`,
  só o quadro parado pelo intervalo inteiro;
- sem transparência e sem tabelas locais; cada quadro cobre a tela toda.
