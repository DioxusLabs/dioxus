use futures_channel::mpsc::unbounded;

#[async_std::test]
async fn channels() {
    let (sender, mut receiver) = unbounded::<u32>();

    // drop(sender);

    match receiver.try_next() {
        Ok(a) => {
            dbg!(a);
        }
        Err(no) => {
            dbg!(no);
        }
    }

    sender.unbounded_send(1).unwrap();
}
