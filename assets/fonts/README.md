# Fontes embutidas (substitutas das TrueType do Creative Writer)

O Creative Writer usa cinco TrueType que vinham nos disquetes de instalação
(Lucida Sans, Lucida Handwriting, Lucida Blackletter, Impact e Stencil). Elas
não podem ir com o Ateliê Livre; estas fontes livres as substituem
(`crates/engine/src/writer/fonts.rs`, recurso `embedded-fonts` do crate
`engine`, ligado por padrão). Escolha e medidas: `docs/telas/writer-fontes.md`.

| Arquivo | Substitui | Origem | Licença |
|---|---|---|---|
| `DejaVuSans-cp1252.ttf` | Lucida Sans | DejaVu Sans 2.37 (`fonts-dejavu-core`), só os caracteres do Windows-1252, sem hinting (ver abaixo) | Bitstream Vera + domínio público: `LICENSE-DejaVu.txt` |
| `Merienda-wght.ttf` | Lucida Handwriting | `google/fonts`, `ofl/merienda/Merienda[wght].ttf`, sem alteração (só o nome do arquivo) | SIL OFL 1.1: `OFL-Merienda.txt` |
| `UnifrakturMaguntia-Book.ttf` | Lucida Blackletter | `google/fonts`, `ofl/unifrakturmaguntia/`, sem alteração | SIL OFL 1.1: `OFL-UnifrakturMaguntia.txt` |
| `Anton-Regular.ttf` | Impact | `google/fonts`, `ofl/anton/`, sem alteração | SIL OFL 1.1: `OFL-Anton.txt` |
| `StardosStencil-Bold.ttf` | Stencil | `google/fonts`, `ofl/stardosstencil/`, sem alteração | SIL OFL 1.1: `OFL-StardosStencil.txt` |

As fontes OFL vão sem nenhuma alteração (a OFL proíbe usar os nomes
reservados em versões modificadas). A licença da Bitstream Vera permite
modificar desde que o nome não contenha "Bitstream" nem "Vera"; o recorte da
DejaVu Sans foi feito com:

```
pyftsubset DejaVuSans.ttf --unicodes-file=cp1252.txt --no-hinting \
  --layout-features='' --name-IDs='*' --name-languages='*' \
  --notdef-outline --output-file=DejaVuSans-cp1252.ttf
```

com `cp1252.txt` = U+0020–U+007E, U+00A0–U+00FF e os 27 caracteres de
0x80–0x9F do Windows-1252 (€, ‚, ƒ, „, …, †, ‡, ˆ, ‰, Š, ‹, Œ, Ž, ‘, ’, “, ”,
•, –, —, ˜, ™, š, ›, œ, ž, Ÿ) — os mesmos que o documento `.MAX` guarda.
