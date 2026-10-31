# Entrada, cadastro e Imaginópolis — desenho

Subprojeto do Fine Artist: as telas entre a abertura e a rua.

## Fluxo do original (medido no Wine; capturas em `scratch/orig_01_entrada.png`
… `orig_04_depois_segue.png`, fora do repositório)

1. **Entrada** (fundo `4139`, 640×480: McZee de livro na mão sobre as nuvens).
   Balão `WHAT 4400[1]` ("Oi, eu sou o McZee! Diga quem é você!") no canto
   superior esquerdo. Quadro-negro verde com a lista de usuários (caminho
   `C:\MSKIDS\USERS\` no alto, 8 linhas) e três botões: **Sou eu**,
   **Primeira vez**, **Procurar** (ícones próprios; ver captura). Botões
   "Volta / Para / Segue" (`02f?`-ish, ver captura) aparecem nas telas do
   cadastro.
2. **Sou eu** com um nome escolhido → balão `4400[2]` "@! Que bom ver você de
   novo!" com as opções: Ir direto para Imaginópolis / Mudar o que você sabe
   sobre mim / Conte mais sobre o seu mundo / Ir direto para o Ateliê de
   Pintura.
3. **Primeira vez** → cadastro em balões sucessivos com campos de texto:
   `4400[4]` nome e sobrenome (duas caixas `[0:90]`), `4400[8]` bicho
   preferido (12 opções com figuras `237`…`242`), `4400[6]` disco (no port:
   não existe; pule), `4400[5]` apelido (até 8 caracteres; `4402[2]` se
   inválido, `4402[0]` se repetido, `4402[1]` se nome vazio). Depois
   `4400[3]` "Obrigado por ter me contado coisas sobre você, @" com: É, por
   favor conte mais (→ história do McZee, `4200`, álbum `4201` com as fotos
   `420b`…) / Não precisa, vamos direto para Imaginópolis / Quero mudar o que
   você sabe sobre mim.
4. **Imaginópolis** (fundo `6cfd`, 641×480): a porta do Museu Mágico em
   (302,172) leva à rua (`SceneId::Street`); o McZee no carrinho (ATAG
   `0x2bc`, já tocável pelo `Animator`) atravessa de vez em quando.

## Port (usável primeiro)

- `SceneId::Entry`, `SceneId::Imaginopolis`, `SceneId::Story` (álbum).
  Abertura → Entrada (em vez de ir à rua). Config `--user NOME` ou
  `usuario = "..."` no `config.toml` pula a entrada.
- Usuários em `$XDG_DATA_HOME/atelie-livre/usuarios/<apelido>/perfil.toml`
  (`nome`, `sobrenome`, `apelido`, `bicho`); a pasta de desenhos continua
  única por enquanto (`desenhos/`), e o apelido vira o `@` dos balões
  (`Room` usa `USER_NAME` hoje — troque por um valor passado na criação da
  cena ou guardado num `UserContext` do `app`).
- Entrada: lista com os apelidos existentes (Pablo 8 pt, linhas de 16 px
  no quadro-negro), clique seleciona, **Sou eu** abre `4400[2]`; **Primeira
  vez** inicia o cadastro; **Procurar** não faz nada (sem discos) — mostra
  `4400[6]`? Não: ignore com um `Effect::Log`.
- Cadastro: balões de `WHAT 4400` com campos de texto editáveis (caixa
  branca com cursor, `Input::Text`/`Key`), botões Volta / Para / Segue
  desenhados com os ids da captura (ou retângulos com texto se os ids não
  forem achados). O bicho preferido: 12 figuras `237`…`242` em 4×3,
  escolhida com uma marca.
- História do McZee (`4200`): página do álbum `4201` (495×314) centrada com
  a foto do passo (`420b`, `423b`, `4253`, `4273`, `4293`, `42b3`, `42d3`,
  `42f3`…, uma por balão) e o balão correspondente; Segue avança, Para
  sai para Imaginópolis.
- Imaginópolis: fundo `6cfd` cortado a 640, porta (302,172) → rua; gag do
  carrinho via `GagPlayer` com o roteiro `0x2bc`.
- Testes: cadastro grava `perfil.toml` e o apelido aparece na lista na
  próxima entrada; apelido com 9 letras é recusado (`4402[2]`); entrada com
  `--user` pula para Imaginópolis; porta leva à rua.
