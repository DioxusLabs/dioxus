use crate::desktop_context::{DesktopContext, UserWindowEvent};
use dioxus_core::*;
use dioxus_html::events::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::from_value;
use std::any::Any;
use std::rc::Rc;
use std::{
    collections::HashMap,
    sync::Arc,
    sync::{atomic::AtomicBool, Mutex},
    time::Duration,
};
use wry::{
    self,
    application::{event_loop::ControlFlow, event_loop::EventLoopProxy, window::WindowId},
    webview::WebView,
};

macro_rules! match_data {
    (
        $m:ident;
        $name:ident;
        $(
            $tip:ty => $($mname:literal)|* ;
        )*
    ) => {
        match $name {
            $( $($mname)|* => {
                println!("casting to type {:?}", std::any::TypeId::of::<$tip>());
                let val: $tip = from_value::<$tip>($m).ok()?;
                Rc::new(val) as Rc<dyn Any>
            })*
            _ => return None,
        }
    };
}

pub(super) struct DesktopController {
    pub(super) webviews: HashMap<WindowId, WebView>,
    pub(super) pending_edits: Arc<Mutex<Vec<String>>>,
    pub(super) quit_app_on_close: bool,
    pub(super) is_ready: Arc<AtomicBool>,
}

impl DesktopController {
    // Launch the virtualdom on its own thread managed by tokio
    // returns the desktop state
    pub(super) fn new_on_tokio<P: Send + 'static>(
        root: Component<P>,
        props: P,
        proxy: EventLoopProxy<UserWindowEvent>,
        mut event_rx: UnboundedReceiver<serde_json::Value>,
    ) -> Self {
        let edit_queue = Arc::new(Mutex::new(Vec::new()));

        let pending_edits = edit_queue.clone();
        let desktop_context_proxy = proxy.clone();

        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "tokio::spawn" onto multiple threads
            // I'd personally not require tokio to be built-in to Dioxus-Desktop, but the DX is worse without it
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            let mut dom = VirtualDom::new_with_props(root, props)
                .with_root_context(DesktopContext::new(desktop_context_proxy));

            {
                let edits = dom.rebuild();
                let mut queue = edit_queue.lock().unwrap();
                queue.push(serde_json::to_string(&edits.template_mutations).unwrap());
                queue.push(serde_json::to_string(&edits.edits).unwrap());
                proxy.send_event(UserWindowEvent::Update).unwrap();
            }

            runtime.block_on(async move {
                loop {
                    tokio::select! {
                        _ = dom.wait_for_work() => {}
                        Some(json_value) = event_rx.next() => {
                            if let Ok(value) = serde_json::from_value::<EventMessage>(json_value) {
                                let name = value.event.clone();
                                let el_id = ElementId(value.mounted_dom_id);
                                let evt = decode_event(value);

                                if let Some(evt) = evt {
                                    dom.handle_event(&name,  evt, el_id, true, EventPriority::Medium);
                                }
                            }
                        }
                    }

                    let muts = dom
                        .render_with_deadline(tokio::time::sleep(Duration::from_millis(16)))
                        .await;

                    {
                        let mut queue = edit_queue.lock().unwrap();
                        queue.push(serde_json::to_string(&muts.template_mutations).unwrap());
                        queue.push(serde_json::to_string(&muts.edits).unwrap());
                        let _ = proxy.send_event(UserWindowEvent::Update);
                    }
                }
            })
        });

        Self {
            pending_edits,
            webviews: HashMap::new(),
            is_ready: Arc::new(AtomicBool::new(false)),
            quit_app_on_close: true,
        }
    }

    pub(super) fn close_window(&mut self, window_id: WindowId, control_flow: &mut ControlFlow) {
        self.webviews.remove(&window_id);

        if self.webviews.is_empty() && self.quit_app_on_close {
            *control_flow = ControlFlow::Exit;
        }
    }

    pub(super) fn try_load_ready_webviews(&mut self) {
        if self.is_ready.load(std::sync::atomic::Ordering::Relaxed) {
            let mut new_queue = Vec::new();

            {
                let mut queue = self.pending_edits.lock().unwrap();
                std::mem::swap(&mut new_queue, &mut *queue);
            }

            let (_id, view) = self.webviews.iter_mut().next().unwrap();

            for edit in new_queue.drain(..) {
                view.evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                    .unwrap();
            }
        }
    }
}

#[derive(Deserialize)]
struct EventMessage {
    contents: serde_json::Value,
    event: String,
    mounted_dom_id: usize,
}

fn decode_event(value: EventMessage) -> Option<Rc<dyn Any>> {
    let val = value.contents;
    let name = value.event.as_str();
    let el_id = ElementId(value.mounted_dom_id);
    type DragData = MouseData;

    let evt = match_data! { val; name;
        MouseData => "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter" | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup";
        ClipboardData => "copy" | "cut" | "paste";
        CompositionData => "compositionend" | "compositionstart" | "compositionupdate";
        KeyboardData => "keydown" | "keypress" | "keyup";
        FocusData => "blur" | "focus" | "focusin" | "focusout";
        FormData => "change" | "input" | "invalid" | "reset" | "submit";
        DragData => "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart" | "drop";
        PointerData => "pointerlockchange" | "pointerlockerror" | "pointerdown" | "pointermove" | "pointerup" | "pointerover" | "pointerout" | "pointerenter" | "pointerleave" | "gotpointercapture" | "lostpointercapture";
        SelectionData => "selectstart" | "selectionchange" | "select";
        TouchData => "touchcancel" | "touchend" | "touchmove" | "touchstart";
        ScrollData => "scroll";
        WheelData => "wheel";
        MediaData => "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied"
            | "encrypted" | "ended" | "interruptbegin" | "interruptend" | "loadeddata"
            | "loadedmetadata" | "loadstart" | "pause" | "play" | "playing" | "progress"
            | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend" | "timeupdate"
            | "volumechange" | "waiting" | "error" | "load" | "loadend" | "timeout";
        AnimationData => "animationstart" | "animationend" | "animationiteration";
        TransitionData => "transitionend";
        ToggleData => "toggle";
        // ImageData => "load" | "error";
        // OtherData => "abort" | "afterprint" | "beforeprint" | "beforeunload" | "hashchange" | "languagechange" | "message" | "offline" | "online" | "pagehide" | "pageshow" | "popstate" | "rejectionhandled" | "storage" | "unhandledrejection" | "unload" | "userproximity" | "vrdisplayactivate" | "vrdisplayblur" | "vrdisplayconnect" | "vrdisplaydeactivate" | "vrdisplaydisconnect" | "vrdisplayfocus" | "vrdisplaypointerrestricted" | "vrdisplaypointerunrestricted" | "vrdisplaypresentchange";
    };

    Some(evt)
}
