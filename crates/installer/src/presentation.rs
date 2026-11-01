use eframe::egui::{self,Color32,FontFamily,FontId,Stroke};

pub const VERSION:&str="v3.1";
pub const BG:Color32=Color32::from_rgb(18,32,26);
pub const PAPER:Color32=Color32::from_rgb(245,235,209);
pub const CARD:Color32=Color32::from_rgb(255,249,232);
pub const INK:Color32=Color32::from_rgb(35,24,16);
pub const MUTED:Color32=Color32::from_rgb(112,92,58);
pub const GOLD:Color32=Color32::from_rgb(184,146,65);
pub const SELECTED:Color32=Color32::from_rgb(49,34,19);
pub const HIGHLIGHT:Color32=Color32::from_rgb(255,211,93);

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Launch {pub steam_deck:bool,pub fullscreen:bool}
impl Launch {
    pub fn parse(args:impl IntoIterator<Item=std::ffi::OsString>)->Self{
        Self::for_platform(args,cfg!(target_os="linux"))
    }
    fn for_platform(args:impl IntoIterator<Item=std::ffi::OsString>,linux:bool)->Self{
        let args:Vec<_>=args.into_iter().collect();
        let named=args.first().and_then(|s|std::path::Path::new(s).file_stem())
            .is_some_and(|s|s.to_string_lossy().eq_ignore_ascii_case("FFTAInstaller-SteamDeck"));
        let steam_deck=linux && (named || args.iter().skip(1).any(|s|s=="--steamdeck"));
        Self{steam_deck,fullscreen:steam_deck && !args.iter().skip(1).any(|s|s=="--windowed")}
    }
    pub fn size(self)->[f32;2]{if self.steam_deck{[1280.,720.]}else{[900.,740.]}}
    pub fn viewport(self,size:[f32;2])->egui::ViewportBuilder{
        let builder=egui::ViewportBuilder::default().with_inner_size(size)
            .with_fullscreen(self.fullscreen).with_maximized(false)
            .with_resizable(false).with_maximize_button(false);
        if self.steam_deck {builder} else {builder.with_min_inner_size(size).with_max_inner_size(size)}
    }

}

pub fn apply(ctx:&egui::Context){
    let mut fonts=egui::FontDefinitions::default();
    fonts.font_data.insert("Inter".into(),egui::FontData::from_static(include_bytes!("../../menu/assets/fonts/Inter.ttf")));
    fonts.font_data.insert("Cinzel".into(),egui::FontData::from_static(include_bytes!("../../menu/assets/fonts/Cinzel.ttf")));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0,"Inter".into());
    fonts.families.insert(FontFamily::Name("Display".into()),vec!["Cinzel".into()]);
    ctx.set_fonts(fonts);
    let mut v=egui::Visuals::light();
    v.panel_fill=BG;v.window_fill=PAPER;v.override_text_color=Some(INK);
    v.extreme_bg_color=CARD;v.faint_bg_color=CARD;v.hyperlink_color=MUTED;
    v.selection.bg_fill=GOLD;v.selection.stroke=Stroke::new(1.,INK);
    for widget in [&mut v.widgets.inactive,&mut v.widgets.hovered,&mut v.widgets.active,&mut v.widgets.noninteractive]{
        widget.rounding=egui::Rounding::same(5.);widget.bg_stroke=Stroke::new(1.,GOLD);
        widget.fg_stroke=Stroke::new(1.,INK);
    }
    v.widgets.inactive.weak_bg_fill=CARD;
    v.widgets.hovered.weak_bg_fill=Color32::from_rgb(242,220,161);
    v.widgets.active.weak_bg_fill=Color32::from_rgb(229,203,142);
    ctx.set_visuals(v);
    ctx.style_mut(|s|{
        s.spacing.button_padding=egui::vec2(16.,10.);
        s.spacing.item_spacing=egui::vec2(10.,10.);
        s.text_styles.insert(egui::TextStyle::Body,FontId::proportional(15.));
        s.text_styles.insert(egui::TextStyle::Button,FontId::proportional(15.));
        s.text_styles.insert(egui::TextStyle::Heading,FontId::new(25.,FontFamily::Name("Display".into())));
    });
}

#[cfg(test)]mod tests{
    use super::*;
    fn launch(args:&[&str])->Launch{Launch::for_platform(args.iter().map(std::ffi::OsString::from),true)}
    #[test]fn desktop_does_not_take_over_the_screen(){let l=launch(&["installer"]);assert!(!l.fullscreen);assert_eq!(l.size(),[900.,740.]);}
    #[test]fn deck_name_or_flag_selects_720p_fullscreen(){for a in [vec!["/tmp/FFTAInstaller-SteamDeck"],vec!["installer","--steamdeck"]]{let l=launch(&a);assert!(l.fullscreen);assert_eq!(l.size(),[1280.,720.]);}}
    #[test]fn desktop_window_has_fixed_bounds_and_no_maximize_button(){
        let l=launch(&["installer"]);let v=l.viewport(l.size());
        assert_eq!(v.resizable,Some(false));assert_eq!(v.maximize_button,Some(false));
        assert_eq!(v.maximized,Some(false));assert_eq!(v.fullscreen,Some(false));
        assert_eq!(v.min_inner_size,Some(egui::vec2(900.,740.)));
        assert_eq!(v.max_inner_size,v.min_inner_size);
    }
    #[test]fn windows_never_selects_fullscreen(){let l=Launch::for_platform(["FFTAInstaller-SteamDeck","--steamdeck"].into_iter().map(std::ffi::OsString::from),false);assert!(!l.fullscreen);assert!(!l.steam_deck);}
    #[test]fn windowed_preview_keeps_deck_layout(){let l=launch(&["installer","--steamdeck","--windowed"]);assert!(l.steam_deck);assert!(!l.fullscreen);}
}
