use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::{
    input::{
        keyboard::{Key as BevyKey, KeyCode as BevyKeyCode, KeyboardInput},
        mouse::{MouseButton, MouseButtonInput},
        ButtonInput, ButtonState, InputSystems,
    },
    render::{
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract, RenderApp,
    },
    window::{CursorMoved, WindowResized},
};

use anyrender_vello::VelloScenePainter;
use blitz_dom::{Document as _, DocumentConfig};
use blitz_paint::paint_scene;
use blitz_traits::events::{
    BlitzKeyEvent, BlitzMouseButtonEvent, KeyState, MouseEventButton, MouseEventButtons, UiEvent,
};
use blitz_traits::net::{NetCallback, NetProvider};
use blitz_traits::shell::{ColorScheme, Viewport};
use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender};
use data_url::DataUrl;
use dioxus::prelude::*;
use dioxus_devtools::DevserverMsg;
use dioxus_native_dom::DioxusDocument;
use vello::{
    peniko::color::AlphaColor, RenderParams, Renderer as VelloRenderer, RendererOptions, Scene,
};

// Constant scale factor and color scheme for example purposes
const SCALE_FACTOR: f32 = 1.0;
const COLOR_SCHEME: ColorScheme = ColorScheme::Light;
const CATCH_EVENTS_CLASS: &str = "catch-events";

pub struct DioxusInBevyPlugin<UIProps> {
    pub ui: fn(props: UIProps) -> Element,
    pub props: UIProps,
}

#[derive(Resource)]
struct AnimationTime(Instant);

impl<UIProps: std::marker::Send + std::marker::Sync + std::clone::Clone + 'static> Plugin
    for DioxusInBevyPlugin<UIProps>
{
    fn build(&self, app: &mut App) {
        let epoch = AnimationTime(Instant::now());
        let (s, r) = crossbeam_channel::unbounded();

        // Create the dioxus virtual dom and the dioxus-native document
        // The viewport will be set in setup_ui after we get the window size
        let vdom = VirtualDom::new_with_props(self.ui, self.props.clone());
        // FIXME add a NetProvider
        let mut dioxus_doc = DioxusDocument::new(vdom, DocumentConfig::default());

        // Setup NetProvider
        let net_provider = BevyNetProvider::shared(s.clone());
        dioxus_doc.set_net_provider(net_provider);

        // Setup DocumentProxy to process CreateHeadElement messages
        let proxy = Rc::new(DioxusDocumentProxy::new(s.clone()));
        dioxus_doc.vdom.in_scope(ScopeId::ROOT, move || {
            provide_context(proxy as Rc<dyn dioxus::document::Document>);
        });

        dioxus_doc.initial_build();
        dioxus_doc.resolve(0.0);

        // Dummy waker
        struct NullWake;
        impl std::task::Wake for NullWake {
            fn wake(self: std::sync::Arc<Self>) {}
        }
        let waker = std::task::Waker::from(std::sync::Arc::new(NullWake));

        // Setup devtools listener for hot-reloading
        dioxus_devtools::connect(move |msg| s.send(DioxusMessage::Devserver(msg)).unwrap());
        app.insert_resource(DioxusMessages(r));

        app.insert_non_send_resource(dioxus_doc);
        app.insert_non_send_resource(waker);
        app.insert_resource(epoch);

        app.add_systems(Startup, setup_ui);
        app.add_systems(
            PreUpdate,
            (
                handle_window_resize,
                handle_mouse_events.after(InputSystems),
                handle_keyboard_events.after(InputSystems),
            )
                .chain(),
        );
        app.add_systems(Update, update_ui);
    }

    fn finish(&self, app: &mut App) {
        // Add the UI rendrer
        let render_app = app.sub_app(RenderApp);
        let render_device = render_app.world().resource::<RenderDevice>();
        let device = render_device.wgpu_device();
        let vello_renderer = VelloRenderer::new(device, RendererOptions::default()).unwrap();
        app.insert_non_send_resource(vello_renderer);

        // Setup communication between main world and render world, to send
        // and receive the texture
        let (s, r) = crossbeam_channel::unbounded();
        app.insert_resource(MainWorldReceiver(r));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(bevy::render::ExtractSchedule, extract_texture_image);
        render_app.insert_resource(RenderWorldSender(s));

        // Add a render graph node to get the GPU texture
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(TextureGetterNode, TextureGetterNodeDriver);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, TextureGetterNode);
    }
}

struct RenderTexture {
    pub texture_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<RenderTexture>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<RenderTexture>);

fn create_ui_texture(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::STORAGE_BINDING
        | wgpu::TextureUsages::TEXTURE_BINDING;
    image
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct TextureGetterNode;

#[derive(Default)]
struct TextureGetterNodeDriver;

impl render_graph::Node for TextureGetterNodeDriver {
    fn update(&mut self, world: &mut World) {
        // Get the GPU texture from the texture image, and send it to the main world
        if let Some(sender) = world.get_resource::<RenderWorldSender>() {
            if let Some(image) = world
                .get_resource::<ExtractedTextureImage>()
                .and_then(|e| e.0.as_ref())
            {
                if let Some(gpu_images) = world
                    .get_resource::<RenderAssets<GpuImage>>()
                    .and_then(|a| a.get(image))
                {
                    let _ = sender.send(RenderTexture {
                        texture_view: (*gpu_images.texture_view).clone(),
                        width: gpu_images.size.width,
                        height: gpu_images.size.height,
                    });
                    if let Some(mut extracted_image) =
                        world.get_resource_mut::<ExtractedTextureImage>()
                    {
                        // Reset the image, so it is not sent again, unless it changes
                        extracted_image.0 = None;
                    }
                }
            }
        }
    }
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> bevy::prelude::Result<(), NodeRunError> {
        Ok(())
    }
}

#[derive(Resource)]
pub struct TextureImage(Handle<Image>);

#[derive(Resource)]
pub struct ExtractedTextureImage(Option<Handle<Image>>);

fn extract_texture_image(
    mut commands: Commands,
    texture_image: Extract<Option<Res<TextureImage>>>,
    mut last_texture_image: Local<Option<Handle<Image>>>,
) {
    if let Some(texture_image) = texture_image.as_ref() {
        if let Some(last_texture_image) = &*last_texture_image {
            if last_texture_image == &texture_image.0 {
                return;
            }
        }
        commands.insert_resource(ExtractedTextureImage(Some(texture_image.0.clone())));
        *last_texture_image = Some(texture_image.0.clone());
    }
}

struct HeadElement {
    name: String,
    attributes: Vec<(String, String)>,
    contents: Option<String>,
}

enum DioxusMessage {
    Devserver(DevserverMsg),
    CreateHeadElement(HeadElement),
    ResourceLoad(blitz_dom::net::Resource),
}

#[derive(Resource, Deref)]
struct DioxusMessages(Receiver<DioxusMessage>);

#[derive(Component)]
pub struct DioxusUiQuad;

fn setup_ui(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut animation_epoch: ResMut<AnimationTime>,
    windows: Query<&Window>,
) {
    let window = windows
        .iter()
        .next()
        .expect("Should have at least one window");
    let width = window.physical_width();
    let height = window.physical_height();

    debug!("Initial window size: {}x{}", width, height);

    // Set the initial viewport
    animation_epoch.0 = Instant::now();
    dioxus_doc.set_viewport(Viewport::new(width, height, SCALE_FACTOR, COLOR_SCHEME));
    dioxus_doc.resolve(0.0);

    // Create Bevy Image from the texture data
    let image = create_ui_texture(width, height);
    let handle = images.add(image);

    // Create a quad to display the texture
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(ColorMaterial {
            texture: Some(handle.clone()),
            ..default()
        })),
        Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0)),
        DioxusUiQuad,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: isize::MAX,
            ..default()
        },
    ));

    commands.insert_resource(TextureImage(handle));
}

#[allow(clippy::too_many_arguments)]
fn update_ui(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    dioxus_messages: Res<DioxusMessages>,
    waker: NonSendMut<std::task::Waker>,
    vello_renderer: Option<NonSendMut<VelloRenderer>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    receiver: Res<MainWorldReceiver>,
    animation_epoch: Res<AnimationTime>,
    mut cached_texture: Local<Option<RenderTexture>>,
) {
    while let Ok(msg) = dioxus_messages.0.try_recv() {
        match msg {
            DioxusMessage::Devserver(devserver_msg) => match devserver_msg {
                dioxus_devtools::DevserverMsg::HotReload(hotreload_message) => {
                    // Apply changes to vdom
                    dioxus_devtools::apply_changes(&dioxus_doc.vdom, &hotreload_message);

                    // Reload changed assets
                    for asset_path in &hotreload_message.assets {
                        if let Some(url) = asset_path.to_str() {
                            dioxus_doc.reload_resource_by_href(url);
                        }
                    }
                }
                dioxus_devtools::DevserverMsg::FullReloadStart => {}
                _ => {}
            },
            DioxusMessage::CreateHeadElement(el) => {
                dioxus_doc.create_head_element(&el.name, &el.attributes, &el.contents);
                dioxus_doc.poll(Some(std::task::Context::from_waker(&waker)));
            }
            DioxusMessage::ResourceLoad(resource) => {
                dioxus_doc.load_resource(resource);
            }
        };
    }

    while let Ok(texture) = receiver.try_recv() {
        *cached_texture = Some(texture);
    }

    if let (Some(texture), Some(mut vello_renderer)) = ((*cached_texture).as_ref(), vello_renderer)
    {
        // Poll the vdom
        dioxus_doc.poll(Some(std::task::Context::from_waker(&waker)));

        // Refresh the document
        let animation_time = animation_epoch.0.elapsed().as_secs_f64();
        dioxus_doc.resolve(animation_time);

        // Create a `vello::Scene` to paint into
        let mut scene = Scene::new();

        // Paint the document
        paint_scene(
            &mut VelloScenePainter::new(&mut scene),
            &dioxus_doc,
            SCALE_FACTOR as f64,
            texture.width,
            texture.height,
        );

        // Render the `vello::Scene` to the Texture using the `VelloRenderer`
        vello_renderer
            .render_to_texture(
                render_device.wgpu_device(),
                render_queue.into_inner(),
                &scene,
                &texture.texture_view,
                &RenderParams {
                    base_color: AlphaColor::TRANSPARENT,
                    width: texture.width,
                    height: texture.height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .expect("failed to render to texture");
    }
}

fn handle_window_resize(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut resize_events: MessageReader<WindowResized>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    texture_image: Option<Res<TextureImage>>,
    mut query: Query<(&mut Transform, &mut MeshMaterial2d<ColorMaterial>), With<DioxusUiQuad>>,
) {
    for resize_event in resize_events.read() {
        let width = resize_event.width as u32;
        let height = resize_event.height as u32;

        debug!("Window resized to: {}x{}", width, height);

        // Update the dioxus viewport
        dioxus_doc.set_viewport(Viewport::new(width, height, SCALE_FACTOR, COLOR_SCHEME));
        // dioxus_doc.resolve();

        // Create a new texture with the new size
        let new_image = create_ui_texture(width, height);
        let new_handle = images.add(new_image);

        // Update the quad mesh to match the new size
        if let Ok((mut trans, mut mat)) = query.single_mut() {
            *trans = Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0));
            materials.get_mut(&mut mat.0).unwrap().texture = Some(new_handle.clone());
        }

        // Remove the old texture
        if let Some(texture_image) = texture_image.as_ref() {
            images.remove(&texture_image.0);
        }

        // Insert the new texture resource
        commands.insert_resource(TextureImage(new_handle));
    }
}

#[derive(Resource, Default)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: MouseEventButtons,
    pub mods: Modifiers,
}

fn does_catch_events(dioxus_doc: &DioxusDocument, node_id: usize) -> bool {
    if let Some(node) = dioxus_doc.get_node(node_id) {
        let class = node.attr(blitz_dom::local_name!("class")).unwrap_or("");
        if class
            .split_whitespace()
            .any(|word| word == CATCH_EVENTS_CLASS)
        {
            true
        } else if let Some(parent) = node.parent {
            does_catch_events(dioxus_doc, parent)
        } else {
            false
        }
    } else {
        false
    }
}

fn handle_mouse_events(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut mouse_button_input_events: ResMut<Messages<MouseButtonInput>>,
    mut mouse_buttons: ResMut<ButtonInput<MouseButton>>,
    mut last_mouse_state: Local<MouseState>,
) {
    if cursor_moved.is_empty() && mouse_button_input_events.is_empty() {
        return;
    }

    let mouse_state = &mut last_mouse_state;

    for cursor_event in cursor_moved.read() {
        mouse_state.x = cursor_event.position.x;
        mouse_state.y = cursor_event.position.y;
        dioxus_doc.handle_ui_event(UiEvent::MouseMove(BlitzMouseButtonEvent {
            x: mouse_state.x,
            y: mouse_state.y,
            button: Default::default(),
            buttons: mouse_state.buttons,
            mods: mouse_state.mods,
        }));
    }

    for event in mouse_button_input_events
        .get_cursor()
        .read(&mouse_button_input_events)
    {
        let button_blitz = match event.button {
            MouseButton::Left => MouseEventButton::Main,
            MouseButton::Right => MouseEventButton::Secondary,
            MouseButton::Middle => MouseEventButton::Auxiliary,
            MouseButton::Back => MouseEventButton::Fourth,
            MouseButton::Forward => MouseEventButton::Fifth,
            _ => continue,
        };
        let buttons_blitz = MouseEventButtons::from(button_blitz);
        match event.state {
            ButtonState::Pressed => {
                mouse_state.buttons |= buttons_blitz;
                dioxus_doc.handle_ui_event(UiEvent::MouseDown(BlitzMouseButtonEvent {
                    x: mouse_state.x,
                    y: mouse_state.y,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                }));
            }
            ButtonState::Released => {
                mouse_state.buttons &= !buttons_blitz;
                dioxus_doc.handle_ui_event(UiEvent::MouseUp(BlitzMouseButtonEvent {
                    x: mouse_state.x,
                    y: mouse_state.y,
                    button: button_blitz,
                    buttons: mouse_state.buttons,
                    mods: mouse_state.mods,
                }));
            }
        }
    }

    let should_catch_events = dioxus_doc
        .hit(mouse_state.x, mouse_state.y)
        .map(|hit| does_catch_events(&dioxus_doc, hit.node_id))
        .unwrap_or(false);
    if should_catch_events {
        mouse_button_input_events.clear();
        mouse_buttons.reset_all();
    }

    // dioxus_doc.resolve();
}

fn handle_keyboard_events(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut keyboard_input_events: ResMut<Messages<KeyboardInput>>,
    mut keys: ResMut<ButtonInput<BevyKeyCode>>,
    mut last_mouse_state: Local<MouseState>,
) {
    if keyboard_input_events.is_empty() {
        return;
    }

    for event in keyboard_input_events
        .get_cursor()
        .read(&keyboard_input_events)
    {
        let modifier = match event.logical_key {
            BevyKey::Alt => Some(Modifiers::ALT),
            BevyKey::AltGraph => Some(Modifiers::ALT_GRAPH),
            BevyKey::CapsLock => Some(Modifiers::CAPS_LOCK),
            BevyKey::Control => Some(Modifiers::CONTROL),
            BevyKey::Fn => Some(Modifiers::FN),
            BevyKey::FnLock => Some(Modifiers::FN_LOCK),
            BevyKey::NumLock => Some(Modifiers::NUM_LOCK),
            BevyKey::ScrollLock => Some(Modifiers::SCROLL_LOCK),
            BevyKey::Shift => Some(Modifiers::SHIFT),
            BevyKey::Symbol => Some(Modifiers::SYMBOL),
            BevyKey::SymbolLock => Some(Modifiers::SYMBOL_LOCK),
            BevyKey::Meta => Some(Modifiers::META),
            BevyKey::Hyper => Some(Modifiers::HYPER),
            BevyKey::Super => Some(Modifiers::SUPER),
            _ => None,
        };
        if let Some(modifier) = modifier {
            match event.state {
                ButtonState::Pressed => last_mouse_state.mods.insert(modifier),
                ButtonState::Released => last_mouse_state.mods.remove(modifier),
            };
        }
        let key_state = match event.state {
            ButtonState::Pressed => KeyState::Pressed,
            ButtonState::Released => KeyState::Released,
        };
        let blitz_key_event = BlitzKeyEvent {
            key: bevy_key_to_blitz_key(&event.logical_key),
            code: bevy_key_code_to_blitz_code(&event.key_code),
            modifiers: last_mouse_state.mods,
            location: Location::Standard,
            is_auto_repeating: event.repeat,
            is_composing: false,
            state: key_state,
            text: event.text.clone(),
        };

        match key_state {
            KeyState::Pressed => {
                dioxus_doc.handle_ui_event(UiEvent::KeyDown(blitz_key_event));
            }
            KeyState::Released => {
                dioxus_doc.handle_ui_event(UiEvent::KeyUp(blitz_key_event));
            }
        }
    }

    let should_catch_events = dioxus_doc
        .hit(last_mouse_state.x, last_mouse_state.y)
        .map(|hit| does_catch_events(&dioxus_doc, hit.node_id))
        .unwrap_or(false);
    if should_catch_events {
        keyboard_input_events.clear();
        keys.reset_all();
    }

    // dioxus_doc.resolve();
}

pub struct DioxusDocumentProxy {
    sender: Sender<DioxusMessage>,
}

impl DioxusDocumentProxy {
    fn new(sender: Sender<DioxusMessage>) -> Self {
        Self { sender }
    }
}

impl dioxus::document::Document for DioxusDocumentProxy {
    fn eval(&self, _js: String) -> dioxus::document::Eval {
        dioxus::document::NoOpDocument.eval(_js)
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        self.sender
            .send(DioxusMessage::CreateHeadElement(HeadElement {
                name: name.to_string(),
                attributes: attributes
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.clone()))
                    .collect(),
                contents,
            }))
            .unwrap();
    }

    fn set_title(&self, title: String) {
        self.create_head_element("title", &[], Some(title));
    }

    fn create_meta(&self, props: dioxus::document::MetaProps) {
        let attributes = props.attributes();
        self.create_head_element("meta", &attributes, None);
    }

    fn create_script(&self, props: dioxus::document::ScriptProps) {
        let attributes = props.attributes();
        self.create_head_element("script", &attributes, props.script_contents().ok());
    }

    fn create_style(&self, props: dioxus::document::StyleProps) {
        let attributes = props.attributes();
        self.create_head_element("style", &attributes, props.style_contents().ok());
    }

    fn create_link(&self, props: dioxus::document::LinkProps) {
        let attributes = props.attributes();
        self.create_head_element("link", &attributes, None);
    }

    fn create_head_component(&self) -> bool {
        true
    }
}

struct BevyNetCallback {
    sender: Sender<DioxusMessage>,
}

use blitz_dom::net::Resource as BlitzResource;
use blitz_traits::net::NetHandler;

impl NetCallback<BlitzResource> for BevyNetCallback {
    fn call(&self, _doc_id: usize, result: core::result::Result<BlitzResource, Option<String>>) {
        if let Ok(res) = result {
            self.sender.send(DioxusMessage::ResourceLoad(res)).unwrap();
        }
    }
}

pub struct BevyNetProvider {
    callback: Arc<dyn NetCallback<BlitzResource> + 'static>,
}
impl BevyNetProvider {
    fn shared(sender: Sender<DioxusMessage>) -> Arc<dyn NetProvider<BlitzResource>> {
        Arc::new(Self::new(sender)) as _
    }

    fn new(sender: Sender<DioxusMessage>) -> Self {
        Self {
            callback: Arc::new(BevyNetCallback { sender }) as _,
        }
    }
}

impl NetProvider<BlitzResource> for BevyNetProvider {
    fn fetch(
        &self,
        doc_id: usize,
        request: blitz_traits::net::Request,
        handler: Box<dyn NetHandler<BlitzResource>>,
    ) {
        match request.url.scheme() {
            // Load Dioxus assets
            "dioxus" => match dioxus_asset_resolver::native::serve_asset(request.url.path()) {
                Ok(res) => handler.bytes(doc_id, res.into_body().into(), self.callback.clone()),
                Err(_) => {
                    self.callback.call(
                        doc_id,
                        Err(Some(String::from("Error loading Dioxus asset"))),
                    );
                }
            },
            // Decode data URIs
            "data" => {
                let Ok(data_url) = DataUrl::process(request.url.as_str()) else {
                    self.callback
                        .call(doc_id, Err(Some(String::from("Failed to parse data uri"))));
                    return;
                };
                let Ok(decoded) = data_url.decode_to_vec() else {
                    self.callback
                        .call(doc_id, Err(Some(String::from("Failed to decode data uri"))));
                    return;
                };
                let bytes = Bytes::from(decoded.0);
                handler.bytes(doc_id, bytes, Arc::clone(&self.callback));
            }
            // TODO: support http requests
            _ => {
                self.callback
                    .call(doc_id, Err(Some(String::from("UnsupportedScheme"))));
            }
        }
    }
}

fn bevy_key_to_blitz_key(key: &BevyKey) -> Key {
    match key {
        BevyKey::Character(c) => Key::Character(c.to_string()),
        BevyKey::Unidentified(_) => Key::Unidentified,
        BevyKey::Dead(_) => Key::Dead,
        BevyKey::Alt => Key::Alt,
        BevyKey::AltGraph => Key::AltGraph,
        BevyKey::CapsLock => Key::CapsLock,
        BevyKey::Control => Key::Control,
        BevyKey::Fn => Key::Fn,
        BevyKey::FnLock => Key::FnLock,
        BevyKey::NumLock => Key::Meta,
        BevyKey::ScrollLock => Key::NumLock,
        BevyKey::Shift => Key::ScrollLock,
        BevyKey::Symbol => Key::Shift,
        BevyKey::SymbolLock => Key::Symbol,
        BevyKey::Meta => Key::SymbolLock,
        BevyKey::Hyper => Key::Hyper,
        BevyKey::Super => Key::Super,
        BevyKey::Enter => Key::Enter,
        BevyKey::Tab => Key::Tab,
        BevyKey::Space => Key::Character(" ".to_string()),
        BevyKey::ArrowDown => Key::ArrowDown,
        BevyKey::ArrowLeft => Key::ArrowLeft,
        BevyKey::ArrowRight => Key::ArrowRight,
        BevyKey::ArrowUp => Key::ArrowUp,
        BevyKey::End => Key::End,
        BevyKey::Home => Key::Home,
        BevyKey::PageDown => Key::PageDown,
        BevyKey::PageUp => Key::PageUp,
        BevyKey::Backspace => Key::Backspace,
        BevyKey::Clear => Key::Clear,
        BevyKey::Copy => Key::Copy,
        BevyKey::CrSel => Key::CrSel,
        BevyKey::Cut => Key::Cut,
        BevyKey::Delete => Key::Delete,
        BevyKey::EraseEof => Key::EraseEof,
        BevyKey::ExSel => Key::ExSel,
        BevyKey::Insert => Key::Insert,
        BevyKey::Paste => Key::Paste,
        BevyKey::Redo => Key::Redo,
        BevyKey::Undo => Key::Undo,
        BevyKey::Accept => Key::Accept,
        BevyKey::Again => Key::Again,
        BevyKey::Attn => Key::Attn,
        BevyKey::Cancel => Key::Cancel,
        BevyKey::ContextMenu => Key::ContextMenu,
        BevyKey::Escape => Key::Escape,
        BevyKey::Execute => Key::Execute,
        BevyKey::Find => Key::Find,
        BevyKey::Help => Key::Help,
        BevyKey::Pause => Key::Pause,
        BevyKey::Play => Key::Play,
        BevyKey::Props => Key::Props,
        BevyKey::Select => Key::Select,
        BevyKey::ZoomIn => Key::ZoomIn,
        BevyKey::ZoomOut => Key::ZoomOut,
        BevyKey::BrightnessDown => Key::BrightnessDown,
        BevyKey::BrightnessUp => Key::BrightnessUp,
        BevyKey::Eject => Key::Eject,
        BevyKey::LogOff => Key::LogOff,
        BevyKey::Power => Key::Power,
        BevyKey::PowerOff => Key::PowerOff,
        BevyKey::PrintScreen => Key::PrintScreen,
        BevyKey::Hibernate => Key::Hibernate,
        BevyKey::Standby => Key::Standby,
        BevyKey::WakeUp => Key::WakeUp,
        BevyKey::AllCandidates => Key::AllCandidates,
        BevyKey::Alphanumeric => Key::Alphanumeric,
        BevyKey::CodeInput => Key::CodeInput,
        BevyKey::Compose => Key::Compose,
        BevyKey::Convert => Key::Convert,
        BevyKey::FinalMode => Key::FinalMode,
        BevyKey::GroupFirst => Key::GroupFirst,
        BevyKey::GroupLast => Key::GroupLast,
        BevyKey::GroupNext => Key::GroupNext,
        BevyKey::GroupPrevious => Key::GroupPrevious,
        BevyKey::ModeChange => Key::ModeChange,
        BevyKey::NextCandidate => Key::NextCandidate,
        BevyKey::NonConvert => Key::NonConvert,
        BevyKey::PreviousCandidate => Key::PreviousCandidate,
        BevyKey::Process => Key::Process,
        BevyKey::SingleCandidate => Key::SingleCandidate,
        BevyKey::HangulMode => Key::HangulMode,
        BevyKey::HanjaMode => Key::HanjaMode,
        BevyKey::JunjaMode => Key::JunjaMode,
        BevyKey::Eisu => Key::Eisu,
        BevyKey::Hankaku => Key::Hankaku,
        BevyKey::Hiragana => Key::Hiragana,
        BevyKey::HiraganaKatakana => Key::HiraganaKatakana,
        BevyKey::KanaMode => Key::KanaMode,
        BevyKey::KanjiMode => Key::KanjiMode,
        BevyKey::Katakana => Key::Katakana,
        BevyKey::Romaji => Key::Romaji,
        BevyKey::Zenkaku => Key::Zenkaku,
        BevyKey::ZenkakuHankaku => Key::ZenkakuHankaku,
        BevyKey::Soft1 => Key::Soft1,
        BevyKey::Soft2 => Key::Soft2,
        BevyKey::Soft3 => Key::Soft3,
        BevyKey::Soft4 => Key::Soft4,
        BevyKey::ChannelDown => Key::ChannelDown,
        BevyKey::ChannelUp => Key::ChannelUp,
        BevyKey::Close => Key::Close,
        BevyKey::MailForward => Key::MailForward,
        BevyKey::MailReply => Key::MailReply,
        BevyKey::MailSend => Key::MailSend,
        BevyKey::MediaClose => Key::MediaClose,
        BevyKey::MediaFastForward => Key::MediaFastForward,
        BevyKey::MediaPause => Key::MediaPause,
        BevyKey::MediaPlay => Key::MediaPlay,
        BevyKey::MediaPlayPause => Key::MediaPlayPause,
        BevyKey::MediaRecord => Key::MediaRecord,
        BevyKey::MediaRewind => Key::MediaRewind,
        BevyKey::MediaStop => Key::MediaStop,
        BevyKey::MediaTrackNext => Key::MediaTrackNext,
        BevyKey::MediaTrackPrevious => Key::MediaTrackPrevious,
        BevyKey::New => Key::New,
        BevyKey::Open => Key::Open,
        BevyKey::Print => Key::Print,
        BevyKey::Save => Key::Save,
        BevyKey::SpellCheck => Key::SpellCheck,
        BevyKey::Key11 => Key::Key11,
        BevyKey::Key12 => Key::Key12,
        BevyKey::AudioBalanceLeft => Key::AudioBalanceLeft,
        BevyKey::AudioBalanceRight => Key::AudioBalanceRight,
        BevyKey::AudioBassBoostDown => Key::AudioBassBoostDown,
        BevyKey::AudioBassBoostToggle => Key::AudioBassBoostToggle,
        BevyKey::AudioBassBoostUp => Key::AudioBassBoostUp,
        BevyKey::AudioFaderFront => Key::AudioFaderFront,
        BevyKey::AudioFaderRear => Key::AudioFaderRear,
        BevyKey::AudioSurroundModeNext => Key::AudioSurroundModeNext,
        BevyKey::AudioTrebleDown => Key::AudioTrebleDown,
        BevyKey::AudioTrebleUp => Key::AudioTrebleUp,
        BevyKey::AudioVolumeDown => Key::AudioVolumeDown,
        BevyKey::AudioVolumeUp => Key::AudioVolumeUp,
        BevyKey::AudioVolumeMute => Key::AudioVolumeMute,
        BevyKey::MicrophoneToggle => Key::MicrophoneToggle,
        BevyKey::MicrophoneVolumeDown => Key::MicrophoneVolumeDown,
        BevyKey::MicrophoneVolumeUp => Key::MicrophoneVolumeUp,
        BevyKey::MicrophoneVolumeMute => Key::MicrophoneVolumeMute,
        BevyKey::SpeechCorrectionList => Key::SpeechCorrectionList,
        BevyKey::SpeechInputToggle => Key::SpeechInputToggle,
        BevyKey::LaunchApplication1 => Key::LaunchApplication1,
        BevyKey::LaunchApplication2 => Key::LaunchApplication2,
        BevyKey::LaunchCalendar => Key::LaunchCalendar,
        BevyKey::LaunchContacts => Key::LaunchContacts,
        BevyKey::LaunchMail => Key::LaunchMail,
        BevyKey::LaunchMediaPlayer => Key::LaunchMediaPlayer,
        BevyKey::LaunchMusicPlayer => Key::LaunchMusicPlayer,
        BevyKey::LaunchPhone => Key::LaunchPhone,
        BevyKey::LaunchScreenSaver => Key::LaunchScreenSaver,
        BevyKey::LaunchSpreadsheet => Key::LaunchSpreadsheet,
        BevyKey::LaunchWebBrowser => Key::LaunchWebBrowser,
        BevyKey::LaunchWebCam => Key::LaunchWebCam,
        BevyKey::LaunchWordProcessor => Key::LaunchWordProcessor,
        BevyKey::BrowserBack => Key::BrowserBack,
        BevyKey::BrowserFavorites => Key::BrowserFavorites,
        BevyKey::BrowserForward => Key::BrowserForward,
        BevyKey::BrowserHome => Key::BrowserHome,
        BevyKey::BrowserRefresh => Key::BrowserRefresh,
        BevyKey::BrowserSearch => Key::BrowserSearch,
        BevyKey::BrowserStop => Key::BrowserStop,
        BevyKey::AppSwitch => Key::AppSwitch,
        BevyKey::Call => Key::Call,
        BevyKey::Camera => Key::Camera,
        BevyKey::CameraFocus => Key::CameraFocus,
        BevyKey::EndCall => Key::EndCall,
        BevyKey::GoBack => Key::GoBack,
        BevyKey::GoHome => Key::GoHome,
        BevyKey::HeadsetHook => Key::HeadsetHook,
        BevyKey::LastNumberRedial => Key::LastNumberRedial,
        BevyKey::Notification => Key::Notification,
        BevyKey::MannerMode => Key::MannerMode,
        BevyKey::VoiceDial => Key::VoiceDial,
        BevyKey::TV => Key::TV,
        BevyKey::TV3DMode => Key::TV3DMode,
        BevyKey::TVAntennaCable => Key::TVAntennaCable,
        BevyKey::TVAudioDescription => Key::TVAudioDescription,
        BevyKey::TVAudioDescriptionMixDown => Key::TVAudioDescriptionMixDown,
        BevyKey::TVAudioDescriptionMixUp => Key::TVAudioDescriptionMixUp,
        BevyKey::TVContentsMenu => Key::TVContentsMenu,
        BevyKey::TVDataService => Key::TVDataService,
        BevyKey::TVInput => Key::TVInput,
        BevyKey::TVInputComponent1 => Key::TVInputComponent1,
        BevyKey::TVInputComponent2 => Key::TVInputComponent2,
        BevyKey::TVInputComposite1 => Key::TVInputComposite1,
        BevyKey::TVInputComposite2 => Key::TVInputComposite2,
        BevyKey::TVInputHDMI1 => Key::TVInputHDMI1,
        BevyKey::TVInputHDMI2 => Key::TVInputHDMI2,
        BevyKey::TVInputHDMI3 => Key::TVInputHDMI3,
        BevyKey::TVInputHDMI4 => Key::TVInputHDMI4,
        BevyKey::TVInputVGA1 => Key::TVInputVGA1,
        BevyKey::TVMediaContext => Key::TVMediaContext,
        BevyKey::TVNetwork => Key::TVNetwork,
        BevyKey::TVNumberEntry => Key::TVNumberEntry,
        BevyKey::TVPower => Key::TVPower,
        BevyKey::TVRadioService => Key::TVRadioService,
        BevyKey::TVSatellite => Key::TVSatellite,
        BevyKey::TVSatelliteBS => Key::TVSatelliteBS,
        BevyKey::TVSatelliteCS => Key::TVSatelliteCS,
        BevyKey::TVSatelliteToggle => Key::TVSatelliteToggle,
        BevyKey::TVTerrestrialAnalog => Key::TVTerrestrialAnalog,
        BevyKey::TVTerrestrialDigital => Key::TVTerrestrialDigital,
        BevyKey::TVTimer => Key::TVTimer,
        BevyKey::AVRInput => Key::AVRInput,
        BevyKey::AVRPower => Key::AVRPower,
        BevyKey::ColorF0Red => Key::ColorF0Red,
        BevyKey::ColorF1Green => Key::ColorF1Green,
        BevyKey::ColorF2Yellow => Key::ColorF2Yellow,
        BevyKey::ColorF3Blue => Key::ColorF3Blue,
        BevyKey::ColorF4Grey => Key::ColorF4Grey,
        BevyKey::ColorF5Brown => Key::ColorF5Brown,
        BevyKey::ClosedCaptionToggle => Key::ClosedCaptionToggle,
        BevyKey::Dimmer => Key::Dimmer,
        BevyKey::DisplaySwap => Key::DisplaySwap,
        BevyKey::DVR => Key::DVR,
        BevyKey::Exit => Key::Exit,
        BevyKey::FavoriteClear0 => Key::FavoriteClear0,
        BevyKey::FavoriteClear1 => Key::FavoriteClear1,
        BevyKey::FavoriteClear2 => Key::FavoriteClear2,
        BevyKey::FavoriteClear3 => Key::FavoriteClear3,
        BevyKey::FavoriteRecall0 => Key::FavoriteRecall0,
        BevyKey::FavoriteRecall1 => Key::FavoriteRecall1,
        BevyKey::FavoriteRecall2 => Key::FavoriteRecall2,
        BevyKey::FavoriteRecall3 => Key::FavoriteRecall3,
        BevyKey::FavoriteStore0 => Key::FavoriteStore0,
        BevyKey::FavoriteStore1 => Key::FavoriteStore1,
        BevyKey::FavoriteStore2 => Key::FavoriteStore2,
        BevyKey::FavoriteStore3 => Key::FavoriteStore3,
        BevyKey::Guide => Key::Guide,
        BevyKey::GuideNextDay => Key::GuideNextDay,
        BevyKey::GuidePreviousDay => Key::GuidePreviousDay,
        BevyKey::Info => Key::Info,
        BevyKey::InstantReplay => Key::InstantReplay,
        BevyKey::Link => Key::Link,
        BevyKey::ListProgram => Key::ListProgram,
        BevyKey::LiveContent => Key::LiveContent,
        BevyKey::Lock => Key::Lock,
        BevyKey::MediaApps => Key::MediaApps,
        BevyKey::MediaAudioTrack => Key::MediaAudioTrack,
        BevyKey::MediaLast => Key::MediaLast,
        BevyKey::MediaSkipBackward => Key::MediaSkipBackward,
        BevyKey::MediaSkipForward => Key::MediaSkipForward,
        BevyKey::MediaStepBackward => Key::MediaStepBackward,
        BevyKey::MediaStepForward => Key::MediaStepForward,
        BevyKey::MediaTopMenu => Key::MediaTopMenu,
        BevyKey::NavigateIn => Key::NavigateIn,
        BevyKey::NavigateNext => Key::NavigateNext,
        BevyKey::NavigateOut => Key::NavigateOut,
        BevyKey::NavigatePrevious => Key::NavigatePrevious,
        BevyKey::NextFavoriteChannel => Key::NextFavoriteChannel,
        BevyKey::NextUserProfile => Key::NextUserProfile,
        BevyKey::OnDemand => Key::OnDemand,
        BevyKey::Pairing => Key::Pairing,
        BevyKey::PinPDown => Key::PinPDown,
        BevyKey::PinPMove => Key::PinPMove,
        BevyKey::PinPToggle => Key::PinPToggle,
        BevyKey::PinPUp => Key::PinPUp,
        BevyKey::PlaySpeedDown => Key::PlaySpeedDown,
        BevyKey::PlaySpeedReset => Key::PlaySpeedReset,
        BevyKey::PlaySpeedUp => Key::PlaySpeedUp,
        BevyKey::RandomToggle => Key::RandomToggle,
        BevyKey::RcLowBattery => Key::RcLowBattery,
        BevyKey::RecordSpeedNext => Key::RecordSpeedNext,
        BevyKey::RfBypass => Key::RfBypass,
        BevyKey::ScanChannelsToggle => Key::ScanChannelsToggle,
        BevyKey::ScreenModeNext => Key::ScreenModeNext,
        BevyKey::Settings => Key::Settings,
        BevyKey::SplitScreenToggle => Key::SplitScreenToggle,
        BevyKey::STBInput => Key::STBInput,
        BevyKey::STBPower => Key::STBPower,
        BevyKey::Subtitle => Key::Subtitle,
        BevyKey::Teletext => Key::Teletext,
        BevyKey::VideoModeNext => Key::VideoModeNext,
        BevyKey::Wink => Key::Wink,
        BevyKey::ZoomToggle => Key::ZoomToggle,
        BevyKey::F1 => Key::F1,
        BevyKey::F2 => Key::F2,
        BevyKey::F3 => Key::F3,
        BevyKey::F4 => Key::F4,
        BevyKey::F5 => Key::F5,
        BevyKey::F6 => Key::F6,
        BevyKey::F7 => Key::F7,
        BevyKey::F8 => Key::F8,
        BevyKey::F9 => Key::F9,
        BevyKey::F10 => Key::F10,
        BevyKey::F11 => Key::F11,
        BevyKey::F12 => Key::F12,
        BevyKey::F13 => Key::F13,
        BevyKey::F14 => Key::F14,
        BevyKey::F15 => Key::F15,
        BevyKey::F16 => Key::F16,
        BevyKey::F17 => Key::F17,
        BevyKey::F18 => Key::F18,
        BevyKey::F19 => Key::F19,
        BevyKey::F20 => Key::F20,
        BevyKey::F21 => Key::F21,
        BevyKey::F22 => Key::F22,
        BevyKey::F23 => Key::F23,
        BevyKey::F24 => Key::F24,
        BevyKey::F25 => Key::F25,
        BevyKey::F26 => Key::F26,
        BevyKey::F27 => Key::F27,
        BevyKey::F28 => Key::F28,
        BevyKey::F29 => Key::F29,
        BevyKey::F30 => Key::F30,
        BevyKey::F31 => Key::F31,
        BevyKey::F32 => Key::F32,
        BevyKey::F33 => Key::F33,
        BevyKey::F34 => Key::F34,
        BevyKey::F35 => Key::F35,
        _ => Key::Unidentified,
    }
}

fn bevy_key_code_to_blitz_code(key_code: &BevyKeyCode) -> Code {
    match key_code {
        BevyKeyCode::Unidentified(_) => Code::Unidentified,
        BevyKeyCode::Backquote => Code::Backquote,
        BevyKeyCode::Backslash => Code::Backslash,
        BevyKeyCode::BracketLeft => Code::BracketLeft,
        BevyKeyCode::BracketRight => Code::BracketRight,
        BevyKeyCode::Comma => Code::Comma,
        BevyKeyCode::Digit0 => Code::Digit0,
        BevyKeyCode::Digit1 => Code::Digit1,
        BevyKeyCode::Digit2 => Code::Digit2,
        BevyKeyCode::Digit3 => Code::Digit3,
        BevyKeyCode::Digit4 => Code::Digit4,
        BevyKeyCode::Digit5 => Code::Digit5,
        BevyKeyCode::Digit6 => Code::Digit6,
        BevyKeyCode::Digit7 => Code::Digit7,
        BevyKeyCode::Digit8 => Code::Digit8,
        BevyKeyCode::Digit9 => Code::Digit9,
        BevyKeyCode::Equal => Code::Equal,
        BevyKeyCode::IntlBackslash => Code::IntlBackslash,
        BevyKeyCode::IntlRo => Code::IntlRo,
        BevyKeyCode::IntlYen => Code::IntlYen,
        BevyKeyCode::KeyA => Code::KeyA,
        BevyKeyCode::KeyB => Code::KeyB,
        BevyKeyCode::KeyC => Code::KeyC,
        BevyKeyCode::KeyD => Code::KeyD,
        BevyKeyCode::KeyE => Code::KeyE,
        BevyKeyCode::KeyF => Code::KeyF,
        BevyKeyCode::KeyG => Code::KeyG,
        BevyKeyCode::KeyH => Code::KeyH,
        BevyKeyCode::KeyI => Code::KeyI,
        BevyKeyCode::KeyJ => Code::KeyJ,
        BevyKeyCode::KeyK => Code::KeyK,
        BevyKeyCode::KeyL => Code::KeyL,
        BevyKeyCode::KeyM => Code::KeyM,
        BevyKeyCode::KeyN => Code::KeyN,
        BevyKeyCode::KeyO => Code::KeyO,
        BevyKeyCode::KeyP => Code::KeyP,
        BevyKeyCode::KeyQ => Code::KeyQ,
        BevyKeyCode::KeyR => Code::KeyR,
        BevyKeyCode::KeyS => Code::KeyS,
        BevyKeyCode::KeyT => Code::KeyT,
        BevyKeyCode::KeyU => Code::KeyU,
        BevyKeyCode::KeyV => Code::KeyV,
        BevyKeyCode::KeyW => Code::KeyW,
        BevyKeyCode::KeyX => Code::KeyX,
        BevyKeyCode::KeyY => Code::KeyY,
        BevyKeyCode::KeyZ => Code::KeyZ,
        BevyKeyCode::Minus => Code::Minus,
        BevyKeyCode::Period => Code::Period,
        BevyKeyCode::Quote => Code::Quote,
        BevyKeyCode::Semicolon => Code::Semicolon,
        BevyKeyCode::Slash => Code::Slash,
        BevyKeyCode::AltLeft => Code::AltLeft,
        BevyKeyCode::AltRight => Code::AltRight,
        BevyKeyCode::Backspace => Code::Backspace,
        BevyKeyCode::CapsLock => Code::CapsLock,
        BevyKeyCode::ContextMenu => Code::ContextMenu,
        BevyKeyCode::ControlLeft => Code::ControlLeft,
        BevyKeyCode::ControlRight => Code::ControlRight,
        BevyKeyCode::Enter => Code::Enter,
        BevyKeyCode::SuperLeft => Code::MetaLeft,
        BevyKeyCode::SuperRight => Code::MetaRight,
        BevyKeyCode::ShiftLeft => Code::ShiftLeft,
        BevyKeyCode::ShiftRight => Code::ShiftRight,
        BevyKeyCode::Space => Code::Space,
        BevyKeyCode::Tab => Code::Tab,
        BevyKeyCode::Convert => Code::Convert,
        BevyKeyCode::KanaMode => Code::KanaMode,
        BevyKeyCode::Lang1 => Code::Lang1,
        BevyKeyCode::Lang2 => Code::Lang2,
        BevyKeyCode::Lang3 => Code::Lang3,
        BevyKeyCode::Lang4 => Code::Lang4,
        BevyKeyCode::Lang5 => Code::Lang5,
        BevyKeyCode::NonConvert => Code::NonConvert,
        BevyKeyCode::Delete => Code::Delete,
        BevyKeyCode::End => Code::End,
        BevyKeyCode::Help => Code::Help,
        BevyKeyCode::Home => Code::Home,
        BevyKeyCode::Insert => Code::Insert,
        BevyKeyCode::PageDown => Code::PageDown,
        BevyKeyCode::PageUp => Code::PageUp,
        BevyKeyCode::ArrowDown => Code::ArrowDown,
        BevyKeyCode::ArrowLeft => Code::ArrowLeft,
        BevyKeyCode::ArrowRight => Code::ArrowRight,
        BevyKeyCode::ArrowUp => Code::ArrowUp,
        BevyKeyCode::NumLock => Code::NumLock,
        BevyKeyCode::Numpad0 => Code::Numpad0,
        BevyKeyCode::Numpad1 => Code::Numpad1,
        BevyKeyCode::Numpad2 => Code::Numpad2,
        BevyKeyCode::Numpad3 => Code::Numpad3,
        BevyKeyCode::Numpad4 => Code::Numpad4,
        BevyKeyCode::Numpad5 => Code::Numpad5,
        BevyKeyCode::Numpad6 => Code::Numpad6,
        BevyKeyCode::Numpad7 => Code::Numpad7,
        BevyKeyCode::Numpad8 => Code::Numpad8,
        BevyKeyCode::Numpad9 => Code::Numpad9,
        BevyKeyCode::NumpadAdd => Code::NumpadAdd,
        BevyKeyCode::NumpadBackspace => Code::NumpadBackspace,
        BevyKeyCode::NumpadClear => Code::NumpadClear,
        BevyKeyCode::NumpadClearEntry => Code::NumpadClearEntry,
        BevyKeyCode::NumpadComma => Code::NumpadComma,
        BevyKeyCode::NumpadDecimal => Code::NumpadDecimal,
        BevyKeyCode::NumpadDivide => Code::NumpadDivide,
        BevyKeyCode::NumpadEnter => Code::NumpadEnter,
        BevyKeyCode::NumpadEqual => Code::NumpadEqual,
        BevyKeyCode::NumpadHash => Code::NumpadHash,
        BevyKeyCode::NumpadMemoryAdd => Code::NumpadMemoryAdd,
        BevyKeyCode::NumpadMemoryClear => Code::NumpadMemoryClear,
        BevyKeyCode::NumpadMemoryRecall => Code::NumpadMemoryRecall,
        BevyKeyCode::NumpadMemoryStore => Code::NumpadMemoryStore,
        BevyKeyCode::NumpadMemorySubtract => Code::NumpadMemorySubtract,
        BevyKeyCode::NumpadMultiply => Code::NumpadMultiply,
        BevyKeyCode::NumpadParenLeft => Code::NumpadParenLeft,
        BevyKeyCode::NumpadParenRight => Code::NumpadParenRight,
        BevyKeyCode::NumpadStar => Code::NumpadStar,
        BevyKeyCode::NumpadSubtract => Code::NumpadSubtract,
        BevyKeyCode::Escape => Code::Escape,
        BevyKeyCode::Fn => Code::Fn,
        BevyKeyCode::FnLock => Code::FnLock,
        BevyKeyCode::PrintScreen => Code::PrintScreen,
        BevyKeyCode::ScrollLock => Code::ScrollLock,
        BevyKeyCode::Pause => Code::Pause,
        BevyKeyCode::BrowserBack => Code::BrowserBack,
        BevyKeyCode::BrowserFavorites => Code::BrowserFavorites,
        BevyKeyCode::BrowserForward => Code::BrowserForward,
        BevyKeyCode::BrowserHome => Code::BrowserHome,
        BevyKeyCode::BrowserRefresh => Code::BrowserRefresh,
        BevyKeyCode::BrowserSearch => Code::BrowserSearch,
        BevyKeyCode::BrowserStop => Code::BrowserStop,
        BevyKeyCode::Eject => Code::Eject,
        BevyKeyCode::LaunchApp1 => Code::LaunchApp1,
        BevyKeyCode::LaunchApp2 => Code::LaunchApp2,
        BevyKeyCode::LaunchMail => Code::LaunchMail,
        BevyKeyCode::MediaPlayPause => Code::MediaPlayPause,
        BevyKeyCode::MediaSelect => Code::MediaSelect,
        BevyKeyCode::MediaStop => Code::MediaStop,
        BevyKeyCode::MediaTrackNext => Code::MediaTrackNext,
        BevyKeyCode::MediaTrackPrevious => Code::MediaTrackPrevious,
        BevyKeyCode::Power => Code::Power,
        BevyKeyCode::Sleep => Code::Sleep,
        BevyKeyCode::AudioVolumeDown => Code::AudioVolumeDown,
        BevyKeyCode::AudioVolumeMute => Code::AudioVolumeMute,
        BevyKeyCode::AudioVolumeUp => Code::AudioVolumeUp,
        BevyKeyCode::WakeUp => Code::WakeUp,
        BevyKeyCode::Meta => Code::Hyper,
        BevyKeyCode::Hyper => Code::Super,
        BevyKeyCode::Turbo => Code::Turbo,
        BevyKeyCode::Abort => Code::Abort,
        BevyKeyCode::Resume => Code::Resume,
        BevyKeyCode::Suspend => Code::Suspend,
        BevyKeyCode::Again => Code::Again,
        BevyKeyCode::Copy => Code::Copy,
        BevyKeyCode::Cut => Code::Cut,
        BevyKeyCode::Find => Code::Find,
        BevyKeyCode::Open => Code::Open,
        BevyKeyCode::Paste => Code::Paste,
        BevyKeyCode::Props => Code::Props,
        BevyKeyCode::Select => Code::Select,
        BevyKeyCode::Undo => Code::Undo,
        BevyKeyCode::Hiragana => Code::Hiragana,
        BevyKeyCode::Katakana => Code::Katakana,
        BevyKeyCode::F1 => Code::F1,
        BevyKeyCode::F2 => Code::F2,
        BevyKeyCode::F3 => Code::F3,
        BevyKeyCode::F4 => Code::F4,
        BevyKeyCode::F5 => Code::F5,
        BevyKeyCode::F6 => Code::F6,
        BevyKeyCode::F7 => Code::F7,
        BevyKeyCode::F8 => Code::F8,
        BevyKeyCode::F9 => Code::F9,
        BevyKeyCode::F10 => Code::F10,
        BevyKeyCode::F11 => Code::F11,
        BevyKeyCode::F12 => Code::F12,
        BevyKeyCode::F13 => Code::F13,
        BevyKeyCode::F14 => Code::F14,
        BevyKeyCode::F15 => Code::F15,
        BevyKeyCode::F16 => Code::F16,
        BevyKeyCode::F17 => Code::F17,
        BevyKeyCode::F18 => Code::F18,
        BevyKeyCode::F19 => Code::F19,
        BevyKeyCode::F20 => Code::F20,
        BevyKeyCode::F21 => Code::F21,
        BevyKeyCode::F22 => Code::F22,
        BevyKeyCode::F23 => Code::F23,
        BevyKeyCode::F24 => Code::F24,
        BevyKeyCode::F25 => Code::F25,
        BevyKeyCode::F26 => Code::F26,
        BevyKeyCode::F27 => Code::F27,
        BevyKeyCode::F28 => Code::F28,
        BevyKeyCode::F29 => Code::F29,
        BevyKeyCode::F30 => Code::F30,
        BevyKeyCode::F31 => Code::F31,
        BevyKeyCode::F32 => Code::F32,
        BevyKeyCode::F33 => Code::F33,
        BevyKeyCode::F34 => Code::F34,
        BevyKeyCode::F35 => Code::F35,
    }
}
