//! Roteiros dos adesivos animados do primeiro rolo, gravados do Fine Artist
//! original no Wine (scratch/rec.sh + analyze.py + gen_scripts.py).
//! Cada passo: duração em quadros de 1/15 s e imagens (id em ARTIST.PIC,
//! deslocamento do canto superior esquerdo em relação ao clique).
//! Gerado; não editar à mão.

/// Um passo da animação.
pub struct ScriptStep {
    pub frames: u8,
    pub images: &'static [(u16, i16, i16)],
}

/// Roteiro: passos, imagem de repouso (onde o adesivo fica) e, se houver,
/// o deslocamento por volta de um roteiro que se repete até sair do quadro.
pub struct Script {
    pub steps: &'static [ScriptStep],
    pub rest: Option<(u16, i16, i16)>,
    pub repeat_shift: Option<(i16, i16)>,
}

/// Quadros por segundo dos roteiros.
pub const SCRIPT_FPS: f32 = 15.0;

/// Roteiros dos 16 bichos (`None`: não gravado; usa a animação aproximada).
pub const SCRIPTS: [Option<Script>; 16] = [
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 5,
                images: &[(0x753b, -109, -33)],
            },
            ScriptStep {
                frames: 4,
                images: &[(0x7543, -53, -41)],
            },
            ScriptStep {
                frames: 5,
                images: &[(0x754b, -41, -35)],
            },
            ScriptStep {
                frames: 4,
                images: &[],
            },
        ],
        rest: None,
        repeat_shift: Some((127, 0)),
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x755b, -62, -70)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7563, 103, -27)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x756b, 78, 10)],
            },
            ScriptStep {
                frames: 6,
                images: &[(0x7573, 59, 6)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7553, 139, 13)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x755b, 129, -27)],
            },
        ],
        rest: Some((0x7553, -52, -30)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x7583, -115, -25)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7583, -165, -25)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x757b, -215, -25)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7583, -265, -25)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7583, -315, -25)],
            },
            ScriptStep {
                frames: 3,
                images: &[],
            },
        ],
        rest: Some((0x757b, -65, -25)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x7593, -18, -77)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x758b, -19, -98)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7593, -18, -119)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x758b, -19, -140)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7593, -18, -161)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x758b, -19, -182)],
            },
            ScriptStep {
                frames: 12,
                images: &[],
            },
        ],
        rest: Some((0x758b, -19, -56)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x75a3, -100, -22)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75a3, -65, -22)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75ab, -2, -51)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75b3, 78, -87)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x759b, 77, -57)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75a3, 60, -22)],
            },
            ScriptStep {
                frames: 18,
                images: &[],
            },
        ],
        rest: Some((0x759b, -83, -57)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x75c3, -50, -101)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75cb, -40, -119)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75d3, -20, -143)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75bb, 3, -173)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x75c3, 7, -189)],
            },
            ScriptStep {
                frames: 19,
                images: &[],
            },
        ],
        rest: Some((0x75bb, -55, -85)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 2,
                images: &[(0x75e3, -82, -86)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75eb, -97, -88)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75f3, -101, -73)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75db, -116, -71)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75e3, -144, -86)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75eb, -159, -88)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75f3, -162, -73)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75db, -178, -71)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75e3, -205, -86)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75eb, -221, -88)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75f3, -224, -73)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75db, -240, -71)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75e3, -267, -86)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75eb, -282, -88)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75f3, -286, -73)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75db, -301, -71)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75e3, -329, -86)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75eb, -344, -88)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75f3, -348, -73)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x75db, -363, -71)],
            },
            ScriptStep {
                frames: 5,
                images: &[],
            },
        ],
        rest: Some((0x75db, -54, -71)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 1,
                images: &[(0x7603, -86, -114)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x760b, -81, -116)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x7613, -26, -55)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x761b, 17, -115)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x7623, 10, -117)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x75fb, 70, -53)],
            },
            ScriptStep {
                frames: 1,
                images: &[(0x7603, 95, -114)],
            },
            ScriptStep {
                frames: 2,
                images: &[(0x760b, 100, -116)],
            },
            ScriptStep {
                frames: 9,
                images: &[],
            },
        ],
        rest: Some((0x75fb, -111, -53)),
        repeat_shift: None,
    }),
    Some(Script {
        steps: &[
            ScriptStep {
                frames: 3,
                images: &[(0x7633, -6, -112)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x763b, 45, -89)],
            },
            ScriptStep {
                frames: 3,
                images: &[(0x7643, 145, -91)],
            },
            ScriptStep {
                frames: 6,
                images: &[],
            },
        ],
        rest: Some((0x762b, -64, -91)),
        repeat_shift: None,
    }),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
];
