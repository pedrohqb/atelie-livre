//! Coloca-adesivos dos Transformadores (`WHAT 1006[20]`, "Você criou o seu
//! próprio adesivo no Cavalete de adesivos? Para colocar um adesivo no rolo
//! • Clique no adesivo"): o adesivo, a palavra ou o balão clicado, como está
//! na tela (esticado, girado, invertido), vira um adesivo do usuário
//! (`meu-adesivo-<data>-<id>.png` na pasta de desenhos) no rolo "Meus
//! adesivos" do Seleciona-adesivos. O quadro não muda.

use super::*;
use crate::paint::store::{save_stamp_png, timestamp};
use crate::scenes::stickers::USER_PREFIX;
use crate::scenes::transformers::Op;

impl Paint {
    /// Clique no quadro com o Coloca-adesivos: verdadeiro se o clique era
    /// dele (havia um objeto no ponto; ele foi para o rolo, se deu para gravar).
    pub(super) fn add_to_user_roll(&mut self, cx: i32, cy: i32, fx: &mut Vec<Effect>) -> bool {
        if self.transformers.op() != Op::AddSticker {
            return false;
        }
        let Some(id) = self.objects.hit(cx, cy) else {
            return false;
        };
        let Some(stamp) = self.object_stamp(id) else {
            return false;
        };
        let Some(dir) = self.drawings.clone() else {
            fx.push(Effect::Log(
                "coloca-adesivos: sem pasta de desenhos para guardar o adesivo".into(),
            ));
            return true;
        };
        let name = format!("{USER_PREFIX}{}-{id}.png", timestamp(super::now_secs()));
        let saved = std::fs::create_dir_all(&dir)
            .and_then(|()| save_stamp_png(&stamp, &self.pal, &dir.join(&name)));
        match saved {
            Ok(()) => {
                self.stickers.add_user(name, stamp, &self.pal);
                fx.push(sfx::play(sfx::STICKER_PICK));
            }
            Err(e) => fx.push(Effect::Log(format!("coloca-adesivos: {e}"))),
        }
        true
    }
}
