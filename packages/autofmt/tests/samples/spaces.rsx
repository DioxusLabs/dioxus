rsx! {
    if let Some(Some(record)) = &*records.read_unchecked() {
        {
            let (label, value): (Vec<String>, Vec<f64>) = record.iter().rev().map(|d| (d.model.clone().unwrap(), d.row_total)).collect();

            rsx! {
                BarChart {
                    id: "bar-plot".to_string(),
                    x: value,
                    y: label,
                }
            }
        }
    }
}
