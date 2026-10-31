# Animações das telas (roteiros `ATAG` de `ARTIST.EXE`)

`ARTIST.EXE` traz 162 roteiros `ATAG` (formato em `docs/formats/atag.md`;
levantamento com `tools/wine/atag_survey.py`). Este documento diz qual
roteiro toca em qual tela e com que confiança, e como o motor os usa
(`crates/engine/src/anim.rs`, tocador; `crates/engine/src/scenes/gags.rs`,
tabelas e sorteio).

## Como os roteiros foram atribuídos às telas

Três fontes, da mais à menos segura:

1. **Gravações do original no Wine** (`tools/wine/wfa.sh`, 15 quadros/s):
   título, Imaginópolis, rua, saguão, Ateliê, Oficina de Projetos e Truques.
   Onde um roteiro apareceu, as posições quadro a quadro bateram exatamente
   com as posições acumuladas do roteiro (origem + deslocamentos), o que
   também confirma a semântica do tipo 5 do tocador.
2. **Origem e formato das imagens** sobrepostos ao fundo da tela (ex.: o
   buraco de onde o McZee sai no título está em (333, 225) no fundo 0x6ca7).
3. **Agrupamento por id**: os recursos vêm em blocos de ids decimais redondos
   (101–116, 205–212, 300–306, 400–401, 500–505, 600–606, 700, 801–808,
   907–916, 1000–1007, 1111–1129, 1200–1208, 1225–1230, 1250–1254,
   1275–1279, 1301–1305, 1401–1406, 1450, 1500, 1600, 1650, 1702–1707,
   1805–1814, 1900–1907, 14400–14420, 16400–16420), cada bloco com os
   roteiros de uma tela. Os blocos de "travessias" repetem os mesmos bichos
   com origens ligeiramente diferentes, ajustadas ao fundo de cada sala.

As imagens das células são as versões de 1 bit; a colorida é a imagem de id
imediatamente anterior no `.PIC` e a máscara é `id + 1` (vale para todas as
células dos 162 roteiros).

## Mapeamento

Confiança: **alta** = visto na gravação do original; **média** = origem e
imagens batem com o fundo, sem gravação; **baixa** = só pelo agrupamento de
ids (por eliminação).

### Título (fundo 0x6ca7) — recursos 0x65–0x74 (101–116)

| Roteiro | O quê | Origem | Confiança |
|---|---|---|---|
| 0x67 | pássaro vermelho atravessa da direita para a esquerda (48 células de 100 ms, sons 0x217, 0x20a, 0x22f) | (628, 134) | média |
| 0x66 | porco cor-de-rosa atravessa (som 0x239) | (601, 289) | média |
| 0x68 | gafanhoto verde atravessa (som 0x264) | (644, 248) | média |
| 0x69 | menino de patins atravessa (som 0x2fc) | (624, 284) | média |
| 0x6a–0x74 | McZee sai do buraco com um chapéu (onze chapéus: cartola, coroa, fez, capacete...), fica 3 s e volta; sons 0x2f1 e 0x2f4 | (333, 225) | **alta** (sobreposição exata ao buraco do fundo; mesmo roteiro visto na Oficina) |
| 0x65 | porco parado, 200 ms (sobra de autoria; repete em 0x12c, 0x321–0x323, 0x3e8, 0x457–0x45b) | (401, 189) | — |

No Wine o título fica só 3 s antes do cadastro, então os roteiros não chegam
a aparecer na gravação; o motor toca o pássaro primeiro e depois sorteia os
demais enquanto a abertura dura.

### Imaginópolis (mapa, tela ainda não reimplementada) — 0x2bc (700)

McZee no carrinho de supermercado descendo a montanha-russa e sumindo ao
longe ("AAAI!"). **Alta**: visto na gravação logo após a entrada.

### Rua (fundo 0x6d01) — 0x5dc, 0x640, 0x672 (1500, 1600, 1650)

Roteiros gravados em relação a um ponto (origens pequenas); o ponto foi
medido na gravação:

| Roteiro | O quê | Ponto | Confiança |
|---|---|---|---|
| 0x5dc | hidrante treme e esguicha (sons 0x2bc, 0x2be, 0x221, 0x26e, 0x303) | (489, 387) | **alta** (clique no hidrante) |
| 0x672 | lata de lixo amassa, some e volta (sons 0x2f8, 0x30a, 0x2fd, 0x309) | (69, 333) | **alta** (clique na lata) |
| 0x640 | porta do prédio abrindo e fechando (som 0x2f9) | porta (≈ 296, 300) | média (não gravado; o motor não o usa) |

Na gravação a rua ficou 70 s sem animação espontânea: os roteiros tocam ao
clicar.

### Saguão (fundo 0x6a6b) — 0x321–0x328 (801–808)

| Roteiro | O quê | Origem |
|---|---|---|
| 0x324 | canguru de camiseta "ROOS" atravessa pulando (som 0x239) | (586, 220) |
| 0x325 | pássaro vermelho atravessa | (628, 194) |
| 0x326 | tiranossauro atravessa da esquerda (som 0x2e6) | (−359, 307) |
| 0x327 | menino de patins | (624, 284) |
| 0x328 | Maggie de skate (som 0x2fc) | (574, 269) |

**Alta**: o canguru apareceu 7 s depois de entrar no saguão, nas posições
exatas do roteiro 0x324 (célula 1 em (534, 234), célula 3 em (459, 355)); o
bloco 0x1f4–0x1f9 tem o mesmo canguru com origem (586, 229), que não bate.

### Oficina de Projetos (fundo 0x67fb) — 0x457–0x469 (1111–1129)

| Roteiro | O quê | Origem |
|---|---|---|
| 0x45c, 0x45e | dragão roxo voa da direita | (607, 179) |
| 0x45d | pássaro | (630, 105) |
| 0x45f | menino de patins | (624, 284) |
| 0x460–0x469 | McZee sai do buraco da parede com um chapéu (dez chapéus; sons 0x2f1, 0x281, 0x2f4) | (355, 184) |

**Alta**: os chapéus apareceram na gravação nas posições exatas de 0x460/0x461
(buraco em (355, 184), chapéu em (346, 127)).

### Ateliê de Pintura (fundo 0x6c6b) — 0x579–0x57e (1401–1406)

Gafanhoto (622, 346), porco (615, 246), tiranossauro (−398, 364), nave
espacial com a faixa "veja também o creative writer" (636, 110; 68 células),
patins (624, 284), Maggie de skate (574, 269). **Baixa**: por eliminação
entre os blocos de travessias que sobram (1401–1406 e 1702–1707); 100 s de
gravação do Ateliê não mostraram nenhuma travessia.

### Truques de Desenho (fundo 0x687b) — 0x6a6–0x6ab (1702–1707)

Porco (615, 273), tubarão (−241, 124; 34 células), baleia (−225, 128),
patins, gafanhoto, Maggie de skate. **Baixa**: por eliminação (ver acima).

### Bloco 0x1f4–0x1f9 (500–505)

Canguru (586, 229), baleia (−225, 68), robô vermelho e branco correndo
(−74, 287), tiranossauro, patins, Maggie de skate. Tela desconhecida (talvez
a Galeria ou outra sala com o canguru). Não usado.

### Chegadas pela corda, à esquerda — 0xcd–0xd4, 0x38b–0x394, 0x4b0–0x4b8, 0x70d–0x716

Quatro blocos (205–212, 907–916, 1200–1208, 1805–1814) com os mesmos
roteiros: um personagem desce pela borda esquerda da tela (x ≈ −110…−15, de
y < 0 até o chão) e o inverso, subindo. Personagens: McZee pendurado
(0x60e8, deita no chão em 0x60f0), menino de camiseta vermelha (0x6100),
Maggie segurando a vara, que termina em pé no disco do elevador em
(−53, 290) (0x6110/0x611c), rato na corda (0x64f0), sapo-bobo-da-corte
(0x6500), morsa (0x651c/0x6520) e lagarto (0x6538); mais quatro pedaços de
corda/gancho parados por 3 s (0x6558, 0x6560, 0x6568, 0x66d8, som 0x222).
Nos quatro blocos as origens diferem alguns pixels, o que sugere um bloco
por sala (as quatro do prédio), mas nenhuma chegada foi vista nas gravações
(as trocas de andar pelo diretório e pela vara foram instantâneas) e os
personagens das salas ficam parados (abaixo). **Baixa**; não usado.

### Gags do buraco do rato — 0x12d–0x132, 0x258–0x25e, 0x3e9–0x3ef, 0x515–0x519, 0x76c–0x773

Cinco blocos (300–306, 600–606, 1000–1007, 1301–1305, 1900–1907) de
roteiros com origem pequena (relativos a um ponto, como os da rua): rato
de cartola que espirra fumaça (0x66e0/0x66f0), rato espiando (0x66f8), ratos
dançando (0x6790), rato na roda (0x6740), morcego (0x67d0), tiranossauro
pequeno (0x67c0), casa que cresce (0x6700), nuvem com lanças (0x61a4),
pinguim (0x617c), barbatanas (0x6578), besouro (0x67a8), aviões com a faixa
"Microsoft"/"Almoce no Joca" (0x6124, 0x6730, 0x67e8). Provavelmente
ancorados no buraco do rato que há em cada sala (e no título?); os cinco
blocos correspondem a cinco telas. Âncoras não medidas. **Baixa**; não usado.

### Outros

- 0x190/0x191 (400–401): a seta "SOBE" do indicador do elevador descendo e
  subindo 72 px em (101, 137) — fundo desconhecido (talvez a tela preta do
  elevador). Média; não usado.
- 0x4c9–0x4ce, 0x4e2–0x4e6, 0x4fb–0x4ff (1225–1279): uma figura parada por
  3 s com o som 0x222 (sino, capa, rosa, teia, cabeça de touro, fitas,
  óculos, bolo, chapéu, gravata...): adereços dos Truques de Desenho ou do
  Mundo. Baixa; não usado.
- 0x5aa (1450): bola de beisebol voando com estrelas, de (−65, 318). Tela
  desconhecida.
- 0x3840, 0x384a, 0x3854, 0x4010, 0x401a, 0x4024 (14400–16420): a menina de
  macacão verde (do Creative Writer) correndo e pulando corda, os únicos
  roteiros do tipo 9 (deslizes interpolados) e os únicos com imagens fora
  da faixa 0x60xx–0x67xx (0x380c…, 0x4018…). As origens têm y negativo
  (−20, −39, −50) e as imagens são grandes: são da tela de propaganda do
  Creative Writer ("Write" ampliado, vista na gravação entre Imaginópolis e
  a rua), não do título. Média; não usado (o tocador os reproduz, ver os
  testes de `anim.rs`).

## Personagens parados das salas (não são roteiros)

Medidos nas gravações (`tools/wine`-style `locate`, casamento exato):

| Sala | Figura | Posição |
|---|---|---|
| Saguão | McZee com uma fantasia sorteada (14 figuras coloridas de 149×354: 0x6d93, 0x6d9f, 0x6da3, 0x6da7, 0x6dab, 0x6daf, 0x6db3, 0x6db7, 0x6dbb, 0x6dbf, 0x6dc3, 0x6dc7, 0x6dcb, 0x6dcf; 1 bit e máscara são as duas imagens seguintes) | (−28, 129) |
| Ateliê | idem (vistas 0x6d93 e 0x6dc3) | (−18, 129) |
| Oficina de Projetos e Truques | Maggie numa pose sorteada: 0x6d5f (encostada), 0x6d6b (mãos na cintura), 0x6d6f (braços cruzados), 0x6d73 (acenando) | (−2, 253) |

A fantasia muda a cada visita à sala, inclusive no original. Na gravação os
personagens não piscam nem se mexem; a "animação" das salas são as
travessias acima.

## O que o motor faz

- `Splash`: pássaro 0x67 aos 0,3 s, depois os outros roteiros do título em
  ordem sorteada, com pausas de 0,5–1,5 s.
- `Street`: clique no hidrante ou na lata toca o roteiro correspondente.
- `Room`: personagem parado por andar (sorteado) e uma travessia do bloco do
  andar entre 4 e 10 s depois de chegar, depois a cada 12–30 s.
- Sons das células viram `Effect::PlaySound(SoundRef::Snd(Artist, id))`.
- Sem os dados (recurso ou imagem ausente) nada toca e nada quebra.
