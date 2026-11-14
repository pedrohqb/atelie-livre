# Creative Writer: abertura, Entrada e ligação ao prédio

Levantamento do original em `cw/report/building.md` (§2–4); dados no código
em `crates/engine/src/scenes/writer_entry.rs`; contrato dos specs em
`docs/arquitetura.md` ("Creative Writer: specs por programa").

## Fluxo

```
título 6737 (clique ou 5 s) → Entrada do Writer (CwScene::Entry)
  Sou eu → "Que bom ver você de novo"
     1 Ir direto para Imaginópolis → CwScene::Imaginopolis
     2 Mudar o que você sabe sobre mim → cadastro
     3 Conte mais sobre o seu mundo → CwScene::Story
     4 Ir direto para o Escritório → CwScene::NewDocument (área de escrita, documento novo)
  Primeira vez → cadastro → "Obrigado…" → história | Imaginópolis | refazer
Imaginópolis → rua → saguão → andares (CwScene::Building(Floor)), Biblioteca (CwScene::Library)
```

No original o título do Writer é estático (não há roteiros `ATAG` de
título no `WRITER.EXE`); no port ele vai à Entrada (antes ia direto ao
editor). `--user` pula a Entrada e vai à Imaginópolis do Writer, como no
Fine Artist.

## Entrada

A mesma tela do Fine Artist (`scenes/entry.rs`) com o `WRITER_ENTRY`:

| | Fine Artist | Creative Writer |
|---|---|---|
| fundo | `4139` | `3c41` (igual pixel a pixel) |
| McZee com livro (435,112) | `42f9` | `4c43` (máscara duas adiante) |
| balões | `WHAT 4400` / `4402` | `WHAT 4e00` / `4e02` |
| "EPA!" de saída | `1001[2]` "Sair do Fine Artist" | `0805[12]` "Sair do Creative Writer" (223 px) |
| nome na lista | `230` | `22d` |
| McZee, Procurar | `252`, `26d` | `281` |
| bolinha, "Continuar trabalhando" | `254` | `2ff` |
| opção 4 | pintura (`Paint`) | área de escrita com documento novo, com `304` |

Os botões (Sou eu, Primeira vez, Procurar, Coloque aqui), as setas, as
bolinhas, os botões Volta/Para/Segue e a arte "EPA!" têm os mesmos desenhos
nos dois `.PIC`; os balões ainda os desenham do `ARTIST.PIC`.

O cadastro e a pasta de usuários (`usuarios/<apelido>/perfil.toml`) são os
mesmos nos dois programas: um usuário criado num aparece no outro, como o
`~USER.INF` compartilhado do original.

## Prédio

As salas do Writer usam a mesma cena (`Room`) com os `RoomSpec` do Writer
(`writer_rooms` e vizinhos). Enquanto uma sala não tem dados próprios ela
mostra a do Fine Artist, mas as saídas vão às telas do Writer: a seta da
Galeria leva à Biblioteca (`CwScene::Library`), o clique duplo no
cavalete/prancheta à área de escrita com documento novo, o quadro da parede
à área de escrita, o livro e a maleta da bolsa à história e à pasta do
Writer.
