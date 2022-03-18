use stretch2 as stretch;

#[test]
fn relayout() {
    let mut stretch = stretch::Stretch::new();
    let node1 = stretch
        .new_node(
            stretch::style::Style {
                position: stretch::geometry::Point {
                    x: stretch::style::Dimension::Points(10f32),
                    y: stretch::style::Dimension::Points(10f32),
                },
                size: stretch::geometry::Size {
                    width: stretch::style::Dimension::Points(10f32),
                    height: stretch::style::Dimension::Points(10f32),
                },
                ..Default::default()
            },
            &[],
        )
        .unwrap();
    let node0 = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size {
                    width: stretch::style::Dimension::Percent(1f32),
                    height: stretch::style::Dimension::Percent(1f32),
                },
                ..Default::default()
            },
            &[node1],
        )
        .unwrap();
    let node = stretch
        .new_node(
            stretch::style::Style {
                size: stretch::geometry::Size {
                    width: stretch::style::Dimension::Points(100f32),
                    height: stretch::style::Dimension::Points(100f32),
                },
                ..Default::default()
            },
            &[node0],
        )
        .unwrap();
    for _ in 0..10 {
        stretch
            .compute_layout(node, stretch::geometry::Size::undefined())
            .unwrap();
        assert_eq!(stretch.layout(node).unwrap().size.width, 100f32);
        assert_eq!(stretch.layout(node).unwrap().size.height, 100f32);
        assert_eq!(stretch.layout(node).unwrap().location.x, 0f32);
        assert_eq!(stretch.layout(node).unwrap().location.y, 0f32);
        assert_eq!(stretch.layout(node1).unwrap().size.width, 10f32);
        assert_eq!(stretch.layout(node1).unwrap().size.height, 10f32);
        assert_eq!(stretch.layout(node1).unwrap().location.x, 0f32);
        assert_eq!(stretch.layout(node1).unwrap().location.y, 0f32);
        assert_eq!(stretch.layout(node0).unwrap().size.width, 100f32);
        assert_eq!(stretch.layout(node0).unwrap().size.height, 100f32);
        assert_eq!(stretch.layout(node0).unwrap().location.x, 0f32);
        assert_eq!(stretch.layout(node0).unwrap().location.y, 0f32);
    }
}
