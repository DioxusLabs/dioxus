use dioxus_native_core::layout_attributes::UnitSystem;
use std::io::Stdout;
use stretch2::{
    geometry::Point,
    prelude::{Layout, Size},
    Stretch,
};
use tui::{backend::CrosstermBackend, layout::Rect};

use crate::{
    style::{RinkColor, RinkStyle},
    style_attributes::{BorderEdge, BorderStyle},
    widget::{RinkBuffer, RinkCell, RinkWidget, WidgetWithContext},
    Config, Dom, Node,
};

const RADIUS_MULTIPLIER: [f32; 2] = [1.0, 0.5];

pub(crate) fn render_vnode(
    frame: &mut tui::Frame<CrosstermBackend<Stdout>>,
    layout: &Stretch,
    rdom: &Dom,
    node: &Node,
    cfg: Config,
) {
    use dioxus_native_core::real_dom::NodeType;

    if let NodeType::Placeholder = &node.node_type {
        return;
    }

    let Layout { location, size, .. } = layout.layout(node.state.layout.node.unwrap()).unwrap();
    // println!("rendering {node:?} {location:?} {size:?}");

    let Point { x, y } = location;
    let Size { width, height } = size;

    match &node.node_type {
        NodeType::Text { text } => {
            #[derive(Default)]
            struct Label<'a> {
                text: &'a str,
                style: RinkStyle,
            }

            impl<'a> RinkWidget for Label<'a> {
                fn render(self, area: Rect, mut buf: RinkBuffer) {
                    for (i, c) in self.text.char_indices() {
                        let mut new_cell = RinkCell::default();
                        new_cell.set_style(self.style);
                        new_cell.symbol = c.to_string();
                        buf.set(area.left() + i as u16, area.top(), new_cell);
                    }
                }
            }

            let label = Label {
                text,
                style: node.state.style.style,
            };
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(WidgetWithContext::new(label, cfg), area);
            }
        }
        NodeType::Element { children, .. } => {
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(WidgetWithContext::new(node, cfg), area);
            }

            for c in children {
                render_vnode(frame, layout, rdom, &rdom[c.0], cfg);
            }
        }
        NodeType::Placeholder => unreachable!(),
    }
}

impl RinkWidget for &Node {
    fn render(self, area: Rect, mut buf: RinkBuffer<'_>) {
        use tui::symbols::line::*;

        enum Direction {
            Left,
            Right,
            Up,
            Down,
        }

        fn draw(
            buf: &mut RinkBuffer,
            points_history: [[i32; 2]; 3],
            symbols: &Set,
            pos: [u16; 2],
            color: &Option<RinkColor>,
        ) {
            let [before, current, after] = points_history;
            let start_dir = match [before[0] - current[0], before[1] - current[1]] {
                [1, 0] => Direction::Right,
                [-1, 0] => Direction::Left,
                [0, 1] => Direction::Down,
                [0, -1] => Direction::Up,
                [a, b] => {
                    panic!(
                        "draw({:?} {:?} {:?}) {}, {} no cell adjacent",
                        before, current, after, a, b
                    )
                }
            };
            let end_dir = match [after[0] - current[0], after[1] - current[1]] {
                [1, 0] => Direction::Right,
                [-1, 0] => Direction::Left,
                [0, 1] => Direction::Down,
                [0, -1] => Direction::Up,
                [a, b] => {
                    panic!(
                        "draw({:?} {:?} {:?}) {}, {} no cell adjacent",
                        before, current, after, a, b
                    )
                }
            };

            let mut new_cell = RinkCell::default();
            if let Some(c) = color {
                new_cell.fg = *c;
            }
            new_cell.symbol = match [start_dir, end_dir] {
                [Direction::Down, Direction::Up] => symbols.vertical,
                [Direction::Down, Direction::Right] => symbols.top_left,
                [Direction::Down, Direction::Left] => symbols.top_right,
                [Direction::Up, Direction::Down] => symbols.vertical,
                [Direction::Up, Direction::Right] => symbols.bottom_left,
                [Direction::Up, Direction::Left] => symbols.bottom_right,
                [Direction::Right, Direction::Left] => symbols.horizontal,
                [Direction::Right, Direction::Up] => symbols.bottom_left,
                [Direction::Right, Direction::Down] => symbols.top_left,
                [Direction::Left, Direction::Up] => symbols.bottom_right,
                [Direction::Left, Direction::Right] => symbols.horizontal,
                [Direction::Left, Direction::Down] => symbols.top_right,
                _ => panic!(
                    "{:?} {:?} {:?} cannont connect cell to itself",
                    before, current, after
                ),
            }
            .to_string();
            buf.set(
                (current[0] + pos[0] as i32) as u16,
                (current[1] + pos[1] as i32) as u16,
                new_cell,
            );
        }

        fn draw_arc(
            pos: [u16; 2],
            starting_angle: f32,
            arc_angle: f32,
            radius: f32,
            symbols: &Set,
            buf: &mut RinkBuffer,
            color: &Option<RinkColor>,
        ) {
            if radius < 0.0 {
                return;
            }

            let num_points = (radius * arc_angle) as i32;
            let starting_point = [
                (starting_angle.cos() * (radius * RADIUS_MULTIPLIER[0])) as i32,
                (starting_angle.sin() * (radius * RADIUS_MULTIPLIER[1])) as i32,
            ];
            // keep track of the last 3 point to allow filling diagonals
            let mut points_history = [
                [0, 0],
                {
                    // change the x or y value based on which one is changing quicker
                    let ddx = -starting_angle.sin();
                    let ddy = starting_angle.cos();
                    if ddx.abs() > ddy.abs() {
                        [starting_point[0] - ddx.signum() as i32, starting_point[1]]
                    } else {
                        [starting_point[0], starting_point[1] - ddy.signum() as i32]
                    }
                },
                starting_point,
            ];

            for i in 1..=num_points {
                let angle = (i as f32 / num_points as f32) * arc_angle + starting_angle;
                let x = angle.cos() * radius * RADIUS_MULTIPLIER[0];
                let y = angle.sin() * radius * RADIUS_MULTIPLIER[1];
                let new = [x as i32, y as i32];

                if new != points_history[2] {
                    points_history = [points_history[1], points_history[2], new];

                    let dx = points_history[2][0] - points_history[1][0];
                    let dy = points_history[2][1] - points_history[1][1];
                    // fill diagonals
                    if dx != 0 && dy != 0 {
                        let connecting_point = match [dx, dy] {
                            [1, 1] => [points_history[1][0] + 1, points_history[1][1]],
                            [1, -1] => [points_history[1][0], points_history[1][1] - 1],
                            [-1, 1] => [points_history[1][0], points_history[1][1] + 1],
                            [-1, -1] => [points_history[1][0] - 1, points_history[1][1]],
                            _ => todo!(),
                        };
                        draw(
                            buf,
                            [points_history[0], points_history[1], connecting_point],
                            symbols,
                            pos,
                            color,
                        );
                        points_history = [points_history[1], connecting_point, points_history[2]];
                    }

                    draw(buf, points_history, symbols, pos, color);
                }
            }

            points_history = [points_history[1], points_history[2], {
                // change the x or y value based on which one is changing quicker
                let ddx = -(starting_angle + arc_angle).sin();
                let ddy = (starting_angle + arc_angle).cos();
                if ddx.abs() > ddy.abs() {
                    [
                        points_history[2][0] + ddx.signum() as i32,
                        points_history[2][1],
                    ]
                } else {
                    [
                        points_history[2][0],
                        points_history[2][1] + ddy.signum() as i32,
                    ]
                }
            }];

            draw(buf, points_history, symbols, pos, color);
        }

        fn get_radius(border: &BorderEdge, area: Rect) -> f32 {
            match border.style {
                BorderStyle::Hidden => 0.0,
                BorderStyle::None => 0.0,
                _ => match border.radius {
                    UnitSystem::Percent(p) => p * area.width as f32 / 100.0,
                    UnitSystem::Point(p) => p,
                }
                .abs()
                .min((area.width as f32 / RADIUS_MULTIPLIER[0]) / 2.0)
                .min((area.height as f32 / RADIUS_MULTIPLIER[1]) / 2.0),
            }
        }

        if area.area() == 0 {
            return;
        }

        // todo: only render inside borders
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                let mut new_cell = RinkCell::default();
                if let Some(c) = self.state.style.style.bg {
                    new_cell.bg = c;
                }
                buf.set(x, y, new_cell);
            }
        }

        let borders = &self.state.style.modifier.borders;

        let last_edge = &borders.left;
        let current_edge = &borders.top;
        if let Some(symbols) = current_edge.style.symbol_set() {
            // the radius for the curve between this line and the next
            let r = get_radius(current_edge, area);
            let radius = [
                (r * RADIUS_MULTIPLIER[0]) as u16,
                (r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            // the radius for the curve between this line and the last
            let last_r = get_radius(last_edge, area);
            let last_radius = [
                (last_r * RADIUS_MULTIPLIER[0]) as u16,
                (last_r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            let color = current_edge.color.or(self.state.style.style.fg);
            let mut new_cell = RinkCell::default();
            if let Some(c) = color {
                new_cell.fg = c;
            }
            for x in (area.left() + last_radius[0] + 1)..(area.right() - radius[0]) {
                new_cell.symbol = symbols.horizontal.to_string();
                buf.set(x, area.top(), new_cell.clone());
            }
            draw_arc(
                [area.right() - radius[0] - 1, area.top() + radius[1]],
                std::f32::consts::FRAC_PI_2 * 3.0,
                std::f32::consts::FRAC_PI_2,
                r,
                &symbols,
                &mut buf,
                &color,
            );
        }

        let last_edge = &borders.top;
        let current_edge = &borders.right;
        if let Some(symbols) = current_edge.style.symbol_set() {
            // the radius for the curve between this line and the next
            let r = get_radius(current_edge, area);
            let radius = [
                (r * RADIUS_MULTIPLIER[0]) as u16,
                (r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            // the radius for the curve between this line and the last
            let last_r = get_radius(last_edge, area);
            let last_radius = [
                (last_r * RADIUS_MULTIPLIER[0]) as u16,
                (last_r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            let color = current_edge.color.or(self.state.style.style.fg);
            let mut new_cell = RinkCell::default();
            if let Some(c) = color {
                new_cell.fg = c;
            }
            for y in (area.top() + last_radius[1] + 1)..(area.bottom() - radius[1]) {
                new_cell.symbol = symbols.vertical.to_string();
                buf.set(area.right() - 1, y, new_cell.clone());
            }
            draw_arc(
                [area.right() - radius[0] - 1, area.bottom() - radius[1] - 1],
                0.0,
                std::f32::consts::FRAC_PI_2,
                r,
                &symbols,
                &mut buf,
                &color,
            );
        }

        let last_edge = &borders.right;
        let current_edge = &borders.bottom;
        if let Some(symbols) = current_edge.style.symbol_set() {
            // the radius for the curve between this line and the next
            let r = get_radius(current_edge, area);
            let radius = [
                (r * RADIUS_MULTIPLIER[0]) as u16,
                (r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            // the radius for the curve between this line and the last
            let last_r = get_radius(last_edge, area);
            let last_radius = [
                (last_r * RADIUS_MULTIPLIER[0]) as u16,
                (last_r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            let color = current_edge.color.or(self.state.style.style.fg);
            let mut new_cell = RinkCell::default();
            if let Some(c) = color {
                new_cell.fg = c;
            }
            for x in (area.left() + radius[0])..(area.right() - last_radius[0] - 1) {
                new_cell.symbol = symbols.horizontal.to_string();
                buf.set(x, area.bottom() - 1, new_cell.clone());
            }
            draw_arc(
                [area.left() + radius[0], area.bottom() - radius[1] - 1],
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::FRAC_PI_2,
                r,
                &symbols,
                &mut buf,
                &color,
            );
        }

        let last_edge = &borders.bottom;
        let current_edge = &borders.left;
        if let Some(symbols) = current_edge.style.symbol_set() {
            // the radius for the curve between this line and the next
            let r = get_radius(current_edge, area);
            let radius = [
                (r * RADIUS_MULTIPLIER[0]) as u16,
                (r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            // the radius for the curve between this line and the last
            let last_r = get_radius(last_edge, area);
            let last_radius = [
                (last_r * RADIUS_MULTIPLIER[0]) as u16,
                (last_r * RADIUS_MULTIPLIER[1]) as u16,
            ];
            let color = current_edge.color.or(self.state.style.style.fg);
            let mut new_cell = RinkCell::default();
            if let Some(c) = color {
                new_cell.fg = c;
            }
            for y in (area.top() + radius[1])..(area.bottom() - last_radius[1] - 1) {
                new_cell.symbol = symbols.vertical.to_string();
                buf.set(area.left(), y, new_cell.clone());
            }
            draw_arc(
                [area.left() + radius[0], area.top() + radius[1]],
                std::f32::consts::PI,
                std::f32::consts::FRAC_PI_2,
                r,
                &symbols,
                &mut buf,
                &color,
            );
        }
    }
}
