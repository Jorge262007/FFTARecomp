//! Render native menu previews without starting a game or reading a ROM.
use input_config::{AvConfig, InputConfig};
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
    if let Some(path) = args.get(6) {
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
    let cfg = InputConfig::default();
    let mut menu = Menu::new(true, true);
    let index = match page {
        "fastforward" => 1,
        "bubbles" => 2,
        "audio" => 3,
        "video" | "dropdown" => 4,
        "keyboard" | "gamepad" | "mouse" => 5,
        "exit" => 7,
        _ => 0,
    };
    if page == "debug" || page=="debug-missions" {
        menu.open_debug();

        if page=="debug-missions" {
            for _ in 0..3 {menu.handle(MenuInput::Down,&mut av);}
            menu.handle(MenuInput::Confirm,&mut av);
            for _ in 0..40 {menu.scroll(false,&mut av);}
        }
    } else if page != "main" {
        for _ in 0..index {
            menu.handle(MenuInput::Down, &mut av);
        }
        menu.handle(MenuInput::Confirm, &mut av);
    }
    if page == "gamepad" || page=="mouse" {
        menu.handle(MenuInput::Right, &mut av);
    }
    if page=="mouse" {menu.handle(MenuInput::Right,&mut av);}
    for _ in 0..args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0) {
        menu.handle(MenuInput::Down, &mut av);
    }
    if page == "dropdown" {
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
