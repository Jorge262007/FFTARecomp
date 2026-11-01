//! Native-pixel scrollbar geometry, independent of a particular game/menu.
#[derive(Clone,Copy,Debug)]
pub struct Bar {pub track:[i32;4],pub thumb:[i32;4],pub maximum:u16}
impl Bar {
    pub fn new(bounds:[i32;4],first:u16,visible:u16,total:u16)->Option<Self>{
        Self::inset(bounds,first,visible,total,16,4)
    }
    pub fn centered(bounds:[i32;4],first:u16,visible:u16,total:u16)->Option<Self>{
        Self::inset(bounds,first,visible,total,4,4)
    }
    fn inset(bounds:[i32;4],first:u16,visible:u16,total:u16,above:i32,below:i32)->Option<Self>{
        if visible==0 || total<=visible{return None;}
        let [_,top,right,bottom]=bounds;let top=top+above;let bottom=bottom-below;
        let height=bottom-top;if height<12{return None;}
        let h=((height as f32*visible as f32/total as f32).round()as i32).clamp(6,height);
        let maximum=total-visible;let y=top+((height-h)as f32*first.min(maximum)as f32/maximum as f32).round()as i32;
        Some(Self{track:[right-6,top,right-2,bottom],thumb:[right-6,y,right-2,y+h],maximum})
    }
    pub fn hit(self,x:f32,y:f32)->bool {x>=(self.track[0]-2)as f32 && x<(self.track[2]+2)as f32 && y>=self.track[1]as f32 && y<self.track[3]as f32}
    pub fn grab(self,y:f32)->f32 {if y>=self.thumb[1]as f32 && y<self.thumb[3]as f32 {y-self.thumb[1]as f32}else{(self.thumb[3]-self.thumb[1])as f32*0.5}}
    pub fn first_at(self,y:f32,grab:f32)->u16 {
        let travel=self.track[3]-self.track[1]-(self.thumb[3]-self.thumb[1]);
        if travel<=0{return 0;}
        (((y-grab-self.track[1]as f32)/travel as f32).clamp(0.,1.)*self.maximum as f32).round()as u16
    }
    pub fn paint(self,out:&mut Vec<[u8;4]>){
        out.resize(480*160,[0,0,0,16]);out.fill([0,0,0,16]);
        self.paint_over(out);
    }
    /// Overlay on an existing menu canvas, preserving every other pixel.
    pub fn paint_over(self,out:&mut Vec<[u8;4]>){
        out.resize(480*160,[0,0,0,16]);
        for (r,color)in [(self.track,2u16|(5<<5)|(13<<10)),(self.thumb,10u16|(24<<5)|(31<<10))]{
            let pixel=[color as u8,(color>>8)as u8,16,0];
            for y in r[1].max(0)..r[3].min(160){for x in r[0].max(-120)..r[2].min(360){out[y as usize*480+(x+120)as usize]=pixel;}}
        }
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn overlay_keeps_existing_menu_pixels_and_transparency(){
        let original=[7,12,14,9];let mut pixels=vec![original;480*160];
        let b=Bar::new([136,32,240,160],0,6,20).unwrap();b.paint_over(&mut pixels);
        for y in 0..160 {for x in -120..360 {
            let p=pixels[y as usize*480+(x+120)as usize];
            if x<b.track[0] || x>=b.track[2] || y<b.track[1] || y>=b.track[3]{assert_eq!(p,original);}
        }}
    }
    #[test]fn only_overflow_shows_a_bar_and_thumb_is_proportional(){
        assert!(Bar::new([32,8,216,160],0,8,8).is_none());
        let nearly=Bar::new([32,8,216,160],0,8,9).unwrap();let long=Bar::new([32,8,216,160],0,8,42).unwrap();
        assert!(nearly.thumb[3]-nearly.thumb[1]>long.thumb[3]-long.thumb[1]);
        assert!(nearly.thumb[3]-nearly.thumb[1]>(nearly.track[3]-nearly.track[1])*4/5);
        assert_eq!(long.first_at(-100.,0.),0);assert_eq!(long.first_at(1000.,0.),34);
    }
    #[test]fn abilities_track_has_equal_vertical_margins(){
        let b=Bar::centered([136,48,240,144],1,6,8).unwrap();
        assert_eq!(b.track[1]-48,144-b.track[3]);
        assert_eq!(b.track[1]+b.track[3],48+144);
        assert_eq!(b.first_at(-100.,0.),0);
        assert_eq!(b.first_at(1000.,0.),2);
    }
    #[test]fn drag_preserves_grab_offset_and_reaches_both_ends(){
        for first in 0..=34{let b=Bar::new([32,8,216,160],first,8,42).unwrap();let y=b.thumb[1]as f32+3.;assert_eq!(b.first_at(y,b.grab(y)),first);}
    }
}
