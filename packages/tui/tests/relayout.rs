use stretch::style::Dimension;
use stretch2 as stretch;

#[test]
fn relayout() {
    let mut stretch = stretch::Stretch::new();
    let node1 = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size { width: Dimension::Points(8f32), height: Dimension::Points(80f32) },
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    let node0 = stretch
        .new_node(
            stretch::style::Style {
                align_self: stretch::prelude::AlignSelf::Center,
                size: stretch::geometry::Size { width: Dimension::Auto, height: Dimension::Auto },
                // size: stretch::geometry::Size { width: Dimension::Percent(1.0), height: Dimension::Percent(1.0) },
                ..Default::default()
            },
            &[node1],
        )
        .unwrap();
    let node = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size { width: Dimension::Percent(1f32), height: Dimension::Percent(1f32) },
                ..Default::default()
            },
            &[node0],
        )
        .unwrap();
    println!("0:");
    stretch
        .compute_layout(
            node,
            stretch::geometry::Size {
                width: stretch::prelude::Number::Defined(100f32),
                height: stretch::prelude::Number::Defined(100f32),
            },
        )
        .unwrap();
    let initial = stretch.layout(node).unwrap().location;
    let initial0 = stretch.layout(node0).unwrap().location;
    let initial1 = stretch.layout(node1).unwrap().location;
    for i in 1..10 {
        println!("\n\n{i}:");
        stretch
            .compute_layout(
                node,
                stretch::geometry::Size {
                    width: stretch::prelude::Number::Defined(100f32),
                    height: stretch::prelude::Number::Defined(100f32),
                },
            )
            .unwrap();
        assert_eq!(stretch.layout(node).unwrap().location, initial);
        assert_eq!(stretch.layout(node0).unwrap().location, initial0);
        assert_eq!(stretch.layout(node1).unwrap().location, initial1);
    }
}
