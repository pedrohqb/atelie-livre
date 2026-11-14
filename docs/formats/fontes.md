# Fontes usadas pelo Creative Writer

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

## Lista do programa (`STR#` 0x206)

| Nome | Tipo | Onde está | Uso |
|---|---|---|---|
| Pablo Graphos | bitmap `.FON` | `SHARED/STUFF/PABLO.FON` | balões, barra de status, menus desenhados, nomes dos tamanhos; também opção do Seleciona-letras |
| Lucida Sans | TrueType | instalada no `WINDOWS\SYSTEM` pelo Setup (não está nos dados) | **fonte padrão do documento** (`STR#` 0x800 [1]; ftc 0 do `FONT` de todo `.MAX`) |
| Lucida Handwriting | TrueType | idem | Seleciona-letras, Splot |
| Lucida Blackletter | TrueType | idem | idem |
| Impact | TrueType | idem | idem (Splots dos exemplos de jornal) |
| Stencil | TrueType | idem | idem |
| `pablo` | — | — | nome interno (provável chave da Pablo Graphos) |

`STR#` 0x806 [3]: "Eu não consigo achar as fontes TrueType…" — o programa
recusa iniciar sem elas. No Wine sem essas fontes o Seleciona-letras mostra
5 vezes "Liberation Sans" (`shots/formats_menu_fontes.png`) e o `.MAX`
grava "Liberation Sans" na tabela `FONT` (ver `max.md`).

## `PABLO.FON`

DLL NE de fontes (`FONTDIR` + 3 recursos `FONT`, versão 2.00, conjunto
ANSI, caracteres 32–255), nome de face **"Pablo Graphos"**:

| Recurso | Pontos | Altura (px) | Ascendente | Largura |
|---|---|---|---|---|
| 0x29 | 6 | 12 | 9 | proporcional |
| 0x2a | 8 | 13 | 11 | proporcional |
| 0x2b | 16 | 21 | 17 | proporcional |

O leitor `formats::fnt` do port já decodifica (v2 e v3) e o motor a usa
(`Assets`, `SHARED/STUFF/PABLO.FON`). Os nomes de tamanho no menu do
Dimensiona-palavras (Gigante…Micro) são desenhados com ela.

## Tamanhos do texto do documento

Dimensiona-palavras (`shots/formats_menu_tamanhos.png`) e sprm `0x0f`:

| Item | Pontos |
|---|---|
| Gigante | 48 |
| Enorme | 36 |
| Grande | 24 |
| Média | 18 |
| Pequena | 14 |
| **Mínima** (padrão) | 12 |
| Micro | 9 |

O port hoje (`engine::writer::doc::Size`) tem 6 tamanhos com outros nomes
(Gigante, Enorme, Grande, Médio, Pequeno, Minúsculo) e desenha tudo com a
Pablo Graphos ampliada; para fidelidade são 7 tamanhos com esses nomes,
em pontos, numa fonte TrueType.

## Substitutos livres para o port

As TrueType do produto não podem ir com o port. O `engine::paint::ttf` já
lista as fontes instaladas no sistema; falta uma tabela de substituição por
nome (usada ao ler o `FONT` de um `.MAX` e ao montar o Seleciona-letras):

| Original | Substituto sugerido (métricas/estilo parecidos) |
|---|---|
| Lucida Sans | DejaVu Sans / Open Sans |
| Lucida Handwriting | Comic Neue / Kalam |
| Lucida Blackletter | UnifrakturMaguntia |
| Impact | Anton / Oswald Bold |
| Stencil | Stardos Stencil / Allerta Stencil |

Se a fonte original estiver instalada (cópia do usuário), usar a original.
