# Metarquivos do Windows (parte `META`)

As figuras dos `CHNK` (`PICT n` nas bibliotecas `.ART`, nos `.BOR`, no
`WORKSHOP.CHK` e nos documentos `.MAX`) e as peças das bordas (`WBDR`, ver
[`bor.md`](bor.md)) são metarquivos do Windows 3.x (WMF) precedidos de um
cabeçalho próprio:

```
4 bytes  "META"
i16 x, i16 y     origem da janela lógica (pode ser negativa: −10, −30)
u16 largura, u16 altura
WMF              cabeçalho padrão de 18 bytes (tipo 1, 9 palavras)
                 registros { u32 tamanho em palavras, u16 função, parâmetros }
                 até o registro 0 (EOF); pode haver lixo depois dele
```

Os parâmetros dos registros vêm na ordem do WMF: coordenadas de trás para
frente (`SetWindowExt` = altura, largura; `Rectangle` = base, direita, topo,
esquerda).

## Dois usos

**Figuras** (`PICT`): um único `StretchDIB` (0x0F43, `SRCCOPY`) com um DIB
de 1, 4 ou 8 bits, cercado de `SetWindowOrg/Ext` (iguais ao cabeçalho),
`SetStretchBltMode`, `SetBkMode` e afins. `formats::wmf::meta_dib` só
extrai o DIB (adesivos, rébus, clip-art).

**Peças de borda** (`WBDR`): vetoriais. Registros encontrados nas 24 bordas
(contagem):

| função | registro | uso |
|---|---|---|
| `SetWindowOrg` / `SetWindowExt` | 0x020B / 0x020C | janela (0, 0)–(largura, altura) da peça |
| `SetROP2` | 0x0104 | sempre 13 (`R2_COPYPEN`) |
| `SetBkMode`, `SetBkColor`, `SetTextColor`, `SetTextAlign` | 0x0102, 0x0201, 0x0209, 0x012E | transparente; sem efeito no desenho |
| `SaveDC` / `RestoreDC` | 0x001E / 0x0127 | 54 cada; isolam recortes |
| `IntersectClipRect` | 0x0416 | 17 (Light Bulb…): recorta um traço |
| `CreatePenIndirect` | 0x02FA | 112: estilo 6 (`PS_INSIDEFRAME`) com larguras até 41 unidades, ou 5 (nula) |
| `CreateBrushIndirect` | 0x02FC | 99: sólido (0) ou nulo (1) |
| `DibCreatePatternBrush` | 0x0142 | 6 (Party): padrão 8×8 de 1 bit (pontilhado) |
| `SelectObject` / `DeleteObject` | 0x012D / 0x01F0 | tabela de objetos: cada `Create*` ocupa o menor índice livre |
| `Polygon` | 0x0324 | 277 |
| `Polyline` | 0x0325 | 41 |
| `MoveTo` / `LineTo` | 0x0214 / 0x0213 | 17 |
| `Ellipse` | 0x0418 | 82 |
| `Rectangle` | 0x041B | 27 |
| `RoundRect` | 0x061C | 1 |
| `Arc` | 0x0817 | 16 (Seattle) |

Não aparecem `PolyPolygon`, `Pie` nem `Chord`, mas o rasterizador os trata.
Muitas peças começam com um polígono branco que cobre a célula inteira (o
fundo da peça é opaco).

## Rasterizador do port (`formats::wmf_draw`)

`Wmf::from_meta` lê o cabeçalho e os registros; `Wmf::draw(img, dest)`
toca o metarquivo num `Image` RGBA levando a janela lógica corrente ao
retângulo `dest` (pixels, fracionários) e recortando nele (`draw_clipped`
para outro recorte). Regras, escolhidas para reproduzir o GDI do Windows
3.1 sem suavização:

- um pixel pertence a uma figura quando o seu **centro** está dentro dela
  (polígonos por varredura, regra par-ímpar ou de voltas conforme
  `SetPolyFillMode`; elipses, retângulos e retângulos arredondados por
  teste de ponto);
- caneta com largura convertida pela escala horizontal, no mínimo 1 pixel;
  `PS_INSIDEFRAME` (e a de 1 pixel) fica **dentro** da caixa de
  `Rectangle`/`Ellipse`/`RoundRect`/`Pie`/`Chord`; nas outras figuras e nas
  outras canetas ela é centrada na linha, com pontas e junções redondas;
  linhas de 1 pixel por Bresenham;
- arcos no sentido anti-horário da tela, do raio do ponto inicial ao do
  final (invertido se a janela espelha um eixo);
- pincéis: sólido, nulo, hachurado (`HS_*` em células de 8 pixels; fundo
  conforme `SetBkMode`) e padrão DIB ladrilhado a partir da origem do
  `Image`;
- `SaveDC` guarda caneta, pincel, recorte, janela e modos; `RestoreDC(n)`
  aceita `n` negativo (relativo) e positivo (absoluto); `DeleteObject`
  libera o índice mas não desfaz a seleção;
- `StretchDIB`, `DIBStretchBlt` e `DIBBitBlt` por vizinho mais próximo, com
  as ROPs `SRCCOPY`, `SRCAND`, `SRCPAINT`, `SRCINVERT`, `NOTSRCCOPY`,
  `MERGEPAINT` e `DSna` (0x220326; as demais copiam); a origem `ySrc` conta a
  partir da base do DIB;
- ROP2 1, 4, 6, 7, 9, 11, 13, 15, 16; as demais copiam;
- `SetViewport*`, `SetMapMode`, texto, paletas e regiões são ignorados
  (`CreateFontIndirect`, `CreatePalette`, `CreateRegion` só ocupam a
  tabela); `ExcludeClipRect` não é tratado.

Validação: as 24 bordas comparadas às suas miniaturas (`real_bor.rs`;
Checkered 100 %, Tribe 99 %) e as páginas comparadas às capturas do
original (`bor_compare.png`).
