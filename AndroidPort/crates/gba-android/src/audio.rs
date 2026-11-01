//! PCM resampling and desktop-style normal-speed fast-forward backlog.
use crate::normal_audio::NormalAudio;
use std::path::Path;
use std::sync::{Arc,Mutex};
#[derive(Default)]
pub struct Audio {
    phase:u32, previous:[i16;2], key:(usize,u32),
    queue:Option<Arc<Mutex<NormalAudio>>>, staged:Vec<i16>, cadence:u64,
}
impl Audio {
    pub fn begin(&mut self,speed:usize,mode:u32,directory:&Path)->Result<(),String>{
        self.begin_half(speed*2,mode,directory)
    }
    /// Twice the requested rate, so 1.5x has a stable audio key and phase.
    pub fn begin_half(&mut self,speed_half:usize,mode:u32,directory:&Path)->Result<(),String>{
        let key=(speed_half,mode);
        if key!=self.key {
            self.queue=None;crate::normal_audio::publish(None);self.phase=0;self.cadence=0;self.previous=[0;2];self.key=key;
            if speed_half>2 && mode==2 {
                std::fs::create_dir_all(directory).map_err(|e|e.to_string())?;
                self.queue=Some(Arc::new(Mutex::new(NormalAudio::new_in(directory).map_err(|e|e.to_string())?)));
                crate::normal_audio::publish(self.queue.clone());
            }
        }
        self.staged.clear();
        Ok(())
    }
    pub fn feed(&mut self,pcm:&[i16]){
        let (speed,mode)=self.key;
        if mode==0{return;}
        let denominator=if mode==1{32768*speed as u32}else{65536};
        for pair in pcm.chunks_exact(2){
            self.phase+=48000;
            if self.phase>=denominator {
                self.phase-=denominator;
                let fraction=(48000-self.phase) as i64;
                for channel in 0..2 {
                    self.staged.push((self.previous[channel] as i64+
                        (pair[channel] as i64-self.previous[channel] as i64)*fraction/48000) as i16);
                }
            }
            self.previous.copy_from_slice(pair);
        }
    }
    pub fn finish(&mut self,out:&mut Vec<i16>)->Result<(),String>{
        out.clear();
        if let Some(queue)=&mut self.queue {
            let mut queue=queue.lock().map_err(|_|"Normal audio queue poisoned")?;
            queue.push(&self.staged).map_err(|e|e.to_string())?;
            if crate::normal_audio::external_reader(){return Ok(());}
            self.cadence+=48000*280896;
            let pairs=self.cadence/16777216;
            self.cadence%=16777216;
            for _ in 0..pairs {
                if let Some((left,right))=queue.next_pair().map_err(|e|e.to_string())? {
                    out.extend([left,right]);
                } else {break;}
            }
        }else{out.extend_from_slice(&self.staged);}
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accelerated_audio_has_one_host_frame_of_samples_at_every_speed(){
        for speed in [1,2,4,8] {
            let mut audio=Audio::default();let mut out=Vec::new();let mut total=0;
            for _ in 0..60{
                audio.begin(speed,1,&std::env::temp_dir()).unwrap();
                audio.feed(&vec![1000;1097*2*speed]);
                audio.finish(&mut out).unwrap();
                assert!(out.len()<=1610);total+=out.len();
                assert!(out.iter().all(|&sample|sample==1000 || sample==0));
            }
            assert!((total as i32-96416).abs()<10);
        }
    }
    #[test]fn one_and_half_speed_preserves_resampler_phase_between_one_and_two_frame_batches(){
        let mut a=Audio::default();let mut out=Vec::new();let mut total=0;
        for n in 0..60 {
            a.begin_half(3,1,&std::env::temp_dir()).unwrap();
            a.feed(&vec![1000;1097*2*if n%2==0{1}else{2}]);
            a.finish(&mut out).unwrap();total+=out.len();
        }
        assert!((total as i32-96416).abs()<10);
        a.begin_half(3,2,&std::env::temp_dir()).unwrap();
        let q=a.queue.as_ref().unwrap().clone();
        for _ in 0..3 {a.begin_half(3,2,&std::env::temp_dir()).unwrap();assert!(Arc::ptr_eq(&q,a.queue.as_ref().unwrap()));}
        a.begin_half(2,2,&std::env::temp_dir()).unwrap();assert!(a.queue.is_none());
    }
    #[test]
    fn normal_audio_keeps_pitch_and_resets_backlog_on_return_to_normal(){
        let mut audio=Audio::default();let mut out=Vec::new();
        audio.begin(8,2,&std::env::temp_dir()).unwrap();
        audio.feed(&vec![1234;1097*2*8]);audio.finish(&mut out).unwrap();
        assert!((1606..=1608).contains(&out.len()));
        assert_eq!(out[100],1234);
        audio.begin(1,2,&std::env::temp_dir()).unwrap();
        audio.feed(&vec![2345;1097*2]);audio.finish(&mut out).unwrap();
        assert_eq!(out[100],2345);
        audio.begin(8,0,&std::env::temp_dir()).unwrap();
        audio.feed(&vec![2345;1097*2*8]);audio.finish(&mut out).unwrap();
        assert!(out.is_empty());
    }
}

#[cfg(test)]
mod independent_output_tests {
    use super::*;
    #[test]
    fn normal_audio_reader_consumes_full_batches_independent_of_game_batches(){
        crate::normal_audio::set_external_reader(true);
        let mut audio=Audio::default();let mut out=Vec::new();
        audio.begin(4,2,&std::env::temp_dir()).unwrap();
        let input=(0..8192i16).flat_map(|i|[i,-i]).collect::<Vec<_>>();
        audio.feed(&input);audio.finish(&mut out).unwrap();assert!(out.is_empty());
        let mut received=Vec::new();let mut chunk=[0;2048];
        loop{let n=crate::normal_audio::pull(&mut chunk).unwrap();if n==0{break;}received.extend_from_slice(&chunk[..n]);}
        // All resampled audio is drained, not just one 16.74ms frame.
        assert!(received.len()>10000);
        assert!(received.chunks_exact(2).all(|p|p[0]==-p[1]));
        assert!(received.chunks_exact(2).map(|p|p[0]).collect::<Vec<_>>().windows(2).all(|p|p[0]<=p[1]));
        audio.begin(1,2,&std::env::temp_dir()).unwrap();
        assert_eq!(crate::normal_audio::pull(&mut chunk).unwrap(),0);
        crate::normal_audio::set_external_reader(false);
    }
}
