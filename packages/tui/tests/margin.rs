use stretch2 as stretch;

#[test]
fn margin_and_flex_row() {
    let mut stretch = stretch::Stretch::new();
    let node0 = stretch
        .new_node(
            stretch::style::Style {
                flex_grow: 1f32,
                margin: stretch::geometry::Rect {
                    start: stretch::style::Dimension::Points(10f32),
                    end: stretch::style::Dimension::Points(10f32),
                    ..Default::default()
                },
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    let node = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size {
                    width: stretch::style::Dimension::Points(100f32),
                    height: stretch::style::Dimension::Points(100f32),
                    ..Default::default()
                },
                ..Default::default()
            },
            &[node0],
        )
        .unwrap();
    stretch
        .compute_layout(node, stretch::geometry::Size::undefined())
        .unwrap();
    assert_eq!(stretch.layout(node).unwrap().size.width, 100f32);
    assert_eq!(stretch.layout(node).unwrap().size.height, 100f32);
    assert_eq!(stretch.layout(node).unwrap().location.x, 0f32);
    assert_eq!(stretch.layout(node).unwrap().location.y, 0f32);
    assert_eq!(stretch.layout(node0).unwrap().size.width, 80f32);
    assert_eq!(stretch.layout(node0).unwrap().size.height, 100f32);
    assert_eq!(stretch.layout(node0).unwrap().location.x, 10f32);
    assert_eq!(stretch.layout(node0).unwrap().location.y, 0f32);
}

#[test]
fn margin_and_flex_row2() {
    let mut stretch = stretch::Stretch::new();
    let node0 = stretch
        .new_node(
            stretch::style::Style {
                flex_grow: 1f32,
                margin: stretch::geometry::Rect {
                    // left
                    start: stretch::style::Dimension::Points(10f32),

                    // right?
                    end: stretch::style::Dimension::Points(10f32),

                    // top?
                    // top: stretch::style::Dimension::Points(10f32),

                    // bottom?
                    // bottom: stretch::style::Dimension::Points(10f32),
                    ..Default::default()
                },
                ..Default::default()
            },
            &[],
        )
        .unwrap();

    let node = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size {
                    width: stretch::style::Dimension::Points(100f32),
                    height: stretch::style::Dimension::Points(100f32),
                    ..Default::default()
                },
                ..Default::default()
            },
            &[node0],
        )
        .unwrap();

    stretch
        .compute_layout(node, stretch::geometry::Size::undefined())
        .unwrap();

    assert_eq!(stretch.layout(node).unwrap().size.width, 100f32);
    assert_eq!(stretch.layout(node).unwrap().size.height, 100f32);
    assert_eq!(stretch.layout(node).unwrap().location.x, 0f32);
    assert_eq!(stretch.layout(node).unwrap().location.y, 0f32);

    dbg!(stretch.layout(node0));

    // assert_eq!(stretch.layout(node0).unwrap().size.width, 80f32);
    // assert_eq!(stretch.layout(node0).unwrap().size.height, 100f32);
    // assert_eq!(stretch.layout(node0).unwrap().location.x, 10f32);
    // assert_eq!(stretch.layout(node0).unwrap().location.y, 0f32);
}
