//! Bounded, opt-in local snapshots for short-lived rendering/input failures.
//! Enabled only by FFTA_VISUAL_TRACE=1; never uploads data or writes a save.
use std::{path::PathBuf,sync::{Mutex,OnceLock,atomic::{AtomicBool,Ordering}}};
static ACTIVE:AtomicBool=AtomicBool::new(false);
static INITIALIZED:OnceLock<()>=OnceLock::new();
fn enabled()->bool{INITIALIZED.get_or_init(||ACTIVE.store(std::env::var("FFTA_VISUAL_TRACE").as_deref()==Ok("1"),Ordering::Relaxed));ACTIVE.load(Ordering::Relaxed)}
pub fn toggle(){let old=enabled();ACTIVE.store(!old,Ordering::Relaxed);eprintln!("FFTA_VISUAL_TRACE recording={} (F9 toggles; local visual-trace folder)",!old);}
struct Recorder {dir:PathBuf,frames:usize,clicks:usize,burst_until:u64,burst_frames:u64,failed:bool}
pub fn capture(mem:&gba_core::MemMap,scene:&gba_core::ffta_ui::Scene,mouse:Option<(f64,f64)>,view:[f32;4],click:bool){
    if !enabled(){return;}

    static STATE:OnceLock<Mutex<Recorder>>=OnceLock::new();
    let mut recorder=STATE.get_or_init(||{
        let dir=std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("visual-trace")))
            .unwrap_or_else(||PathBuf::from("target/visual-trace"));
        let burst_frames=std::env::var("FFTA_VISUAL_TRACE_BURST_FRAMES").ok()
            .and_then(|v|v.parse::<u64>().ok()).unwrap_or(120).clamp(120,1200);
        Mutex::new(Recorder{dir,frames:0,clicks:0,burst_until:0,burst_frames,failed:false})
    }).lock().unwrap();
    if recorder.failed{return;}
    if click {recorder.burst_until=mem.frames as u64+recorder.burst_frames;}
    if !click && mem.frames as u64>recorder.burst_until && mem.frames%6!=0{return;}
    let slot=if click{let n=240+recorder.clicks%16;recorder.clicks+=1;n}
        else{let n=recorder.frames%240;recorder.frames+=1;n};
    let metadata=format!("frame={} click={click} mouse={mouse:?} view={view:?}\nscene={scene:?}\nstyle={:?}\nstage_bases={:?} stage_origins={:?} stage_valid={:?}\nmovement_origins={:?} movement_valid={:?}\nhost_sprites={:?}\n",
        mem.frames,mem.ffta_dialogue.style,mem.ffta_stage.bases,mem.ffta_stage.origins,mem.ffta_stage.valid,
        mem.ffta_movement.origins,mem.ffta_movement.valid,mem.ffta_sprites.active);
    let mut data=Vec::with_capacity(600_000);data.extend_from_slice(b"FFTATR01");
    data.extend_from_slice(&(metadata.len()as u32).to_le_bytes());data.extend_from_slice(metadata.as_bytes());
    for bytes in [&mem.ewram,&mem.iwram,&mem.io,&mem.palette,&mem.vram,&mem.oam]{data.extend_from_slice(bytes);}
    for pixel in &mem.framebuffer{data.extend_from_slice(&pixel.to_le_bytes());}
    let result=std::fs::create_dir_all(&recorder.dir).and_then(|_|std::fs::write(recorder.dir.join(format!("sample-{slot:02}.trace")),data));
    if let Err(e)=result{recorder.failed=true;eprintln!("FFTA_VISUAL_TRACE disabled: {e}");}
    else if click || recorder.frames==1{eprintln!("FFTA_VISUAL_TRACE frame={} click={click} path={}",mem.frames,recorder.dir.display());}
}

