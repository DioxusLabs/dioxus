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

    #[cfg(test)]
    name: &'static str,
}

impl ActiveFrame {
    pub fn new() -> Self {
        let frame_a = BumpFrame {
            bump: Bump::new(),
            head_node: NodeFactory::unstable_place_holder(),

            #[cfg(test)]
            name: "old",
        };
        let frame_b = BumpFrame {
            bump: Bump::new(),
            head_node: NodeFactory::unstable_place_holder(),

            #[cfg(test)]
            name: "new",
        };
        Self {
            generation: 0.into(),
            frames: [frame_a, frame_b],
        }
    }

    pub fn prev_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub fn prev_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    pub fn cur_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub fn cur_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }
    /// Give out our self-referential item with our own borrowed lifetime
    pub fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let cur_head = &self.cur_frame().head_node;
        unsafe { std::mem::transmute::<&VNode<'static>, &VNode<'b>>(cur_head) }
    }

    /// Give out our self-referential item with our own borrowed lifetime
    pub fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let cur_head = &self.prev_frame().head_node;
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
            let old = frames.prev_frame();
            let new = frames.cur_frame();
            assert_eq!(old.name, "old");
            assert_eq!(new.name, "new");
            frames.cycle_frame();

            let old = frames.prev_frame();
            let new = frames.cur_frame();
            assert_eq!(old.name, "new");
            assert_eq!(new.name, "old");
            frames.cycle_frame();
        }
        assert_eq!(frames.generation.get(), 10);
    }
}
