//! Android's in-game character customizer.
use super::{Game, Page, Worn};
use crate::ha_launcher::{
    account::{Provider, session},
    documents,
    skins::{
        self, cloud,
        library::{Kind, Library},
    },
};
use horizon_client::{hud::HudBatch, own_skin::CapeChoice};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

const LOCAL_CAPE: &str = "capes/starling-local";
struct AccountCapes {
    rows: Vec<(String, String, bool)>,
    equipped_url: Option<String>,
    activated: bool,
}

#[derive(Default)]
pub(super) struct Wardrobe {
    open: bool,
    picking: bool,
    profile: Option<Option<String>>,
    cape: CapeChoice,
    cape_image: Option<Vec<u8>>,
    uploaded: bool,
    account_worker: Option<Receiver<Result<AccountCapes, String>>>,
    account_cape_url: Option<String>,
}

impl Game<'_> {
    pub(in crate::game) fn draw_wardrobe_preview(&mut self, batch: &mut HudBatch) {
        let Some([left, top, right, bottom]) = self
            .ui
            .options
            .wardrobe_preview_rect(batch.width, batch.height)
        else {
            return;
        };
        let in_world = self.title.is_none();
        let mut mob = if in_world {
            let Some(mob) = self.player_model.clone() else {
                return;
            };
            mob
        } else {
            let worn = self.own_worn();
            if self.own_skin.title.is_none() {
                self.read_title_player();
            }
            let Some(player) = &self.own_skin.title else {
                return;
            };
            let mut mob = player.mob.clone();
            (mob.skin, mob.slim) = match worn {
                Worn::Skin(skin, slim) => (Some(skin), slim),
                Worn::Default | Worn::Waiting => (None, false),
            };
            mob
        };
        if in_world {
            if let Some((skin, slim)) = self.own_skin_sprite() {
                mob.skin = Some(skin);
                mob.slim = slim;
            }
        }
        self.ensure_wardrobe_profile();
        let server_cape = mob.cape.filter(|location| location != LOCAL_CAPE);
        let account_cape = if self.own_skin.wardrobe.cape == CapeChoice::Account {
            self.account_cape_preview().or(server_cape)
        } else {
            None
        };
        mob.cape = self.wardrobe_cape(account_cape);
        if !self.settings.file.bool("modelPart_cape") {
            mob.cape = None;
        }
        let data = if self.res.rest_ready() {
            self.res.mob_data()
        } else if let Some(data) = &self.own_skin.title_data {
            data
        } else {
            return;
        };
        let cx = (left + right) * 0.5;
        let cy = (top + bottom) * 0.5;
        let size = ((right - left) * 0.68).min((bottom - top) * 0.43);
        self.mob_preview(
            &mob,
            data,
            batch,
            [left + 2.0, top + 2.0, right - 2.0, bottom - 2.0],
            size,
            0.0625,
            (cx, cy),
            self.ui.options.wardrobe_rotation(),
            false,
        );
    }

    pub(super) fn show_wardrobe(&mut self) {
        let open = self.ui.options.page() == Page::Skin;
        if open {
            let changed = self.ensure_wardrobe_profile();
            if changed || !self.own_skin.wardrobe.open {
                self.refresh_wardrobe();
            }
        }
        self.own_skin.wardrobe.open = open;
        if let Some(result) = self
            .own_skin
            .wardrobe
            .account_worker
            .as_ref()
            .and_then(|worker| match worker.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Could not load account capes".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            })
        {
            self.own_skin.wardrobe.account_worker = None;
            self.ui.options.wardrobe_account_busy = false;
            self.ui.options.refresh_skin_page();
            match result {
                Ok(capes) => {
                    self.ui.options.wardrobe_account_capes = Some(capes.rows);
                    self.own_skin.wardrobe.account_cape_url = capes.equipped_url;
                    if capes.activated {
                        self.choose_own_cape(CapeChoice::Account);
                        self.reload_own_profile_textures();
                    }
                }
                Err(err) => self.ui.options.skin_warning(err),
            }
        }
    }

    pub(in crate::game) fn account_capes(&mut self, id: Option<String>) {
        self.ensure_wardrobe_profile();
        if self.own_skin.wardrobe.account_worker.is_some() {
            return;
        }
        let key = crate::accounts::player_key();
        let root = Self::wardrobe_root();
        let (send, receive) = mpsc::channel();
        self.own_skin.wardrobe.account_worker = Some(receive);
        self.ui.options.wardrobe_account_busy = true;
        self.ui.options.wardrobe_account_capes = None;
        std::thread::spawn(move || {
            let result = (|| {
                let (account, secrets) = session::selected_session("")?
                    .ok_or("Sign in to Microsoft to manage account capes.")?;
                if account.provider != Provider::Microsoft || Some(account.key()) != key {
                    return Err("The playing Microsoft account changed.".into());
                }
                if let Some(id) = &id {
                    cloud::activate_cape(&account, &secrets, Some(id))?;
                }
                let capes = cloud::fetch(&account, &secrets, &root)?.snapshot.capes;
                let equipped_url = capes
                    .iter()
                    .find(|cape| cape.active)
                    .map(|cape| cape.url.clone());
                let rows = capes
                    .into_iter()
                    .map(|cape| (cape.id, cape.title, cape.active))
                    .collect();
                Ok(AccountCapes {
                    rows,
                    equipped_url,
                    activated: id.is_some(),
                })
            })();
            let _ = send.send(result);
        });
    }

    fn refresh_wardrobe(&mut self) {
        let root = Self::wardrobe_root();
        let result = Library::load(&root).and_then(|library| {
            let skin_dir = Self::skins_dir();
            let mut titles = std::collections::HashMap::new();
            let mut capes = Vec::new();
            for entry in library.entries {
                let source = Library::image(&root, &entry.id)?;
                if !source.is_file() {
                    continue;
                }
                match entry.kind {
                    Kind::Skin => {
                        let name = format!("wardrobe-{}.png", entry.id);
                        let path = skin_dir.join(&name);
                        if !path.is_file() {
                            std::fs::create_dir_all(&skin_dir).map_err(|e| e.to_string())?;
                            let part = path.with_extension("png.part");
                            std::fs::copy(source, &part).map_err(|e| e.to_string())?;
                            std::fs::rename(part, path).map_err(|e| e.to_string())?;
                        }
                        titles.insert(name, entry.title);
                    }
                    Kind::Cape => capes.push((entry.id, entry.title)),
                }
            }
            capes.sort_by_cached_key(|(_, title)| title.to_lowercase());
            self.ui.options.wardrobe_skin_titles = titles;
            self.ui.options.wardrobe_capes = capes;
            Ok(())
        });
        self.ui.options.refresh_skin_page();
        self.ui.options.wardrobe_cape = self.own_skin.wardrobe.cape.clone();
        if let Err(error) = result {
            self.ui.options.skin_warning(error);
        }
    }

    pub(in crate::game) fn pick_wardrobe_png(&mut self, kind: Kind) {
        match documents::begin_appearance(Self::wardrobe_root(), kind) {
            Ok(()) => self.own_skin.wardrobe.picking = true,
            Err(error) => self.ui.options.skin_warning(error),
        }
    }

    pub(in crate::game) fn poll_wardrobe_import(&mut self) {
        if self.own_skin.wardrobe.picking && !documents::busy() {
            self.own_skin.wardrobe.picking = false;
            let notice = documents::poll();
            self.refresh_wardrobe();
            if let Some(notice) = notice
                && (notice.text.starts_with("Import failed")
                    || notice.text == "Android document picker is unavailable")
            {
                self.ui.options.skin_warning(notice.text);
            }
        }
    }

    fn ensure_wardrobe_profile(&mut self) -> bool {
        let key = crate::accounts::player_key();
        if self.own_skin.wardrobe.profile.as_ref() == Some(&key) {
            return false;
        }
        self.own_skin.wardrobe.account_worker = None;
        self.own_skin.wardrobe.account_cape_url = match crate::accounts::profiles().player() {
            Some(starling_account::Player::Account(account)) => account.cape,
            _ => None,
        };
        self.ui.options.wardrobe_account_capes = None;
        self.ui.options.wardrobe_account_busy = false;
        let choice = CapeChoice::load(&Self::player_dir(), key.as_deref()).unwrap_or_default();
        let image = Self::cape_image(&choice).ok().flatten();
        self.set_wardrobe_cape(key, choice, image);
        true
    }

    fn set_wardrobe_cape(
        &mut self,
        key: Option<String>,
        choice: CapeChoice,
        image: Option<Vec<u8>>,
    ) {
        let wardrobe = &mut self.own_skin.wardrobe;
        wardrobe.profile = Some(key);
        wardrobe.cape = choice.clone();
        wardrobe.cape_image = image;
        wardrobe.uploaded = false;
        self.ui.options.wardrobe_cape = choice;
    }

    pub(in crate::game) fn choose_own_cape(&mut self, choice: CapeChoice) {
        let key = crate::accounts::player_key();
        let result = Self::cape_image(&choice).and_then(|image| {
            choice
                .save(&Self::player_dir(), key.as_deref())
                .map_err(|e| e.to_string())?;
            Ok(image)
        });
        match result {
            Ok(image) => self.set_wardrobe_cape(key, choice, image),
            Err(error) => self.ui.options.skin_warning(error),
        }
    }

    pub(in crate::game) fn wardrobe_cape(&mut self, account: Option<String>) -> Option<String> {
        self.ensure_wardrobe_profile();
        match &self.own_skin.wardrobe.cape {
            CapeChoice::Account => account,
            CapeChoice::Hidden => None,
            CapeChoice::Local(_) => {
                if !self.own_skin.wardrobe.uploaded || !self.show_skin(LOCAL_CAPE) {
                    let image = self.own_skin.wardrobe.cape_image.clone()?;
                    self.store_skin(LOCAL_CAPE.into(), image);
                    self.own_skin.wardrobe.uploaded = true;
                }
                self.show_skin(LOCAL_CAPE).then(|| LOCAL_CAPE.into())
            }
        }
    }

    fn account_cape_preview(&mut self) -> Option<String> {
        use horizon_client::skins::{Download, Kind, Textures};
        let url = self.own_skin.wardrobe.account_cape_url.clone()?;
        let downloaded = self.skin_downloads.borrow_mut().state(Kind::Cape, &url);
        if downloaded != Download::Ready {
            return None;
        }
        self.store_ready_skins();
        let location = horizon_client::skins::cape(&Textures {
            cape: Some(url),
            ..Default::default()
        })?;
        self.again_skin(&location).then_some(location)
    }

    fn wardrobe_root() -> PathBuf {
        Self::player_dir().join("game-wardrobe")
    }

    fn cape_image(choice: &CapeChoice) -> Result<Option<Vec<u8>>, String> {
        let CapeChoice::Local(id) = choice else {
            return Ok(None);
        };
        let path = Library::image(&Self::wardrobe_root(), id)?;
        let image = skins::read_cape(&skins::read_file(&path)?)?;
        horizon_client::skins::process_cape(image.width(), image.height(), image.as_raw())
            .map(Some)
            .ok_or_else(|| "Could not read this cape".into())
    }
}
