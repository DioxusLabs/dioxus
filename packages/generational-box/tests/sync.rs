// Regression test for https://github.com/DioxusLabs/dioxus/issues/2636

use std::time::Duration;

use generational_box::{AnyStorage, GenerationalBox, SyncStorage};

#[test]
fn race_condition_regression() {
    let handle = {
        let owner = SyncStorage::owner();
        let key = owner.insert(1u64);
        let handle = std::thread::spawn(move || reader(key));

        std::thread::sleep(Duration::from_secs(1));
        handle
        // owner is dropped now
    };
    // owner is *recycled*
    let owner = SyncStorage::owner();
    let _key = owner.insert(2u64);
    let _ = handle.join();
}

fn reader(key: GenerationalBox<u64, SyncStorage>) {
    for _ in 0..1000000 {
        match key.try_read() {
            Ok(value) => {
                if *value == 2 {
                    println!("CAUGHT A BAD ONE");
                } else {
                    // fine
                }
            }
            Err(err) => {
                eprintln!("bailing out - {err:?}");
                break;
            }
        }
    }
}
