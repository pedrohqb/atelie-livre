# Menus, atalhos e caixas de diálogo (`MENU`, `ACCELERATOR`, `DIALOG`)

Recursos padrão do Windows 3.x nos dois executáveis. Leitor:
`formats::winres_ui` (`parse_menu`, `parse_accelerators`, `parse_dialog`).
Todos os inteiros são little-endian; os textos são Windows-1252 terminados
em zero. Os dados que o NE entrega vêm com preenchimento de alinhamento no
fim, que os leitores ignoram.

Os dois programas têm uma **barra de menus do Windows** escondida na tela
cheia (o diálogo Sobre diz "Olhe a barra de menu para ver comandos úteis"),
um punhado de diálogos e, no Writer, uma tabela de atalhos.

## `MENU`

```
u16  versão (0)
u16  tamanho do cabeçalho extra (0)
itens:
  u16  flags (MF_*)
  se flags & 0x10 (POPUP):  texto\0, seguido dos itens do submenu
  senão:                    u16 id, texto\0      (id 0 e texto vazio = separador)
  o último item de cada nível tem flags & 0x80 (END)
```

Flags vistas: `0x01` cinza (`MF_GRAYED`, estado inicial; o programa habilita
ao abrir um documento), `0x10` submenu, `0x80` fim do nível. No texto, `&`
marca a letra sublinhada (`&&` é um `&` literal) e o que vem depois de um tab
é o atalho mostrado à direita ("Salvar &Como...\tCtrl+S").

| Programa | Recurso | Conteúdo |
|---|---|---|
| Creative Writer | `MENU 0x3e8` | Arquivo (Novo 0x1f41, Abrir… 0x1f42, Salvar Como… 0x1f43, Salvar Como Texto… 0x1f44, Opções de Impressão… 0x1f45, Voltar ao Creative Writer e Sair 0x1f46, Sobre o Creative Writer… 0x1f5c), Editar (Desfazer 0x1f47, Recortar 0x1f49, Copiar 0x1f4a, Colar 0x1f4b, Limpar 0x1f4c), Ferramentas (Controle de Som… 0x1f4d, Decodificar Documento… 0x1f4f, Redefinir Máquina de Idéias… 0x1f50) |
| Fine Artist | `MENU "ADULTMENU"` | Arquivo (Novo 0x01, Abrir 0x02, Salvar Como… 0x1e, Salvar Como BMP… 0x1f, Opções de Impressão… 0x0a, Voltar ao Fine Artist 0x0b, Sair 0x0c, Sobre o Fine Artist… 0x28), Editar (Desfazer 0x03, Recortar 0xc9, Copiar 0xca, Colar 0xcb, Limpar 0xcc), Ferramentas (Controle de Som… 0x14) |

"Voltar ao …" e "Sair" do Writer têm o mesmo comando (0x1f46).

## `ACCELERATOR`

Registros de 5 bytes:

```
u8   flags: 0x01 tecla virtual, 0x02 sem inverter o menu, 0x04 Shift,
            0x08 Ctrl, 0x10 Alt, 0x80 último registro
u16  tecla: código VK_* (com 0x01) ou caractere
u16  comando (id do menu)
```

O Writer (`ACCELERATOR 0x3e9`) usa caracteres de controle ASCII, sem flags:
`^N` 0x1f41, `^O` 0x1f42, `^S` 0x1f43, `^Q` 0x1f46, `^Z` 0x1f47, `^X` 0x1f49,
`^C` 0x1f4a, `^V` 0x1f4b (`Accelerator::ctrl_letter` devolve a letra). O
Fine Artist não tem tabela de atalhos (trata as teclas no código).

## `DIALOG`

```
u32  estilo (WS_*, DS_*)
u8   número de controles
i16  x, y, cx, cy          unidades de diálogo
menu    nome\0, ou 0xff + u16
classe  nome\0, ou 0xff + u16
título\0
se estilo & 0x40 (DS_SETFONT):  u16 pontos, nome da fonte\0
controles:
  i16  x, y, cx, cy
  u16  id (0xffff nos textos fixos)
  u32  estilo
  classe: um byte com o bit 0x80 (0x80 BUTTON, 0x81 EDIT, 0x82 STATIC,
          0x83 LISTBOX, 0x84 SCROLLBAR, 0x85 COMBOBOX) ou nome\0
  texto\0, ou 0xff + u16 (número de ícone)
  u8   tamanho dos dados de criação, e os dados
```

O botão padrão (Enter) tem `BS_DEFPUSHBUTTON` (estilo & 0x0f = 1); nos
textos fixos o mesmo valor é `SS_CENTER`. Uma unidade de diálogo vale ¼ da
largura média e ⅛ da altura do caractere da fonte do diálogo.

### Creative Writer

| Id | Título | Textos e botões (id) |
|---|---|---|
| 0x7d0 | Creative Writer (Sobre) | "Versão 1.0", "20 de outubro de 1995", "® 1993-1995  Microsoft Corporation. Direitos reservados.", "Olhe a barra de menu para ver comandos úteis."; OK (1) |
| 0xbb8 | Controle de Som | "Liga e desliga sons enquanto você usa ferramentas e se move de lá para cá."; Ligar sons (0xbba), **Desligar sons** (0xbb9) |
| 0x1388 | Decodificar o Documento | "O McZee vai decodificar um documento se você esquecer a senha!", "1. Abra o documento.", "2. Escolha o Agente Secreto", "3. Digite:", "Detetive McZee para salvar o dia", "4. Segure Control+Shift e clique em OK"; Valeu, McZee (1) |
| 0x1770 | Redefinir a Máquina de Idéias | "Redefinir as entradas originais da Máquina de Idéias.", "Remove todas as entradas\nque você colocou.", "Cuidado! Você não vai poder recuperar as entradas!"; Deixar como está (2), Apagar (3) |
| 0x1b58 | Opções de Impressão | "Opções de controle para impressão.", "Você vai ver a caixa Impressora.", "Escolha as opções e clique em OK.", "Definir opções só para este computador:", "Definir opções para todos os usuários:", "Este comando não imprime o documento. Ele define opções para imprimir no futuro."; Opções locais (0x1b5a), Opções globais (0x1b59), Deixar como está (2) |

Em negrito, o botão padrão quando não é o primeiro. Todos usam MS Sans
Serif 8.

### Fine Artist

Diálogos com nome: `ABOUTSPLAT` (Sobre, igual ao do Writer com o título
"Fine Artist"), `PRINTOPTIONS` (Opções de Impressão; botões 0x65–0x67),
`SAVEAS` ("Salvar Como": salvar uma cópia como BMP; fonte System),
`SOUNDDIALOG` (Controle de Som; botões 1 e 2) e `TTDIALOG` (sem título:
"Não consigo achar as fontes TrueType…", Continuar/Sair).
