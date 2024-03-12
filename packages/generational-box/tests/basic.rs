use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

/// # Example
///
/// ```compile_fail
/// let data = String::from("hello world");
/// let owner = UnsyncStorage::owner();
/// let key = owner.insert(&data);
/// drop(data);
/// assert_eq!(*key.read(), "hello world");
/// ```
#[allow(unused)]
fn compile_fail() {}

#[test]
fn reused() {
    let first_ptr;
    {
        let owner = UnsyncStorage::owner();
        first_ptr = owner.insert(1).raw_ptr();
        drop(owner);
    }
    {
        let owner = UnsyncStorage::owner();
        let second_ptr = owner.insert(1234).raw_ptr();
        assert_eq!(first_ptr, second_ptr);
        drop(owner);
    }
}

#[test]
fn leaking_is_ok() {
    let data = String::from("hello world");
    let key;
    {
        // create an owner
        let owner = UnsyncStorage::owner();
        // insert data into the store
        key = owner.insert(data);
        // don't drop the owner
        std::mem::forget(owner);
    }
    assert_eq!(
        key.try_read().as_deref().unwrap(),
        &"hello world".to_string()
    );
}

#[test]
fn drops() {
    let data = String::from("hello world");
    let key;
    {
        // create an owner
        let owner = UnsyncStorage::owner();
        // insert data into the store
        key = owner.insert(data);
        // drop the owner
    }
    assert!(key.try_read().is_err());
}

#[test]
fn works() {
    let owner = UnsyncStorage::owner();
    let key = owner.insert(1);

    assert_eq!(*key.read(), 1);
}

#[test]
fn insert_while_reading() {
    let owner = UnsyncStorage::owner();
    let key;
    {
        let data: String = "hello world".to_string();
        key = owner.insert(data);
    }
    let value = key.read();
    owner.insert(&1);
    assert_eq!(*value, "hello world");
}

#[test]
#[should_panic]
fn panics() {
    let owner = UnsyncStorage::owner();
    let key = owner.insert(1);
    drop(owner);

    assert_eq!(*key.read(), 1);
}

#[test]
fn fuzz() {
    fn maybe_owner_scope(
        valid_keys: &mut Vec<GenerationalBox<String>>,
        invalid_keys: &mut Vec<GenerationalBox<String>>,
        path: &mut Vec<u8>,
    ) {
        let branch_cutoff = 5;
        let children = if path.len() < branch_cutoff {
            rand::random::<u8>() % 4
        } else {
            rand::random::<u8>() % 2
        };

        for i in 0..children {
            let owner = UnsyncStorage::owner();
            let key = owner.insert(format!("hello world {path:?}"));
            valid_keys.push(key);
            path.push(i);
            // read all keys
            println!("{:?}", path);
            for key in valid_keys.iter() {
                let value = key.read();
                println!("{:?}", &*value);
                assert!(value.starts_with("hello world"));
            }
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            for key in invalid_keys.iter() {
                assert!(key.try_read().is_err());
            }
            maybe_owner_scope(valid_keys, invalid_keys, path);
            invalid_keys.push(valid_keys.pop().unwrap());
            path.pop();
        }
    }

    for _ in 0..10 {
        maybe_owner_scope(&mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }
}
