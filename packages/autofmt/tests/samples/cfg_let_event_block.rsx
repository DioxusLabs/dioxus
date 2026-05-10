rsx! {
    button {
        onclick: {
            #[cfg(target_os = "android")]
            let create_photo_collection_gallery = create_photo_collection.clone();

            move |_| {
                uploading.set(true);
                error.set(String::new());

                #[cfg(target_os = "android")]
                let create_photo_collection_gallery_call =
                    create_photo_collection_gallery.clone();

                if let Err(err) = Err::<(), _>("x") {
                    error.set(format!("{}: {}", "err", err));
                    uploading.set(false);
                    return;
                }
            }
        },
        "x"
    }
}
