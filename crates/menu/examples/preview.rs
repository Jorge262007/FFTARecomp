//! Render native menu previews without starting a game or reading a ROM.
use input_config::{AvConfig, InputConfig, Language};
use menu::{Menu, MenuInput};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let output = args.get(1).ok_or("output PPM path required")?;
    let page = args.get(2).map(String::as_str).unwrap_or("main");
    let w = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(568usize);
    let h = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(320usize);
    let mut rgba = vec![[0, 0, 0, 255]; w * h];
    for y in 0..h {
        for x in 0..w {
            rgba[y * w + x] = [
                30 + (x * 45 / w) as u8,
                75 + (y * 75 / h) as u8,
                115 + (x * 60 / w) as u8,
                255,
            ];
        }
    }
    if let Some(path) = args.get(6).filter(|path| !path.is_empty()) {
        let data = std::fs::read(path)?;
        let mut parts = data.splitn(4, |&b| b == b'\n');
        assert_eq!(parts.next().unwrap(), b"P6");
        let dims = std::str::from_utf8(parts.next().unwrap())?
            .split_whitespace()
            .map(str::parse::<usize>)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(parts.next().unwrap(), b"255");
        let rgb = parts.next().unwrap();
        for y in 0..h {
            for x in 0..w {
                let i = ((y * dims[1] / h) * dims[0] + x * dims[0] / w) * 3;
                rgba[y * w + x][..3].copy_from_slice(&rgb[i..i + 3]);
            }
        }
    }
    let mut av = AvConfig::default();
    av.menu_language = Language::parse(args.get(7).map(String::as_str).unwrap_or("en"));
    let cfg = InputConfig::default();
    let mut menu = Menu::new(true, true);
    let section = match page {
        "saves" => 1,
        "fastforward" => 2,
        "bubbles" => 3,
        "audio" => 4,
        "video" | "dropdown" => 5,
        "keyboard" | "gamepad" | "mouse" => 6,
        "system" | "language" | "exit" => 7,
        _ => 0,
    };
    menu.atlas_section(menu::clan_atlas::Domain::Game, section);
    if page == "gamepad" || page == "mouse" {
        menu.handle(MenuInput::Up, &mut av);
        menu.handle(MenuInput::Right, &mut av);
        if page == "mouse" { menu.handle(MenuInput::Right, &mut av); }
        menu.handle(MenuInput::Confirm, &mut av);
    }
    if page == "exit" {
        for _ in 0..3 { menu.handle(MenuInput::Down, &mut av); }
        menu.handle(MenuInput::Confirm, &mut av);
    }
    for _ in 0..args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0) {
        menu.handle(MenuInput::Down, &mut av);
    }
    if page == "dropdown" || page == "language" {
        menu.handle(MenuInput::Confirm, &mut av);
    }
    menu.draw_inputs(&av, &cfg, &mut rgba, w, h);
    let mut bytes = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in rgba {
        bytes.extend_from_slice(&p[..3]);
    }
    std::fs::write(output, bytes)?;
    println!("MENU_PREVIEW {page} {w}x{h} {output}");
    Ok(())
}
