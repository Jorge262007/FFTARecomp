fn centered_ui_panel(window:(u32,u32),base:f32,size:f32,b:[i32;4],top:bool)->crate::ui_layout::Layout {
    let mut l=crate::ui_layout::Layout::centered(window,base,size);
    let k=l.rect[2]/480.;let s=l.clip[3]/160.;
    l.rect[0]=(l.clip[0]+l.clip[2]*0.5-(120.+(b[0]+b[2])as f32*0.5)*k).round();
    l.rect[1]=if top {(l.clip[1]+6.*s-b[1]as f32*k).round()}
        else{(l.clip[1]+l.clip[3]*0.5-(b[1]+b[3])as f32*0.5*k).round()};
    l
}
pub(crate) fn host_ui_layout(window:(u32,u32),base:f32,style:gba_core::ffta_dialogue::DialogueStyle,node:gba_core::ffta_ui::Node)->crate::ui_layout::Layout {
    use gba_core::ffta_ui::Anchor;
    use crate::ui_layout::Layout;
    let length=if node.length {style.length}else{1.};
    let mut size=node.fitted_scale_with_length(style.scale,base,length);
    if matches!(node.role,gba_core::ffta_ui::Role::Stats|gba_core::ffta_ui::Role::ActionInfo|gba_core::ffta_ui::Role::Funds) {
        size=size.min(148./(node.bounds[3]-node.bounds[1]).max(1)as f32).max(0.5);
    }
    let mut layout=match node.anchor {
        Anchor::Edge{right,vertical}=>Layout::new_with_anchor(window,base,right,size,
            if node.role==gba_core::ffta_ui::Role::Dialogue {[style.offset_x,style.offset_y]}else{[0.,0.]},vertical),
        Anchor::Corner{right,bottom}=>Layout::at_corner(window,base,right,bottom,size,length,node.bounds),
        Anchor::Center=>Layout::centered(window,base,size),
        Anchor::Modal|Anchor::PanelCenter=>centered_ui_panel(window,base,size,node.bounds,false),
        Anchor::TopCenter=>centered_ui_panel(window,base,size,node.bounds,true),
    };
    if matches!(node.role,gba_core::ffta_ui::Role::Stats|gba_core::ffta_ui::Role::ActionInfo|gba_core::ffta_ui::Role::Funds) {
        let margin=6.*layout.clip[3]/160.;
        let bottom=matches!(node.anchor,Anchor::Corner{bottom:true,..}|Anchor::Edge{vertical:160.,..});
        layout.rect[1]+=if bottom{-margin}else{margin};
    }
    layout
}
