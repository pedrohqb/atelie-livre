//! Pasta de documentos do Creative Writer, aberta pela maleta da Bolsa
//! Mágica ou por Ctrl+O nas salas (tarefa T13; `cw/report/building.md`
//! §8.3, capturas `building_31_pasta`, `33_pasta_max_open`,
//! `34_pasta_doc`): o mesmo fichário verde do Fine Artist
//! (`portfolio::Binder`), com os documentos `.MAX` agrupados por área
//! ("2 Escritório", "3 Cartões", "3 Faixas", "3 Jornais"), a prévia da
//! primeira página (o `THMB` do documento, 108×144, sem redução) e
//! "Título"/**"Autor"**. Os do usuário (`<usuários>/<apelido>/*.MAX`, a pasta
//! de `writer::prefs`) vêm primeiro; embaixo, os exemplos do Max
//! (`MSKIDS/SHARED/USERS/MAX`).
//!
//! JÁ (ou clique duplo) abre o documento: ele vira o último documento do
//! usuário (`FILR 0` do `~MAXUSER.INF`) e a cena vai à área de escrita
//! (`SceneId::Writer`), que o abre por `writer::prefs::last_document`. Os
//! cartões, faixas e jornais também vão à área de escrita enquanto os
//! projetos do Writer não existem.

use super::portfolio::{self, Action as BinderAction, Binder, BinderItem, BinderSpec, Group};
use super::writer_office_room::bitmap_image;
use crate::assets::{Assets, Program, SoundRef};
use crate::paths::find_ci;
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::ui::{Rect, inside};
use crate::writer::prefs;
use crate::{Frame, Image};
use formats::max::{FLOOR_PROJECTS, Project};
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

/// Prévia da página na página direita do fichário (medida em
/// `building_34_pasta_doc`: a miniatura inteira em (363,126)).
pub const THUMB: Rect = (363, 126, 108, 144);

/// A pasta do Writer.
pub const WRITER_BINDER: BinderSpec = BinderSpec {
    program: Program::Writer,
    // "Para ver mais escolhas, clique aqui." (`0bb8[53]`)
    help: (0x0bb8, 53),
    search: (0x0213, 0),
    author_label: "Autor",
    samples_owner: "MAX",
    thumb: THUMB,
};

/// Áreas da árvore (andar e rótulo, como no original).
pub const OFFICE: Group = Group {
    floor: 2,
    label: "Escritório",
};
pub const CARDS: Group = Group {
    floor: 3,
    label: "Cartões",
};
pub const BANNERS: Group = Group {
    floor: 3,
    label: "Faixas",
};
pub const NEWSPAPERS: Group = Group {
    floor: 3,
    label: "Jornais",
};

/// Área de um documento pelo projeto (`WIZA`) e pelo andar (`FLOR`).
pub fn area(project: Project, floor: u16) -> Group {
    match project {
        Project::Card => CARDS,
        Project::Banner => BANNERS,
        Project::Newspaper => NEWSPAPERS,
        _ if floor == FLOOR_PROJECTS => CARDS,
        _ => OFFICE,
    }
}

/// Um documento da pasta.
#[derive(Debug, Clone)]
pub struct DocItem {
    pub path: PathBuf,
    pub title: String,
    pub group: Group,
    pub sample: bool,
    pub thumb: Option<Image>,
}

impl BinderItem for DocItem {
    fn name(&self) -> &str {
        &self.title
    }
    fn sample(&self) -> bool {
        self.sample
    }
    fn author(&self) -> &str {
        // os exemplos mostram o dono da pasta ("MAX"), não o autor gravado
        ""
    }
    fn group(&self) -> Group {
        self.group
    }
    fn draw_thumb(&self, f: &mut Frame, r: Rect, _pal: &[Rgb]) {
        let Some(img) = &self.thumb else { return };
        let (x, y) = (r.0 + (r.2 - img.width as i32) / 2, r.1);
        f.rect(
            x - 1,
            y - 1,
            img.width as i32 + 2,
            img.height as i32 + 2,
            [0, 0, 0],
        );
        f.blit(img, x, y);
    }
}

/// Lê um `.MAX` como item da pasta (título, área e prévia).
fn doc_item(path: &Path, sample: bool, assets: &Assets) -> Option<DocItem> {
    let doc = formats::max::read(&std::fs::read(path).ok()?).ok()?;
    let pal = assets.palettes(Program::Writer);
    let title = if doc.add.title.is_empty() {
        path.file_stem()?.to_string_lossy().to_string()
    } else {
        doc.add.title.clone()
    };
    Some(DocItem {
        path: path.to_path_buf(),
        title,
        group: area(doc.add.project, doc.add.floor),
        sample,
        thumb: doc
            .thumbnail_bitmap()
            .map(|b| bitmap_image(&b, &pal.pal16, &pal.pal256)),
    })
}

/// `.MAX` de uma pasta, em ordem de nome.
fn max_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("max")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Os documentos do usuário e os exemplos do Max, cada grupo em ordem de
/// área e de título.
pub fn list_documents(users: Option<&Path>, user: &str, assets: &Assets) -> Vec<DocItem> {
    let mut own: Vec<DocItem> = users
        .map(|u| max_files(&u.join(user)))
        .unwrap_or_default()
        .iter()
        .filter_map(|p| doc_item(p, false, assets))
        .collect();
    let samples_dir = find_ci(assets.data_dir(), &prefs::SAMPLES_DIR.join("/"));
    let mut samples: Vec<DocItem> = samples_dir
        .map(|d| max_files(&d))
        .unwrap_or_default()
        .iter()
        .filter_map(|p| doc_item(p, true, assets))
        .collect();
    for v in [&mut own, &mut samples] {
        v.sort_by(|a, b| {
            (a.group.floor, a.group.label, a.title.to_lowercase()).cmp(&(
                b.group.floor,
                b.group.label,
                b.title.to_lowercase(),
            ))
        });
    }
    own.extend(samples);
    own
}

/// A pasta, por cima da sala.
pub struct WriterPortfolio {
    common: KidCommon,
    binder: Binder,
    items: Vec<DocItem>,
    users: Option<PathBuf>,
    user: String,
    data: PathBuf,
    clock: f32,
}

impl WriterPortfolio {
    pub fn new(
        click: Option<SoundRef>,
        users: Option<PathBuf>,
        user: &str,
        animal: Option<usize>,
        assets: &Assets,
    ) -> WriterPortfolio {
        let items = list_documents(users.as_deref(), user, assets);
        let mut binder = Binder::new("C:\\".to_string()).with_spec(&WRITER_BINDER);
        binder.set_user(user, animal, &items);
        binder.open(true, assets);
        WriterPortfolio {
            common: KidCommon::new(click),
            binder,
            items,
            users,
            user: user.to_string(),
            data: assets.data_dir().to_path_buf(),
            clock: 0.0,
        }
    }

    pub fn items(&self) -> &[DocItem] {
        &self.items
    }

    pub fn binder(&self) -> &Binder {
        &self.binder
    }

    /// Abre o documento `i` na área de escrita (ele vira o último).
    fn open(&mut self, i: usize, fx: &mut Vec<Effect>) {
        let Some(it) = self.items.get(i) else { return };
        if let Some(u) = &self.users
            && let Err(e) = prefs::remember_last(u, &self.user, &self.data, &it.path, &it.title)
        {
            fx.push(Effect::Log(format!("pasta: {e}")));
        }
        fx.push(Effect::PlaySound(SoundRef::Snd(Program::Writer, 0x304)));
        fx.push(Effect::ChangeScene(SceneId::Writer));
    }
}

impl Scene for WriterPortfolio {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.clock += crate::scene::click_time(dt);
        for i in input {
            if !matches!(i, Input::Click { .. }) {
                self.binder.key(i, &self.items, &mut fx, assets);
                continue;
            }
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            // fora do fichário: fecha e volta à sala
            if !inside(portfolio::BINDER, x, y) && !self.binder.search_open() {
                fx.push(Effect::Back);
                break;
            }
            match self
                .binder
                .click(x, y, self.clock, &self.items, &mut fx, assets)
            {
                Some(BinderAction::Close) => {
                    fx.push(Effect::Back);
                    break;
                }
                Some(BinderAction::Open(k)) => {
                    self.open(k, &mut fx);
                    break;
                }
                None => {}
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        self.binder.draw(frame, assets, &self.items, &[]);
    }

    fn overlay(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    #[test]
    fn areas_follow_project_and_floor() {
        assert_eq!(area(Project::None, 2), OFFICE);
        assert_eq!(area(Project::Card, 3), CARDS);
        assert_eq!(area(Project::Banner, 3), BANNERS);
        assert_eq!(area(Project::Newspaper, 3), NEWSPAPERS);
    }

    #[test]
    fn empty_portfolio_closes() {
        let (_d, a) = empty_assets();
        let mut p = WriterPortfolio::new(None, None, "ana", None, &a);
        assert!(p.items().is_empty());
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        let fx = p.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 5,
                y: 5,
            }],
            0.0,
            &a,
        );
        assert!(fx.contains(&Effect::Back));
    }

    /// Com os dados reais: os exemplos do Max agrupados por área ("O Ponto
    /// de Fuga" no Escritório), com prévia; JÁ abre o escolhido, que vira
    /// o último documento.
    #[test]
    fn samples_by_area_with_real_data() {
        let Some(a) = assets() else { return };
        let users = std::env::temp_dir().join(format!("atelie-cw-pasta-{}", std::process::id()));
        std::fs::create_dir_all(users.join("ana")).unwrap();
        let mut p = WriterPortfolio::new(None, Some(users.clone()), "ana", None, &a);
        assert_eq!(p.items().len(), 21);
        let first = &p.items()[0];
        assert_eq!(
            (first.title.as_str(), first.group),
            ("O Ponto de Fuga", OFFICE)
        );
        assert!(p.items().iter().all(|i| i.sample && i.thumb.is_some()));
        let groups: Vec<&str> = p.items().iter().map(|i| i.group.label).collect();
        assert!(groups.contains(&"Cartões") && groups.contains(&"Jornais"));
        // JÁ com o primeiro escolhido
        let (gx, gy, _, _) = portfolio::BUTTON_GO;
        let fx = p.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: gx + 5,
                y: gy + 5,
            }],
            0.0,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Writer)), "{fx:?}");
        let (d, _) = prefs::last_document(Some(&users), "ana", a.data_dir()).unwrap();
        assert_eq!(d.file, "SAMP18.MAX");
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "cw_portfolio");
    }
}
