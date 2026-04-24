rsx! {
    {
        match state() {
            State::Loading => rsx! {
                LoadingScreen {}
            },
            State::Ready => rsx! {
                ReadyScreen { name }
            },
        }
    }
}
