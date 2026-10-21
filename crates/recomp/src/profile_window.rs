//! One diagnostic traversal of producer-published guest frames. No runtime policy.
/// A single traversal of producer-published guest frames, end exclusive.
/// Rewinding after entry closes the capture rather than merging two timelines.
pub(crate) struct FrameWindow {
    pub start: u64,
    pub end: u64,
    pub frame: Option<u64>,
    pub closed: bool,
    pub interrupted: bool,
}

impl FrameWindow {
    pub fn parse(value: &str, option: &str) -> Result<Self, String> {
        let invalid = || format!("{option} must be START:END with integer frames and START < END");
        let (start, end) = value.split_once(':').ok_or_else(invalid)?;
        let start = start.parse::<u64>().map_err(|_| invalid())?;
        let end = end.parse::<u64>().map_err(|_| invalid())?;
        if start >= end {
            return Err(invalid());
        }
        Ok(Self {
            start,
            end,
            frame: None,
            closed: false,
            interrupted: false,
        })
    }

    pub fn active(&self) -> bool {
        !self.closed
            && self
                .frame
                .is_some_and(|f| (self.start..self.end).contains(&f))
    }

    pub fn publish(&mut self, frame: u64) {
        if self.closed {
            return;
        }
        if self
            .frame
            .is_some_and(|last| last >= self.start && frame < last)
        {
            self.interrupted = true;
            self.closed = true;
        } else if frame >= self.end {
            self.closed = true;
        }
        self.frame = Some(frame);
    }
}

impl FrameWindow {
    pub fn restore(&mut self, frame: u64) {
        if self.active() {
            self.closed = true;
            self.interrupted = true;
        }
        self.publish(frame);
    }
}
