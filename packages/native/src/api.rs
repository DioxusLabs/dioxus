use std::{any::Any, rc::Rc, sync::Arc};

use wgpu::{ImageCopyTextureBase, Texture};
use winit::window::Window;

use crate::contexts::NativeDocument;

pub fn document() -> Rc<NativeDocument> {
    dioxus_core::prelude::consume_context::<Rc<NativeDocument>>()
}

#[derive(Clone)]
pub struct SharedNativeTexture {
    pub inner: ImageCopyTextureBase<Arc<Texture>>,
}

#[derive(Clone)]
pub struct ReservedNativeTexture {
    pub texture: SharedNativeTexture,
}
