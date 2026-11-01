use gba_core::Machine;
fn main() {
    let rom = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mouse = gba_core::ffta_mouse::LanguageMouse::new(&rom, true);
    let mut m = Machine::new(rom);
    gba_core::mem::set_extra(164);
    gba_core::mem::set_extra_y(80);
    m.bus.ffta_dialogue.separate = true;
    let mut stale_seen = false;
    let mut bad_old = 0;
    let mut last = None;
    let mut selected = false;
    let mut begun = false;
    std::fs::create_dir_all("target/startup-probe").unwrap();
    for frame in 0..1050 {
        let scene = mouse.scene(&m.bus);
        let base = scene
            .as_ref()
            .and_then(|s| s.targets.first().map(|t| t.index));
        if base != last {
            println!("SCENE frame={frame} base={base:?}");
            last = base;
        }
        m.bus.keys = 0x3ff;
        if frame % 40 == 1 {
            if base == Some(0) {
                m.bus.keys = 0x3fe;
                selected = true;
            } else if !begun && base == Some(32) {
                m.bus.keys = 0x3fe;
                begun = true;
                println!("NEW_GAME frame={frame}");
            } else if selected && !begun {
                m.bus.keys = 0x3f7;
            }
        }
        m.run_frame(2_000_000);
        if std::env::var_os("FFTA_PROBE_CAPTURE").is_some()
            && begun
            && ((580..740).contains(&frame) || frame % 30 == 0)
        {
            let mut ppm = format!(
                "P6\n{} {}\n255\n",
                gba_core::mem::render_width(),
                gba_core::mem::render_height()
            )
            .into_bytes();
            for p in &m.bus.wide_framebuffer {
                for shift in [0, 5, 10] {
                    let x = ((p >> shift) & 31) as u8;
                    ppm.push((x << 3) | (x >> 2));
                }
            }
            std::fs::write(format!("target/startup-probe/{frame:04}.ppm"), ppm).unwrap();
        }
        let bad = m
            .bus
            .ffta_sprites
            .active
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|s| {
                s.replaces
                    && s.attrs.iter().enumerate().any(|(i, a)| {
                        m.bus.oam.get(s.slot * 8 + i * 2..s.slot * 8 + i * 2 + 2)
                            != Some(&a.to_le_bytes())
                    })
            })
            .count();
        stale_seen |= bad > 0;
        if bad != bad_old {
            println!(
                "STALE frame={frame} bad={bad} native={:02x?}",
                &m.bus.oam[..32]
            );
            bad_old = bad;
        }

        if frame % 60 == 0 {
            let host = m.bus.ffta_sprites.active.as_deref().unwrap_or(&[]);
            println!(
                "FRAME {frame} task={:08x} host={} wide_only={} pc={:08x}",
                u32::from_le_bytes(m.bus.iwram[0x2804..0x2808].try_into().unwrap()),
                host.len(),
                host.iter().filter(|s| !s.replaces).count(),
                m.cpu.regs[15]
            );
        }
    }
    assert!(begun, "replay must reach New Game");
    assert!(
        !stale_seen,
        "expanded list must never retain mismatching OAM objects"
    );
    println!("STARTUP_REGRESSION frames=1050 new_game=true stale_host_objects=0");
}
