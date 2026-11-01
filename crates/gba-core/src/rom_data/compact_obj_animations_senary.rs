//! Caller-witnessed EUR compact OBJ bank at 6c6928.
//! Missing selectors and source gaps retain the original owner/raw semantics.
use super::{compact_obj_animations::{Frame, Sequence, SourceSpan}, obj_lists, SourceRange};
use crate::ffta_rom;
use serde::{Deserialize, Serialize};

pub const ROOT: u32 = 0x6c6928;
pub const LIST_ROOT: u32 = ROOT + 4;
pub const SEQUENCE_ROOT: u32 = 0x6c6b80;
pub const LIST_INDICES: [u16; 60] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59];
pub const SELECTORS: [u16; 7] = [0,1,2,3,4,5,6];
pub const SPANS: [(u32,u32);2] = [(0x6c6928,0x6c6b7e),(0x6c6b80,0x6c6c1e)];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedList { pub index: u16, pub relative_offset: u16, pub list: obj_lists::List }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedSequence { pub relative_offset: u16, pub sequence: Sequence }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactAnimationsSenary { pub lists: Vec<IndexedList>, pub sequences: Vec<IndexedSequence> }
impl CompactAnimationsSenary {
    pub fn validate(&self) -> Result<(), String> {
        if self.lists.len()!=60 || self.sequences.len()!=7 { return Err("Invalid senary compact OBJ projection count".into()); }
        let mut objects=0;let mut next=0x6c69a4;
        for (entry,index) in self.lists.iter().zip(LIST_INDICES) {
            entry.list.validate()?;
            if entry.index!=index || entry.list.source_offset!=next || LIST_ROOT+u32::from(entry.relative_offset)!=next {
                return Err("Invalid senary compact OBJ list location".into());
            }
            objects+=entry.list.objects.len();next+=entry.list.source_len() as u32;
        }
        if next!=0x6c6b7e || objects!=59 {return Err("Invalid senary compact OBJ list shape".into());}
        let mut frames=0;let mut next=0x6c6b8e;
        for (i,(entry,selector)) in self.sequences.iter().zip(SELECTORS).enumerate() {
            let seq=&entry.sequence;
            if seq.selector!=selector || seq.source_offset!=next || SEQUENCE_ROOT+u32::from(entry.relative_offset)!=next
                || seq.frames.len()!=[8,8,8,8,8,8,17][i] || seq.frames.iter().any(|f| !LIST_INDICES.contains(&u16::from(f.list))) {
                return Err("Invalid senary compact OBJ sequence".into());
            }
            frames+=seq.frames.len();next+=2+seq.frames.len() as u32*2;
        }
        if next!=0x6c6c1e || frames!=65 {return Err("Invalid senary compact OBJ frames".into());}
        Ok(())
    }
    pub fn sequence(&self, selector:u16)->Option<&Sequence> {self.sequences.iter().find(|x|x.sequence.selector==selector).map(|x|&x.sequence)}
    pub fn list(&self,index:u16)->Option<&obj_lists::List> {self.lists.iter().find(|x|x.index==index).map(|x|&x.list)}
    /// A miss is not a guest bounds check.
    pub fn byte_at(&self, offset:u32)->Option<u8> {self.source_spans().ok()?.into_iter().find_map(|(at,bytes,_)|bytes.get(offset.checked_sub(at)? as usize).copied())}
    pub fn source_spans(&self)->Result<Vec<SourceSpan>,String> {
        self.validate()?;
        let mut pieces:Vec<(u32,Vec<u8>)>=vec![(ROOT,vec![12,0,0x58,2])];
        for item in &self.lists {
            pieces.push((LIST_ROOT+u32::from(item.index)*2,item.relative_offset.to_le_bytes().to_vec()));
            pieces.push((item.list.source_offset,item.list.source_bytes()?));
        }
        for item in &self.sequences {
            let seq=&item.sequence;pieces.push((SEQUENCE_ROOT+u32::from(seq.selector)*2,item.relative_offset.to_le_bytes().to_vec()));
            let mut bytes=(seq.frames.len() as u16).to_le_bytes().to_vec();
            for frame in &seq.frames {bytes.extend([frame.list,frame.duration]);}
            pieces.push((seq.source_offset,bytes));
        }
        pieces.sort_by_key(|x|x.0);let mut spans:Vec<(u32,Vec<u8>)>=Vec::new();
        for (offset,bytes) in pieces {
            if let Some((start,prev))=spans.last_mut() {
                let end=*start+prev.len() as u32;
                if offset<end {return Err("Overlapping senary compact OBJ fields".into());}
                if offset==end {prev.extend(bytes);continue;}
            }
            spans.push((offset,bytes));
        }
        if spans.len()!=SPANS.len() || spans.iter().zip(SPANS).any(|((at,bytes),(start,end))|*at!=start || bytes.len()!=(end-start) as usize) {
            return Err("Invalid senary compact OBJ source spans".into());
        }
        Ok(spans.into_iter().map(|(at,bytes)|(at,bytes,"conditional-compact-obj-animations-senary".into())).collect())
    }
    pub fn verify_source(&self,rom:&[u8])->Result<(),String> {
        self.validate()?;if self!=&Catalog::from_rom(rom)?.decoded {return Err("Senary compact OBJ differs from authenticated EUR source".into());}Ok(())
    }
}
pub struct Catalog {pub decoded:CompactAnimationsSenary,pub sources:Vec<SourceRange>}
impl Catalog {
    pub fn from_rom(rom:&[u8])->Result<Self,String> {
        if rom.len()!=0x1000000 || ffta_rom::digest(rom)!=ffta_rom::EUROPE_SHA256 {return Err("Senary compact OBJ requires exact EUR ROM; USA pending".into());}
        let half=|at:u32|u16::from_le_bytes([rom[at as usize],rom[at as usize+1]]);
        if half(ROOT)!=12 || half(ROOT+2)!=0x258 {return Err("Invalid senary compact OBJ header".into());}
        let mut lists=Vec::new();
        for index in LIST_INDICES {
            let relative_offset=half(LIST_ROOT+u32::from(index)*2);let at=LIST_ROOT+u32::from(relative_offset);let len=2+usize::from(half(at))*6;
            let bytes=rom.get(at as usize..at as usize+len).ok_or("Truncated senary OBJ list")?;
            lists.push(IndexedList{index,relative_offset,list:obj_lists::List::decode(at,bytes)?});
        }
        let mut sequences=Vec::new();
        for selector in SELECTORS {
            let relative_offset=half(SEQUENCE_ROOT+u32::from(selector)*2);let at=SEQUENCE_ROOT+u32::from(relative_offset);let count=usize::from(half(at));
            let bytes=rom.get(at as usize+2..at as usize+2+count*2).ok_or("Truncated senary OBJ sequence")?;
            sequences.push(IndexedSequence{relative_offset,sequence:Sequence{selector,source_offset:at,frames:bytes.chunks_exact(2).map(|v|Frame{list:v[0],duration:v[1]}).collect()}});
        }
        let decoded=CompactAnimationsSenary{lists,sequences};
        let sources=decoded.source_spans()?.into_iter().map(|(offset,raw,role)|SourceRange{offset,length:raw.len() as u32,sha256:ffta_rom::digest(&raw),role}).collect();
        Ok(Self{decoded,sources})
    }
}
