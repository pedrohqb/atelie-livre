# Ateliê Livre — Subprojeto 2c: adesivos (primeira versão)

Data: 2026-10-31
Status: decidido pelo agente (Pedro pediu para seguir sem ele)
Depende de: 2b (tela de pintura), concluído

## 1. O que o original faz (medido no Wine)

A ferramenta Seleciona-adesivos mostra, na faixa de opções, rolos de
adesivos (7 por página, com setas). Clicar num rolo abre, sobre a borda
esquerda do quadro, uma tira de filme vertical com miniaturas em traço (10
visíveis, setas embaixo). Clicar numa miniatura fecha a tira e o adesivo passa
a acompanhar o ponteiro; um clique no quadro o coloca, centrado no clique, e
ele se anima (o pássaro, por exemplo, sai andando pela tela e volta) até
parar no primeiro quadro. Depois o McZee explica que clicar no adesivo o anima
de novo e que dá para arrastá-lo.

Os quadros dos bichos do primeiro rolo estão em `ARTIST.PIC`, de `0x753b` a
`0x773b`, em grupos de 8 ids: `N` colorido (4 bits), `N+5` em traço (1 bit) e
`N+6` máscara (preto = opaco). Os roteiros das animações (ordem dos quadros,
deslocamentos, sons) não foram encontrados nos recursos; devem estar no
código do programa.

## 2. Decisões

1. **Um rolo, 16 bichos** (o primeiro rolo do original, "bichos"), nesta
   ordem e com estes quadros: cobra `753b 7543 754b`; peixe-espada
   `7553 755b 7563`; rinoceronte `757b 7583`; lagarto `758b 7593`; polvo
   `759b 75a3 75ab 75b3`; tartaruga `75bb 75c3 75cb 75d3`; pássaro
   `75db 75e3 75eb 75f3`; dodô `75fb 7603 760b 7613 761b 7623`; jogador
   `762b 7633 763b 7643 764b 7653`; dinossauro `765b 7663 766b 7673 767b`;
   porco `7683 768b 7693 769b`; galo `76a3 76ab 76b3 76bb`; dragão-de-komodo
   `76c3 76cb 76d3 76db 76e3 76eb`; canguru `76f3 76fb 7703 770b`; baleia
   `7713 771b 7723 772b`; dragão `7733 773b`. Os outros rolos ficam para
   quando os roteiros forem decifrados.
2. **Animação aproximada**: sem os roteiros, o adesivo colocado percorre seus
   quadros no lugar, a 8 quadros por segundo, duas vezes, alinhados pelo
   centro da base, e para no primeiro quadro. Ao parar, ele é **colado no
   quadro** (vira pintura; o ovo desfaz). Adesivos como objetos móveis e
   reanimáveis ficam para depois.
3. **Interface** (medida no original): a faixa mostra um rolo (moldura
   `10cf` em (9,58), com a miniatura da cobra); clicar nele abre ou fecha a
   tira. A tira ocupa x 33..74, y 58..466: no topo, a casa do rolo (clicar ali
   fecha a tira); depois 9 casas de 28×32 em `(40, 103 + 39·k)` e as setas
   `02db` (39,452) e `02df` (57,452) para rolar. As miniaturas são os bitmaps
   em traço do original: `7743` (cobra) e `77ec`…`77fa`, na ordem dos bichos.
   Com a tira aberta, ela recebe os cliques antes do quadro.
4. **Adesivo na mão**: depois de escolhido, o primeiro quadro é desenhado
   centrado no ponteiro sobre o quadro; um clique fora do quadro (em outra
   ferramenta, cor etc.) o solta sem colocar. Cada escolha coloca um adesivo.
5. **Cores**: os pixels coloridos do adesivo viram o índice mais próximo da
   paleta de 256 cores; fora da máscara, nada é pintado.
6. Clicar durante uma animação termina-a na hora (cola o adesivo) e então
   trata o clique. Sair da tela também cola o adesivo em animação antes de
   gravar.

## 3. Testes

Escolher o rolo abre a tira; escolher a casa k segura o bicho k; clicar no
quadro inicia a animação e, depois do tempo, o quadro tem pixels pintados e o
ovo os desfaz; clique fora do quadro solta o adesivo; rolar a tira; colagem
respeita a máscara e as bordas; `leave()` durante a animação grava o adesivo.
Com dados reais: os 16 primeiros quadros existem e têm máscara do mesmo
tamanho.
