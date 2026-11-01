use crate::compiler::{self, NativeCode};
use gba_core::{Machine, cpu::FLAG_I, mem};
use sha1::{Digest, Sha1};
use std::{collections::BTreeSet, path::PathBuf, sync::{Mutex, atomic::{AtomicU32, Ordering}}, time::Instant};
pub static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
pub static STATUS: Mutex<String> = Mutex::new(String::new());
pub static KEYS: AtomicU32 = AtomicU32::new(0x3ff);
pub const MAX_STEPS: u64 = 5_000_000;
pub fn status(message: impl Into<String>) { *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = message.into(); }
pub struct Engine {
    battle_hud_pixels:[Vec<[u8;4]>;4],
    pub(crate) battle_hud_motion:crate::ui_motion::BattleHud,
    pub ui_motion:crate::ui_motion::Motion,
    ui_scene:gba_core::ffta_ui::Scene,
    ui_pixels:Vec<[u8;4]>,
    panel_pixels:Vec<[u8;4]>,
    ui_auxiliary:[Vec<[u8;4]>;2],
    ui_pack_cache:[gba_core::ffta_dialogue::PackedLayer;4],
    pub saves: Option<crate::save_states::AutoSave>,
    save_pixels: Vec<u16>,
    presentation_pixels: Vec<u16>,
    save_held: u16,
    rate_phase:usize,rate_units:usize,
    pub mouse: gba_core::ffta_mouse::LanguageMouse,
    pub touch_enabled: bool,
    pub ui_base: f32,
    pub ui_export_ns: u64,
    pub ui_window: (u32,u32),
    pub world_rect: [f32;4],
    pub machine: Box<Machine>,
    code: Option<NativeCode>,
    extensions: Vec<NativeCode>,
    pending: Option<std::sync::mpsc::Receiver<Result<NativeCode,String>>>,
    missed: BTreeSet<u32>,
    adaptive: bool,
    pub pixels: Vec<i32>,
    pub audio: Vec<i16>,
    pcm: crate::audio::Audio,
    pub native_blocks: u64,
    pub fallback_steps: u64,
    pub frame_ns: u64,
    pub prepare_ms: u64,
    pub save_path: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub cache_hit: bool,
    rom_cache_key: String,
}
pub fn verify_rom(bytes: &[u8]) -> bool {
    let sha1=format!("{:x}", Sha1::digest(bytes));
    let sha256=gba_core::ffta_rom::digest(bytes);
    (sha1 == "4ac05441f4de70a4ec3dd932116346c61b8783d9"
        && sha256 == gba_core::ffta_rom::SHA256)
        || (sha1 == "9efaf328cbbcbc830be14940e42e4b92d90dfb58"
        && sha256 == "8fab1e390cb9d5e695bfdb86e81246892e1aa8c0cb4014db8235ff76538ea632")
}
/// Normalize unexpanded frontend rows BEFORE drawing host menus. A host
/// overlay always indexes a full render_width * render_height canvas.
fn presentation_frame<'a>(bus:&'a gba_core::MemMap,padding:&'a mut Vec<u16>)->&'a [u16]{
    let w=mem::render_width();let h=mem::render_height();let ey=mem::extra_y();
    if ey==0 {
        return if mem::extra()==0 && !bus.ffta_dialogue.naming_layout {&bus.framebuffer[..]}else{&bus.wide_framebuffer[..w*h]};
    }
    if gba_core::ppu::ffta_zoom_rows_ready(bus){return &bus.wide_framebuffer[..w*h];}
    padding.resize(w*h,0);padding.fill(0);
    for y in (0..ey).chain(ey+160..h){
        gba_core::ppu::render_frontend_background_row(bus,y as i32-ey as i32,&mut padding[y*w..(y+1)*w]);
    }
    padding[ey*w..(ey+160)*w].copy_from_slice(&bus.wide_framebuffer[..w*160]);
    padding
}
impl Engine {
    pub fn new(rom: Vec<u8>) -> Self {
        let rom_cache_key=gba_core::ffta_rom::digest(&rom);
        let mouse=gba_core::ffta_mouse::LanguageMouse::new(&rom,false);
        Self {battle_hud_pixels:Default::default(),battle_hud_motion:Default::default(),ui_motion:Default::default(),ui_scene:Default::default(),ui_pixels:Vec::new(),panel_pixels:Vec::new(),ui_auxiliary:Default::default(),ui_pack_cache:Default::default(),saves:None,save_pixels:Vec::new(),presentation_pixels:Vec::new(),save_held:0,rate_phase:0,rate_units:0,mouse,touch_enabled:false,ui_base:240.,ui_export_ns:0,ui_window:(240,160),world_rect:[0.,0.,240.,160.],machine: Box::new(Machine::new(rom)), code: None, extensions: Vec::new(), pending: None,
            missed: BTreeSet::new(), adaptive: cfg!(target_os="android"), pixels: Vec::new(),
            audio: Vec::with_capacity(4096), pcm: crate::audio::Audio::default(), native_blocks: 0,
            fallback_steps: 0, frame_ns: 0, prepare_ms: 0, save_path: None, cache_dir: None, cache_hit: false,
            rom_cache_key }
    }
    pub fn prepare(&mut self) -> Result<(), String> {
        let start = Instant::now();
        if let Some(path)=self.cache_path("base") {
            if valid_cache(&path) {
                if let Ok(code)=compiler::load_object(&path) {
                    self.code=Some(code);
                    self.cache_hit=true;
                    for index in 0..16 {
                        if let Some(hot)=self.cache_path(&format!("hot-{index}")) {
                            if valid_cache(&hot) {
                                if let Ok(code)=compiler::load_object(&hot) {
                                    self.code.as_mut().unwrap().blocks.extend(code.blocks.iter().map(|(k,v)|(*k,*v)));
                                    self.extensions.push(code);
                                }
                            }
                        }
                    }
                    self.prepare_ms=start.elapsed().as_millis() as u64;
                    status("Ready (cached game)");
                    return Ok(());
                }
            }
        }
        status("Profiling boot code (90 frames)...");
        mem::set_extra(0);
        mem::set_extra_y(0);
        // Separate machine: preparation never advances the user's game/save.
        let mut profile = Machine::new(self.machine.bus.rom.clone());
        if let Some(save) = self.machine.bus.save_data() { profile.bus.load_save_data(save); }
        let mut seeds = BTreeSet::from([0x08000000]);
        for frame in 0..90 {
            let before = profile.bus.frames;
            let mut previous_end = u32::MAX;
            for _ in 0..MAX_STEPS {
                let pc = profile.cpu.regs[15];
                if matches!(pc >> 24, 2 | 3 | 8) && pc != previous_end {
                    seeds.insert(pc | profile.cpu.thumb() as u32);
                }
                previous_end = pc.wrapping_add(if profile.cpu.thumb() {2} else {4});
                profile.step();
                if profile.bus.frames != before { break; }
            }
            profile.bus.audio_buf.clear();
            if frame % 15 == 0 { status(format!("Profiling boot: {frame}/90 frames")); }
        }
        status("Discovering native ARM/Thumb blocks...");
        let view = recomp_core::analyze::View { rom: &profile.bus.rom, ewram: Some(&profile.bus.ewram), iwram: Some(&profile.bus.iwram), bios: None };
        let analysis = recomp_core::analyze::analyze(&view, &seeds.into_iter().collect::<Vec<_>>());
        let object=self.cache_path("base");
        self.code = Some(compiler::compile(&analysis, &view, object.as_deref(), |done, total| {
            status(format!("Compiling native blocks: {done}/{total}"));
        })?);
        if let Some(path)=&object { seal_cache(path)?; }
        self.prepare_ms = start.elapsed().as_millis() as u64;
        status(format!("Ready: {} native blocks, {} ms", self.code.as_ref().unwrap().blocks.len(), self.prepare_ms));
        Ok(())
    }
    pub fn frame(&mut self, count: usize, extra_x: usize, extra_y: usize, sound: bool) -> Result<(), String> {
        self.frame_audio(count,extra_x,extra_y,if sound{1}else{0})
    }
    pub fn frame_audio(&mut self,count:usize,extra_x:usize,extra_y:usize,mode:u32)->Result<(),String>{
        self.frame_batch(count, count.clamp(1,8)*2,extra_x,extra_y,mode)
    }
    pub fn frame_speed(&mut self,speed:f32,extra_x:usize,extra_y:usize,mode:u32)->Result<(),String>{
        if ![1.,1.5,2.,3.,4.].contains(&speed){return Err("Invalid speed".into());}
        let units=(speed*2.)as usize;
        if self.rate_units!=units{self.rate_phase=0;self.rate_units=units;}
        self.rate_phase+=units;
        let count=self.rate_phase/2;self.rate_phase%=2;
        self.frame_batch(count,units,extra_x,extra_y,mode)
    }
    fn frame_batch(&mut self,count:usize,speed_half:usize,extra_x:usize,extra_y:usize,mode:u32)->Result<(),String>{
        let start = Instant::now();
        self.update_saves()?;
        self.collect_compilation();
        mem::set_extra(extra_x);
        mem::set_extra_y(extra_y);
        self.machine.bus.ffta_dialogue.battle_hud_overlay=true;
        self.machine.bus.ffta_dialogue.view_width=mem::render_width()as f32;
        self.machine.bus.ffta_dialogue.view_height=mem::render_height()as f32;
        self.machine.bus.ffta_dialogue.view_zoom=mem::render_height()as f32/160.;
        self.audio.clear();
        self.pcm.begin_half(speed_half,mode,
            self.cache_dir.as_deref().unwrap_or(&std::env::temp_dir()))?;
        for _ in 0..count.clamp(1, 8) {
            let mut physical=(KEYS.load(Ordering::Relaxed) as u16)&0x3ff;
            if self.saves.as_ref().is_some_and(|s|s.screen_active(&self.machine.bus)) {
                let pressed=(!physical)&0x3ff;let edge=pressed&!self.save_held;self.save_held=pressed;
                let modal=self.saves.as_ref().is_some_and(|s|s.delete_confirm.is_some());
                self.save_buttons(edge)?;physical|=if modal{0x3ff}else{0x3f9};
            } else { self.save_held=0; }
            let m = &mut *self.machine;
            let keys=self.mouse.keys(&mut m.bus,physical);
            m.bus.keys = gba_core::ffta_battle_mouse::keys(&mut m.bus,keys);
            m.bus.frame_ready = false;
            let mut previous_end = u32::MAX;
            for _ in 0..MAX_STEPS {
                if m.bus.frame_ready { break; }
                let pc = m.cpu.regs[15];
                let native = if m.bus.halted
                    || (pc == gba_core::machine::IRQ_RETURN_ADDR && m.cpu.mode() == gba_core::Mode::Irq)
                    || (m.bus.irq_pending() && !m.cpu.flag(FLAG_I))
                    || m.bus.ffta_battle_mouse.interprets(pc) || m.ffta_sprite_interpret(pc) { None }
                    else { self.code.as_ref().and_then(|code| code.blocks.get(&(pc | m.cpu.thumb() as u32))).copied() };
                if let Some(function) = native {
                    if let Some(h) = m.bus.mp2k.as_deref() {
                        let key = pc | m.cpu.thumb() as u32;
                        if h.active && h.hook_match(key) { m.bus.mp2k_frame_hook(key); }
                    }
                    function(&gba_core::capi::RT_API, m as *mut _ as *mut _);
                    self.native_blocks += 1;
                    previous_end = u32::MAX;
                } else {
                    if self.adaptive && pc != previous_end && matches!(pc>>24,2|3|8) && self.missed.len()<2048
                        && self.code.as_ref().is_some_and(|code|!code.blocks.contains_key(&(pc|m.cpu.thumb() as u32))) {
                        self.missed.insert(pc | m.cpu.thumb() as u32);
                    }
                    previous_end = pc.wrapping_add(if m.cpu.thumb(){2}else{4});
                    m.step(); self.fallback_steps += 1;
                }
            }
            if !m.bus.frame_ready { return Err(format!("Frame stalled at PC {:08x}", m.cpu.regs[15])); }
            self.pcm.feed(&m.bus.audio_buf);
            m.bus.audio_buf.clear();
        }
        self.pcm.finish(&mut self.audio)?;
        let size = mem::render_width() * mem::render_height();
        self.pixels.resize(size, 0);
        if let Some(saves)=self.saves.as_mut(){saves.route_entry(&mut self.machine);}
        let save_screen=self.saves.as_ref().is_some_and(|s|s.screen_active(&self.machine.bus));
        let source=presentation_frame(&self.machine.bus,&mut self.presentation_pixels);
        let source=if save_screen {
            crate::save_states::paint(&self.machine.bus,source,&mut self.save_pixels,self.saves.as_ref().unwrap());
            self.save_pixels.as_slice()
        }else{source};
        let colors=rgba_colors();
        for (out,&pixel) in self.pixels.iter_mut().zip(source){
            *out=colors[(pixel&0x7fff) as usize];
        }
        self.prepare_ui();
        self.frame_ns = start.elapsed().as_nanos() as u64;
        if self.adaptive && self.machine.bus.frames % 120 < count as u64 { self.start_compilation(); }
        Ok(())
    }
    fn collect_compilation(&mut self) {
        let result = self.pending.as_ref().and_then(|rx|rx.try_recv().ok());
        if let Some(result) = result {
            self.pending = None;
            match result {
                Ok(code) => {
                    if let Some(primary) = &mut self.code { primary.blocks.extend(code.blocks.iter().map(|(k,v)|(*k,*v))); }
                    self.extensions.push(code);
                }
                Err(error) => status(format!("Background compiler: {error}; interpreter remains available")),
            }
        }
    }
    fn start_compilation(&mut self) {
        if self.pending.is_some() || self.code.is_none() || self.missed.is_empty() || self.extensions.len()>=16 { return; }
        let seeds=std::mem::take(&mut self.missed);
        let rom=self.machine.bus.rom.clone();
        let ewram=self.machine.bus.ewram.clone();
        let iwram=self.machine.bus.iwram.clone();
        let object=self.cache_path(&format!("hot-{}",self.extensions.len()));
        let (tx,rx)=std::sync::mpsc::channel();
        self.pending=Some(rx);
        std::thread::spawn(move || {
            #[cfg(target_os="android")]
            unsafe { libc::setpriority(libc::PRIO_PROCESS,0,10); }
            let view=recomp_core::analyze::View{rom:&rom,ewram:Some(&ewram),iwram:Some(&iwram),bios:None};
            let mut analysis=recomp_core::analyze::analyze(&view,&seeds.iter().copied().collect::<Vec<_>>());
            // Only observed misses are compiled; generated RAM blocks keep content guards.
            analysis.blocks.retain(|block|seeds.contains(&block.key()));
            let result=compiler::compile(&analysis,&view,object.as_deref(),|_,_|{}).and_then(|code|{
                if let Some(path)=&object{seal_cache(path)?;}
                Ok(code)
            });
            let _=tx.send(result);
        });
    }
    fn cache_path(&self,kind:&str)->Option<PathBuf> {
        let directory=self.cache_dir.as_ref()?;
        if std::fs::create_dir_all(directory).is_err(){return None;}
        Some(directory.join(format!("{kind}-{}-{}.o",env!("FFTA_ANDROID_CACHE_TAG"),self.rom_cache_key)))
    }
    pub(crate) fn prepare_ui(&mut self){
        gba_core::ffta_battle_hud::capture_layers(&self.machine.bus,&mut self.battle_hud_pixels);
        self.battle_hud_motion.sync(&self.machine.bus,&self.battle_hud_pixels);
        if let Some((pixels,_))=self.machine.bus.ffta_dialogue.layer() {
            self.ui_pixels.resize(pixels.len(),[0;4]);
            let state=&self.machine.bus.ffta_dialogue;
            if state.action_layout {
                self.ui_pack_cache[0].update_with_bounds(pixels,&state.backing,if state.action_banner.is_some(){1.}else{state.style.length},false,Some([0,48,160,160]),&mut self.ui_pixels);
            } else if state.town_layout || state.shop_layout {
                self.ui_pack_cache[0].update(pixels,&[],1.,true,&mut self.ui_pixels);
            } else {self.ui_pack_cache[0].update(pixels,&[],1.,true,&mut self.ui_pixels);}
        } else {self.ui_pixels.clear();self.ui_pack_cache[0].invalidate();}
        let extra=&self.machine.bus.ffta_dialogue.extra;
        if self.machine.bus.ffta_dialogue.separate && extra.active() {
            self.panel_pixels.resize(extra.pixels.len(),[0;4]);
            let b=self.machine.bus.ffta_dialogue.stats_target.or(extra.bounds).or(self.ui_pack_cache[1].bounds).unwrap_or([0;4]);
            let state=&self.machine.bus.ffta_dialogue;
            let geometry=gba_core::ffta_ui::Scene::capture(state).nodes[1].map(|n|n.bounds).or(state.stats_target.or(extra.bounds));
            self.ui_pack_cache[1].update_with_bounds(&extra.pixels,&extra.backing,if state.town_layout || extra.kind==gba_core::ffta_dialogue::PanelKind::Notice {1.}else{state.style.length},state.town_layout || b[0]+b[2]>=240,geometry,&mut self.panel_pixels);
        } else {self.panel_pixels.clear();self.ui_pack_cache[1].invalidate();}
        self.ui_scene=gba_core::ffta_ui::Scene::capture_with_bounds(&self.machine.bus.ffta_dialogue,
            [self.ui_pack_cache[0].bounds,self.ui_pack_cache[1].bounds]);
        let mut auxiliary_used=[false;2];
        let target=&self.machine.bus.ffta_dialogue.battle_stats;
        if target.active() {
            if let Some(bounds)=self.machine.bus.ffta_dialogue.stats_target.or(target.bounds) {
                let right=self.machine.bus.ffta_dialogue.stats_right;
                self.ui_pack_cache[2].update_with_bounds(&target.pixels,&target.backing,
                    self.machine.bus.ffta_dialogue.style.length,right,Some(bounds),&mut self.ui_auxiliary[0]);
                auxiliary_used[0]=true;
                self.ui_scene.nodes[2]=Some(gba_core::ffta_ui::Node{slot:2,role:gba_core::ffta_ui::Role::Stats,
                    anchor:gba_core::ffta_ui::Anchor::Corner{right,bottom:true},bounds,length:true});
            }
        }
        if let Some(bounds)=self.machine.bus.ffta_dialogue.funds_bounds {
            self.ui_pack_cache[2].update_with_bounds(&self.machine.bus.ffta_dialogue.funds_pixels,&[],1.,true,Some(bounds),&mut self.ui_auxiliary[0]);
            auxiliary_used[0]=true;
            self.ui_scene.nodes[2]=Some(gba_core::ffta_ui::Node{slot:2,role:gba_core::ffta_ui::Role::Funds,
                anchor:gba_core::ffta_ui::Anchor::Edge{right:true,vertical:160.},bounds,length:false});
        }
        let speech=&self.machine.bus.ffta_dialogue.service_speech;
        if speech.active() {
            if let Some(bounds)=speech.bounds {
                self.ui_pack_cache[3].update_with_bounds(&speech.pixels,&[],1.,false,Some(bounds),&mut self.ui_auxiliary[1]);
                auxiliary_used[1]=true;
                self.ui_scene.nodes[3]=Some(gba_core::ffta_ui::Node{slot:3,role:gba_core::ffta_ui::Role::Dialogue,
                    anchor:gba_core::ffta_ui::Anchor::Edge{right:self.machine.bus.ffta_dialogue.service_speech_right,
                        vertical:if bounds[1]+bounds[3]>=160{160.}else{0.}},bounds,length:false});
            }
        }
        if let Some(bounds)=self.machine.bus.ffta_dialogue.action_banner {
            extract_ui_rectangle(&mut self.ui_pixels,&mut self.ui_auxiliary[1],bounds);
            self.ui_pack_cache[3].invalidate();auxiliary_used[1]=true;
            self.ui_scene.nodes[3]=Some(gba_core::ffta_ui::Node{slot:3,role:gba_core::ffta_ui::Role::AttackName,
                anchor:gba_core::ffta_ui::Anchor::TopCenter,bounds,length:false});
        }
        for (i,used) in auxiliary_used.into_iter().enumerate() {
            if !used {self.ui_auxiliary[i].clear();self.ui_pack_cache[i+2].invalidate();}
        }
        if self.machine.bus.ffta_dialogue.action_banner.is_some() {self.ui_pack_cache[0].invalidate();}
        self.ui_motion.sync(&self.machine.bus,self.ui_scene.nodes,
            [&self.ui_pixels,&self.panel_pixels,&self.ui_auxiliary[0],&self.ui_auxiliary[1]]);
    }
    pub fn init_saves(&mut self) {
        if self.saves.is_none(){if let Some(path)=&self.save_path {
            let name=if path.file_name().is_some_and(|n|n=="benchmark.sav"){"benchmark-saves"}else{"Saves"};
            let dir=path.parent().unwrap_or(std::path::Path::new(".")).join(name);
            self.saves=Some(crate::save_states::AutoSave::new_at(&path.to_string_lossy(),&self.rom_cache_key,dir));
        }}
    }
    fn update_saves(&mut self)->Result<(),String>{
        self.init_saves();
        if let Some(s)=self.saves.as_mut(){
            if s.pending_load {
                s.pending_load=false;
                let style=self.machine.bus.ffta_dialogue.style;
                s.load(&mut self.machine)?;
                self.machine.bus.ffta_dialogue.style=style;
                self.machine.bus.ffta_dialogue.separate=true;
                self.machine.bus.ffta_battle_mouse.set_enabled(self.touch_enabled);
                let sigs=gba_core::mp2k::detect(&self.machine.bus.rom);
                if !sigs.is_empty(){self.machine.bus.mp2k=Some(Box::new(gba_core::mp2k::Mp2kHle::new(&sigs)));}
                self.ui_motion=Default::default();for c in &mut self.ui_pack_cache{c.invalidate();}
                self.mouse.cancel();self.machine.bus.ffta_battle_mouse.cancel();
                KEYS.store(0x3ff,Ordering::Relaxed);self.machine.bus.keys=0x3ff;
                self.pcm=Default::default();self.audio.clear();
            }
            s.route_entry(&mut self.machine);s.observe_session(&self.machine);
            let eligible=s.eligible(&self.machine.bus);s.tick(&self.machine,eligible);
        }
        Ok(())
    }
    pub fn save(&mut self) -> Result<(), String> { self.update_saves() }
    fn activate_card(&mut self,id:usize)->Result<(),String>{
        let s=self.saves.as_mut().ok_or("Save storage unavailable")?;
        if crate::save_states::is_saving(&self.machine.bus){s.save_and_return(id.saturating_sub(1))}
        else if id==0 || s.new_slot(id-1){s.queue_load(id);Ok(())}
        else if !s.deleted(id){crate::save_states::activate_manual(&mut self.machine,id.saturating_sub(1))?;s.import_after_load(id.saturating_sub(1));Ok(())}
        else{Ok(())}
    }
    fn save_buttons(&mut self,edge:u16)->Result<(),String>{
        let Some(s)=self.saves.as_mut() else{return Ok(())};
        if s.delete_confirm.is_some(){
            if edge&0x30!=0{s.delete_accept=!s.delete_accept;}
            if edge&2!=0{return s.finish_delete(false);}
            if edge&1!=0{return s.finish_delete(s.delete_accept);}
            return Ok(());
        }
        let saving=crate::save_states::is_saving(&self.machine.bus);
        if edge&0x30!=0{s.move_carousel(saving,edge&0x10!=0);}
        if edge&1!=0{let id=s.selected;return self.activate_card(id);}
        Ok(())
    }
    pub fn save_tap(&mut self,x:f32,y:f32,back:bool)->bool{
        let Some(s)=self.saves.as_mut() else{return false};
        if !s.screen_active(&self.machine.bus){return false;}
        let Some((nx,ny))=save_canvas_point(self.ui_window,self.world_rect,
            (mem::render_width(),mem::render_height()),(mem::extra(),mem::extra_y()),x,y) else{return true;};
        let result=if s.delete_confirm.is_some(){
            if back{s.finish_delete(false)}else if (100. ..121.).contains(&ny) && (79. ..160.).contains(&nx){s.finish_delete(nx>=131.)}else{Ok(())}
        }else{
            let saving=crate::save_states::is_saving(&self.machine.bus);
            if back {if let Err(e)=crate::save_states::cancel_selector(&mut self.machine){s.message=Some(e);}return true;}
            if let Some(right)=s.arrow_hit(saving,nx,ny){s.scroll_carousel(saving,right);Ok(())}
            else if let Some(id)=s.trash_hit(saving,nx,ny){s.request_delete(id);Ok(())}
            else if let Some(id)=s.hit_card(saving,nx,ny){s.selected=id;self.activate_card(id)}else{Ok(())}
        };
        if let Err(e)=result{if let Some(s)=self.saves.as_mut(){s.message=Some(e);}}
        true
    }
    pub fn diagnostics(&self) -> String {
        let state=&self.machine.bus.ffta_dialogue;
        let ui_hash=state.layer().map(|(p,_)|p).unwrap_or(&[]).iter()
            .chain(if state.extra.active(){state.extra.pixels.iter()}else{[].iter()})
            .flat_map(|pixel|pixel.packed()).fold(0xcbf29ce484222325u64,|h,b|(h^b as u64).wrapping_mul(0x100000001b3));
        let hash = self.pixels.iter().fold(0xcbf29ce484222325u64, |h,p| (h ^ *p as u32 as u64).wrapping_mul(0x100000001b3));
        let oam_hash=self.machine.bus.oam.iter().fold(0xcbf29ce484222325u64,|h,&v|(h^v as u64).wrapping_mul(0x100000001b3));
        format!("UI FNV64: {ui_hash:016x}\nOAM FNV64: {oam_hash:016x}\nBackend: {}\nNative blocks: {}\nNative dispatches: {}\nInterpreter fallback instructions: {}\nFrames: {}\nLast frame CPU: {:.2} ms\nPreparation: {} ms\nRender: {}x{}\nFramebuffer FNV64: {hash:016x}\nPC: {:08x}\nGame audio: real GBA PCM 65536 -> 48000 Hz\nCache hit: {}\n", 
            if self.code.is_some() {"libtcc in-process native blocks"} else {"cached interpreter (not recompiled)"},
            self.code.as_ref().map_or(0,|c|c.blocks.len()),self.native_blocks,self.fallback_steps,self.machine.bus.frames,
            self.frame_ns as f64/1e6,self.prepare_ms,mem::render_width(),mem::render_height(),self.machine.cpu.regs[15],self.cache_hit)
    }
}
pub(crate) fn rgba_colors()->&'static[i32]{
    static COLORS:std::sync::OnceLock<Box<[i32]>>=std::sync::OnceLock::new();
    COLORS.get_or_init(||(0u32..32768).map(|pixel|{
        let r=(pixel&31)*255/31;
        let g=((pixel>>5)&31)*255/31;
        let b=((pixel>>10)&31)*255/31;
        (0xff000000|(r<<16)|(g<<8)|b) as i32
    }).collect::<Vec<_>>().into_boxed_slice())
}
fn seal_cache(path:&std::path::Path)->Result<(),String>{
    let data=std::fs::read(path).map_err(|e|e.to_string())?;
    std::fs::write(path.with_extension("sha256"),gba_core::ffta_rom::digest(&data)).map_err(|e|e.to_string())
}
fn valid_cache(path:&std::path::Path)->bool{
    if !std::fs::metadata(path).is_ok_and(|meta|meta.len()>0 && meta.len()<64*1024*1024){return false;}
    match (std::fs::read(path),std::fs::read_to_string(path.with_extension("sha256"))){
        (Ok(data),Ok(hash))=>gba_core::ffta_rom::digest(&data)==hash,
        _=>false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_color_conversion_matches_every_hardware_color(){
        for pixel in 0u32..65536{
            let r=(pixel&31)*255/31;let g=((pixel>>5)&31)*255/31;let b=((pixel>>10)&31)*255/31;
            assert_eq!(rgba_colors()[(pixel&0x7fff) as usize],(0xff000000|(r<<16)|(g<<8)|b) as i32);
        }
    }
    static VIEW_TEST:std::sync::Mutex<()>=std::sync::Mutex::new(());
    #[test]
    fn rejects_large_arbitrary_file() { assert!(!verify_rom(&vec![0; 8*1024*1024])); }
    #[test]
    fn real_bus_keys_and_ppu_match_desktop_for_600_frames() {
        let _lock=VIEW_TEST.lock().unwrap();
        let mut rom = vec![0; 192];
        rom[..4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
        let mut mobile = Engine::new(rom.clone());
        let mut reference = Machine::new(rom);
        for frame in 0..600 {
            let keys = if frame % 2 == 0 {0x3fe} else {0x3ff};
            KEYS.store(keys, Ordering::Relaxed);
            reference.bus.keys = keys as u16;
            reference.run_frame(MAX_STEPS);
            mobile.frame(1,0,0,false).unwrap();
            assert_eq!(mobile.machine.bus.framebuffer, reference.bus.framebuffer);
            assert_eq!(mobile.machine.bus.oam, reference.bus.oam);
            assert_eq!(mobile.machine.bus.keys, reference.bus.keys);
            assert_eq!(mobile.machine.cpu.regs, reference.cpu.regs);
        }
    }
    #[test]
    fn square_view_does_not_show_stale_rows_outside_battle() {
        let _lock=VIEW_TEST.lock().unwrap();
        let mut rom=vec![0;192];
        rom[..4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
        let mut engine=Engine::new(rom);
        engine.machine.bus.io[0]=0x80;
        engine.frame(1,8,48,false).unwrap();
        assert_eq!(engine.pixels.len(),256*256);
        assert_eq!(engine.pixels[0],0xff000000u32 as i32);
        assert_eq!(engine.pixels[48*256],0xffffffffu32 as i32);
        assert_eq!(engine.pixels[255*256],0xff000000u32 as i32);
        mem::set_extra(0);mem::set_extra_y(0);
    }
    #[test]
    #[ignore = "requires a native TinyCC library and the user's exact ROM"]
    fn recompiled_ffta_matches_desktop_for_600_frames() {
        let rom = std::fs::read(std::env::var_os("FFTA_TEST_ROM").expect("FFTA_TEST_ROM")).unwrap();
        assert!(verify_rom(&rom));
        let mut mobile = Engine::new(rom.clone());
        let mut reference = Machine::new(rom);
        mobile.adaptive = true;
        let cache = std::env::var_os("FFTA_ANDROID_CACHE_TEST").map(PathBuf::from);
        mobile.cache_dir = cache.clone();
        println!("FFTA_ANDROID test: preparing native code");
        mobile.prepare().unwrap();
        if cache.is_some() {
            let mut warm=Engine::new(mobile.machine.bus.rom.clone());
            warm.cache_dir=cache;
            warm.adaptive=true;
            warm.prepare().unwrap();
            assert!(warm.cache_hit,"warm start must load objects without profiling/recompiling");
            println!("FFTA_ANDROID warm cache hit: {} ms",warm.prepare_ms);
            mobile=warm;
        }
        println!("FFTA_ANDROID test: prepared {} blocks", mobile.code.as_ref().unwrap().blocks.len());
        mobile.configure_touch(false,240.,1.,1.);
        for _ in 0..600 {
            KEYS.store(0x3ff,Ordering::Relaxed);
            reference.run_frame(MAX_STEPS);
            reference.bus.audio_buf.clear();
            mobile.frame(1,0,0,false).unwrap();
            assert_eq!(mobile.machine.bus.framebuffer,reference.bus.framebuffer,"frame {}",reference.bus.frames);
            assert_eq!(mobile.machine.bus.oam,reference.bus.oam,"OAM frame {}",reference.bus.frames);
            for (&pixel,&color) in mobile.pixels.iter().zip(&reference.bus.framebuffer) {
                assert_eq!(pixel,rgba_colors()[(color&0x7fff) as usize],"presented zero-extra pixels");
            }
        }
        println!("FFTA_ANDROID 600 frames framebuffer/OAM IDENTICAL\n{}",mobile.diagnostics());
    }
}

#[cfg(test)]
mod tutorial_replay_tests {
    use super::*;
    #[test]
    #[ignore = "requires user ROM, replay and backup save; writes metrics only"]
    fn tutorial_characters_replay() {
        use std::io::Write;
        let rom=std::fs::read(std::env::var_os("FFTA_TEST_ROM").unwrap()).unwrap();assert!(verify_rom(&rom));
        let mut engine=Engine::new(rom);engine.adaptive=true;
        engine.machine.bus.load_save_data(&std::fs::read(std::env::var_os("FFTA_TEST_SAVE").unwrap()).unwrap());
        engine.prepare().unwrap();
        engine.configure_touch(false,240.,1.,1.);
        let replay=std::fs::read_to_string(std::env::var_os("FFTA_TEST_INPUT").unwrap()).unwrap();
        let points=replay.lines().skip(1).filter(|s|!s.trim().is_empty()).map(|s|{
            let mut p=s.split_whitespace();(p.next().unwrap().parse::<u64>().unwrap(),u32::from_str_radix(p.next().unwrap(),16).unwrap())
        }).collect::<Vec<_>>();
        let mut file=std::fs::File::create(std::env::var_os("FFTA_METRIC_OUT").unwrap()).unwrap();
        writeln!(file,"frame,roster_valid,present,drawn,ids,drawn_ids,duplicates,missing_positions").unwrap();
        let mut cursor=0;let mut keys=1023;
        for frame in 0..13100u64 {
            while cursor<points.len() && points[cursor].0<=frame{keys=points[cursor].1;cursor+=1;}
            KEYS.store(keys,Ordering::Relaxed);
            gba_core::ffta_sprites::set_character_metrics(frame>=11899);
            engine.frame(1,136,176,false).unwrap();
            if frame>=11899{writeln!(file,"{}",gba_core::ffta_sprites::tutorial_metric(&engine.machine.bus)).unwrap();}
        }
        // Own debug data only: decoded OAM attributes, never ROM code.
        let bus=&engine.machine.bus;
        let path=PathBuf::from(std::env::var_os("FFTA_METRIC_OUT").unwrap()).with_extension("oam.csv");
        let mut out=std::fs::File::create(path).unwrap();
        writeln!(out,"slot,a0,a1,a2,host_replacements").unwrap();
        for(slot,entry)in bus.oam.chunks_exact(8).enumerate(){
            let a=|i|u16::from_le_bytes([entry[i],entry[i+1]]);
            let replacements=bus.ffta_sprites.active.iter().flatten().filter(|o|o.replaces && o.slot==slot).count();
            writeln!(out,"{slot},{:04x},{:04x},{:04x},{replacements}",a(0),a(2),a(4)).unwrap();
        }
        gba_core::ffta_sprites::set_character_metrics(false);mem::set_extra(0);mem::set_extra_y(0);
    }
}

fn extract_ui_rectangle(source:&mut Vec<[u8;4]>,out:&mut Vec<[u8;4]>,b:[i32;4]) {
    out.resize(480*160,[0,0,0,16]);out.fill([0,0,0,16]);
    if source.len()!=480*160{return;}
    for y in b[1].max(0)..b[3].min(160) {for x in b[0].max(-120)..b[2].min(360) {
        let i=y as usize*480+(x+120)as usize;out[i]=source[i];source[i]=[0,0,0,16];
    }}
}

#[cfg(test)]
mod save_port_tests {
    use super::*;
    #[test]
    fn saves_use_private_saves_directory_and_never_rewrite_cartridge(){
        let dir=std::env::temp_dir().join(format!("ffta-android-port-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path=dir.join("ffta.sav");std::fs::write(&path,b"original cartridge data").unwrap();
        let mut e=Engine::new(vec![0;192]);e.save_path=Some(path.clone());e.init_saves();
        assert!(dir.join("Saves").is_dir());e.save().unwrap();drop(e);
        assert_eq!(std::fs::read(&path).unwrap(),b"original cartridge data");
        std::fs::remove_file(path).unwrap();
        // Only remove the known empty test directories; never recurse over user saves.
        let _=std::fs::remove_dir(dir.join("Saves"));let _=std::fs::remove_dir(dir);
    }
}

#[cfg(test)]mod frontend_canvas_tests {
    use super::*;
    #[test]fn zoomed_frontend_is_padded_before_save_cards_index_it(){
        mem::set_extra(218);mem::set_extra_y(80);
        let mut bus=gba_core::MemMap::new(vec![]);
        let w=mem::render_width();let h=mem::render_height();
        bus.wide_framebuffer[..w*160].fill(31);
        bus.wide_framebuffer[w*160..w*h].fill(0x7c00);
        bus.io[..2].copy_from_slice(&0x1140u16.to_le_bytes());
        bus.io[8..10].copy_from_slice(&0x3000u16.to_le_bytes());
        bus.vram[..32].fill(0x11);bus.palette[2..4].copy_from_slice(&992u16.to_le_bytes());
        let mut padding=Vec::new();let frame=presentation_frame(&bus,&mut padding);
        assert_eq!(frame.len(),w*h);
        assert!(frame[..w*80].iter().all(|&p|p==992));
        assert!(frame[w*80..w*240].iter().all(|&p|p==31));
        assert!(frame[w*240..].iter().all(|&p|p==992));
        let frame=frame.to_vec();bus.ffta_dialogue.menu_pattern=true;
        let dir=std::env::temp_dir().join(format!("ffta-front-save-{}",std::process::id()));
        let auto=crate::save_states::AutoSave::new_at("test.sav",&"0".repeat(64),dir.clone());
        let mut painted=Vec::new();
        crate::save_states::paint(&bus,&frame,&mut painted,&auto);
        assert_eq!(painted.len(),w*h);
        assert!(painted[..w*80].iter().all(|&p|p==992),"save background must cover top margin");
        assert!(painted[w*240..].iter().all(|&p|p==992),"save background must cover bottom margin");
        // Defensive path also accepts the old 160-row input without panicking.
        crate::save_states::paint(&bus,&bus.wide_framebuffer[..w*160],&mut painted,&auto);
        assert_eq!(painted.len(),w*h);
        drop(auto);let _=std::fs::remove_dir_all(dir);
        mem::set_extra(0);mem::set_extra_y(0);
    }
}

#[cfg(test)]mod fractional_speed_tests {
    use super::*;
    #[test]fn one_and_half_executes_ninety_guest_frames_in_sixty_host_ticks(){
        let mut e=Engine::new(vec![0xfe,0xff,0xff,0xea]);
        e.machine.cpu.regs[15]=0x08000000;
        KEYS.store(0x3ff,Ordering::Relaxed);
        for _ in 0..60{e.frame_speed(1.5,0,0,0).unwrap();}
        assert_eq!(e.machine.bus.frames,90);
        e.frame_speed(2.,0,0,0).unwrap();assert_eq!(e.machine.bus.frames,92);
        e.frame_speed(4.,0,0,0).unwrap();assert_eq!(e.machine.bus.frames,96);
        assert!(e.frame_speed(8.,0,0,0).is_err());assert_eq!(e.machine.bus.frames,96);
    }
}

/// Invert the actual rendered canvas, including its native 240x160 inset.
fn save_canvas_point(window:(u32,u32),rect:[f32;4],canvas:(usize,usize),inset:(usize,usize),x:f32,y:f32)->Option<(f32,f32)> {
    if !x.is_finite() || !y.is_finite() || rect[2]<=0. || rect[3]<=0. {return None;}
    let px=x*window.0 as f32;let py=y*window.1 as f32;
    if px<rect[0] || px>=rect[0]+rect[2] || py<rect[1] || py>=rect[1]+rect[3] {return None;}
    Some(((px-rect[0])/rect[2]*canvas.0 as f32-inset.0 as f32,
        (py-rect[1])/rect[3]*canvas.1 as f32-inset.1 as f32))
}
#[cfg(test)]mod save_canvas_tests {
    use super::save_canvas_point;
    #[test]fn native_card_centers_stay_clickable_with_full_height_background(){
        for (window,canvas,inset,rect) in [((720,720),(240,240),(0,40),[0.,0.,720.,720.]),
            ((720,720),(256,256),(8,48),[0.,0.,720.,720.]),
            ((2280,1080),(338,160),(49,0),[0.,0.,2280.,1080.]),
            ((720,720),(240,160),(0,0),[0.,120.,720.,480.])] {
            for nx in [43.,120.,197.] {
                let ny=100.;let x=(rect[0]+(nx+inset.0 as f32)/canvas.0 as f32*rect[2])/window.0 as f32;
                let y=(rect[1]+(ny+inset.1 as f32)/canvas.1 as f32*rect[3])/window.1 as f32;
                let (a,b)=save_canvas_point(window,rect,canvas,inset,x,y).unwrap();
                assert!((a-nx).abs()<0.001 && (b-ny).abs()<0.001);
            }
        }
        assert!(save_canvas_point((720,720),[0.,120.,720.,480.],(240,160),(0,0),0.5,0.).is_none());
    }
}
