use super::*;
use crate::{
    clan_atlas::Domain, diagnostics, AvConfig, Changed, Device, InputConfig, Menu, MenuAction,
    MenuInput, Page, Row,
};
use std::collections::BTreeSet;

fn texts(menu: &Menu, av: &AvConfig, w: usize, h: usize) -> Vec<diagnostics::TextRun> {
    let mut px = vec![[30, 70, 100, 255]; w * h];
    diagnostics::capture(|| menu.draw_inputs(av, &InputConfig::default(), &mut px, w, h))
        .unwrap()
        .runs
}

#[test]
fn catalog_keys_are_unique_and_translated_placeholders_match() {
    for language in Language::ALL {
        let mut keys = BTreeSet::new();
        for &(key, value) in language.messages() {
            assert!(keys.insert(key), "Duplicate catalog key: {key}");
            assert!(!value.is_empty(), "Empty translation: {key}");
        }
    }
    assert_eq!(
        text(Language::default(), "unknown future key"),
        "unknown future key"
    );
    assert_eq!(
        text(Language::parse("it"), "unknown future key"),
        "unknown future key"
    );
    assert_eq!(
        Row::Key(crate::Button::Select).localized_label(Language::parse("it")),
        "Select"
    );
    let args = [("stamp", "{stamp} <private> àèìòù".into())];
    assert_eq!(
        format(
            Language::parse("it"),
            "Last .sav modification: {stamp}",
            &args
        ),
        "Ultima modifica file .sav: {stamp} <private> àèìòù"
    );
}

#[test]
fn language_requires_confirmation_cancel_preserves_preference_and_commit_updates_immediately() {
    let mut menu = Menu::new(true, true);
    let mut av = AvConfig::default();
    menu.atlas_section(Domain::Game, 7);
    assert_eq!(menu.rows()[menu.selected], Row::Language);
    menu.handle(MenuInput::Confirm, &mut av);
    for _ in 0..Language::ALL
        .iter()
        .position(|l| l.token() == "it")
        .unwrap()
    {
        menu.handle(MenuInput::Down, &mut av);
    }
    assert_eq!(av.menu_language, Language::default());
    menu.handle(MenuInput::Cancel, &mut av);
    assert_eq!(av.menu_language, Language::default());
    menu.handle(MenuInput::Confirm, &mut av);
    for _ in 0..Language::ALL
        .iter()
        .position(|l| l.token() == "it")
        .unwrap()
    {
        menu.handle(MenuInput::Down, &mut av);
    }
    assert_eq!(
        menu.handle(MenuInput::Confirm, &mut av),
        Some(MenuAction::Changed(Changed::Overlay))
    );
    assert_eq!(av.menu_language, Language::parse("it"));
    assert_eq!(menu.language(), Language::parse("it"));
    assert_eq!(menu.tr("Panoramica"), "Panoramica");
    let loaded = AvConfig::parse(&av.serialize());
    let reopened = Menu::new(true, true);
    let runs = texts(&reopened, &loaded, 854, 480);
    assert!(runs.iter().any(|r| r.text == "Torna al gioco"));
    assert_eq!(reopened.language(), Language::parse("it"));
    menu.handle(MenuInput::Confirm, &mut av);
    for _ in 0..Language::ALL
        .iter()
        .position(|l| l.token() == "it")
        .unwrap()
    {
        menu.handle(MenuInput::Up, &mut av);
    }
    assert_eq!(
        menu.handle(MenuInput::Confirm, &mut av),
        Some(MenuAction::Changed(Changed::Overlay))
    );
    assert_eq!(menu.language(), Language::default());
    assert_eq!(menu.tr("Panoramica"), "Overview");
}

#[test]
fn mouse_language_choice_and_translated_video_options_keep_canonical_values() {
    let mut menu = Menu::new(true, true);
    let mut av = AvConfig::default();
    menu.atlas_section(Domain::Game, 7);
    let r = crate::clan_atlas::row_rect(&menu, 0, 854, 480).unwrap();
    menu.click(r.x + 3, r.y + 3, 854, 480, true, &mut av);
    let target = Language::ALL
        .iter()
        .position(|l| l.token() == "it")
        .unwrap();
    for _ in 0..target {
        menu.handle(MenuInput::Down, &mut av);
    }
    let dd = menu.dropdown.as_ref().unwrap();
    let (x, y, _, _, _, first) = crate::theme::dropdown_geometry(dd, 854, 480);
    let pitch = (crate::clan_atlas::geometry(854, 480).body.h / 15).clamp(14, 40);
    assert_eq!(
        menu.click(
            x + 10,
            y + 6 + pitch * (target - first) + 2,
            854,
            480,
            true,
            &mut av
        ),
        Some(MenuAction::Changed(Changed::Overlay))
    );
    assert_eq!(av.menu_language, Language::parse("it"));
    menu.atlas_section(Domain::Game, 5);
    menu.selected = menu.rows().iter().position(|r| *r == Row::Fps).unwrap();
    menu.handle(MenuInput::Confirm, &mut av);
    menu.handle(MenuInput::Down, &mut av);
    assert!(texts(&menu, &av, 854, 480)
        .iter()
        .any(|r| r.text == "INTERNO"));
    menu.handle(MenuInput::Confirm, &mut av);
    assert_eq!(av.video_fps, "internal");
}

#[test]
fn both_languages_render_navigation_saves_and_hints_at_supported_sizes() {
    for language in [Language::default(), Language::parse("it")] {
        let av = AvConfig {
            menu_language: language,
            ..AvConfig::default()
        };
        for (w, h) in [(480, 640), (854, 480), (1280, 720)] {
            let mut menu = Menu::new(true, true);
            menu.set_standard_save_timestamp(0);
            let runs = texts(&menu, &av, w, h);
            for key in ["Panoramica", "Salvataggi", "Sistema", "Torna al gioco"] {
                assert!(
                    runs.iter().any(|r| r.text.contains(text(language, key))),
                    "Missing {key:?} at {w}x{h}"
                );
            }
            let expected = if language == Language::default() {
                "Last .sav modification: 1970-01-01 00:00 UTC"
            } else {
                "Ultima modifica file .sav: 1970-01-01 00:00 UTC"
            };
            assert!(runs.iter().any(|r| r.text == expected));
            menu.atlas_section(Domain::Game, 7);
            for key in ["Language", "Restart ROM", "Exit"] {
                menu.selected = menu.rows().iter().position(|r| r.label() == key).unwrap();
                let runs = texts(&menu, &av, w, h);
                let run = runs
                    .iter()
                    .find(|r| r.text == text(language, key))
                    .unwrap_or_else(|| {
                        panic!(
                            "Missing {key} for {language:?} at {w}x{h}: {:?}",
                            runs.iter().map(|r| &r.text).collect::<Vec<_>>()
                        )
                    });
                assert!(
                    !run.width_stopped && !run.canvas_clipped,
                    "Clipped {key} at {w}x{h}"
                );
            }
            menu.open(Page::SaveStates);
            assert!(texts(&menu, &av, w, h)
                .iter()
                .any(|r| r.text == text(language, "Load state")));
            menu.atlas_section(Domain::Game, 0);
        }
        let menu = Menu::new(true, true);
        menu.note_input(Device::Gamepad);
        let _ = texts(&menu, &av, 854, 480);
        assert!(
            menu.control_hints()[0].contains(if language == Language::default() {
                "back"
            } else {
                "indietro"
            })
        );
    }
}

#[test]
fn independent_menus_and_retained_undo_details_use_the_current_language() {
    let mut italian = Menu::new(true, true);
    italian.open(Page::SaveStates);
    italian.set_state_undo_description(Some(("Undo load (slot {slot})", 9)));
    let italian_config = AvConfig {
        menu_language: Language::parse("it"),
        ..AvConfig::default()
    };
    assert!(texts(&italian, &italian_config, 854, 480)
        .iter()
        .any(|r| r.text == "Annulla caricamento (slot 9)"));
    let english = Menu::new(true, true);
    assert!(texts(&english, &AvConfig::default(), 854, 480)
        .iter()
        .any(|r| r.text == "Return to game"));
    assert_eq!(italian.language(), Language::parse("it"));
    assert_eq!(english.language(), Language::default());
    assert!(texts(&italian, &AvConfig::default(), 854, 480)
        .iter()
        .any(|r| r.text == "Undo load (slot 9)"));
    assert_eq!(
        italian.state_undo_description,
        Some(("Undo load (slot {slot})", 9))
    );
}

#[test]
fn every_discovered_language_is_listed_selectable_and_persisted_by_code() {
    let mut menu = Menu::new(true, true);
    let mut av = AvConfig::default();
    menu.atlas_section(Domain::Game, 7);
    assert_eq!(
        menu.row_choices(Row::Language),
        Language::ALL
            .iter()
            .map(|l| l.name().to_string())
            .collect::<Vec<_>>()
    );
    for (index, language) in Language::ALL.into_iter().enumerate() {
        menu.handle(MenuInput::Confirm, &mut av);
        for _ in 0..Language::ALL.len() {
            if menu.dropdown.as_ref().unwrap().index == index {
                break;
            }
            menu.handle(MenuInput::Down, &mut av);
        }
        menu.handle(MenuInput::Confirm, &mut av);
        assert_eq!(av.menu_language, language);
        assert_eq!(
            AvConfig::parse(&av.serialize()).menu_language.token(),
            language.token()
        );
    }
}
