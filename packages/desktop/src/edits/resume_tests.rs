use super::*;
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

fn connect(queue: &WryQueue) -> WebSocket<TcpStream> {
    let (url, expected_key) = queue.connection_details();
    let authority = url
        .strip_prefix("ws://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let (mut socket, _) = tungstenite::client(url.as_str(), stream).unwrap();
    assert_eq!(socket.read().unwrap(), Message::Text(expected_key.into()));
    socket
}

fn read_edits(socket: &mut WebSocket<TcpStream>) -> Vec<u8> {
    match socket
        .read()
        .expect("resumed webview must receive UI updates")
    {
        Message::Binary(bytes) => bytes.to_vec(),
        message => panic!("expected UI updates, got {message:?}"),
    }
}

fn acknowledge(socket: &mut WebSocket<TcpStream>, frame: &[u8]) {
    socket
        .send(Message::Binary(frame[..8].to_vec().into()))
        .unwrap();
}

fn wait_for_ack(mut ack: oneshot::Receiver<()>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if ack.try_recv().unwrap().is_some() {
            return;
        }
        assert!(Instant::now() < deadline, "render acknowledgement stalled");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn idle_webview_can_reconnect_before_the_next_edit() {
    let mut server = EditWebsocket::start();
    let queue = server.create_queue();
    let mut socket = connect(&queue);
    let ack = server.send_edits(0, vec![1, 2, 3]);
    let frame = read_edits(&mut socket);
    acknowledge(&mut socket, &frame);
    wait_for_ack(ack);
    drop(socket);
    let mut resumed = connect(&queue);
    let ack = server.send_edits(0, vec![4, 5, 6]);
    let frame = read_edits(&mut resumed);
    assert_eq!(&frame[8..], &[4, 5, 6]);
    acknowledge(&mut resumed, &frame);
    wait_for_ack(ack);
}

#[test]
fn interrupted_updates_are_replayed_until_acknowledged() {
    let mut server = EditWebsocket::start();
    let queue = server.create_queue();
    let mut socket = connect(&queue);
    let mut ack = server.send_edits(0, vec![7, 8, 9]);
    let original = read_edits(&mut socket);
    drop(socket);
    let mut resumed = connect(&queue);
    assert_eq!(read_edits(&mut resumed), original);
    assert_eq!(ack.try_recv().unwrap(), None);
    acknowledge(&mut resumed, &original);
    wait_for_ack(ack);
    let ack = server.send_edits(0, vec![10]);
    let next = read_edits(&mut resumed);
    assert_ne!(&next[..8], &original[..8]);
    assert_eq!(&next[8..], &[10]);
    acknowledge(&mut resumed, &next);
    wait_for_ack(ack);
}

#[test]
fn repeated_reconnections_cannot_drop_or_stall_updates() {
    let mut server = EditWebsocket::start();
    let queue = server.create_queue();
    let mut socket = connect(&queue);
    for value in 0..20 {
        let ack = server.send_edits(0, vec![value]);
        let original = read_edits(&mut socket);
        // Reconnect while the old socket is still open and awaiting ACK.
        let mut resumed = connect(&queue);
        assert_eq!(read_edits(&mut resumed), original);
        acknowledge(&mut resumed, &original);
        wait_for_ack(ack);
        socket = resumed;
    }
}

#[test]
fn invalid_key_does_not_replace_the_active_webview() {
    let mut server = EditWebsocket::start();
    let queue = server.create_queue();
    let mut socket = connect(&queue);
    let (url, _) = queue.connection_details();
    let authority = url
        .strip_prefix("ws://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let bad_url = format!("ws://{authority}/0/invalid-key");
    let stream = TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    assert!(tungstenite::client(bad_url.as_str(), stream).is_err());
    let ack = server.send_edits(0, vec![42]);
    let frame = read_edits(&mut socket);
    assert_eq!(&frame[8..], &[42]);
    acknowledge(&mut socket, &frame);
    wait_for_ack(ack);
}

#[test]
fn listener_restart_preserves_pending_edits_with_new_credentials() {
    let mut server = EditWebsocket::start();
    let queue = server.create_queue();
    let old_details = queue.connection_details();
    let mut socket = connect(&queue);
    let ack = server.send_edits(0, vec![42]);
    let original = read_edits(&mut socket);
    drop(socket);

    let (location, listener) = start_server();
    *server.current_location.lock().unwrap() = location;
    server.server_location.notify_waiters();
    let location = server.current_location.clone();
    let connections = server.connections.clone();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        EditWebsocket::handle_connection(stream, location, connections);
    });
    assert_ne!(queue.connection_details(), old_details);
    let waker = futures_util::task::noop_waker();
    let mut context = std::task::Context::from_waker(&waker);
    assert!(queue.poll_new_edits_location(&mut context).is_ready());
    let mut resumed = connect(&queue);
    assert_eq!(read_edits(&mut resumed), original);
    acknowledge(&mut resumed, &original);
    wait_for_ack(ack);
}
