//! Lista de figuras do Quadro de Figuras (`ideas.md` §4): as figuras do
//! usuário, das mais novas para as mais velhas, e depois as 28 do Max na
//! ordem do `PWDG` (`PICT 0..27` do `WORKSHOP.CHK`). É a ordem da lista
//! suspensa; OUTRA sorteia; POE põe uma figura nova no começo; TIRA só joga
//! fora figuras do usuário.

use super::machine::RemoveError;
use super::session::BoardKey;
use super::store::TitledPicture;
use crate::Image;
use crate::rng::XorShift;
use formats::workshop::Workshop;

/// Figura do Max (`PICT n` do `WORKSHOP.CHK`).
#[derive(Debug, Clone)]
pub struct MaxPicture {
    pub pict: u16,
    pub title: String,
    pub image: Image,
}

/// As figuras do Max na ordem da lista, já decodificadas (as que não
/// decodificam são puladas).
pub fn max_pictures(w: &Workshop<'_>) -> Vec<MaxPicture> {
    w.pictures_in_order()
        .into_iter()
        .filter_map(|p| {
            Some(MaxPicture {
                pict: p.id,
                title: p.title.clone(),
                image: formats::wmf::meta_dib(p.meta).ok()?,
            })
        })
        .collect()
}

/// Lê `SHARED/STUFF/WORKSHOP.CHK` da pasta `MSKIDS` (vazio se faltar).
pub fn load_max_pictures(data_dir: &std::path::Path) -> Vec<MaxPicture> {
    let Some(path) = crate::paths::find_ci(data_dir, "SHARED/STUFF/WORKSHOP.CHK") else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    Workshop::parse(&bytes)
        .map(|w| max_pictures(&w))
        .unwrap_or_default()
}

/// O conteúdo do Quadro e a figura à mostra.
#[derive(Debug, Clone)]
pub struct Board {
    max: Vec<MaxPicture>,
    user: Vec<TitledPicture>,
    current: usize,
    /// As figuras do usuário mudaram desde a leitura (gravar ao sair).
    dirty: bool,
}

impl Board {
    pub fn new(max: Vec<MaxPicture>, user: Vec<TitledPicture>) -> Board {
        Board {
            max,
            user,
            current: 0,
            dirty: false,
        }
    }

    pub fn len(&self) -> usize {
        self.user.len() + self.max.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_user(&self, i: usize) -> bool {
        i < self.user.len()
    }

    pub fn title(&self, i: usize) -> &str {
        if let Some(p) = self.user.get(i) {
            &p.title
        } else {
            self.max
                .get(i - self.user.len())
                .map_or("", |p| p.title.as_str())
        }
    }

    pub fn image(&self, i: usize) -> Option<&Image> {
        if let Some(p) = self.user.get(i) {
            Some(&p.image)
        } else {
            self.max.get(i - self.user.len()).map(|p| &p.image)
        }
    }

    /// Chave da figura `i` (para lembrar dela na sessão).
    pub fn key(&self, i: usize) -> Option<BoardKey> {
        if let Some(p) = self.user.get(i) {
            Some(BoardKey::User(p.title.clone()))
        } else {
            self.max
                .get(i - self.user.len())
                .map(|p| BoardKey::Max(p.pict))
        }
    }

    pub fn find(&self, k: &BoardKey) -> Option<usize> {
        (0..self.len()).find(|&i| self.key(i).as_ref() == Some(k))
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn select(&mut self, i: usize) {
        if i < self.len() {
            self.current = i;
        }
    }

    /// OUTRA: outra figura sorteada (diferente da atual, se houver outra).
    pub fn other(&mut self, rng: &mut XorShift) {
        let n = self.len();
        if n > 1 {
            let r = rng.below(n - 1);
            self.current = if r >= self.current { r + 1 } else { r };
        }
    }

    /// POE: a figura nova entra no começo da lista e fica à mostra.
    pub fn add_user(&mut self, title: &str, image: Image) {
        self.user.insert(0, TitledPicture::new(title, image));
        self.current = 0;
        self.dirty = true;
    }

    /// Título da figura do usuário `i` (digitado depois do POE).
    pub fn set_title(&mut self, i: usize, title: &str) {
        if let Some(p) = self.user.get_mut(i) {
            p.title = title.trim().to_string();
            self.dirty = true;
        }
    }

    /// Figura `i` modificada no cavalete. As coisas do Max não mudam: a
    /// figura modificada entra como figura nova do usuário, com o mesmo
    /// título, no começo da lista.
    pub fn modify(&mut self, i: usize, image: Image) {
        if let Some(p) = self.user.get_mut(i) {
            p.set_image(image);
            self.dirty = true;
        } else if i < self.len() {
            let title = self.title(i).to_string();
            self.add_user(&title, image);
        }
    }

    /// TIRA: joga fora a figura do usuário `i`; a atual passa a ser a que
    /// estava depois dela.
    pub fn remove(&mut self, i: usize) -> Result<TitledPicture, RemoveError> {
        if !self.is_user(i) {
            return Err(RemoveError::MaxEntry);
        }
        let p = self.user.remove(i);
        self.dirty = true;
        self.current = self.current.min(self.len().saturating_sub(1));
        Ok(p)
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Figuras do usuário (para gravar); depois de gravar, `saved`.
    pub fn user_pictures_mut(&mut self) -> &mut [TitledPicture] {
        &mut self.user
    }

    pub fn saved(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32) -> Image {
        Image {
            width: w,
            height: 2,
            rgba: vec![255; (w * 2 * 4) as usize],
        }
    }

    fn board() -> Board {
        let max = (0..3)
            .map(|k| MaxPicture {
                pict: k,
                title: format!("Max {k}"),
                image: img(10 + k as u32),
            })
            .collect();
        Board::new(max, vec![TitledPicture::new("Minha", img(5))])
    }

    #[test]
    fn user_pictures_come_first_then_max_in_order() {
        let b = board();
        assert_eq!(b.len(), 4);
        let titles: Vec<&str> = (0..4).map(|i| b.title(i)).collect();
        assert_eq!(titles, ["Minha", "Max 0", "Max 1", "Max 2"]);
        assert!(b.is_user(0) && !b.is_user(1));
        assert_eq!(b.key(2), Some(BoardKey::Max(1)));
        assert_eq!(b.find(&BoardKey::User("Minha".into())), Some(0));
    }

    #[test]
    fn other_never_repeats_the_current_picture() {
        let mut b = board();
        let mut rng = XorShift::new(7);
        for _ in 0..50 {
            let before = b.current();
            b.other(&mut rng);
            assert_ne!(b.current(), before);
            assert!(b.current() < b.len());
        }
    }

    #[test]
    fn new_picture_goes_first_and_max_ones_cannot_be_removed() {
        let mut b = board();
        b.add_user("", img(600));
        assert_eq!(b.current(), 0);
        assert!(b.dirty());
        b.set_title(0, " Meu rabisco ");
        assert_eq!(b.title(0), "Meu rabisco");
        assert_eq!(b.remove(3).unwrap_err(), RemoveError::MaxEntry);
        b.select(1);
        assert_eq!(b.remove(1).unwrap().title, "Minha");
        assert_eq!(b.len(), 4);
        assert_eq!(b.title(1), "Max 0");
    }

    #[test]
    fn modifying_a_max_picture_makes_a_user_copy() {
        let mut b = board();
        b.modify(2, img(3));
        assert_eq!(b.len(), 5);
        assert_eq!(b.title(0), "Max 1");
        assert!(b.is_user(0));
        assert_eq!(b.current(), 0);
        b.modify(0, img(4));
        assert_eq!(b.len(), 5);
        assert_eq!(b.image(0).unwrap().width, 4);
    }
}
