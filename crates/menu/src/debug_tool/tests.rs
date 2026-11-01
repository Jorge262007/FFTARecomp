use super::*;
use crate::{ToolField, ToolFieldKind};
fn fields() -> Vec<ToolField> {
    vec![
        ToolField {
            key: "value".into(),
            label: "Value".into(),
            value: "12".into(),
            help: "A typed value".into(),
            kind: ToolFieldKind::Text,
        },
        ToolField {
            key: "choice".into(),
            label: "Choice".into(),
            value: "off".into(),
            help: String::new(),
            kind: ToolFieldKind::Choice(vec!["off".into(), "on".into()]),
        },
        ToolField {
            key: "input".into(),
            label: "Input".into(),
            value: "none".into(),
            help: String::new(),
            kind: ToolFieldKind::Input(ContentSlot::Recording),
        },
    ]
}
#[test]
fn editing_cancel_commit_choices_and_inputs_never_start_a_tool() {
    let mut f = ToolForm::default();
    f.set(DebugTool::Step, fields(), String::new());
    assert!(f.handle(MenuInput::Confirm).is_none());
    f.text(" è");
    f.finish(true);
    assert_eq!(f.fields[0].value, "12");
    assert_eq!(f.revision(), 0);
    f.edit();
    f.backspace();
    f.text("4");
    f.finish(false);
    assert_eq!(f.fields[0].value, "14");
    assert_eq!(f.revision(), 1);
    f.handle(MenuInput::Down);
    f.handle(MenuInput::Confirm);
    f.handle(MenuInput::Right);
    assert_eq!(f.fields[1].value, "off");
    f.handle(MenuInput::Confirm);
    assert_eq!(f.fields[1].value, "on");
    f.handle(MenuInput::Down);
    assert_eq!(
        f.handle(MenuInput::Confirm),
        Some(MenuAction::ChooseToolInput(ContentSlot::Recording))
    );
    f.handle(MenuInput::Down);
    assert_eq!(
        f.handle(MenuInput::Confirm),
        Some(MenuAction::ToolRequest(ToolRequest::Validate))
    );
    assert_eq!(f.values().len(), 2);
}
#[test]
fn held_busy_requests_refuse_but_back_and_edit_cancellation_remain_available() {
    let mut f = ToolForm::default();
    f.set(DebugTool::Replay, fields(), String::new());
    f.focus = f.fields.len() + 3;
    f.busy = true;
    assert!(f.handle(MenuInput::Confirm).is_none());
    assert!(f.message.contains("finishing"));
    assert_eq!(
        f.handle(MenuInput::Cancel),
        Some(MenuAction::ToolRequest(ToolRequest::Back))
    );
    f.focus = 0;
    f.edit();
    f.text(&"α".repeat(3000));
    assert!(f.editor.edited_value().chars().count() <= 2048);
    f.handle(MenuInput::Cancel);
    assert!(!f.text_focus());
    assert_eq!(f.fields[0].value, "12");
}
#[test]
fn mouse_text_keyboard_and_measured_layout_cover_small_and_large_clients() {
    let mut f = ToolForm::default();
    f.set(DebugTool::Issue, fields(), "long message ".repeat(100));
    f.summary = "identity/path ".repeat(200);
    for (w, h) in [(400, 300), (620, 480), (940, 620), (1600, 1000)] {
        let mut pixels = vec![[0, 0, 0, 255]; w * h];
        draw(&f, &mut pixels, w, h, input_config::Language::default());
        let mineral=pixels.iter().filter(|p|p[2]>p[0]&&p[1]>p[0]).count();
        assert!(mineral>w*h/4,"form must copy its visible canvas to the frontend");
        let focus = f.focus(w, h);
        assert!(focus.selected_bounds.is_some());
    }
    let g = geometry(940, 620);
    assert!(f
        .click(g.list[0] + 5, g.list[1] + 5, 940, 620, true)
        .is_none());
    assert!(f.text_focus());
    f.handle(MenuInput::Up);
    for _ in 0..3 {
        f.handle(MenuInput::Right);
    }
    f.handle(MenuInput::Confirm);
    f.text("Résumé");
    f.handle(MenuInput::Right);
    f.handle(MenuInput::Confirm);
    assert_eq!(f.fields[0].value, "Résumé");
    assert!(!f.text_focus());
}

#[test]
fn library_has_explicit_cancel_while_busy_and_distinct_guest_inspector() {
    let mut f=ToolForm::default(); f.set(DebugTool::Homepage,fields(),String::new());
    f.busy=true; f.focus=f.fields.len()+3;
    assert_eq!(f.handle(MenuInput::Confirm),Some(MenuAction::ToolRequest(ToolRequest::Prepare)));
    f.focus=f.fields.len()+4; assert!(f.handle(MenuInput::Confirm).is_none());
    assert_eq!(request_label(ToolRequest::Prepare,DebugTool::Homepage),"Cancel process");
    assert_eq!(request_label(ToolRequest::Prepare,DebugTool::GuestScripts),"Gameplay inspector");
}
