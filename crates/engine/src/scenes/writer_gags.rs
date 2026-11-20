//! Animações, personagens e surpresas do Creative Writer (tarefas T11 e
//! T12), tirados do código do `WRITER.EXE` (`cw/report/building.md`
//! §8.1, §8.4, §8.7–8.8) e conferidos no port: todos os roteiros
//! decodificam (`writer_gags_load_with_real_data`); as poses do Max e a
//! fantasia padrão do McZee foram achadas pelo contorno nas capturas do
//! original; os acessórios dos bichos da parede foram encaixados nos dois
//! bichos e a ordem do relatório estava trocada (ver [`DONKEY_HATS`]). As
//! salas (`writer_lobby`, `writer_office_room`, `writer_floors`) usam estes
//! nomes.

use super::room_objects::Surprises;

/// Fantasias do McZee (cor, 149×354; 1 bit e máscara são as duas seguintes
/// no `WRITER.PIC`), sorteadas a cada chegada numa sala.
pub const MCZEE_COSTUMES: &[u16] = &[
    0x7d91, 0x7dcb, 0x7dcf, 0x7dd7, 0x7ddb, 0x7ddf, 0x7de3, 0x7de7, 0x7deb, 0x7df3, 0x7df7, 0x7dfb,
];
/// McZee no saguão, no Escritório e na Biblioteca; no elevador.
pub const MCZEE_POS: (i32, i32) = (-18, 129);
pub const MCZEE_ELEVATOR: (i32, i32) = (-5, 180);

/// Max na Oficina de Idéias, sorteado a cada chegada como as fantasias do
/// McZee: de pé com a mão no quadril e o braço levantado
/// (`7e0f`, 148×206, em (9,253): todas as capturas da chegada,
/// `building_57_floor3`…`73_i_max`) ou encostado com as pernas cruzadas
/// (`7e03`, 143×198, em (−4,219): `ideas_sala_oficina`, `ideas_71_tchau`).
/// A tabela de pontos quentes do `WRITER.EXE` põe a `7e03` em (−2,282),
/// mas nas capturas ela está 63 px acima.
pub const MAX_POSES: &[(u16, (i32, i32))] = &[MAX_STANDING, (0x7e03, (-4, 219))];
/// Max de pé (`7e0f`): a única pose dos Projetos (a encostada se apoia na
/// Máquina de Idéias, que só existe no andar 4).
pub const MAX_STANDING: (u16, (i32, i32)) = (0x7e0f, (9, 253));
/// Max em pé, de frente (`7e0b`, 83×188), como ele volta da Máquina de
/// Idéias ou do Quadro de Figuras (`ideas_26_volta_oficina`).
pub const MAX_BACK: (u16, (i32, i32)) = (0x7e0b, (9, 283));
/// Max sentado (153×144), nas telas da Máquina de Idéias e do Quadro de
/// Figuras (tabela de pontos quentes; as cenas são de outra tarefa).
pub const MAX_SITTING: u16 = 0x7dff;
pub const MAX_SITTING_POS: (i32, i32) = (-2, 339);
/// Retângulo do clique no Max (todas as poses ficam dentro dele).
pub const MAX_RECT: (i32, i32, i32, i32) = (0, 215, 160, 265);

/// Luminária "Tchau" (`6917`, 56×132) e a mesma com a fumaça (`691b`,
/// 77×133, 1 px acima), que aparece enquanto o "EPA!" está aberto
/// (`building_25_lamp`, `ideas_71_tchau`). No Fine Artist são `681b` e
/// `6853` (`rooms_data::ARTIST_LAMP`).
pub const LAMP: u16 = 0x6917;
pub const LAMP_SMOKE: u16 = 0x691b;

/// Luminária na posição `(x, y)` (a da fumaça 1 px acima).
pub const fn lamp_at(x: i32, y: i32) -> super::room_spec::Lamp {
    super::room_spec::Lamp::at(LAMP, LAMP_SMOKE, x, y)
}

/// Carrinho de Imaginópolis (só na chegada).
pub const IMAGINOPOLIS_CAR: u16 = 0x0861;

/// Bichos ociosos por tela.
pub const STREET_GAGS: &[u16] = &[0x0800, 0x0801, 0x0802, 0x0803, 0x0804, 0x0805];
pub const LOBBY_GAGS: &[u16] = &[0x0880, 0x0881, 0x0882, 0x0883, 0x0884, 0x0885];
pub const LIBRARY_GAGS: &[u16] = &[0x0868, 0x0869, 0x086a, 0x086b];
pub const OFFICE_GAGS: &[u16] = &[0x0910, 0x0911, 0x0912, 0x0913, 0x0914];
pub const PROJECTS_GAGS: &[u16] = &[
    0x08b9, 0x08ba, 0x08bb, 0x08bc, 0x08bd, 0x08be, 0x08bf, 0x08c0, 0x08c1, 0x08c2, 0x08c3, 0x08c4,
    0x08c5, 0x08c6, 0x08c7, 0x08c8, 0x08c9, 0x08ca,
];
pub const IDEAS_GAGS: &[u16] = &[
    0x0836, 0x0837, 0x0838, 0x0839, 0x083a, 0x083b, 0x083c, 0x083d, 0x083e, 0x083f, 0x0840, 0x0841,
    0x0842, 0x0843, 0x0844,
];

/// Surpresas da Varinha Mágica por sala. Ponto de ancoragem: como no Fine
/// Artist, o canto do mastro e do buraco do rato da sala (a conferir).
pub const LOBBY_SURPRISES: Surprises = Surprises {
    pole: &[
        0x09c6, 0x09c7, 0x09c8, 0x09c9, 0x09ca, 0x09cb, 0x09cc, 0x09cd, 0x09ce, 0x09cf, 0x09d0,
    ],
    pole_anchor: (596, 0),
    hole: &[
        0x08a2, 0x08a3, 0x08a4, 0x08a5, 0x08a6, 0x08a7, 0x08a8, 0x08a9, 0x08aa, 0x08ab,
    ],
    hole_anchor: (497, 347),
};
pub const LIBRARY_SURPRISES: Surprises = Surprises {
    pole: &[],
    pole_anchor: (0, 0),
    hole: &[0x0870, 0x0871, 0x0872, 0x0873, 0x0874, 0x0875, 0x0876],
    hole_anchor: (552, 273),
};
pub const OFFICE_SURPRISES: Surprises = Surprises {
    pole: &[
        0x0a8f, 0x0a90, 0x0a91, 0x0a92, 0x0a93, 0x0a94, 0x0a95, 0x0a96, 0x0a97, 0x0a98, 0x0a99,
    ],
    pole_anchor: (591, -9),
    hole: &[0x092e, 0x092f, 0x0930, 0x0931, 0x0932, 0x0933, 0x0934],
    hole_anchor: (490, 313),
};
pub const PROJECTS_SURPRISES: Surprises = Surprises {
    pole: &[
        0x0a28, 0x0a29, 0x0a2a, 0x0a2b, 0x0a2c, 0x0a2d, 0x0a2e, 0x0a2f, 0x0a30,
    ],
    pole_anchor: (573, -22),
    hole: &[0x0906, 0x0907, 0x0908, 0x0909, 0x090a],
    hole_anchor: (479, 299),
};
pub const IDEAS_SURPRISES: Surprises = Surprises {
    pole: &[0x0961, 0x0962, 0x0963, 0x0964, 0x0965, 0x0966],
    pole_anchor: (573, 0),
    hole: &[0x0857, 0x0858, 0x0859, 0x085a, 0x085b, 0x085c],
    hole_anchor: (472, 347),
};

/// Acessórios da varinha nos bichos da parede dos Projetos (1 célula, 3 s,
/// som `293`), com as origens somadas ao canto da figura do bicho.
pub const MOOSE_HATS: &[u16] = &[0x08fc, 0x08fd, 0x08fe, 0x08ff, 0x0900, 0x0901];
/// Burro: cachecol, óculos, gorro, boné, óculos 3D, guarda-chuva. O
/// relatório (`building.md` §8.7) dava `08d9`–`08de` ao burro, sem
/// confirmar; encaixando as figuras nos dois bichos, as `08e3`–`08e8`
/// caem no focinho e na cabeça do burro (e as `08d9`–`08de` no
/// rinoceronte, fora dele no burro).
pub const DONKEY_HATS: &[u16] = &[0x08e3, 0x08e4, 0x08e5, 0x08e6, 0x08e7, 0x08e8];
/// Rinoceronte: lápis, sombreiro, óculos, bigode, brinco, chapéu-coco.
pub const RHINO_HATS: &[u16] = &[0x08d9, 0x08da, 0x08db, 0x08dc, 0x08dd, 0x08de];
