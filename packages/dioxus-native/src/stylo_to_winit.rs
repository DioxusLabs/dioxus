use style::values::computed::ui::CursorKind as StyloCursorKind;
use winit::window::CursorIcon as WinitCursor;

pub(crate) fn cursor(cursor: StyloCursorKind) -> WinitCursor {
    match cursor {
        StyloCursorKind::None => todo!("set the cursor to none"),
        StyloCursorKind::Default => WinitCursor::Default,
        StyloCursorKind::Pointer => WinitCursor::Pointer,
        StyloCursorKind::ContextMenu => WinitCursor::ContextMenu,
        StyloCursorKind::Help => WinitCursor::Help,
        StyloCursorKind::Progress => WinitCursor::Progress,
        StyloCursorKind::Wait => WinitCursor::Wait,
        StyloCursorKind::Cell => WinitCursor::Cell,
        StyloCursorKind::Crosshair => WinitCursor::Crosshair,
        StyloCursorKind::Text => WinitCursor::Text,
        StyloCursorKind::VerticalText => WinitCursor::VerticalText,
        StyloCursorKind::Alias => WinitCursor::Alias,
        StyloCursorKind::Copy => WinitCursor::Copy,
        StyloCursorKind::Move => WinitCursor::Move,
        StyloCursorKind::NoDrop => WinitCursor::NoDrop,
        StyloCursorKind::NotAllowed => WinitCursor::NotAllowed,
        StyloCursorKind::Grab => WinitCursor::Grab,
        StyloCursorKind::Grabbing => WinitCursor::Grabbing,
        StyloCursorKind::EResize => WinitCursor::EResize,
        StyloCursorKind::NResize => WinitCursor::NResize,
        StyloCursorKind::NeResize => WinitCursor::NeResize,
        StyloCursorKind::NwResize => WinitCursor::NwResize,
        StyloCursorKind::SResize => WinitCursor::SResize,
        StyloCursorKind::SeResize => WinitCursor::SeResize,
        StyloCursorKind::SwResize => WinitCursor::SwResize,
        StyloCursorKind::WResize => WinitCursor::WResize,
        StyloCursorKind::EwResize => WinitCursor::EwResize,
        StyloCursorKind::NsResize => WinitCursor::NsResize,
        StyloCursorKind::NeswResize => WinitCursor::NeswResize,
        StyloCursorKind::NwseResize => WinitCursor::NwseResize,
        StyloCursorKind::ColResize => WinitCursor::ColResize,
        StyloCursorKind::RowResize => WinitCursor::RowResize,
        StyloCursorKind::AllScroll => WinitCursor::AllScroll,
        StyloCursorKind::ZoomIn => WinitCursor::ZoomIn,
        StyloCursorKind::ZoomOut => WinitCursor::ZoomOut,
        StyloCursorKind::Auto => {
            // todo: we should be the ones determining this based on the UA?
            // https://developer.mozilla.org/en-US/docs/Web/CSS/cursor

            WinitCursor::Default
        }
    }
}
