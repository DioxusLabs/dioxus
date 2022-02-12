use dioxus::core::*;
use std::{collections::HashMap, io::Stdout};
use stretch2::{
    geometry::Point,
    prelude::{Layout, Size},
    Stretch,
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style as TuiStyle},
    widgets::Widget,
};

use crate::{BorderType, TuiNode, UnitSystem};

const RADIUS_MULTIPLIER: [f32; 2] = [1.0, 0.5];

impl<'a> Widget for TuiNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use tui::symbols::line::*;

        enum Direction {
            Left,
            Right,
            Up,
            Down,
        }

        fn draw(
            buf: &mut Buffer,
            points_history: [[i32; 2]; 3],
            symbols: &Set,
            pos: [u16; 2],
            color: &Option<Color>,
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

            let cell = buf.get_mut(
                (current[0] + pos[0] as i32) as u16,
                (current[1] + pos[1] as i32) as u16,
            );
            if let Some(c) = color {
                cell.fg = *c;
            }
            cell.symbol = match [start_dir, end_dir] {
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
        }

        fn draw_arc(
            pos: [u16; 2],
            starting_angle: f32,
            arc_angle: f32,
            radius: f32,
            symbols: &Set,
            buf: &mut Buffer,
            color: &Option<Color>,
        ) {
            if radius < 0.0 {
                return;
            }

            let num_points = (radius * arc_angle) as i32;
            let starting_point = [
                (starting_angle.cos() * (radius * RADIUS_MULTIPLIER[0])) as i32,
                (starting_angle.sin() * (radius * RADIUS_MULTIPLIER[1])) as i32,
            ];
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
                            &symbols,
                            pos,
                            color,
                        );
                        points_history = [points_history[1], connecting_point, points_history[2]];
                    }

                    draw(buf, points_history, &symbols, pos, color);
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

            draw(buf, points_history, &symbols, pos, color);
        }

        if area.area() == 0 {
            return;
        }

        for i in 0..4 {
            // the radius for the curve between this line and the next
            let r = match self.tui_modifier.border_types[(i + 1) % 4] {
                BorderType::HIDDEN => 0.0,
                BorderType::NONE => 0.0,
                _ => match self.tui_modifier.border_radi[i] {
                    UnitSystem::Percent(p) => p * area.width as f32 / 100.0,
                    UnitSystem::Point(p) => p,
                }
                .abs()
                .min((area.width as f32 / RADIUS_MULTIPLIER[0]) / 2.0)
                .min((area.height as f32 / RADIUS_MULTIPLIER[1]) / 2.0),
            };
            let radius = [
                (r * RADIUS_MULTIPLIER[0]) as u16,
                (r * RADIUS_MULTIPLIER[1]) as u16,
            ];

            // the radius for the curve between this line and the last
            let last_idx = if i == 0 { 3 } else { i - 1 };
            let last_r = match self.tui_modifier.border_types[last_idx] {
                BorderType::HIDDEN => 0.0,
                BorderType::NONE => 0.0,
                _ => match self.tui_modifier.border_radi[last_idx] {
                    UnitSystem::Percent(p) => p * area.width as f32 / 100.0,
                    UnitSystem::Point(p) => p,
                }
                .abs()
                .min((area.width as f32 / RADIUS_MULTIPLIER[0]) / 2.0)
                .min((area.height as f32 / RADIUS_MULTIPLIER[1]) / 2.0),
            };
            let last_radius = [
                (last_r * RADIUS_MULTIPLIER[0]) as u16,
                (last_r * RADIUS_MULTIPLIER[1]) as u16,
            ];

            let symbols = match self.tui_modifier.border_types[i] {
                BorderType::DOTTED => NORMAL,
                BorderType::DASHED => NORMAL,
                BorderType::SOLID => NORMAL,
                BorderType::DOUBLE => DOUBLE,
                BorderType::GROOVE => NORMAL,
                BorderType::RIDGE => NORMAL,
                BorderType::INSET => NORMAL,
                BorderType::OUTSET => NORMAL,
                BorderType::HIDDEN => continue,
                BorderType::NONE => continue,
            };

            let color = self.tui_modifier.border_colors[i].or(self.block_style.fg);

            match i {
                0 => {
                    for x in (area.left() + last_radius[0] + 1)..(area.right() - radius[0]) {
                        let cell = buf.get_mut(x, area.top());
                        if let Some(c) = color {
                            cell.fg = c;
                        }
                        cell.symbol = symbols.horizontal.to_string();
                    }
                }
                1 => {
                    for y in (area.top() + last_radius[1] + 1)..(area.bottom() - radius[1]) {
                        let cell = buf.get_mut(area.right() - 1, y);
                        if let Some(c) = color {
                            cell.fg = c;
                        }
                        cell.symbol = symbols.vertical.to_string();
                    }
                }
                2 => {
                    for x in (area.left() + radius[0])..(area.right() - last_radius[0] - 1) {
                        let cell = buf.get_mut(x, area.bottom() - 1);
                        if let Some(c) = color {
                            cell.fg = c;
                        }
                        cell.symbol = symbols.horizontal.to_string();
                    }
                }
                3 => {
                    for y in (area.top() + radius[1])..(area.bottom() - last_radius[1] - 1) {
                        let cell = buf.get_mut(area.left(), y);
                        if let Some(c) = color {
                            cell.fg = c;
                        }
                        cell.symbol = symbols.vertical.to_string();
                    }
                }
                _ => (),
            }

            match i {
                0 => draw_arc(
                    [area.right() - radius[0] - 1, area.top() + radius[1]],
                    std::f32::consts::FRAC_PI_2 * 3.0,
                    std::f32::consts::FRAC_PI_2,
                    r,
                    &symbols,
                    buf,
                    &color,
                ),
                1 => draw_arc(
                    [area.right() - radius[0] - 1, area.bottom() - radius[1] - 1],
                    0.0,
                    std::f32::consts::FRAC_PI_2,
                    r,
                    &symbols,
                    buf,
                    &color,
                ),
                2 => draw_arc(
                    [area.left() + radius[0], area.bottom() - radius[1] - 1],
                    std::f32::consts::FRAC_PI_2,
                    std::f32::consts::FRAC_PI_2,
                    r,
                    &symbols,
                    buf,
                    &color,
                ),
                3 => draw_arc(
                    [area.left() + radius[0], area.top() + radius[1]],
                    std::f32::consts::PI,
                    std::f32::consts::FRAC_PI_2,
                    r,
                    &symbols,
                    buf,
                    &color,
                ),
                _ => panic!("more than 4 sides?"),
            }
        }

        // todo: only render inside borders
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                let cell = buf.get_mut(x, y);
                if let Some(c) = self.block_style.bg {
                    cell.bg = c;
                }
            }
        }
    }
}

pub fn render_vnode<'a>(
    frame: &mut tui::Frame<CrosstermBackend<Stdout>>,
    layout: &Stretch,
    layouts: &mut HashMap<ElementId, TuiNode<'a>>,
    vdom: &'a VirtualDom,
    node: &'a VNode<'a>,
    // this holds the parents syle state for styled text rendering and potentially transparentcy
    style: &TuiStyle,
) {
    match node {
        VNode::Fragment(f) => {
            for child in f.children {
                render_vnode(frame, layout, layouts, vdom, child, style);
            }
            return;
        }

        VNode::Component(vcomp) => {
            let idx = vcomp.scope.get().unwrap();
            let new_node = vdom.get_scope(idx).unwrap().root_node();
            render_vnode(frame, layout, layouts, vdom, new_node, style);
            return;
        }

        VNode::Placeholder(_) => return,

        VNode::Element(_) | VNode::Text(_) => {}
    }

    let id = node.try_mounted_id().unwrap();
    let node = layouts.remove(&id).unwrap();

    let Layout { location, size, .. } = layout.layout(node.layout).unwrap();

    let Point { x, y } = location;
    let Size { width, height } = size;

    match node.node {
        VNode::Text(t) => {
            #[derive(Default)]
            struct Label<'a> {
                text: &'a str,
                style: TuiStyle,
            }

            impl<'a> Widget for Label<'a> {
                fn render(self, area: Rect, buf: &mut Buffer) {
                    buf.set_string(area.left(), area.top(), self.text, self.style);
                }
            }

            // let s = Span::raw(t.text);

            // Block::default().

            let label = Label {
                text: t.text,
                style: *style,
            };
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(label, area);
            }
        }
        VNode::Element(el) => {
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            let new_style = style.patch(node.block_style);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(node, area);
            }

            for el in el.children {
                render_vnode(frame, layout, layouts, vdom, el, &new_style);
            }
        }
        VNode::Fragment(_) => todo!(),
        VNode::Component(_) => todo!(),
        VNode::Placeholder(_) => todo!(),
    }
}
