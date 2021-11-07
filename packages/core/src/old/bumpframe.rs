use crate::innerlude::*;
use bumpalo::Bump;
use std::cell::Cell;

pub(crate) struct ActiveFrame {
    pub cur_generation: Cell<usize>,

    // The double-buffering situation that we will use
    pub frames: [BumpFrame; 2],
}

impl ActiveFrame {
    pub fn new() -> Self {
        let b1 = Bump::new();
        let b2 = Bump::new();

        let frame_a = BumpFrame {
            bump: b1,
            generation: 0.into(),
        };
        let frame_b = BumpFrame {
            bump: b2,
            generation: 0.into(),
        };

        Self {
            frames: [frame_a, frame_b],
            cur_generation: 0.into(),
        }
    }

    pub unsafe fn reset_wip_frame(&self) {
        // todo: unsafecell or something
        let bump = self.wip_frame() as *const _ as *mut BumpFrame;
        let g = &mut *bump;
        g.bump.reset();

        // self.wip_frame_mut().bump.reset()
    }

    /// The "work in progress frame" represents the frame that is currently being worked on.
    pub fn wip_frame(&self) -> &BumpFrame {
        match self.cur_generation.get() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub fn wip_frame_mut(&mut self) -> &mut BumpFrame {
        match self.cur_generation.get() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    /// The finished frame represents the frame that has been "finished" and cannot be modified again
    pub fn finished_frame(&self) -> &BumpFrame {
        match self.cur_generation.get() & 1 == 1 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    // /// Give out our self-referential item with our own borrowed lifetime
    // pub fn fin_head<'b>(&'b self) -> &'b VNode<'b> {
    //     let cur_head = &self.finished_frame().head_node;
    //     unsafe { std::mem::transmute::<&VNode<'static>, &VNode<'b>>(cur_head) }
    // }

    // /// Give out our self-referential item with our own borrowed lifetime
    // pub fn wip_head<'b>(&'b self) -> &'b VNode<'b> {
    //     let cur_head = &self.wip_frame().head_node;
    //     unsafe { std::mem::transmute::<&VNode<'static>, &VNode<'b>>(cur_head) }
    // }

    pub fn cycle_frame(&mut self) {
        self.cur_generation.set(self.cur_generation.get() + 1);
    }
}

#[cfg(test)]
mod tests {
    //! These tests are bad. I don't have a good way of properly testing the ActiveFrame stuff
    use super::*;

    #[test]
    fn test_bump_frame() {
        let mut frames = ActiveFrame::new();

        // just cycle a few times and make sure we get the right frames out
        for _ in 0..5 {
            let fin = frames.finished_frame();
            let wip = frames.wip_frame();
            assert_eq!(wip._name, "wip");
            assert_eq!(fin._name, "fin");
            frames.cycle_frame();

            let fin = frames.finished_frame();
            let wip = frames.wip_frame();
            assert_eq!(wip._name, "fin");
            assert_eq!(fin._name, "wip");
            frames.cycle_frame();
        }
        assert_eq!(frames.cur_generation.get(), 10);
    }
}
