use super::*;

fn rendered(menu: &Menu, cfg: &InputConfig) -> String {
    let mut pixels = vec![[80, 100, 120, 255]; 854 * 480];
    diagnostics::capture(|| menu.draw_inputs(&AvConfig::default(), cfg, &mut pixels, 854, 480))
        .unwrap()
        .runs
        .into_iter()
        .map(|r| r.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn hints_follow_last_input_while_profile_bindings_capture_and_navigation_stay_intact() {
    let mut menu = Menu::new(true, true);
    let cfg = InputConfig {
        device: Device::Gamepad,
        menu_key: "H".into(),
        ..Default::default()
    };
    let before = cfg.serialize();
    assert!(rendered(&menu, &cfg).contains("D-pad"));
    assert!(menu.note_input(Device::Keyboard));
    let text = rendered(&menu, &cfg);
    assert!(
        text.contains("Enter OK") && text.contains("Tab focus") && text.contains("F1 details")
    );
    assert!(!text.contains("VOL LB") && !text.contains("D-pad"));
    assert_eq!(*menu.menu_binding.borrow(), "H");
    menu.capture = Some(Bind::GamepadMenu);
    let navigation = (menu.page, menu.selected, menu.atlas.sidebar_focus);
    assert!(menu.note_input(Device::Gamepad));
    assert!(!menu.note_input(Device::Gamepad));
    let text = rendered(&menu, &cfg);
    assert!(text.contains("A OK") && text.contains("B back"));
    assert_eq!(menu.capturing(), Some(Bind::GamepadMenu));
    assert_eq!(
        navigation,
        (menu.page, menu.selected, menu.atlas.sidebar_focus)
    );
    assert_eq!(cfg.serialize(), before);
    menu.reset();
    assert!(menu.hints_gamepad());
    menu.note_input(Device::MouseKeyboard);
    assert!(rendered(&menu, &cfg).contains("Enter OK"));
    assert_eq!(cfg.device, Device::Gamepad);
}

#[test]
fn left_marks_sidebar_cursor_distinctly_even_at_full_transparency() {
    let mut menu = Menu::new(true, true);
    let cfg = InputConfig::default();
    let mut av = AvConfig {
        menu_transparency: 1.,
        ..Default::default()
    };
    let mut normal = vec![[80, 100, 120, 255]; 854 * 480];
    let mut focus = normal.clone();
    menu.draw_inputs(&av, &cfg, &mut normal, 854, 480);
    assert_eq!(menu.handle(MenuInput::Left, &mut av), None);
    assert!(menu.atlas.sidebar_focus);
    menu.draw_inputs(&av, &cfg, &mut focus, 854, 480);
    let rect = clan_atlas::geometry(854, 480).nav[0];
    let i = (rect.y + rect.h / 2) * 854 + rect.x + rect.w - 8;
    assert_ne!(normal[i], focus[i]);
    assert_eq!(focus[i], [209, 175, 117, 255]);
    menu.handle(MenuInput::Down, &mut av);
    let text = rendered(&menu, &cfg);
    assert!(text.contains("› 02  Saves"));
    menu.handle(MenuInput::Right, &mut av);
    assert_eq!(menu.page, Page::Saves);
    assert!(!menu.atlas.sidebar_focus);
    assert_eq!(av.menu_transparency, 1.);
}
