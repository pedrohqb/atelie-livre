# Fontes TrueType do original e substitutos no port

O Creative Writer usa a Pablo Graphos (`SHARED/STUFF/PABLO.FON`, lida pelo
port) e 5 fontes TrueType que o Setup instalava no `WINDOWS\SYSTEM` e que
**não estão nos dados**: Lucida Sans (fonte padrão do documento: `STR#`
0x800 [1] e ftc 0 do `FONT` de todo `.MAX`), Lucida Handwriting, Lucida
Blackletter, Impact e Stencil (`STR#` 0x206). Sem elas o original se recusa
a iniciar (`STR#` 0x806 [3]; no Fine Artist o diálogo `TTDIALOG`, ver
`winres-ui.md`).

O port não pode distribuí-las. `engine::paint::ttf::substitute(nome)`
escolhe, entre as fontes instaladas (`ttf::installed`, só faces normais),
a mais parecida, nesta ordem:

1. a própria fonte, se o usuário a tiver instalado;
2. o primeiro substituto instalado da tabela abaixo;
3. outra fonte da mesma família ("Gill Sans" → "Gill Sans MT");
4. o alias do fontconfig (`fc-match`), quando ele não devolve só a fonte
   padrão dele (a mesma que daria para um nome inexistente);
5. uma fonte genérica do mesmo estilo (sem serifa: Liberation Sans, Arimo,
   DejaVu Sans, Noto Sans…; serifada: Liberation Serif, Tinos, DejaVu
   Serif…; mono: Liberation Mono, Cousine, DejaVu Sans Mono…; manuscrita:
   Comic Neue, Kalam, Z003…, depois as sem serifa);
6. a fonte padrão do cavalete (`ttf::default_face`).

Nomes são comparados sem diferença de maiúsculas, espaços ou hifens; o
resultado de cada nome fica guardado. A função pura
`ttf::substitute_in(nome, faces, fontconfig)` faz os passos 1–5 sobre
qualquer lista (é a que os testes usam).

| Original | Estilo | Substitutos, em ordem |
|---|---|---|
| Lucida Sans | sem serifa | Lucida Sans Unicode, DejaVu Sans, Bitstream Vera Sans, Open Sans, Verdana, Noto Sans |
| Lucida Handwriting | manuscrita | Comic Neue, Kalam, Comic Sans MS, Comic Relief, Z003, URW Chancery L, TeX Gyre Chorus |
| Lucida Blackletter | gótica (→ serifada) | UnifrakturMaguntia, UnifrakturCook, Old English Text MT |
| Impact | letreiro (→ sem serifa) | Anton, Oswald, League Gothic, Bebas Neue, Haettenschweiler, DejaVu Sans Condensed |
| Stencil | letreiro (→ sem serifa) | Stardos Stencil, Allerta Stencil, Saira Stencil One, Black Ops One |
| Arial | sem serifa | Liberation Sans, Arimo, Nimbus Sans, Helvetica, FreeSans |
| MS Sans Serif | sem serifa | Microsoft Sans Serif, Liberation Sans, Arimo, DejaVu Sans |
| Times New Roman | serifada | Liberation Serif, Tinos, Nimbus Roman, FreeSerif |
| Courier New | mono | Liberation Mono, Cousine, Nimbus Mono PS, FreeMono |
| Comic Sans MS | manuscrita | Comic Neue, Comic Relief, Kalam |

As linhas de Arial em diante cobrem documentos gravados em outras máquinas;
no Wine sem as fontes do produto o original grava "Liberation Sans" na
tabela `FONT` do `.MAX`, que, instalada, é usada direto (passo 1).
