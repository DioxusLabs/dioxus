use futures_util::StreamExt;

/*
furtures_channel provides us some batching simply due to how Rust's async works.

Any hook that uses schedule_update is simply deferring to unbounded_send. Multiple
unbounded_sends can be linked together in succession provided there isn't an "await"
between them. Our internal batching mechanism simply waits for the "schedule_update"
to fire and then pulls any messages off the unbounded_send queue.

Additionally, due to how our "time slicing" works we'll always come back and check
in for new work if the deadline hasn't expired. On average, our deadline should be
about 10ms, which is way more than enough for diffing/creating to happen.
*/
#[async_std::test]
async fn batch() {
    let (sender, mut recver) = futures_channel::mpsc::unbounded::<i32>();

    let _handle = async_std::task::spawn(async move {
        let _msg = recver.next().await;
        while let Ok(msg) = recver.try_next() {
            println!("{:#?}", msg);
        }
        let _msg = recver.next().await;
        while let Ok(msg) = recver.try_next() {
            println!("{:#?}", msg);
        }
    });

    sender.unbounded_send(1).unwrap();
    sender.unbounded_send(2).unwrap();
    sender.unbounded_send(3).unwrap();
    sender.unbounded_send(4).unwrap();

    async_std::task::sleep(std::time::Duration::from_millis(100)).await;

    sender.unbounded_send(5).unwrap();
    sender.unbounded_send(6).unwrap();
    sender.unbounded_send(7).unwrap();
    sender.unbounded_send(8).unwrap();
}
