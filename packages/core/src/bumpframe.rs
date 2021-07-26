use crate::innerlude::*;
use bumpalo::Bump;
use std::cell::Cell;

pub struct ActiveFrame {
    // We use a "generation" for users of contents in the bump frames to ensure their data isn't broken
    pub generation: Cell<usize>,

    // The double-buffering situation that we will use
    pub frames: [BumpFrame; 2],
}

pub struct BumpFrame {
    pub bump: Bump,
    pub head_node: VNode<'static>,

    // used internally for debugging
    name: &'static str,
}

impl ActiveFrame {
    pub fn new() -> Self {
        let frame_a = BumpFrame {
            bump: Bump::new(),
            head_node: NodeFactory::unstable_place_holder(),
            name: "wip",
        };
        let frame_b = BumpFrame {
            bump: Bump::new(),
            head_node: NodeFactory::unstable_place_holder(),
            name: "fin",
        };
        Self {
            generation: 0.into(),
            frames: [frame_a, frame_b],
        }
    }

    /// The "work in progress frame" represents the frame that is currently being worked on.
    pub fn wip_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub fn wip_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    /// The finished frame represents the frame that has been "finished" and cannot be modified again
    pub fn finished_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub fn finished_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }
    /// Give out our self-referential item with our own borrowed lifetime
    pub fn fin_head<'b>(&'b self) -> &'b VNode<'b> {
        let cur_head = &self.finished_frame().head_node;
        unsafe { std::mem::transmute::<&VNode<'static>, &VNode<'b>>(cur_head) }
    }

    /// Give out our self-referential item with our own borrowed lifetime
    pub fn wip_head<'b>(&'b self) -> &'b VNode<'b> {
        let cur_head = &self.wip_frame().head_node;
        unsafe { std::mem::transmute::<&VNode<'static>, &VNode<'b>>(cur_head) }
    }

    pub fn cycle_frame(&mut self) {
        self.generation.set(self.generation.get() + 1);
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
            assert_eq!(wip.name, "wip");
            assert_eq!(fin.name, "fin");
            frames.cycle_frame();

            let fin = frames.finished_frame();
            let wip = frames.wip_frame();
            assert_eq!(wip.name, "fin");
            assert_eq!(fin.name, "wip");
            frames.cycle_frame();
        }
        assert_eq!(frames.generation.get(), 10);
    }
}
