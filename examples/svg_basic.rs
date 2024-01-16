use dioxus::prelude::*;

fn app() -> Element {
    rsx! {
        svg {
            width: "200",
            height: "250",
            xmlns: "http://www.w3.org/2000/svg",
            version: "1.1",
            rect {
                x: "10",
                y: "10",
                width: "30",
                height: "30",
                stroke: "black",
                fill: "transparent",
                stroke_width: "5",
            }
            rect {
                x: "60",
                y: "10",
                width: "30",
                height: "30",
                stroke: "black",
                fill: "transparent",
                stroke_width: "5",
            }
            circle {
                cx: "25",
                cy: "75",
                r: "20",
                stroke: "red",
                fill: "transparent",
                stroke_width: "5",
            }
            ellipse {
                cx: "75",
                cy: "75",
                rx: "20",
                ry: "5",
                stroke: "red",
                fill: "transparent",
                stroke_width: "5",
            }
            line {
                x1: "10",
                x2: "50",
                y1: "110",
                y2: "150",
                stroke: "orange",
                stroke_width: "5",
            }
            polyline {
                points: "60 110 65 120 70 115 75 130 80 125 85 140 90 135 95 150 100 145",
                stroke: "orange",
                fill: "transparent",
                stroke_width: "5",
            }
            polygon {
                points: "50 160 55 180 70 180 60 190 65 205 50 195 35 205 40 190 30 180 45 180",
                stroke: "green",
                fill: "transparent",
                stroke_width: "5",
            }
            path {
                d: "M20,230 Q40,205 50,230 T90,230",
                fill: "none",
                stroke: "blue",
                stroke_width: "5",
            }
            path {
                d: "M9.00001 9C9 62 103.5 124 103.5 178",
                stroke: "#3CC4DC",
                "stroke-linecap": "square",
                "stroke-width": "square",
            }
        }
    }
}

fn main() {
    launch_desktop(app);
}
