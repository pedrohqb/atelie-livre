//! Seleciona-sons na tela de pintura: liga a ferramenta (`scenes::sounds`)
//! aos objetos do quadro. O som escolhido se associa ao adesivo ou à palavra
//! clicada (no fundo, só o balão "Para associar o mesmo som…"); "Sem Som"
//! tira o som do objeto; sem nada na mão, clicar num objeto toca o som dele.
//! Os diálogos (Cachorrinho, Gravador) recebem todos os cliques e as teclas.

use super::*;
use crate::scenes::sounds::{self as snd_tool, Finder, HELP_ATTACHED, Hand, Recorder};

impl Paint {
    /// Clique com o Seleciona-sons escolhido: verdadeiro se foi tratado.
    pub(super) fn sounds_click(
        &mut self,
        x: i32,
        y: i32,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if let Some(d) = self.sounds.finder.as_mut() {
            let r = d.click(x, y);
            fx.extend(r.fx);
            if r.close {
                self.sounds.finder = None;
                self.help = None;
            }
            if let Some(h) = r.chosen {
                self.hold_sound(h, assets);
            }
            return true;
        }
        if let Some(rec) = self.sounds.recorder.as_mut() {
            let r = rec.click(x, y, assets);
            self.recorder_reply(r, fx, assets);
            return true;
        }
        let r = self.sounds.click(x, y, assets);
        if !r.handled {
            fx.extend(self.sounds.stop_effects());
            self.sounds.reset();
            return false;
        }
        fx.extend(r.fx);
        if let Some(k) = r.help {
            match k {
                snd_tool::HELP_DOG => {
                    self.sounds.finder = Some(Finder::new(
                        self.drawings.as_deref(),
                        snd_tool::finder::music_dirs(),
                    ));
                }
                snd_tool::HELP_RECORDER => {
                    self.sounds.recorder = Some(Recorder::new(self.drawings.as_deref()));
                }
                _ => {}
            }
            self.sounds_help(k, assets);
        }
        if r.canvas {
            self.sounds_on_canvas(x, y, fx, assets);
        }
        true
    }

    /// O som escolhido (lista, Cachorrinho ou Gravador) vai para a mão.
    fn hold_sound(&mut self, h: Hand, assets: &Assets) {
        self.sounds.set_hand(h);
        self.sounds_help(snd_tool::HELP_THEN_CLICK, assets);
    }

    fn sounds_help(&mut self, k: usize, assets: &Assets) {
        if self.help_on {
            self.open_help(WHAT_SOUNDS, k, assets);
        }
    }

    fn recorder_reply(
        &mut self,
        r: snd_tool::recorder::Reply,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) {
        fx.extend(r.fx);
        if r.close {
            self.sounds.recorder = None;
            self.help = None;
        }
        if let Some(h) = r.chosen {
            self.hold_sound(h, assets);
        }
    }

    /// Clique no quadro: associar o que está na mão ao objeto, tocar o som
    /// dele, ou (no fundo) o balão de ajuda.
    fn sounds_on_canvas(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let (cx, cy) = self.canvas_pt(x, y);
        let hit = self.sound_target(cx, cy);
        // com a Varinha Mágica na mão, o clique só toca (e a varinha some)
        if std::mem::take(&mut self.wand) {
            fx.extend(
                hit.and_then(|id| self.objects.get(id))
                    .and_then(|o| o.sound.clone())
                    .map(Effect::PlaySound),
            );
            return;
        }
        let Some(id) = hit else {
            self.sounds_help(HELP_ATTACHED, assets);
            return;
        };
        match self.sounds.take_hand() {
            Some(hand) => {
                self.history.record_all(&self.canvas, &self.objects);
                let sound = hand.sound().cloned();
                if let Some(o) = self.objects.get_mut(id) {
                    o.sound = sound.clone();
                }
                fx.extend(sound.map(Effect::PlaySound));
                self.changed = true;
            }
            None => {
                if let Some(s) = self.objects.get(id).and_then(|o| o.sound.clone()) {
                    fx.push(Effect::PlaySound(s));
                }
            }
        }
    }

    /// Objeto que recebe o som: o de cima com pixel opaco no ponto ou, entre
    /// as letras de uma palavra sem balão, a palavra cujo retângulo o contém.
    fn sound_target(&self, cx: i32, cy: i32) -> Option<u32> {
        self.objects.hit(cx, cy).or_else(|| {
            self.objects
                .records()
                .into_iter()
                .rev()
                .find(|r| {
                    matches!(r.source, Source::Words { .. } | Source::Shaped(_))
                        && (r.x..r.x + r.w).contains(&cx)
                        && (r.y..r.y + r.h).contains(&cy)
                })
                .map(|r| r.id)
        })
    }

    /// Entradas que não são cliques: teclas do Gravador e arrasto do volume.
    /// Verdadeiro se a entrada foi consumida.
    pub(super) fn sounds_input(
        &mut self,
        i: &Input,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if self.tool != Tool::Sounds {
            return false;
        }
        if let Some(rec) = self.sounds.recorder.as_mut() {
            match i {
                Input::Text(t) => {
                    rec.type_text(t);
                    return true;
                }
                Input::Key { key, .. } => {
                    let r = rec.key(*key, assets);
                    self.recorder_reply(r, fx, assets);
                    return true;
                }
                _ => {}
            }
        }
        match *i {
            Input::MouseMove { y, .. } => fx.extend(self.sounds.drag(y, assets)),
            Input::MouseUp { .. } => fx.extend(self.sounds.release()),
            _ => {}
        }
        false
    }

    /// A cada quadro: o tempo do Gravador.
    pub(super) fn sounds_tick(&mut self, dt: f32) -> Option<Effect> {
        self.sounds.recorder.as_mut().and_then(|r| r.tick(dt))
    }

    /// Prende aos objetos os pontos de som soltos de versões antigas do port
    /// (cada ponto vai para o objeto debaixo dele, se houver e estiver sem som).
    pub(super) fn sounds_attach_legacy(&mut self) {
        for s in self.sounds.take_legacy() {
            if let Some(o) = self
                .objects
                .hit(s.x, s.y)
                .and_then(|id| self.objects.get_mut(id))
                && o.sound.is_none()
            {
                o.sound = Some(s.sound);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::SoundRef;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn lib(index: usize) -> SoundRef {
        SoundRef::Library {
            file: "MAG100.SND".into(),
            index,
        }
    }

    /// Tela com o Seleciona-sons e um adesivo 20×20 em (100, 100) do quadro.
    fn paint_with_sticker(dir: &Path, a: &Assets) -> (Paint, u32) {
        let mut p = Paint::new(Some(dir.to_path_buf()), a);
        p.update(&[click(170, 30)], 0.0, a);
        assert_eq!(p.tool, Tool::Sounds);
        let stamp = Stamp {
            w: 20,
            h: 20,
            px: vec![Some(3); 400],
        };
        // adesivo do usuário (PNG na pasta), que volta igual ao reabrir
        crate::paint::store::save_stamp_png(&stamp, &p.pal, &dir.join("adesivo.png")).unwrap();
        let id = p
            .objects
            .place(Rc::new(stamp), 100, 100, Source::Png("adesivo.png".into()));
        p.help = None;
        (p, id)
    }

    fn at(id: u32, p: &Paint) -> (i32, i32) {
        let o = p.objects.get(id).unwrap();
        (ORIGIN.0 + o.x + 5, ORIGIN.1 + o.y + 5)
    }

    #[test]
    fn sound_attaches_to_the_clicked_sticker_and_plays_from_it() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let (mut p, id) = paint_with_sticker(dir.path(), &a);
        p.sounds.set_hand(Hand::Sound {
            name: "Sol3".into(),
            sound: lib(4),
        });
        let (x, y) = at(id, &p);
        let fx = p.update(&[click(x, y)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(lib(4))));
        assert_eq!(p.objects.get(id).unwrap().sound, Some(lib(4)));
        assert!(p.sounds.hand().is_none(), "a mão fica vazia");
        // sem nada na mão: clicar toca o som do adesivo
        let fx = p.update(&[click(x, y)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(lib(4))));
        // a Varinha Mágica só toca, mesmo com um som na mão
        p.wand = true;
        p.sounds.set_hand(Hand::Silence);
        let fx = p.update(&[click(x, y)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(lib(4))));
        assert!(!p.wand);
        assert_eq!(p.objects.get(id).unwrap().sound, Some(lib(4)));
        p.sounds.take_hand();
        // o fundo não ganha som (nada de ponto solto)
        p.sounds.set_hand(Hand::Sound {
            name: "Lá3".into(),
            sound: lib(5),
        });
        let fx = p.update(&[click(ORIGIN.0 + 400, ORIGIN.1 + 300)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
        assert_eq!(p.objects.get(id).unwrap().sound, Some(lib(4)));
        // "Sem Som" tira o som; desfazer devolve
        p.sounds.set_hand(Hand::Silence);
        p.update(&[click(x, y)], 0.0, &a);
        assert_eq!(p.objects.get(id).unwrap().sound, None);
        // o som vai para o .objetos.json ao sair
        p.sounds.set_hand(Hand::Sound {
            name: "Sol3".into(),
            sound: SoundRef::File(dir.path().join("sons").join("galo.wav")),
        });
        p.update(&[click(x, y)], 0.0, &a);
        p.leave();
        let q = Paint::new(Some(dir.path().to_path_buf()), &a);
        let sounds: Vec<_> = q
            .objects
            .records()
            .into_iter()
            .filter_map(|r| r.sound)
            .collect();
        assert_eq!(
            sounds,
            vec![SoundRef::File(dir.path().join("sons").join("galo.wav"))]
        );
    }

    #[test]
    fn a_word_takes_the_sound_between_its_letters() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let (mut p, _) = paint_with_sticker(dir.path(), &a);
        // palavra 10×10 toda transparente (só o retângulo)
        let id = p.objects.place(
            Rc::new(Stamp {
                w: 10,
                h: 10,
                px: vec![None; 100],
            }),
            300,
            200,
            Source::Words {
                text: "Oi".into(),
                style: WordsStyle::default(),
                color: 1,
            },
        );
        p.sounds.set_hand(Hand::Sound {
            name: "Sol3".into(),
            sound: lib(4),
        });
        p.update(&[click(ORIGIN.0 + 305, ORIGIN.1 + 205)], 0.0, &a);
        assert_eq!(p.objects.get(id).unwrap().sound, Some(lib(4)));
    }

    #[test]
    fn dialogs_take_clicks_and_keys() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let (mut p, _) = paint_with_sticker(dir.path(), &a);
        // Cachorrinho: abre com o som 02f1; os cliques são dele; X fecha
        let fx = p.update(&[click(420, 75)], 0.0, &a);
        assert!(fx.contains(&snd_tool::snd(snd_tool::SND_DOG)));
        assert!(p.sounds.finder.is_some());
        let fx = p.update(&[click(20, 20)], 0.0, &a);
        assert_eq!(p.tool, Tool::Sounds, "a barra não responde com o diálogo");
        assert!(fx.is_empty());
        p.update(&[click(220, 340)], 0.0, &a);
        assert!(p.sounds.finder.is_none());
        // Gravador: o nome recebe o texto; Esc é o NÃO
        p.update(&[click(470, 75)], 0.0, &a);
        assert!(p.sounds.recorder.is_some());
        p.update(&[Input::Text("oi".into())], 0.0, &a);
        assert_eq!(p.sounds.recorder.as_ref().unwrap().name, "oi");
        let fx = p.update(
            &[Input::Key {
                key: Key::Escape,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert!(p.sounds.recorder.is_none());
        assert!(fx.contains(&snd_tool::snd(snd_tool::SND_CLOSE)));
        // gravar e trocar de ferramenta para a gravação
        p.update(&[click(470, 75)], 0.0, &a);
        p.update(&[click(236, 290)], 0.0, &a);
        assert!(p.sounds.recorder.as_ref().unwrap().recording());
        let fx = p.update(&[], 11.0, &a);
        assert!(fx.iter().any(|e| matches!(e, Effect::RecordStop(_))));
    }

    #[test]
    fn old_loose_spots_go_to_the_sticker_below() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let (mut p, id) = paint_with_sticker(dir.path(), &a);
        p.leave();
        std::fs::write(
            dir.path().join(snd_tool::SPOTS_NAME),
            "[\n  {\"x\": 105, \"y\": 105, \"name\": \"Sol3\", \"file\": \"MAG100.SND\", \"index\": 4},\n  {\"x\": 400, \"y\": 300, \"name\": \"x\", \"file\": \"MAG100.SND\", \"index\": 5}\n]\n",
        )
        .unwrap();
        let mut q = Paint::new(Some(dir.path().to_path_buf()), &a);
        let o = q.objects.records();
        assert_eq!(o.len(), 1);
        let _ = id;
        assert_eq!(o[0].sound, Some(lib(4)));
        q.leave();
        assert!(!dir.path().join(snd_tool::SPOTS_NAME).exists());
    }
}
