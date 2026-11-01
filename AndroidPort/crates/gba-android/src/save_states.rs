//! Android full-machine saves. Same checked on-disk format and carousel as desktop.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
pub struct AutoSave {
    path: PathBuf,
    pub slots: Arc<Mutex<BTreeMap<usize, CardInfo>>>,
    map_state: Option<(Vec<u8>, CardInfo)>,
    map_frame: u64,
    map_was_clean: bool,
    return_after_save: Option<usize>,
    pending_import: Option<usize>,
    pub pending_slot: usize,
    mission_image: Vec<u16>,
    mission_stage: Option<u16>,
    mission_since: u64,
    mission_picked: bool,
    pub shots: Vec<(u64, Vec<u16>)>,
    tx: Option<mpsc::SyncSender<(usize, Vec<u8>, CardInfo)>>,
    worker: Option<thread::JoinHandle<()>>,
    available: Arc<AtomicBool>,
    save_result: Arc<Mutex<Option<Result<usize, String>>>>,
    last: Instant,
    pub info: Arc<Mutex<Option<CardInfo>>>,
    pub selected: usize,
    pub carousel_start: usize,
    pub delete_confirm: Option<usize>,
    pub delete_accept: bool,
    routed: bool,
    pub screen: bool,
    pub pending_load: bool,
    pub message: Option<String>,
}
fn backup(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.bak",
        path.file_name().unwrap().to_string_lossy()
    ))
}
fn pack(state: &[u8], image: &[u16]) -> Vec<u8> {
    let mut body = state.to_vec();
    for p in image.iter().take(38400) {
        body.extend_from_slice(&p.to_le_bytes());
    }
    let mut out = b"FFTASV02".to_vec();
    out.extend_from_slice(&(state.len() as u32).to_le_bytes());
    out.extend_from_slice(&(image.len().min(38400) as u32).to_le_bytes());
    out.extend_from_slice(gba_core::ffta_rom::digest(&body).as_bytes());
    out.extend(body);
    out
}
fn unpack(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u16>), String> {
    if bytes.get(..8) == Some(b"FFTAST01") {
        gba_core::state::preview(bytes)?;
        return Ok((bytes.to_vec(), Vec::new()));
    }
    if bytes.len() < 80 || &bytes[..8] != b"FFTASV02" {
        return Err("Invalid save format".into());
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if count > 38400
        || n > 16 * 1024 * 1024
        || bytes.len() != 80 + n + count * 2
        || gba_core::ffta_rom::digest(&bytes[80..]).as_bytes() != &bytes[16..80]
    {
        return Err("Save checksum or length mismatch".into());
    }
    gba_core::state::preview(&bytes[80..80 + n])?;
    Ok((
        bytes[80..80 + n].to_vec(),
        bytes[80 + n..]
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect(),
    ))
}
fn read_bundle(path: &Path) -> Result<(Vec<u8>, Vec<u16>), String> {
    [path.to_path_buf(), backup(path)]
        .iter()
        .find_map(|p| std::fs::read(p).ok().and_then(|b| unpack(&b).ok()))
        .ok_or_else(|| format!("No valid save: {}", path.display()))
}
fn read_card(path: &Path) -> Option<CardInfo> {
    let (state, image) = read_bundle(path).ok()?;
    let (ram, pixels) = gba_core::state::preview(&state).ok()?;
    let mut card = CardInfo::gameplay(&ram, &pixels);
    if image.len() == 38400 {
        card.image = image;
    }
    Some(card)
}
fn archive_legacy(original: &Path, quick: &Path) {
    let Ok(bytes) = std::fs::read(original) else {
        return;
    };
    let mut present = [false; 2];
    for header in bytes.windows(128).filter(|b| &b[..8] == b"FFTEX000") {
        // Verified EUR headers tag the normal slots 0 and 1 at byte10.
        let tag = header[0x10] as usize;
        if tag < 2 && CardInfo::header(header).is_some() {
            present[tag] = true;
        }
    }
    // Never archive an unrecognised legacy file or hide an unconverted slot.
    if !present.iter().any(|p| *p) {
        return;
    }
    if present
        .iter()
        .enumerate()
        .any(|(i, p)| *p && read_card(&quick.with_file_name(format!("Save{}", i + 1))).is_none())
    {
        return;
    }
    let dest = original.with_file_name(format!(
        "{}.legacy-backup",
        original.file_name().unwrap().to_string_lossy()
    ));
    if dest.exists() {
        return;
    }
    match std::fs::rename(original, &dest) {
        Ok(()) => eprintln!("FFTA_IMPORT archived={}", dest.display()),
        Err(e) => eprintln!("FFTA_IMPORT archive_error={e}"),
    }
}
fn clean_map(mem: &gba_core::MemMap) -> bool {
    let half = |o| u16::from_le_bytes([mem.io[o], mem.io[o + 1]]);
    let word = |b: &[u8], o: usize| {
        b.get(o..o + 4)
            .map(|p| u32::from_le_bytes(p.try_into().unwrap()))
    };
    let cursor = (|| {
        let manager = word(&mem.iwram, 0x2810)?.checked_sub(0x02000000)? as usize;
        let base = word(&mem.ewram, manager.checked_add(4)?)?.checked_sub(0x02000000)? as usize;
        Some(mem.ewram.get(base)? & 3 == 1)
    })()
    .unwrap_or(false);
    // Cursor ownership and the world task state work with mouse disabled too.
    // The backdrop alone stays visible behind System and cannot identify a clean map.
    half(0) & 0x0f87 == 0x0f00
        && half(8) == 0xaa03
        && half(10) == 0xac02
        && word(&mem.iwram, 0x2804) == Some(0x08031601)
        && mem.ewram[0x1c3c4] == 0
        && cursor
}
fn replace(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let temp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name().unwrap().to_string_lossy()
    ));
    let backup = backup(path);
    let mut f = std::fs::File::create(&temp)?;
    use std::io::Write;
    f.write_all(data)?;
    f.sync_all()?;
    drop(f);
    if path.is_file() {
        std::fs::copy(path, &backup)?;
    }
    // On Windows std::fs::rename replaces an existing file atomically.
    std::fs::rename(temp, path)
}
impl AutoSave {
    pub fn new(sav: &str, hash: &str) -> Self {
        let dir = Path::new(sav)
            .parent()
            .unwrap_or(Path::new("."))
            .join("Saves");
        Self::new_at(sav, hash, dir)
    }
    pub fn new_at(sav: &str, hash: &str, dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("FFTA_SAVE directory_error={e}");
        }
        let path = dir.join("QuickSave");
        let old = Path::new(sav).with_extension(format!("{}.autosave.state", &hash[..12]));
        if !path.exists() {
            if let Ok(bytes) = std::fs::read(old) {
                if let Ok((ram, pixels)) = gba_core::state::preview(&bytes) {
                    let _ = replace(
                        &path,
                        &pack(&bytes, &CardInfo::gameplay(&ram, &pixels).image),
                    );
                }
            }
        }
        let initial = read_card(&path);
        let available = Arc::new(AtomicBool::new(initial.is_some()));
        let info = Arc::new(Mutex::new(initial));
        let mut ids = BTreeSet::new();
        if let Ok(files) = std::fs::read_dir(&dir) {
            for entry in files.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(id) = name
                    .strip_prefix("Save")
                    .and_then(|v| v.strip_suffix(".bak").unwrap_or(v).parse::<usize>().ok())
                    .filter(|id| *id > 0 && *id < usize::MAX)
                {
                    ids.insert(id);
                }
            }
        }
        let slots = Arc::new(Mutex::new(
            ids.into_iter()
                .filter_map(|id| read_card(&dir.join(format!("Save{id}"))).map(|c| (id - 1, c)))
                .collect::<BTreeMap<_, _>>(),
        ));
        let ready = available.clone();
        let published = info.clone();
        let manual = slots.clone();
        let save_result = Arc::new(Mutex::new(None));
        let result = save_result.clone();
        let dest = path.clone();
        let legacy = PathBuf::from(sav);
        let original = legacy.clone();
        let (tx, rx) = mpsc::sync_channel::<(usize, Vec<u8>, CardInfo)>(4);
        let worker = thread::spawn(move || {
            while let Ok((slot, bytes, metadata)) = rx.recv() {
                let target = if slot == 0 {
                    dest.clone()
                } else {
                    dest.with_file_name(format!("Save{slot}"))
                };
                match replace(&target, &pack(&bytes, &metadata.image)) {
                    Ok(()) => {
                        if slot == 0 {
                            *published.lock().unwrap() = Some(metadata);
                            ready.store(true, Ordering::Release);
                        } else {
                            manual.lock().unwrap().insert(slot - 1, metadata);
                        }
                        eprintln!(
                            "FFTA_STATE_SAVE slot={slot} bytes={} path={}",
                            bytes.len(),
                            target.display()
                        );
                        if slot > 0 {
                            let _ = std::fs::remove_file(
                                dest.with_file_name(format!("Save{slot}.deleted")),
                            );
                        }
                        if slot > 0 {
                            *result.lock().unwrap() = Some(Ok(slot));
                        }
                        if slot > 0 {
                            archive_legacy(&original, &dest);
                        }
                    }
                    Err(e) => {
                        if slot > 0 {
                            *result.lock().unwrap() = Some(Err(e.to_string()));
                        }
                        eprintln!("FFTA_STATE_SAVE error={e}");
                    }
                }
            }
        });
        Self {
            path,
            slots,
            map_state: None,
            map_frame: 0,
            map_was_clean: false,
            return_after_save: None,
            pending_import: None,
            pending_slot: 0,
            mission_image: Vec::new(),
            mission_stage: None,
            mission_since: 0,
            mission_picked: false,

            shots: read_shots(&Path::new(sav).with_extension("previews")),

            tx: Some(tx),
            worker: Some(worker),
            available,
            save_result,
            info,
            last: Instant::now(),
            selected: 1,
            carousel_start: 0,
            delete_confirm: None,
            delete_accept: false,
            routed: false,
            screen: false,
            pending_load: false,
            message: None,
        }
    }
    pub fn new_slot(&self, slot: usize) -> bool {
        self.slots.lock().unwrap().contains_key(&slot)
    }
    pub fn deleted(&self, id: usize) -> bool {
        id > 0
            && self
                .path
                .with_file_name(format!("Save{id}.deleted"))
                .is_file()
    }
    pub fn trash_hit(&self, saving: bool, x: f32, y: f32) -> Option<usize> {
        if !saving {
            return None;
        }
        self.visible_ids(true)
            .iter()
            .enumerate()
            .find_map(|(i, id)| {
                let r = CARDS[i];
                (*id > 0
                    && *id != self.add_id()
                    && !self.deleted(*id)
                    && x >= (r[0] + 58) as f32
                    && x < (r[0] + 71) as f32
                    && y >= (r[1] + 84) as f32
                    && y < (r[1] + 99) as f32)
                    .then_some(*id)
            })
    }
    pub fn request_delete(&mut self, id: usize) {
        if self.return_after_save.is_none() {
            self.delete_confirm = Some(id);
            self.delete_accept = false;
        }
    }
    pub fn finish_delete(&mut self, accept: bool) -> Result<(), String> {
        let Some(id) = self.delete_confirm.take() else {
            return Ok(());
        };
        if !accept {
            return Ok(());
        }
        if self.return_after_save.is_some() {
            return Err("Save in progress".into());
        }
        let folder = self.path.parent().unwrap().join("Deleted");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        // A tombstone also hides the corresponding legacy .sav entry.
        // Keep deleted data out of the carousel, in a recoverable folder.
        let marker = self.path.with_file_name(format!("Save{id}.deleted"));
        std::fs::write(&marker, b"deleted").map_err(|e| e.to_string())?;
        for source in [
            self.path.with_file_name(format!("Save{id}")),
            backup(&self.path.with_file_name(format!("Save{id}"))),
        ] {
            if source.exists() {
                let dest = folder.join(format!(
                    "{}-{stamp}",
                    source.file_name().unwrap().to_string_lossy()
                ));
                std::fs::rename(&source, dest).map_err(|e| e.to_string())?;
            }
        }
        self.slots.lock().unwrap().remove(&(id - 1));
        self.selected = 1;
        self.ensure_visible(true);
        Ok(())
    }
    pub fn add_id(&self) -> usize {
        self.slots
            .lock()
            .unwrap()
            .keys()
            .next_back()
            .copied()
            .map_or(3, |id| (id + 2).max(3))
    }
    pub fn card_ids(&self, saving: bool) -> Vec<usize> {
        let mut ids = if saving { vec![1, 2] } else { vec![0, 1, 2] };
        for index in self.slots.lock().unwrap().keys() {
            let id = index + 1;
            if id > 2 {
                ids.push(id);
            }
        }
        if saving {
            ids.push(self.add_id());
        }
        ids
    }
    pub fn is_add(&self, mem: &gba_core::MemMap, id: usize) -> bool {
        is_saving(mem) && id == self.add_id()
    }
    pub fn move_carousel(&mut self, saving: bool, right: bool) {
        let ids = self.card_ids(saving);
        let current = ids.iter().position(|id| *id == self.selected).unwrap_or(0);
        let index = if right {
            (current + 1).min(ids.len() - 1)
        } else {
            current.saturating_sub(1)
        };
        self.selected = ids[index];
        self.ensure_visible(saving);
    }
    pub fn scroll_carousel(&mut self, saving: bool, right: bool) {
        let ids = self.card_ids(saving);
        let limit = ids.len().saturating_sub(3);
        self.carousel_start = if right {
            (self.carousel_start + 1).min(limit)
        } else {
            self.carousel_start.saturating_sub(1)
        };
        self.selected = ids[if right {
            (self.carousel_start + 2).min(ids.len() - 1)
        } else {
            self.carousel_start
        }];
    }
    pub fn ensure_visible(&mut self, saving: bool) {
        let ids = self.card_ids(saving);
        let index = ids.iter().position(|id| *id == self.selected).unwrap_or(0);
        self.carousel_start = self.carousel_start.min(ids.len().saturating_sub(3));
        if index < self.carousel_start {
            self.carousel_start = index;
        }
        if index >= self.carousel_start + 3 {
            self.carousel_start = index - 2;
        }
    }
    pub fn visible_ids(&self, saving: bool) -> Vec<usize> {
        self.card_ids(saving)
            .into_iter()
            .skip(self.carousel_start)
            .take(3)
            .collect()
    }
    pub fn arrows(&self, saving: bool) -> (bool, bool) {
        (
            self.carousel_start > 0,
            self.carousel_start + 3 < self.card_ids(saving).len(),
        )
    }
    pub fn hit_card(&self, saving: bool, x: f32, y: f32) -> Option<usize> {
        self.visible_ids(saving).get(hit(x, y)?).copied()
    }
    pub fn arrow_hit(&self, saving: bool, x: f32, y: f32) -> Option<bool> {
        if !(82. ..116.).contains(&y) {
            return None;
        }
        let (left, right) = self.arrows(saving);
        if left && (0. ..6.).contains(&x) {
            Some(false)
        } else if right && (234. ..240.).contains(&x) {
            Some(true)
        } else {
            None
        }
    }
    pub fn queue_load(&mut self, slot: usize) {
        self.pending_slot = slot;
        self.pending_load = true;
    }
    pub fn save_manual(&mut self, slot: usize) -> Result<(), String> {
        if slot == usize::MAX {
            return Err("Invalid save slot".into());
        }
        let (bytes, metadata) = self
            .map_state
            .as_ref()
            .ok_or("Return to the world map before saving")?;
        self.tx
            .as_ref()
            .ok_or("Save writer unavailable")?
            .try_send((slot + 1, bytes.clone(), metadata.clone()))
            .map_err(|e| e.to_string())?;
        self.message = Some("SAVING".into());
        Ok(())
    }
    /// Return to the world only after the selected file is durably written.
    /// Imports use save_manual directly and must not unexpectedly rewind play.
    pub fn save_and_return(&mut self, slot: usize) -> Result<(), String> {
        if self.return_after_save.is_some() {
            return Err("Save already in progress".into());
        }
        self.save_manual(slot)?;
        self.return_after_save = Some(slot + 1);
        Ok(())
    }
    /// Prepare one state at each transition into the unobstructed world map.
    /// Opening Save only queues the already prepared bytes for disk writing.
    pub fn observe_session(&mut self, m: &gba_core::Machine) {
        let mem = &m.bus;
        let stage = gba_core::ffta_surface::Descriptor::read(mem)
            .map(|s| s.id)
            .filter(|id| !matches!(*id, 62 | 63 | 65 | 66 | 67));
        if stage != self.mission_stage {
            self.mission_stage = stage;
            self.mission_since = mem.frames;
            self.mission_picked = false;
        }
        if let Some(id) = stage {
            let delay = 600 + ((id as u64 * 97 + self.mission_since * 13) % 600);
            if !self.mission_picked
                && mem.frames.saturating_sub(self.mission_since) >= delay
                && mem.ffta_dialogue.layer().is_none()
                && mem.framebuffer.len() == 38400
            {
                self.mission_image = mem.framebuffer.clone();
                self.mission_picked = true;
            }
        }
        let map_clean = clean_map(mem);
        if map_clean {
            if !self.map_was_clean || self.map_state.is_none() || self.pending_import.is_some() {
                let mut metadata = CardInfo::gameplay(&mem.ewram, &mem.framebuffer);
                if !self.mission_image.is_empty() {
                    metadata.image = self.mission_image.clone();
                }
                self.map_state = Some((m.save_state(), metadata));
                self.map_frame = mem.frames;
            }
            if let Some(slot) = self.pending_import.take() {
                if let Err(e) = self.save_manual(slot) {
                    self.pending_import = Some(slot);
                    eprintln!("FFTA_IMPORT error={e}");
                }
            }
        }
        self.map_was_clean = map_clean;
    }
    pub fn capture_before_menu(&mut self, m: &gba_core::Machine, _keys: u16) {
        self.observe_session(m);
    }
    pub fn import_after_load(&mut self, slot: usize) {
        self.pending_import = Some(slot);
        self.map_state = None;
        self.map_was_clean = false;
        self.mission_image.clear();
    }
    pub fn available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }
    /// Only live terrain/interiors and the world map. File selectors,
    /// title, Party/Clan/System and other management screens are excluded.
    pub fn eligible(&mut self, mem: &gba_core::MemMap) -> bool {
        if shared_loader(mem)
            || mem.io[0] & 0x80 != 0
            || gba_core::ffta_roster_mouse::Scene::read(mem).is_some()
        {
            return false;
        }
        let half = |o| u16::from_le_bytes([mem.io[o], mem.io[o + 1]]);
        let field = gba_core::ffta_surface::Descriptor::read(mem).is_some();
        let map = half(0) & 0x0f07 == 0x0f00
            && half(8) == 0xaa03
            && half(10) == 0xac02
            && u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()) == 0x08031601;
        let eligible =
            field || (map && gba_core::ffta_battle_mouse::MenuScene::read(mem).is_none());
        eligible
    }
    pub fn tick(&mut self, m: &gba_core::Machine, eligible: bool) {
        let result = self.save_result.lock().unwrap().take();
        if let Some(result) = result {
            match result {
                Ok(slot) => {
                    self.message = Some("SAVED".into());
                    if self.return_after_save == Some(slot) {
                        self.return_after_save = None;
                        self.queue_load(slot);
                    }
                }
                Err(e) => {
                    self.return_after_save = None;
                    self.message = Some(format!("Save failed: {e}"));
                }
            }
        }
        if !eligible {
            self.last = Instant::now();
            return;
        }
        if self.last.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.last = Instant::now();
        let data = m.save_state();
        if let Some(tx) = &self.tx {
            let mut metadata = CardInfo::gameplay(&m.bus.ewram, &m.bus.framebuffer);
            // Persist the mission illustration separately from the resumable
            // hardware state, including across QuickSave/restart.
            if !self.mission_image.is_empty() {
                metadata.image = self.mission_image.clone();
            }
            let _ = tx.try_send((0, data, metadata));
        }
    }
    pub fn load(&mut self, m: &mut gba_core::Machine) -> Result<(), String> {
        let path = if self.pending_slot == 0 {
            self.path.clone()
        } else {
            self.path
                .with_file_name(format!("Save{}", self.pending_slot))
        };
        let (state, image) = read_bundle(&path)?;
        m.load_state(&state)?;
        self.map_state = None;
        self.map_was_clean = false;
        self.return_after_save = None;
        self.pending_import = None;
        self.mission_image = image;
        self.last = Instant::now();
        self.message = None;
        Ok(())
    }
}
impl Drop for AutoSave {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
pub fn selector(mem: &gba_core::MemMap) -> bool {
    // Task 11 owns file cards, 255 is its exit transition. Task 4 is the
    // separate Resume Battle / Load menu, NOT a vertical file selector.
    // Suppress the original cards through the exit, not just their idle BGs.
    mem.ffta_movement.europe_profile
        && matches!(mem.iwram[0x27d1], 11 | 255)
        && u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()) == 0x0813dd75
        && u16::from_le_bytes([mem.io[0], mem.io[1]]) & 7 == 0
}
pub fn is_saving(mem: &gba_core::MemMap) -> bool {
    mem.ffta_movement.europe_profile
        && u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()) == 0x0813d049
}
pub fn confirming_save(_mem: &gba_core::MemMap) -> bool {
    false // manual state writes have no guest overwrite phase
}
pub fn selectable(mem: &gba_core::MemMap, slot: usize) -> bool {
    !is_saving(mem) || slot > 0
}
pub const CARDS: [[i32; 4]; 3] = [[6, 48, 78, 148], [84, 48, 156, 148], [162, 48, 234, 148]];
pub fn hit(x: f32, y: f32) -> Option<usize> {
    CARDS
        .iter()
        .position(|r| x >= r[0] as f32 && x < r[2] as f32 && y >= r[1] as f32 && y < r[3] as f32)
}
/// Header fields verified in EUR 0813FC28 and live clock 02002FBC.
#[derive(Clone, Debug, Default)]
pub struct CardInfo {
    pub name: String,
    pub level: u8,
    pub hours: u16,
    pub minutes: u8,
    pub clan: String,
    pub clan_level: u8,
    pub missions: u16,
    pub image: Vec<u16>,
}
fn h(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}
fn game_name(bytes: &[u8]) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 {
        let code = if bytes[i] == 0x80 {
            i += 1;
            *bytes.get(i).unwrap_or(&0)
        } else {
            bytes[i]
        };
        let ch = match code {
            0xb0..=0xc9 => (b'A' + (code - 0xb0)) as char,
            0xca..=0xe3 => (b'a' + (code - 0xca)) as char,
            0xa0..=0xa9 => (b'0' + (code - 0xa0)) as char,
            0x9f => ' ',
            _ => '?',
        };
        s.push(ch);
        i += 1;
    }
    s
}
impl CardInfo {
    fn gameplay(ram: &[u8], pixels: &[u16]) -> Self {
        let name = game_name(ram.get(0x1f1c..0x1f30).unwrap_or(&[]));
        let image = if pixels.len() == 240 * 160 {
            pixels.to_vec()
        } else {
            Vec::new()
        };
        Self {
            name,
            level: *ram.get(0x89).unwrap_or(&0),
            hours: h(ram, 0x2fbc) & 1023,
            minutes: *ram.get(0x2fbe).unwrap_or(&0).min(&59),
            image,
            clan: game_name(ram.get(0x21a0..0x21b4).unwrap_or(&[])),
            ..Self::default()
        }
    }
    fn header(data: &[u8]) -> Option<Self> {
        if data.get(..8) != Some(b"FFTEX000") {
            return None;
        }
        let clock = h(data, 0x30);
        Some(Self {
            name: game_name(data.get(0x1a..0x2e)?),
            level: *data.get(0x2e)?,
            hours: clock & 1023,
            minutes: ((clock >> 10) & 63) as u8,
            clan: game_name(data.get(0x52..0x66)?),
            clan_level: h(data, 0x74) as u8,
            missions: h(data, 0x76),
            image: Vec::new(),
        })
    }
}
fn header_key(data: &[u8]) -> u64 {
    data[..0x32].iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100000001b3)
    })
}
fn read_shots(path: &Path) -> Vec<(u64, Vec<u16>)> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let count = if bytes.get(..8) == Some(b"FFTAPV02") {
        240 * 160
    } else if bytes.get(..8) == Some(b"FFTAPV01") {
        63 * 42
    } else {
        return Vec::new();
    };
    bytes[8..]
        .chunks_exact(8 + count * 2)
        .take(32)
        .map(|b| {
            let pixels: Vec<u16> = b[8..]
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect();
            let image = if count == 240 * 160 {
                pixels
            } else {
                (0..240 * 160)
                    .map(|i| pixels[(i / 240 * 42 / 160) * 63 + (i % 240 * 63 / 240)])
                    .collect()
            };
            (u64::from_le_bytes(b[..8].try_into().unwrap()), image)
        })
        .collect()
}
/// Invoke the unmodified EUR confirmation handler, on a private CPU/stack.
/// It validates the slot and schedules the original loader; no save is decoded
/// or reconstructed by the host. Guest CPU execution resumes normally afterward.
pub fn activate_manual(m: &mut gba_core::Machine, slot: usize) -> Result<(), String> {
    let saving = is_saving(&m.bus);
    let ready = if saving { 8 } else { 11 };
    if slot > 1 || !shared_loader(&m.bus) || m.bus.iwram[0x27d1] != ready {
        return Err("File selector is not ready".into());
    }
    let root = u32::from_le_bytes(m.bus.iwram[0x27d8..0x27dc].try_into().unwrap())
        .checked_sub(0x02000000)
        .ok_or("Invalid save manager")? as usize;
    if root
        .checked_add(0x2270)
        .is_none_or(|end| end > m.bus.ewram.len())
        || (!saving
            && CardInfo::header(
                &m.bus.ewram[root + 0xe1c + slot * 0x80..root + 0xe9c + slot * 0x80],
            )
            .is_none())
    {
        return Err("This file has no save data".into());
    }
    let verified = if saving {
        m.bus.rom.get(0x13d7e6..0x13d7ec) == Some(&[0x61, 0x88, 0x01, 0x24, 0x0c, 0x40])
    } else {
        m.bus.rom.get(0x13e9dc..0x13e9e2) == Some(&[0x76, 0x88, 0x01, 0x2e, 0x4a, 0xd1])
    };
    if !verified {
        return Err("Unsupported load handler".into());
    }
    m.bus.ewram[root + 0x225d] = slot as u8;
    invoke_save_handler(m, 1)?;
    let state = m.bus.iwram[0x27d1];
    eprintln!(
        "FFTA_MANUAL_{} slot={} state={state} next={}",
        if saving { "SAVE" } else { "LOAD" },
        slot + 1,
        m.bus.ewram[0x1c3ca]
    );
    if state == ready {
        Err("The game rejected this save".into())
    } else {
        Ok(())
    }
}
fn invoke_save_handler(m: &mut gba_core::Machine, edge: u16) -> Result<(), String> {
    let stack = m.bus.iwram[0x7000..0x7400].to_vec();
    let edges = m.bus.iwram[2..6].to_vec();
    m.bus.iwram[2..4].copy_from_slice(&edge.to_le_bytes());
    m.bus.iwram[4..6].fill(0);
    let mut cpu = gba_core::Cpu::new();
    cpu.cpsr |= gba_core::cpu::FLAG_T | gba_core::cpu::FLAG_I;
    cpu.regs[1] = 0x030027d1;
    cpu.regs[13] = 0x03007400;
    cpu.regs[14] = 0x03007001;
    cpu.regs[15] = u32::from_le_bytes(m.bus.iwram[0x2804..0x2808].try_into().unwrap()) & !1;
    let mut returned = false;
    for _ in 0..100_000 {
        if cpu.regs[15] == 0x03007000 {
            returned = true;
            break;
        }
        gba_core::exec::step_hle(&mut cpu, &mut m.bus);
    }
    m.bus.iwram[0x7000..0x7400].copy_from_slice(&stack);
    m.bus.iwram[2..6].copy_from_slice(&edges);
    if !returned {
        return Err("Load confirmation did not return".into());
    }
    Ok(())
}
impl AutoSave {
    pub fn screen_active(&self, mem: &gba_core::MemMap) -> bool {
        // Own every entry/exit frame, not just the idle file selector.
        // This prevents the original menu flashing for a single frame.
        shared_loader(mem)
    }
    /// Saved Game's optional Resume/Load choice is not a file selector.
    /// Enter the original Load branch once; going back exits that branch.
    pub fn route_entry(&mut self, m: &mut gba_core::Machine) {
        if !shared_loader(&m.bus) {
            self.routed = false;
            return;
        }
        if !is_saving(&m.bus) && (self.available() || !self.slots.lock().unwrap().is_empty()) {
            let root = u32::from_le_bytes(m.bus.iwram[0x27d8..0x27dc].try_into().unwrap())
                .wrapping_sub(0x02000000) as usize;
            if let Some(flags) = m.bus.ewram.get_mut(root.saturating_add(0x2155)) {
                *flags |= 3;
            }
        }
        if is_saving(&m.bus) {
            return;
        }
        if m.bus.iwram[0x27d1] != 4 {
            return;
        }
        let root = u32::from_le_bytes(m.bus.iwram[0x27d8..0x27dc].try_into().unwrap())
            .wrapping_sub(0x02000000) as usize;
        if root
            .checked_add(0x2270)
            .is_none_or(|end| end > m.bus.ewram.len())
        {
            return;
        }
        let edge = if self.routed { 2 } else { 1 };
        if !self.routed {
            if m.bus.ewram[root + 0x2155] & 3 == 0 {
                return;
            }
            m.bus.ewram[root + 0x225d] = 1; // Load, not Resume Battle
        }
        match invoke_save_handler(m, edge) {
            Ok(()) => {
                self.routed = true;
                eprintln!("FFTA_SAVE_ENTRY edge={edge} state={}", m.bus.iwram[0x27d1]);
            }
            Err(e) => eprintln!("FFTA_SAVE_ENTRY error={e}"),
        }
    }
}
fn shared_loader(mem: &gba_core::MemMap) -> bool {
    mem.ffta_movement.europe_profile
        && matches!(
            u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap()),
            0x0813dd75 | 0x0813d049
        )
}
fn normal_cards(mem: &gba_core::MemMap, shots: &[(u64, Vec<u16>)]) -> [Option<CardInfo>; 2] {
    let pointer = mem
        .iwram
        .get(0x27d8..0x27dc)
        .map(|p| u32::from_le_bytes(p.try_into().unwrap()))
        .unwrap_or(0);
    let Some(root) = pointer.checked_sub(0x02000000).map(|v| v as usize) else {
        return [None, None];
    };
    std::array::from_fn(|slot| {
        let base = root + 0xe1c + slot * 0x80;
        let header = mem.ewram.get(base..base + 0x80)?;
        let mut info = CardInfo::header(header)?;
        if let Some((_, image)) = shots.iter().find(|(k, _)| *k == header_key(header)) {
            info.image = image.clone();
        }
        Some(info)
    })
}
pub(crate) fn card_entries(mem: &gba_core::MemMap, auto: &AutoSave) -> BTreeMap<usize, Option<CardInfo>> {
    let normal = normal_cards(mem, &auto.shots);
    let mut entries = BTreeMap::from([
        (0, auto.info.lock().unwrap().clone()),
        (1, normal[0].clone()),
        (2, normal[1].clone()),
    ]);
    for (index, card) in auto.slots.lock().unwrap().iter() {
        entries.insert(index + 1, Some(card.clone()));
    }
    for id in [1, 2] {
        if auto.deleted(id) {
            entries.insert(id, None);
        }
    }
    entries
}
fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 14],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 27, 17],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}
fn text_pixels(out: &mut [u16], w: usize, x: i32, y: i32, s: &str, color: u16) {
    for (i, c) in s.chars().enumerate() {
        for (v, row) in glyph(c.to_ascii_uppercase()).iter().enumerate() {
            for u in 0..5 {
                if row & (16 >> u) != 0 {
                    let xx = x + i as i32 * 6 + u;
                    let yy = y + v as i32;
                    if xx >= 0 && xx < w as i32 && yy >= 0 {
                        if let Some(p) = out.get_mut(yy as usize * w + xx as usize) {
                            *p = color;
                        }
                    }
                }
            }
        }
    }
}
fn text(out: &mut [u16], w: usize, x: i32, y: i32, s: &str) {
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        text_pixels(out, w, x + dx, y + dy, s, 0);
    }
    text_pixels(out, w, x, y, s, 0x7fff);
}
fn blend(front: u16, back: u16, a: u16) -> u16 {
    let mut c = 0;
    for shift in [0, 5, 10] {
        c |= ((((front >> shift) & 31) * a + ((back >> shift) & 31) * (16 - a)) / 16) << shift;
    }
    c
}
fn trash(out: &mut [u16], w: usize, x: usize, y: usize, scale: usize) {
    const BIN: [&str; 13] = [
        "...oooo...",
        "...rrrr...",
        "oooooooooo",
        "orrrrrrrro",
        "..orrrro..",
        "..orrrro..",
        "..ororoo..",
        "..ororoo..",
        "..ororoo..",
        "..ororoo..",
        "..orrrro..",
        "..oooooo..",
        "..........",
    ];
    for (dy, row) in BIN.iter().enumerate() {
        for (dx, c) in row.bytes().enumerate() {
            let color = match c {
                b'o' => 1 | (2 << 5) | (5 << 10),
                b'r' => 31 | (4 << 5) | (5 << 10),
                _ => continue,
            };
            for sy in 0..scale {
                for sx in 0..scale {
                    if let Some(p) = out.get_mut((y + dy * scale + sy) * w + x + dx * scale + sx) {
                        *p = color;
                    }
                }
            }
        }
    }
}
pub fn paint(mem: &gba_core::MemMap, _source: &[u16], out: &mut Vec<u16>, auto: &AutoSave) {
    let w = gba_core::mem::render_width();
    let ex = gba_core::mem::extra();
    let ey = gba_core::mem::extra_y();
    let h=gba_core::mem::render_height();
    out.resize(w*h,0);
    let mut bg=vec![0;w];
    for y in 0..h {
        gba_core::ppu::render_menu_background_row(mem,y as i32-ey as i32,&mut bg);
        out[y*w..(y+1)*w].copy_from_slice(&bg);
    }
    // Compact parchment caption, keeping the original patterned screen.
    for yy in 14..33 {
        for xx in 55..185 {
            let border = xx == 55 || xx == 184 || yy == 14 || yy == 32;
            let c = if border {
                3 | (4 << 5) | (5 << 10)
            } else if yy == 15 || xx == 56 {
                31 | (31 << 5) | (23 << 10)
            } else {
                let v = (28 - (yy - 16).max(0) / 8) as u16;
                v | (v << 5) | (18 << 10)
            };
            out[(yy + ey) * w + xx + ex] = c;
        }
    }
    text_pixels(
        out,
        w,
        (ex + 69) as i32,
        (ey + 20) as i32,
        if is_saving(mem) && auto.message.as_deref() == Some("SAVED") {
            "GAME SAVED"
        } else if is_saving(mem) && auto.message.as_deref() == Some("SAVING") {
            "SAVING..."
        } else if is_saving(mem)
            && auto
                .message
                .as_deref()
                .is_some_and(|m| m.starts_with("Save failed:"))
        {
            "SAVE FAILED"
        } else if is_saving(mem) {
            "PICK SLOT TO SAVE"
        } else {
            "PICK SLOT TO LOAD"
        },
        2 | (3 << 5) | (5 << 10),
    );
    let entries = card_entries(mem, auto);
    let visible = auto.visible_ids(is_saving(mem));
    for (position, slot) in visible.iter().copied().enumerate() {
        let r = &CARDS[position];
        let x = r[0] as usize + ex;
        let y = r[1] as usize + ey;
        let green = slot == 0;
        let body = if green {
            3 | (16 << 5) | (8 << 10)
        } else {
            4 | (8 << 5) | (25 << 10)
        };
        let top = if green {
            11 | (23 << 5) | (13 << 10)
        } else {
            14 | (19 << 5) | (30 << 10)
        };
        for dy in 0..100 {
            for dx in 0..72 {
                let corner = (dx + dy)
                    .min(71 - dx + dy)
                    .min(dx + 99 - dy)
                    .min(71 - dx + 99 - dy);
                if corner < 3 {
                    continue;
                }
                let border = corner == 3 || dx == 0 || dx == 71 || dy == 0 || dy == 99;
                let bevel = dx == 1 || dy == 1 || dx == 70 || dy == 98;
                let at = (y + dy) * w + x + dx;
                out[at] = if border {
                    if slot == auto.selected {
                        0x03ff
                    } else {
                        0
                    }
                } else if bevel {
                    if dx == 1 || dy == 1 {
                        17 | (22 << 5) | (26 << 10)
                    } else {
                        2 | (4 << 5) | (8 << 10)
                    }
                } else {
                    blend(
                        if dy < 12 {
                            top
                        } else {
                            blend(body, top, ((100 - dy) / 18).min(4) as u16)
                        },
                        out[at],
                        if dy < 12 { 15 } else { 14 },
                    )
                };
            }
        }
        text(
            out,
            w,
            (x + 7) as i32,
            (y + 3) as i32,
            &if slot == 0 {
                "Auto Save".into()
            } else if auto.is_add(mem, slot) {
                "New Save".into()
            } else {
                format!("File No.{slot}")
            },
        );
        for dx in 4..68 {
            out[(y + 12) * w + x + dx] = 2 | (4 << 5) | (9 << 10);
        }
        for &(dx, dy) in &[(3usize, 3usize), (68, 3), (3, 96), (68, 96)] {
            out[(y + dy) * w + x + dx] = 23 | (25 << 5) | (23 << 10);
        }
        for dx in 4..68 {
            out[(y + 85) * w + x + dx] = 2 | (5 << 5) | (11 << 10);
            out[(y + 86) * w + x + dx] = blend(top, body, 4);
        }
        for dx in [61, 64, 67] {
            for dy in 88..95 {
                out[(y + dy) * w + x + dx] = if dx % 2 == 0 {
                    11 | (15 << 5) | (20 << 10)
                } else {
                    2 | (4 << 5) | (9 << 10)
                };
            }
        }
        if auto.is_add(mem, slot) {
            // Blue cartridge with a gold round button and a dark inset plus.
            let cx = x + 36;
            let cy = y + 56;
            for dy in -15i32..=15 {
                for dx in -15i32..=15 {
                    let d = dx * dx + dy * dy;
                    if d <= 225 {
                        out[(cy as i32 + dy) as usize * w + (cx as i32 + dx) as usize] = if d >= 196
                        {
                            1 | (3 << 5) | (7 << 10)
                        } else if dy < 0 {
                            31 | (27 << 5) | (9 << 10)
                        } else {
                            27 | (20 << 5) | (4 << 10)
                        };
                    }
                }
            }
            for dy in -7i32..=7 {
                for dx in -7i32..=7 {
                    if dx.abs() <= 1 || dy.abs() <= 1 {
                        out[(cy as i32 + dy) as usize * w + (cx as i32 + dx) as usize] =
                            1 | (4 << 5) | (9 << 10);
                    }
                }
            }
        } else if let Some(c) = entries.get(&slot).and_then(|c| c.as_ref()) {
            text(
                out,
                w,
                (x + 6) as i32,
                (y + 17) as i32,
                &c.name.chars().take(10).collect::<String>(),
            );
            text(
                out,
                w,
                (x + 6) as i32,
                (y + 27) as i32,
                &format!("LV {}", c.level),
            );
            // Full-resolution stored frame, nearest sampled only here.
            for yy in 0..46 {
                for xx in 0..67 {
                    if xx < 2 || xx >= 65 || yy < 2 || yy >= 44 {
                        out[(y + 37 + yy) * w + x + 2 + xx] = if xx == 1 || yy == 1 {
                            17 | (21 << 5) | (24 << 10)
                        } else {
                            0
                        };
                    }
                }
            }
            if c.image.len() == 240 * 160 {
                for sy in 0..42 {
                    for sx in 0..63 {
                        out[(y + 39 + sy) * w + x + 4 + sx] =
                            c.image[((sy * 160 + 80) / 42).min(159) * 240
                                + ((sx * 240 + 120) / 63).min(239)];
                    }
                }
            }
            text(
                out,
                w,
                (x + 6) as i32,
                (y + 88) as i32,
                &format!("{:02}:{:02}", c.hours, c.minutes),
            );
        } else {
            text(out, w, (x + 10) as i32, (y + 48) as i32, "NO SAVE");
        }
    }
    if is_saving(mem) {
        for (i, id) in auto.visible_ids(true).iter().enumerate() {
            if *id != auto.add_id() && !auto.deleted(*id) {
                trash(
                    out,
                    w,
                    CARDS[i][0] as usize + ex + 59,
                    CARDS[i][1] as usize + ey + 85,
                    1,
                );
            }
        }
    }
    let (left, right) = auto.arrows(is_saving(mem));
    let phase = (mem.frames / 18) % 2;
    for (show, edge, direction) in [(left, 0i32, 1i32), (right, 239i32, -1i32)] {
        if !show {
            continue;
        }
        // Two explicit poses: stepped spear tip with dark contour and pale
        // leading edge; alternating tail notch gives the classic two-cel pulse.
        let rows: [u8; 11] = if phase == 0 {
            [1, 3, 7, 15, 31, 63, 31, 15, 7, 3, 1]
        } else {
            [0, 1, 3, 7, 31, 63, 31, 7, 3, 1, 0]
        };
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..6i32 {
                if bits & (1 << x) != 0 {
                    let px = (edge + direction * x) as usize + ex;
                    let py = 94 + y + ey;
                    let c = if x == 0 || y == 0 || y == 10 {
                        1 | (3 << 5) | (7 << 10)
                    } else if y < 5 {
                        31 | (30 << 5) | (16 << 10)
                    } else {
                        31 | (22 << 5) | (4 << 10)
                    };
                    out[py * w + px] = c;
                }
            }
        }
    }
    if let Some(_) = auto.delete_confirm {
        for yy in 61..129 {
            for xx in 65..175 {
                out[(yy + ey) * w + xx + ex] = if yy == 61 || yy == 128 || xx == 65 || xx == 174 {
                    0
                } else {
                    27 | (28 << 5) | (20 << 10)
                };
            }
        }
        trash(out, w, ex + 109, ey + 68, 2);
        // Left = cancel, right = the red bin confirms. Default is cancel.
        for (l, active) in [(79, !auto.delete_accept), (131, auto.delete_accept)] {
            for yy in 100..121 {
                for xx in l..l + 29 {
                    out[(yy + ey) * w + xx + ex] =
                        if yy == 100 || yy == 120 || xx == l || xx == l + 28 {
                            if active {
                                0x03ff
                            } else {
                                0
                            }
                        } else {
                            5 | (8 << 5) | (13 << 10)
                        };
                }
            }
        }
        for d in 0..9 {
            for k in 0..2 {
                out[(106 + d + ey) * w + ex + 87 + d + k] = 0x7fff;
                out[(106 + d + ey) * w + ex + 95 - d + k] = 0x7fff;
            }
        }
        trash(out, w, ex + 140, ey + 104, 1);
    }
    if confirming_save(mem) {
        // This is the original game prompt, not a second file selector.
        // The native 240x160 framebuffer includes it even with UI separation.
        let [l, t, r, b] = mem.ffta_dialogue.extra.bounds.unwrap_or([16, 8, 232, 64]);
        let (l, t, r, b) = (
            l.clamp(0, 240) as usize,
            t.clamp(0, 160) as usize,
            r.clamp(0, 240) as usize,
            b.clamp(0, 160) as usize,
        );
        if l < r && t < b {
            for y in t..b {
                out[(y + ey) * w + ex + l..(y + ey) * w + ex + r]
                    .copy_from_slice(&mem.framebuffer[y * 240 + l..y * 240 + r]);
            }
        }
    }
}
/// Preserve the native patterned UI, but sample preview images directly into
/// physical output pixels instead of shrinking them to 63x42 first.
pub fn compose_display(
    mem: &gba_core::MemMap,
    auto: &AutoSave,
    source: &[[u8; 4]],
    out: &mut Vec<[u8; 4]>,
    dims: (usize, usize),
    physical: (usize, usize),
    view: [f32; 4],
    lut: &[[u8; 4]],
) {
    let (w, h) = physical;
    out.resize(w * h, [0, 0, 0, 255]);
    out.fill([0, 0, 0, 255]);
    let (scale, ox, oy) = crate::ui_layout::letterbox_f((w as u32, h as u32), view[2], view[3]);
    let x0 = ox.ceil().max(0.) as usize;
    let y0 = oy.ceil().max(0.) as usize;
    let x1 = (ox + view[2] * scale).floor().min(w as f32) as usize;
    let y1 = (oy + view[3] * scale).floor().min(h as f32) as usize;
    for y in y0..y1 {
        let sy = (view[1] + (y as f32 + 0.5 - oy) / scale)
            .floor()
            .clamp(0., (dims.1 - 1) as f32) as usize;
        for x in x0..x1 {
            let sx = (view[0] + (x as f32 + 0.5 - ox) / scale)
                .floor()
                .clamp(0., (dims.0 - 1) as f32) as usize;
            out[y * w + x] = source[sy * dims.0 + sx];
        }
    }
    let entries = card_entries(mem, auto);
    for (i, id) in auto.visible_ids(is_saving(mem)).iter().enumerate() {
        let Some(c) = entries.get(id).and_then(|c| c.as_ref()) else {
            continue;
        };
        if c.image.len() != 240 * 160 {
            continue;
        }
        let left = CARDS[i][0] as f32 + gba_core::mem::extra() as f32 + 4.;
        let top = CARDS[i][1] as f32 + gba_core::mem::extra_y() as f32 + 39.;
        let px = ox + (left - view[0]) * scale;
        let py = oy + (top - view[1]) * scale;
        let right = (px + 63. * scale).floor().min(w as f32) as usize;
        let bottom = (py + 42. * scale).floor().min(h as f32) as usize;
        for y in py.ceil().max(0.) as usize..bottom {
            for x in px.ceil().max(0.) as usize..right {
                if auto.delete_confirm.is_some() {
                    let nx =
                        view[0] + (x as f32 + 0.5 - ox) / scale - gba_core::mem::extra() as f32;
                    let ny =
                        view[1] + (y as f32 + 0.5 - oy) / scale - gba_core::mem::extra_y() as f32;
                    if nx >= 65. && nx < 175. && ny >= 61. && ny < 129. {
                        continue;
                    }
                }
                if confirming_save(mem) {
                    let [l, t, r, b] = mem.ffta_dialogue.extra.bounds.unwrap_or([16, 8, 232, 64]);
                    let nx =
                        view[0] + (x as f32 + 0.5 - ox) / scale - gba_core::mem::extra() as f32;
                    let ny =
                        view[1] + (y as f32 + 0.5 - oy) / scale - gba_core::mem::extra_y() as f32;
                    if nx >= l as f32 && nx < r as f32 && ny >= t as f32 && ny < b as f32 {
                        continue;
                    }
                }
                let sx = (((x as f32 + 0.5 - px) / (63. * scale)) * 240.)
                    .floor()
                    .clamp(0., 239.) as usize;
                let sy = (((y as f32 + 0.5 - py) / (42. * scale)) * 160.)
                    .floor()
                    .clamp(0., 159.) as usize;
                out[y * w + x] = lut[(c.image[sy * 240 + sx] & 0x7fff) as usize];
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn map_machine() -> gba_core::Machine {
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ffta_movement.europe_profile = true;
        m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        m.bus.io[8..10].copy_from_slice(&0xaa03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0xac02u16.to_le_bytes());
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        m.bus.iwram[0x2810..0x2814].copy_from_slice(&0x0200f3a4u32.to_le_bytes());
        m.bus.ewram[0xf3a8..0xf3ac].copy_from_slice(&0x02002c10u32.to_le_bytes());
        m.bus.ewram[0x2c10] = 1;
        m
    }
    #[test]
    fn trash_is_save_only_and_confirmation_cancel_preserves_every_file() {
        let dir = std::env::temp_dir().join(format!("ffta-trash-cancel-{}", std::process::id()));
        let mut store = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        assert_eq!(store.trash_hit(false, 70., 140.), None);
        assert_eq!(store.trash_hit(true, 70., 140.), Some(1));
        assert_eq!(
            store.trash_hit(true, 226., 140.),
            None,
            "plus tile has no trash"
        );
        store.request_delete(1);
        assert!(!store.delete_accept);
        store.finish_delete(false).unwrap();
        assert!(!store.deleted(1));
        assert!(store.delete_confirm.is_none());
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn confirmed_delete_hides_slot_and_backup_without_losing_recoverable_data() {
        let dir = std::env::temp_dir().join(format!("ffta-trash-confirm-{}", std::process::id()));
        let sav = dir.join("game.sav");
        let mut store = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        let m = map_machine();
        store.observe_session(&m);
        store.save_manual(2).unwrap();
        store.tx.take();
        store.worker.take().unwrap().join().unwrap();
        let path = store.path.with_file_name("Save3");
        let original = std::fs::read(&path).unwrap();
        std::fs::copy(&path, backup(&path)).unwrap();
        store.request_delete(3);
        store.finish_delete(true).unwrap();
        assert!(!path.exists() && !backup(&path).exists());
        assert!(store.deleted(3));
        assert!(!store.new_slot(2));
        let recovered: Vec<_> = std::fs::read_dir(path.parent().unwrap().join("Deleted"))
            .unwrap()
            .map(|p| std::fs::read(p.unwrap().path()).unwrap())
            .collect();
        assert_eq!(recovered.len(), 2);
        assert!(recovered.iter().all(|b| *b == original));
        drop(store);
        let mut reopened = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        assert!(!reopened.new_slot(2));
        assert_eq!(reopened.add_id(), 3);
        reopened.observe_session(&m);
        reopened.save_manual(2).unwrap();
        drop(reopened);
        let reopened = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        assert!(reopened.new_slot(2));
        assert!(!reopened.deleted(3));
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn carousel_has_add_only_in_save_and_arrows_only_for_hidden_cards() {
        let dir = std::env::temp_dir().join(format!("ffta-carousel-{}", std::process::id()));
        let mut store = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        assert_eq!(store.card_ids(true), vec![1, 2, 3]);
        assert_eq!(store.card_ids(false), vec![0, 1, 2]);
        assert_eq!(store.arrows(true), (false, false));
        assert_eq!(store.hit_card(true, 180., 80.), Some(3));
        let mut m = map_machine();
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813d049u32.to_le_bytes());
        assert!(store.is_add(&m.bus, 3));
        for id in 2..5 {
            store.slots.lock().unwrap().insert(id, CardInfo::default());
        }
        assert_eq!(store.card_ids(true), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(store.arrows(true), (false, true));
        store.scroll_carousel(true, true);
        assert_eq!(store.visible_ids(true), vec![2, 3, 4]);
        assert_eq!(store.arrows(true), (true, true));
        assert_eq!(store.arrow_hit(true, 2., 99.), Some(false));
        assert_eq!(store.arrow_hit(true, 238., 99.), Some(true));
        store.selected = 6;
        store.ensure_visible(true);
        assert_eq!(store.visible_ids(true), vec![4, 5, 6]);
        assert_eq!(store.arrows(true), (true, false));
        assert_eq!(store.arrow_hit(true, 238., 99.), None);
        store.move_carousel(true, true);
        assert_eq!(store.selected, 6);
        store.scroll_carousel(true, false);
        assert_eq!(store.visible_ids(true), vec![3, 4, 5]);
        assert_eq!(store.hit_card(true, 100., 80.), Some(4));
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn additional_slots_survive_restart_and_restore_exact_state() {
        let dir = std::env::temp_dir().join(format!("ffta-extra-slots-{}", std::process::id()));
        let sav = dir.join("game.sav");
        let mut m = map_machine();
        m.bus.ewram[0x89] = 33;
        let mut store = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        store.observe_session(&m);
        store.save_manual(2).unwrap();
        store.save_manual(5).unwrap();
        let folder = store.path.parent().unwrap().to_path_buf();
        drop(store);
        assert!(folder.join("Save3").is_file() && folder.join("Save6").is_file());
        let mut store = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        assert!(store.new_slot(2) && store.new_slot(5));
        assert_eq!(store.card_ids(false), vec![0, 1, 2, 3, 6]);
        assert_eq!(store.add_id(), 7);
        store.pending_slot = 6;
        m.bus.ewram[0x89] = 1;
        store.load(&mut m).unwrap();
        assert_eq!(m.bus.ewram[0x89], 33);
        assert!(clean_map(&m.bus));
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn map_snapshot_is_prepared_on_entry_and_not_rebuilt_when_opening_save() {
        let dir = std::env::temp_dir().join(format!("ffta-map-entry-{}", std::process::id()));
        let mut store = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let mut m = map_machine();
        m.bus.ewram[0x89] = 10;
        store.observe_session(&m);
        let initial = store.map_state.as_ref().unwrap().0.clone();
        m.bus.frames += 300;
        m.bus.ewram[0x89] = 20;
        store.capture_before_menu(&m, 0x3f7);
        assert_eq!(store.map_state.as_ref().unwrap().0, initial);
        m.bus.ewram[0x1c3c4] = 8;
        store.observe_session(&m);
        assert_eq!(store.map_state.as_ref().unwrap().0, initial);
        m.bus.ewram[0x1c3c4] = 0;
        store.observe_session(&m);
        assert_ne!(store.map_state.as_ref().unwrap().0, initial);
        assert_eq!(store.map_state.as_ref().unwrap().1.level, 20);
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn successful_manual_write_returns_to_map_but_import_or_failure_does_not() {
        let dir = std::env::temp_dir().join(format!("ffta-save-return-{}", std::process::id()));
        let mut store = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let mut m = map_machine();
        store.observe_session(&m);
        store.save_and_return(0).unwrap();
        assert!(!store.pending_load, "must wait for durable write");
        // Drain the real writer deterministically before observing its result.
        store.tx.take();
        store.worker.take().unwrap().join().unwrap();
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813d049u32.to_le_bytes());
        m.bus.iwram[0x27d1] = 8;
        store.tick(&m, false);
        assert!(store.pending_load);
        assert_eq!(store.pending_slot, 1);
        store.pending_load = false;
        store.load(&mut m).unwrap();
        assert!(clean_map(&m.bus));
        *store.save_result.lock().unwrap() = Some(Ok(2));
        store.tick(&m, false);
        assert!(
            !store.pending_load,
            "background import must not return player"
        );
        store.return_after_save = Some(2);
        *store.save_result.lock().unwrap() = Some(Err("disk error".into()));
        store.tick(&m, false);
        assert!(!store.pending_load);
        assert_eq!(store.message.as_deref(), Some("Save failed: disk error"));
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn mission_photo_avoids_opening_and_survives_map_until_next_midmission() {
        let dir = std::env::temp_dir().join(format!("ffta-mission-photo-{}", std::process::id()));
        let mut store = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let mut m = map_machine();
        m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        m.bus.framebuffer.fill(31);
        m.bus.frames = 100;
        store.observe_session(&m);
        assert!(store.mission_image.is_empty());
        m.bus.frames = 300;
        store.observe_session(&m);
        assert!(store.mission_image.is_empty());
        m.bus.frames = 2000;
        store.observe_session(&m);
        assert_eq!(store.mission_image, vec![31; 38400]);
        m.bus.io[8..10].copy_from_slice(&0xaa03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0xac02u16.to_le_bytes());
        m.bus.framebuffer.fill(992);
        m.bus.frames = 2100;
        store.observe_session(&m);
        assert_eq!(store.mission_image, vec![31; 38400]);
        store.save_manual(0).unwrap();
        m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        m.bus.ewram[0x7f10..0x7f12].copy_from_slice(&1u16.to_le_bytes());
        m.bus.frames = 2200;
        store.observe_session(&m);
        assert_eq!(store.mission_image, vec![31; 38400]);
        m.bus.frames = 4200;
        store.observe_session(&m);
        assert_eq!(store.mission_image, vec![992; 38400]);
        drop(store);
        let card = read_card(&dir.join("Saves/Save1")).unwrap();
        assert_eq!(card.image, vec![31; 38400]);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn clean_map_requires_cursor_owner_not_background_or_mouse_mode() {
        let mut m = map_machine();
        assert!(clean_map(&m.bus));
        m.bus.ewram[0x1c3c4] = 8;
        assert!(!clean_map(&m.bus));
        m.bus.ewram[0x1c3c4] = 0;
        m.bus.ewram[0x2c10] = 0;
        assert!(!clean_map(&m.bus));
        m.bus.ewram[0x2c10] = 0x21;
        assert!(clean_map(&m.bus));
    }
    #[test]
    fn manual_slot_restores_map_not_save_menu_and_keeps_separate_mission_picture() {
        let dir = std::env::temp_dir().join(format!("ffta-state-map-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("game.sav");
        std::fs::write(&legacy, b"legacy untouched").unwrap();
        let mut m = map_machine();
        m.bus.ewram[0x89] = 47;
        m.bus.framebuffer.fill(31);
        let mut store = AutoSave::new(legacy.to_str().unwrap(), "0123456789abcdef");
        store.mission_image = vec![992; 38400];
        store.capture_before_menu(&m, 0x3f7);
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813d049u32.to_le_bytes());
        m.bus.iwram[0x27d1] = 8;
        store.save_manual(0).unwrap();
        let path = store.path.with_file_name("Save1");
        drop(store);
        assert_eq!(std::fs::read(&legacy).unwrap(), b"legacy untouched");
        let (state, image) = read_bundle(&path).unwrap();
        assert_eq!(image, vec![992; 38400]);
        m.load_state(&state).unwrap();
        assert!(clean_map(&m.bus));
        assert_eq!(m.bus.ewram[0x89], 47);
        assert_eq!(m.bus.framebuffer, vec![31; 38400]);
        let mut reopened = AutoSave::new(legacy.to_str().unwrap(), "0123456789abcdef");
        assert!(reopened.new_slot(0));
        assert!(!reopened.new_slot(1));
        reopened.pending_slot = 1;
        reopened.load(&mut m).unwrap();
        assert!(clean_map(&m.bus));
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn legacy_archive_waits_until_both_original_slots_are_preserved() {
        let dir = std::env::temp_dir().join(format!("ffta-state-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("legacy.sav");
        let mut bytes = vec![0; 256];
        for slot in 0..2 {
            let b = &mut bytes[slot * 128..slot * 128 + 128];
            b[..8].copy_from_slice(b"FFTEX000");
            b[16] = slot as u8;
        }
        std::fs::write(&legacy, &bytes).unwrap();
        let quick = dir.join("QuickSave");
        let m = map_machine();
        let state = m.save_state();
        replace(&dir.join("Save1"), &pack(&state, &[])).unwrap();
        archive_legacy(&legacy, &quick);
        assert!(legacy.is_file());
        replace(&dir.join("Save2"), &pack(&state, &[])).unwrap();
        archive_legacy(&legacy, &quick);
        assert!(!legacy.exists());
        assert_eq!(
            std::fs::read(dir.join("legacy.sav.legacy-backup")).unwrap(),
            bytes
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn bundled_states_validate_picture_checksum_and_recover_previous_file() {
        let dir = std::env::temp_dir().join(format!("ffta-state-checksum-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Save1");
        let mut m = map_machine();
        let first = m.save_state();
        replace(&path, &pack(&first, &vec![31; 38400])).unwrap();
        m.bus.ewram[0x89] = 50;
        let mut bytes = pack(&m.save_state(), &vec![992; 38400]);
        replace(&path, &bytes).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        let (state, image) = read_bundle(&path).unwrap();
        assert_eq!(state, first);
        assert_eq!(image, vec![31; 38400]);
        assert!(unpack(&bytes).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn legacy_load_is_imported_only_after_reaching_unobstructed_map() {
        let dir = std::env::temp_dir().join(format!("ffta-import-map-{}", std::process::id()));
        let sav = dir.join("game.sav");
        let mut store = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        let mut m = map_machine();
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
        store.import_after_load(1);
        store.observe_session(&m);
        assert!(store.map_state.is_none());
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        store.observe_session(&m);
        let path = store.path.with_file_name("Save2");
        drop(store);
        let (state, _) = read_bundle(&path).unwrap();
        m.load_state(&state).unwrap();
        assert!(clean_map(&m.bus));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn all_save_entry_phases_are_owned_and_auto_card_cannot_load_when_saving() {
        let dir = std::env::temp_dir().join(format!("ffta-save-phases-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut auto = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ffta_movement.europe_profile = true;
        m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        for callback in [0x0813dd75u32, 0x0813d049] {
            m.bus.iwram[0x2804..0x2808].copy_from_slice(&callback.to_le_bytes());
            for phase in [0, 1, 2, 4, 5, 8, 11, 17, 19, 22, 255] {
                m.bus.iwram[0x27d1] = phase;
                assert!(auto.screen_active(&m.bus));
                assert!(!auto.eligible(&m.bus));
            }
            assert_eq!(selectable(&m.bus, 0), callback == 0x0813dd75);
            assert!(selectable(&m.bus, 1) && selectable(&m.bus, 2));
        }
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        assert!(!auto.screen_active(&m.bus));
        drop(auto);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn original_pattern_can_render_before_cached_menu_flag_is_ready() {
        let mut m = gba_core::Machine::new(vec![]);
        m.bus.ffta_movement.europe_profile = true;
        m.bus.palette[..2].copy_from_slice(&992u16.to_le_bytes());
        let mut row = vec![0; 240];
        for callback in [0x0813dd75u32, 0x0813d049] {
            m.bus.iwram[0x2804..0x2808].copy_from_slice(&callback.to_le_bytes());
            gba_core::ppu::render_menu_background_row(&m.bus, 0, &mut row);
            assert!(row.iter().all(|p| *p == 992));
        }
        m.bus.iwram[0x2804..0x2808].fill(0);
        gba_core::ppu::render_menu_background_row(&m.bus, 0, &mut row);
        assert!(row.iter().all(|p| *p == 0));
    }
    #[test]
    fn physical_preview_reads_full_frame_without_thumbnail_intermediate() {
        let dir = std::env::temp_dir().join(format!("ffta-preview-full-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let auto = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        let mut c = CardInfo::default();
        c.image = (0..240 * 160)
            .map(|i| if i % 2 == 0 { 31 } else { 992 })
            .collect();
        *auto.info.lock().unwrap() = Some(c);
        let mut lut = vec![[0, 0, 0, 255]; 32768];
        lut[31] = [255, 0, 0, 255];
        lut[992] = [0, 255, 0, 255];
        let mut out = Vec::new();
        compose_display(
            &m.bus,
            &auto,
            &vec![[0, 0, 0, 255]; 240 * 160],
            &mut out,
            (240, 160),
            (960, 640),
            [0., 0., 240., 160.],
            &lut,
        );
        assert_eq!(out[348 * 960 + 40], [255, 0, 0, 255]);
        assert_eq!(out[348 * 960 + 41], [0, 255, 0, 255]);
        drop(auto);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejected_manual_load_does_not_mutate_guest_state() {
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ffta_movement.europe_profile = true;
        m.bus.iwram[0x27d1] = 11;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
        m.bus.iwram[0x27d8..0x27dc].copy_from_slice(&0x0200f3d4u32.to_le_bytes());
        let before = m.save_state();
        for slot in [0, 1, 2, usize::MAX] {
            assert!(activate_manual(&mut m, slot).is_err());
        }
        assert_eq!(before, m.save_state());
    }
    #[test]
    fn selector_survives_bg_changes_and_shared_file_phase() {
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ffta_movement.europe_profile = true;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
        for task in [11, 255] {
            m.bus.iwram[0x27d1] = task;
            m.bus.io[8..12].fill(0xff);
            assert!(selector(&m.bus));
        }
        m.bus.iwram[0x27d1] = 4;
        assert!(!selector(&m.bus));
        m.bus.iwram[0x27d1] = 2;
        assert!(!selector(&m.bus));
    }
    #[test]
    fn autosave_only_live_map_and_terrain_not_file_or_management_menus() {
        let dir = std::env::temp_dir().join(format!("ffta-eligibility-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut auto = AutoSave::new(dir.join("game.sav").to_str().unwrap(), "0123456789abcdef");
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ffta_movement.europe_profile = true;
        assert!(!auto.eligible(&m.bus));
        m.bus.io[..2].copy_from_slice(&0x1f40u16.to_le_bytes());
        m.bus.io[8..10].copy_from_slice(&0xaa03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0xac02u16.to_le_bytes());
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x08031601u32.to_le_bytes());
        assert!(auto.eligible(&m.bus));
        m.bus.iwram[0x27d1] = 11;
        m.bus.iwram[0x2804..0x2808].copy_from_slice(&0x0813dd75u32.to_le_bytes());
        assert!(!auto.eligible(&m.bus));
        m.bus.iwram[0x2804..0x2808].fill(0);
        m.bus.io[8..10].copy_from_slice(&0x4c03u16.to_le_bytes());
        m.bus.io[10..12].copy_from_slice(&0x4e02u16.to_le_bytes());
        assert!(auto.eligible(&m.bus));
        m.bus.io[8..10].copy_from_slice(&0xad0bu16.to_le_bytes());
        assert!(!auto.eligible(&m.bus));
        drop(auto);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn preview_retains_every_original_pixel() {
        let ram = vec![0; 262144];
        let pixels: Vec<u16> = (0..240 * 160).map(|i| (i % 32768) as u16).collect();
        assert_eq!(CardInfo::gameplay(&ram, &pixels).image, pixels);
    }
    #[test]
    fn manual_preview_roundtrip_and_header_identity() {
        let dir = std::env::temp_dir().join(format!("ffta-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.previews");
        let mut header = vec![0; 128];
        header[..8].copy_from_slice(b"FFTEX000");
        let key = header_key(&header);
        let mut bytes = b"FFTAPV02".to_vec();
        bytes.extend_from_slice(&key.to_le_bytes());
        for _ in 0..240 * 160 {
            bytes.extend_from_slice(&992u16.to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let shots = read_shots(&path);
        assert_eq!(shots.len(), 1);
        assert_eq!(shots[0].0, key);
        assert_eq!(shots[0].1, vec![992; 240 * 160]);
        header[8] = 1;
        assert_ne!(header_key(&header), key);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn card_hitboxes_exclude_gaps_and_background() {
        assert_eq!(hit(20., 80.), Some(0));
        assert_eq!(hit(100., 80.), Some(1));
        assert_eq!(hit(180., 80.), Some(2));
        assert_eq!(hit(80., 80.), None);
        assert_eq!(hit(100., 40.), None);
    }
    #[test]
    fn five_second_timer_writes_complete_state_and_skips_selectors() {
        let dir = std::env::temp_dir().join(format!("ffta-autosave-timer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sav = dir.join("game.sav");
        let mut auto = AutoSave::new(sav.to_str().unwrap(), "0123456789abcdef");
        let machine = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        auto.tick(&machine, true);
        assert!(!auto.path.exists());
        auto.last = Instant::now() - Duration::from_secs(6);
        auto.tick(&machine, false);
        assert!(
            !auto.path.exists(),
            "a selector must not overwrite gameplay state"
        );
        auto.last = Instant::now() - Duration::from_secs(6);
        auto.tick(&machine, true);
        let path = auto.path.clone();
        drop(auto); // drain background write
        let (bytes, _) = read_bundle(&path).unwrap();
        let mut restored = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        restored.load_state(&bytes).unwrap();
        assert_eq!(bytes, restored.save_state());
        assert!(
            !sav.exists(),
            "cartridge .sav is untouched by automatic state writer"
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn card_metadata_uses_original_live_clock_and_header_fields() {
        let mut ram = vec![0; 262144];
        let encoded = [
            0x80, 0xbc, 0x80, 0xca, 0x80, 0xdb, 0x80, 0xcc, 0x80, 0xd1, 0x80, 0xce,
        ];
        ram[0x1f1c..0x1f28].copy_from_slice(&encoded);
        ram[0x89] = 50;
        ram[0x2fbc..0x2fbe].copy_from_slice(&32u16.to_le_bytes());
        ram[0x2fbe] = 45;
        let info = CardInfo::gameplay(&ram, &vec![31; 240 * 160]);
        assert_eq!(info.name, "Marche");
        assert_eq!(info.level, 50);
        assert_eq!((info.hours, info.minutes), (32, 45));
        assert_eq!(info.image.len(), 240 * 160);
        assert!(info.image.iter().all(|p| *p == 31));
        let mut header = vec![0; 128];
        header[..8].copy_from_slice(b"FFTEX000");
        header[0x1a..0x26].copy_from_slice(&encoded);
        header[0x2e] = 50;
        header[0x30..0x32].copy_from_slice(&(32u16 | (30 << 10)).to_le_bytes());
        let info = CardInfo::header(&header).unwrap();
        assert_eq!(info.name, "Marche");
        assert_eq!((info.hours, info.minutes), (32, 30));
    }
    #[test]
    fn screenshot_and_metadata_survive_application_restart() {
        let mut m = gba_core::Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.bus.ewram[0x89] = 37;
        m.bus.ewram[0x2fbe] = 12;
        m.bus.framebuffer.fill(992);
        let state = m.save_state();
        let (ram, pixels) = gba_core::state::preview(&state).unwrap();
        let info = CardInfo::gameplay(&ram, &pixels);
        assert_eq!(info.level, 37);
        assert_eq!(info.minutes, 12);
        assert!(info.image.iter().all(|p| *p == 992));
        let mut bad = state;
        *bad.last_mut().unwrap() ^= 1;
        assert!(gba_core::state::preview(&bad).is_err());
    }
    #[test]
    fn replacement_keeps_previous_state_backup() {
        let dir = std::env::temp_dir().join(format!("ffta-autosave-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("auto.state");
        replace(&path, b"old").unwrap();
        replace(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::read(backup(&path)).unwrap(), b"old");
        for p in [path.clone(), backup(&path)] {
            std::fs::remove_file(p).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

pub(crate) fn cancel_selector(m:&mut gba_core::Machine)->Result<(),String>{invoke_save_handler(m,2)}
