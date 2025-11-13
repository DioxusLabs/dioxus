use crate::bevy_renderer::BevyRenderer;
use crate::Color;
use dioxus_native::{CustomPaintCtx, CustomPaintSource, DeviceHandle, TextureHandle};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum DemoMessage {
    // Color in RGB format
    SetColor(Color),
}

enum DemoRendererState {
    Active(Box<BevyRenderer>),
    Suspended,
}

pub struct DemoPaintSource {
    state: DemoRendererState,
    start_time: std::time::Instant,
    tx: Sender<DemoMessage>,
    rx: Receiver<DemoMessage>,
    color: Color,
}

impl DemoPaintSource {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self::with_channel(tx, rx)
    }

    pub fn with_channel(tx: Sender<DemoMessage>, rx: Receiver<DemoMessage>) -> Self {
        Self {
            state: DemoRendererState::Suspended,
            start_time: std::time::Instant::now(),
            tx,
            rx,
            color: Color::WHITE,
        }
    }

    pub fn sender(&self) -> Sender<DemoMessage> {
        self.tx.clone()
    }

    fn process_messages(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(_) => return,
                Ok(msg) => match msg {
                    DemoMessage::SetColor(color) => self.color = color,
                },
            }
        }
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
    ) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }
        let DemoRendererState::Active(state) = &mut self.state else {
            return None;
        };

        state.render(ctx, self.color.components, width, height, &self.start_time)
    }
}

impl CustomPaintSource for DemoPaintSource {
    fn resume(&mut self, device_handle: &DeviceHandle) {
        let active_state = BevyRenderer::new(device_handle);
        self.state = DemoRendererState::Active(Box::new(active_state));
    }

    fn suspend(&mut self) {
        self.state = DemoRendererState::Suspended;
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
        _scale: f64,
    ) -> Option<TextureHandle> {
        self.process_messages();
        self.render(ctx, width, height)
    }
}
