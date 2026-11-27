rsx! {
    button {
        onclick: move |_| {
            async move {
                let selected = AsyncFileDialog::new()
                    .set_title("Select local audio")
                    .add_filter("Audio", AUDIO_EXTENSIONS)
                    .pick_file()
                    .await;
                if let Some(file) = selected {
                    update(LocalFileAction::Import(file.path().to_path_buf()));
                } else {
                    busy.set(false);
                }
            }
        },
    }
    div {
        onclick: move |_| {
            let total = first_number_of_the_sum + second_number_of_the_sum
                + third_number_of_the_sum; // comment
            if total > 0 {
                count.set(total);
            } else {
                count.set(0);
            }
        },
    }
}
