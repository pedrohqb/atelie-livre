# Corretor e troca-palavras — a abelha (W16)

Ferramenta 5 da barra do Escritório (`ToolId::Speller`, `scenes/writer/speller.rs`
e `speller/{search,widgets,dictload}.rs`). Pesquisa do original:
`cw/report/office-ferramentas.md` §4 e `office-complemento.md` §4 (capturas
`office_77`..`office_93`, `w26_40_corretor`). Comparação original | port:
`cw/shots/cw-tool-speller_compare_paineis.png`.

Ícone na barra: inativo `091f` (214,11); ativo `0937` (máscara `0939`) em
(214,21), a abelha voando inteira. Balão da ferramenta `0fa4[0]`; som `020d`.

## Faixa

| botão | figura (máscara) | retângulo | balão | abre |
|---|---|---|---|---|
| PULAR PARA (coelho) | `08cd` (`08cf`) | (59,59) 94×38 | `0fa4[1]` | Pular para |
| CORRIGIR (abelha) | `082d` (`082f`) | (204,59) 89×37 | `[4]` | Corrigir |
| TROCAR (dentes) | `0835` (`0837`) | (344,57) 54×42 | `[5]` | Trocar |
| Embaralhador (liquidificador) | `08b6` (`08bb`) | (449,56) 29×43 | `[9]` | Embaralhar |

Agem ao soltar sobre o botão. Som ao abrir: `0233` (Pular para, Corrigir),
`0296` (Trocar), `0297` (Embaralhar).

## Painéis

Cada painel ocupa o lugar da faixa: a barra de cima e a faixa ficam
pontilhadas (pixels com `x + y` par em cinza 100, retângulo (4,4) 632×98) e a
página desce **50** (Pular para), **152** (Corrigir) ou **95** (Trocar,
Embaralhar) pixels. Fundo: linhas pretas em y 106–107, laranja `ff8c32` com
sombra `b96524` à esquerda e embaixo, e as linhas pretas de novo `depth`
abaixo. Células com borda preta de 1 pixel e sombra em cima e à direita.
Enquanto o painel está aberto ele é modal: a barra de cima não responde, o
texto digitado vai para o campo com foco e nenhuma tecla chega ao documento.
Esc = FEITO.

| painel | células (fundo) | faixas-título | botões (figura, máscara, canto) |
|---|---|---|---|
| Pular para | (8,111) 475×42 branca; (486,111) 146×42 cinza | AJUDE A ACHAR `094f` (9,112) | ACHAR `0be5`/`0be7` (487,115); FEITO `0c55`/`0c57` (556,119) |
| Corrigir | (8,111) 310×42 cinza-claro; (8,156) 310×42 branca; (8,201) 310×54 cinza; (321,111) 311×144 cinza | NÃO CONHEÇO ESTA PALAVRA `0943` (8,112); O QUE VOCÊ ACHA DESTA? `0947` (8,157); MAIS ESCOLHAS `094b` (321,112) | MUDAR `0829` (25,212); PULAR `0c51` (124,212); FEITO (224,215); ▲ `02db` (416,234), ▼ `02df` (524,234) |
| Trocar | (8,111) e (8,156) 310×42 brancas; (321,111) e (321,156) 311×42 cinza | TROCAR ISTO `093b` (8,112); POR ISTO `093f` (8,157) | PALAVRAS PARECIDAS `0bfd` (370,122); UMA `0bf1`/`0bf7` (332,161); OUTRA `0be9`/`0bef` (379,163); TODAS `0bf9` (473,160); FEITO (545,164) |
| Embaralhar | como o Trocar | como o Trocar | PALAVRAS EMBARALHADAS `0c4d` (350,122); UMA (350,161); OUTRA (415,163); FEITO (527,164) — sem TODAS |

Campos: texto em Pablo Graphos 16, no **negrito sintético** do Windows (cada
letra desenhada duas vezes, 1 pixel ao lado, avançando 1 pixel a mais:
"gatu" tem 35 pixels). Texto posto pelo programa (palavra do cursor,
sugestão) vem todo selecionado (fundo `3096fa`, letras brancas, da largura
exata do texto) e o cursor preto fica logo depois do texto; digitar troca
tudo. O "Não conheço esta palavra" é só leitura, centrado.

Listas (MAIS ESCOLHAS, 5 linhas a partir de (322,128), 309 de largura; e as
suspensas, 92 de largura sob o botão, a partir de y 142, até 8 linhas): linhas
brancas de 20 pixels separadas por 1 pixel preto, Pablo 8 em negrito; a linha
escolhida em preto com letras brancas. A MAIS ESCOLHAS abre **sem destaque**;
o clique numa linha a destaca e põe a palavra no campo de baixo (som `022d`).
As setas rolam a lista (`021d`).

Com isso o painel do Corrigir (`office_83`) e a lista de anagramas
(`office_91`) saem iguais pixel a pixel ao original.

## Comportamento

**Pular para.** Digita-se no campo; ACHAR (ou Enter) seleciona a próxima
ocorrência depois da seleção e rola o mínimo para ela aparecer abaixo do
painel. Sem Ctrl, só palavras inteiras (`porto` acha `porto` e `Porto`, não
`aeroporto`); com Ctrl, qualquer trecho (balão `[3]`). Maiúsculas não contam;
acentos contam (`cafe` não acha `café`). Um trecho não atravessa parágrafos;
as figuras do Maçã separam palavras. Chegando ao fim: aviso `0805[8]` "Já
procurei no seu documento inteiro" e a próxima busca recomeça do começo; se
não houver nenhuma: `0805[0]`.

**Corrigir.** Percorre as palavras a partir do cursor até o fim, volta ao
começo e para no ponto de partida (que anda junto com as trocas no mesmo
parágrafo). A palavra desconhecida fica selecionada no texto e no campo de
cima; a primeira sugestão vai para o de baixo e todas para a lista; sem
sugestões, a lista mostra "(Sem sugestões)" (`STR# 0808`, inerte) e o campo,
a própria palavra. MUDAR (ou Enter) troca pela do campo de baixo (`021c`;
uma ação do Ovo), PULAR segue. No fim: `0805[2]` "Já acabei de corrigir"; o
clique fecha o aviso e o painel. As palavras se separam como no Writer (o
hífen separa: `vamo-nos` marca `vamo`), e a caixa segue o dicionário (palavra
minúscula aceita Capitalizada e TODA MAIÚSCULA; `Brasil` não aceita `brasil`).

**Trocar / Embaralhar.** TROCAR ISTO começa com a palavra do cursor (que fica
selecionada no texto) ou com a seleção de um parágrafo só. Clicar no POR ISTO
mostra o balão `[6]`/`[10]` (uma vez). PALAVRAS PARECIDAS abre a lista dos
**sinônimos** (`THES.DCT`; `casa` → botoeira, domicílio, edifício,
habitação, lar…); PALAVRAS EMBARALHADAS, a dos **anagramas** em ordem
alfabética (`ramo` → amor, mora, oram, ramo); som `021f`. Sem nenhum, a lista
mostra a própria palavra. Escolher põe a palavra no POR ISTO (`022d`; balão
`[8]`/`[12]`, uma vez); clicar fora só fecha a lista. UMA troca a seleção se
ela for o que se procura e seleciona a próxima (senão só procura); OUTRA pula;
TODAS (só no Trocar) troca tudo de uma vez, numa ação só do Ovo. Ctrl = parte
de palavra (`[13]`). A troca herda o estilo do primeiro caractere trocado e
copia a caixa do achado: `Gato` → `Peixe`, `GATO` → `PEIXE`.

## Balões (`WHAT 0fa4`)

| registro | quando | onde (canto da caixa, ponta do rabicho) |
|---|---|---|
| `[0]` | escolher a ferramenta | junto ao McZee |
| `[1]` | abrir o Pular para | (170,176), rabicho para cima até (240,153) — medido em `office_77` |
| `[2]` | primeira letra digitada no Pular para | idem (não medido) |
| `[3]` | Ctrl+ACHAR | idem (não medido) |
| `[4]` | abrir o Corrigir | junto ao McZee (no original o rabicho vai a (495,96)) |
| `[5]` / `[9]` | abrir o Trocar / o Embaralhar | (20,16), rabicho **para baixo** até (48,118), no TROCAR ISTO — `office_87`, `office_90` |
| `[6]` / `[10]` | primeiro clique no POR ISTO | idem (não medido) |
| `[8]` / `[12]` | primeira escolha na lista | idem (não medido) |
| `[13]` | Ctrl+UMA, Ctrl+TODAS | idem (não medido) |

`[7]` e `[11]` são as dicas abertas pelas ligações de `[6]` e `[10]`. Para os
lugares fora do padrão a ferramenta usa `Cx::help_at(h, ((x, y), ponta))`
(→ `McZee::show_at`); o `HelpBalloon` desenha o rabicho embaixo da caixa
quando a ponta está abaixo dela.

Os avisos do Rex (`0805[0]`, `[2]`, `[8]`) abrem no meio da vista, 40 pixels
abaixo do painel; clique, Enter ou Esc fecham.

## Dicionário

`crate::writer::dict::Dictionary` na variante Brasil: o `SPELL.DCT` e o
`THES.DCT` originais quando há pasta de dados, senão o Hunspell `pt_BR` e o
MyThes do sistema. Decodificar os `.DCT` e montar o índice de anagramas leva
alguns segundos num executável de depuração, então o Escritório começa a
carga numa thread ao entrar (`speller::preload`, uma por pasta e por
processo). Se o Corrigir abre antes de a carga terminar, a procura espera
(`tick`) e continua sozinha; sem dicionário nenhum, o Corrigir mostra logo o
`0805[2]`.

### Ordem das sugestões: original × port

O original ranqueia as sugestões com o modelo de n-gramas do `SPELL.DCT`
(ver `docs/formats/dct.md`), que o port ainda não interpreta; o port gera em
camadas (caixa registrada, troca fonética, uma edição, duas edições) com
empate pela ordem do dicionário, e a abelha pede até 12. Resultado, com o
`SPELL.DCT` (original = capturas no Wine; as listas do original podem ter
mais itens além dos anotados):

| palavra | original | port |
|---|---|---|
| `gatu` | gata, gato, tatu | gata, gato, tatu — **igual** |
| `abaf` | aba, abafa, abafe, abafo, abas | aba, abafa, abafe, abafo, abas — **igual** |
| `pexe` | pese, peixe, peie, pede, pele | pese, mexe, pede, peie, peixe, pele, pene, puxe (`peixe` em 5º; `mexe` a mais) |
| `vamo` | amo, vamos, vago, varo, vaso | amo, gamo, mamo, ramo, vago, vamos, varo, vaso, vazo (`vamos` em 6º) |
| `aco` | Co, ao, aço, Ac, acho | acho, aço, AC, Ac, acto, ado, aio, ajo, amo, ano, ao, arco (`ao` em 11º; sem `Co`) |
| `brasil` | Brasil, brisa, brisas | Brasil, barril, bragal, brami, bramia… (sem `brisa`/`brisas`, que estão a duas edições) |
| `tubares` | turbares | turbares, adubares, atuares… — 1ª igual |
| `pes` | pés, es, P.es, Es, pez | pez, pés, des, ES, Es, es, mes, Paes, PDS, PE, per, pesa (sem `P.es`) |

A primeira sugestão — a que vai para o campo — coincide em `gatu`, `abaf`,
`pexe`, `vamo`, `brasil` e `tubares`, e difere em `aco` (acho × Co) e `pes`
(pez × pés). Com o Hunspell livre a ordem é outra (`gatu` → jatu, gato,
garu…; `pexe` → pese, peche, pexes, peixe…).

## Diferenças conhecidas

- Ordem das sugestões (acima).
- No original, Esc com a lista suspensa aberta derruba o `winevdm`; no port,
  Esc ou clique fora fecham a lista.
- Os lugares dos balões `[2]`, `[3]`, `[6]`, `[8]`, `[10]`, `[12]` e `[13]`
  não foram medidos (copiam o do balão de abertura do painel).
- Fora do painel, a moldura comum do Escritório (`chrome`) está 1 pixel à
  direita e a borda de baixo da faixa 2 pixels abaixo do original — não é da
  abelha.
