use super::*;

#[test]
fn cheat_view_keeps_all_rows_and_never_dispatches_selection(){
    let mut s=Selector::default();s.cheat_mode=true;
    s.set("Cheat".into(),(0..600).map(|i|ContentEntry{id:format!("CH-{i:04}"),name:format!("CH-{i:04}"),available:true,..Default::default()}).collect(),String::new(),0);
    assert_eq!(s.entries.len(),600);s.cursor=599;s.focus=6;assert!(s.handle(MenuInput::Confirm).is_none());assert!(s.detail_mode);assert!(s.confirmed.is_none());
    s.entries[s.visible[599]].reason="ROM sconosciuta; decoder non supportato".into();s.entries[s.visible[599]].description="SYNTHETIC CODE".into();let detail=s.cheat_details();assert!(detail.find("decoder non supportato").unwrap()<detail.find("SYNTHETIC CODE").unwrap());
    let g=geometry(627,218);let hidden=g.actions[4];assert!(s.click(hidden[0]+2,hidden[1]+2,627,218,true).is_none());assert!(s.detail_mode);
    assert_eq!(s.focus(627,218).selected_bounds,Some(g.panel));
    s.handle(MenuInput::Cancel);s.focus=10;assert_eq!(s.handle(MenuInput::Confirm),Some(MenuAction::CheatChoose));s.focus=9;assert_eq!(s.handle(MenuInput::Confirm),Some(MenuAction::CheatReload));
    s.set("Cheat".into(),Vec::new(),String::new(),0);s.focus=6;assert!(s.handle(MenuInput::Confirm).is_none());assert!(s.message.contains("Importa"));
}

#[test]
fn semantic_observation_never_adjusts_geometry_scroll_or_confirms_content(){
    let mut selector=Selector::default();
    selector.set("Synthetic".into(),(0..40).map(|i|ContentEntry{id:format!("id-{i}"),name:format!("row-{i}"),..Default::default()}).collect(),String::new(),1);
    selector.first.set(17);selector.detail_scroll.set(23);
    let before=(selector.first.get(),selector.detail_scroll.get(),selector.cursor,selector.confirmed.clone());
    for _ in 0..20{let _=selector.selection_state();}
    assert_eq!(before,(selector.first.get(),selector.detail_scroll.get(),selector.cursor,selector.confirmed.clone()));
}
fn entries() -> Vec<ContentEntry> {
    (0..30)
        .map(|n| ContentEntry {
            id: format!("content-{n:064}"),
            name: if n < 2 {
                "Same name".into()
            } else {
                format!("Item {n:02}")
            },
            kind: if n % 2 == 0 {
                "replay-profile"
            } else {
                "snapshot"
            }
            .into(),
            location: format!("portable/Data/config/{n}/replay.json"),
            identity: "a".repeat(64),
            description:
                "A long description with Unicode è, Ω and text that must remain readable. "
                    .repeat(8),
            reason: "Listing only; no execution".into(),
            status: "available".into(),
            compatibility: "unknown".into(),
            available: true,
        })
        .collect()
}
fn selector() -> Selector {
    let mut s = Selector::default();
    s.set("Test".into(), entries(), "Refresh explicitly".into(), 7);
    s
}
#[test]
fn same_names_keep_identity_and_explicit_selection_is_distinct_from_highlight() {
    let mut s = selector();
    let first = s.entries.iter().position(|e| e.id.ends_with('0')).unwrap();
    let second = 1;
    s.cursor = s.visible.iter().position(|i| *i == first).unwrap();
    s.focus = 5;
    assert!(s.handle(MenuInput::Confirm).is_none());
    assert_eq!(s.focus, 6);
    assert_eq!(
        s.handle(MenuInput::Confirm),
        Some(MenuAction::ContentSelect {
            index: first,
            generation: 7
        })
    );
    s.cursor = s.visible.iter().position(|i| *i == second).unwrap();
    assert_eq!(
        s.handle(MenuInput::Confirm),
        Some(MenuAction::ContentSelect {
            index: second,
            generation: 7
        })
    );
    assert_ne!(s.entries[first].id, s.entries[second].id);
}
#[test]
fn unicode_search_cancel_filter_sort_favorites_and_recent_do_not_execute() {
    let mut s = selector();
    s.start_search();
    s.text("Same name");
    assert_eq!(s.visible.len(), 2);
    s.finish_search(true);
    assert_eq!(s.visible.len(), 30);
    s.start_search();
    s.text("è");
    assert_eq!(s.visible.len(), 30);
    s.text_backspace();
    s.finish_search(false);
    s.focus = 1;
    assert!(s.handle(MenuInput::Confirm).is_none());
    assert!(s.handle(MenuInput::Right).is_none());
    assert_eq!(s.visible.len(), 30);
    assert!(s.handle(MenuInput::Cancel).is_none());
    assert_eq!(s.visible.len(), 30);
    s.handle(MenuInput::Confirm);
    s.handle(MenuInput::Right);
    s.handle(MenuInput::Confirm);
    assert_eq!(s.visible.len(), 15);
    s.focus = 7;
    s.confirm();
    assert_eq!(s.favorites.len(), 1);
    s.collection = 1;
    s.rebuild(None);
    assert_eq!(s.visible.len(), 1);
    let id = s.highlighted_id().unwrap().to_owned();
    s.mark_selected(&id);
    assert_eq!(s.recent.len(), 1);
    s.collection = 2;
    s.rebuild(None);
    assert_eq!(s.visible.len(), 1);
    s.sort = 2;
    s.rebuild(None);
    assert_eq!(s.highlighted_id(), Some(id.as_str()));
}
#[test]
fn empty_unavailable_and_busy_never_open_external_dialogs_implicitly() {
    let mut s = Selector::default();
    s.set("Empty".into(), Vec::new(), String::new(), 1);
    s.focus = 6;
    assert!(s.confirm().is_none());
    assert!(s.message.contains("No matching"));
    s.focus = 9;
    assert_eq!(s.confirm(), Some(MenuAction::ContentRefresh));
    s.focus = 10;
    assert_eq!(s.confirm(), Some(MenuAction::ContentBrowseFile));
    s.focus = 11;
    assert_eq!(s.confirm(), Some(MenuAction::ContentAddFolder));
    s.busy = true;
    assert!(s.confirm().is_none());
    s.focus = 12;
    assert_eq!(s.confirm(), Some(MenuAction::ContentCancel));
    let mut s = selector();
    for e in &mut s.entries {
        e.available = false;
    }
    s.focus = 6;
    assert!(s.confirm().is_none());
}
#[test]
fn controller_keyboard_supports_edit_backspace_done_and_cancel() {
    let mut s = selector();
    s.focus = 0;
    s.confirm();
    s.key = 0;
    s.handle(MenuInput::Confirm);
    assert_eq!(s.query, "a");
    s.key = 37;
    s.handle(MenuInput::Confirm);
    assert_eq!(s.query, "");
    s.text("Unicode è");
    s.key = 39;
    s.handle(MenuInput::Confirm);
    assert!(!s.editing);
    assert_eq!(s.query, "Unicode è");
    s.start_search();
    s.text(" draft");
    s.handle(MenuInput::Cancel);
    assert_eq!(s.query, "Unicode è");
    assert!(!s.editing);
}
#[test]
fn mouse_hit_testing_scroll_details_and_full_ids_are_accessible() {
    let mut s = selector();
    let g = geometry(940, 620);
    s.hover(g.list[0] + 5, g.list[1] + g.row + 4, 940, 620);
    assert_eq!(s.cursor, 1);
    assert!(s
        .click(g.list[0] + 5, g.list[1] + g.row + 4, 940, 620, true)
        .is_none());
    assert_eq!(s.focus, 5);
    for _ in 0..12 {
        s.wheel(false);
    }
    let f = s.focus(940, 620);
    assert!(f.first_visible > 0);
    assert!(f.label.contains("content-"));
    s.focus = 8;
    assert!(matches!(
        s.confirm(),
        Some(MenuAction::ContentPreview { .. })
    ));
    s.handle(MenuInput::Down);
    assert_eq!(s.detail_scroll.get(), 1);
    s.handle(MenuInput::Cancel);
    assert!(!s.detail_mode);
    assert_eq!(s.focus, 5);
}
#[test]
fn shared_text_wrap_preserves_every_character_and_render_stays_in_canvas() {
    let text = "Very long path / 同名 / è / ".repeat(30);
    let lines = Canvas::wrap_text(&text, 13., 80.);
    assert_eq!(lines.concat(), text);
    let mut s = selector();
    s.entries[0].reason = text;
    s.focus = 5;
    for (w, h) in [(1280, 720), (940, 620), (620, 480), (400, 300)] {
        let mut px = vec![[15, 25, 35, 255]; w * h];
        draw(&s, &mut px, w, h, input_config::Language::default());
        assert_eq!(px.len(), w * h);
        let g = geometry(w, h);
        assert!(g.panel[0] + g.panel[2] <= w && g.panel[1] + g.panel[3] <= h);
        let text = diagnostics::capture(|| draw(&s, &mut px, w, h, input_config::Language::default())).unwrap();
        assert!(!text.runs.is_empty());
        assert!(text.runs.iter().all(|r| !r.canvas_clipped));
    }
}

#[test]
fn required_role_survives_category_changes_and_can_be_cleared_for_generic_browse() {
    let mut s = Selector::default();
    let mut a = ContentEntry::default();
    a.id = "a".into();
    a.name = "Same".into();
    a.kind = "snapshot".into();
    a.available = true;
    let mut b = a.clone();
    b.id = "b".into();
    b.kind = "runtime".into();
    s.set("role".into(), vec![a, b], String::new(), 1);
    s.scope(vec!["snapshot".into()], None);
    assert_eq!(s.visible, vec![0]);
    s.focus = 1;
    s.handle(MenuInput::Confirm);
    s.handle(MenuInput::Right);
    s.handle(MenuInput::Confirm);
    assert!(!s.visible.contains(&1));
    s.handle(MenuInput::Confirm);
    assert!(s.choice_focus());
    s.set("Replaced".into(),Vec::new(),String::new(),2);
    assert!(!s.choice_focus());
    s.set("role".into(),vec![ContentEntry{id:"a".into(),kind:"snapshot".into(),..Default::default()},ContentEntry{id:"b".into(),kind:"runtime".into(),..Default::default()}],String::new(),3);
    s.scope(Vec::new(), None);
    assert_eq!(s.visible.len(), 2);
}

#[test]
fn controller_punctuation_page_can_enter_multi_value_parameter_separators() {
    let mut s = Selector::default();
    s.start_editor("streams".into(), String::new(), 2048);
    for _ in 0..3 {
        s.key = 41;
        s.key_confirm();
    }
    assert_eq!(s.key_labels()[0], ",");
    s.key = 0;
    s.key_confirm();
    assert_eq!(s.edited_value(), ",");
    s.key = 39;
    s.key_confirm();
    assert!(!s.editing());
}

#[test]
fn preference_restore_keeps_favorites_recent_and_highlight_without_confirming_or_emitting() {
    let mut s = selector();
    let id = s.entries[3].id.clone();
    let view = ContentViewPreferences {
        query: "Item".into(),
        kind: "snapshot".into(),
        filter: 0,
        sort: 2,
        collection: 1,
        highlighted: Some(id.clone()),
    };
    let history = ContentHistory {
        favorites: vec![id.clone()],
        recent: vec![id.clone()],
    };
    s.restore_preferences(view.clone(), history.clone())
        .unwrap();
    assert_eq!(s.highlighted_id(), Some(id.as_str()));
    assert!(s.confirmed.is_none());
    assert!(!s.busy);
    assert_eq!(s.history(), history);
    assert_eq!(s.preferences(), view);
    let before = s.preferences();
    let mut invalid = before.clone();
    invalid.sort = 99;
    assert!(s
        .restore_preferences(invalid, ContentHistory::default())
        .is_err());
    assert_eq!(s.preferences(), before);
    assert_eq!(s.history(), history);
}

#[test]
fn preference_export_keeps_committed_search_while_an_editor_draft_is_open() {
    let mut s = selector();
    s.query = "Item".into();
    s.rebuild(None);
    let before = s.preferences();
    s.start_search();
    s.text(" draft");
    assert_eq!(s.preferences(), before);
    s.finish_search(true);
    assert_eq!(s.preferences(), before);
    s.start_search();
    s.text(" 03");
    s.finish_search(false);
    assert_eq!(s.preferences().query, "Item 03");
    assert!(s.confirmed.is_none());
}

#[test]
fn virtual_keyboard_keys_remain_inside_small_atlas_bodies() {
    let s=selector();
    for(w,h)in[(352,212),(352,480),(627,218),(950,330)]{
        let g=geometry(w,h);
        for i in 0..42{let r=s.key_rect(g,i);assert!(r[0]+r[2]<=w&&r[1]+r[3]<=h,"{w}x{h} key{i} {r:?}");}
    }
}
