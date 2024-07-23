use generational_box::{GenerationalBox, Storage, SyncStorage, UnsyncStorage};

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
    fn reused_test<S: Storage<i32>>() {
        let first_ptr;
        {
            let owner = S::owner();
            first_ptr = owner.insert(1).raw_ptr();
            drop(owner);
        }
        {
            let owner = S::owner();
            let second_ptr = owner.insert(1234).raw_ptr();
            assert_eq!(first_ptr, second_ptr);
            drop(owner);
        }
    }

    reused_test::<UnsyncStorage>();
    reused_test::<SyncStorage>();
}

#[test]
fn leaking_is_ok() {
    fn leaking_is_ok_test<S: Storage<String>>() {
        let data = String::from("hello world");
        let key;
        {
            // create an owner
            let owner = S::owner();
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

    leaking_is_ok_test::<UnsyncStorage>();
    leaking_is_ok_test::<SyncStorage>();
}

#[test]
fn drops() {
    fn drops_test<S: Storage<String>>() {
        let data = String::from("hello world");
        let key;
        {
            // create an owner
            let owner = S::owner();
            // insert data into the store
            key = owner.insert(data);
            // drop the owner
        }
        assert!(key.try_read().is_err());
    }

    drops_test::<UnsyncStorage>();
    drops_test::<SyncStorage>();
}

#[test]
fn works() {
    fn works_test<S: Storage<i32>>() {
        let owner = S::owner();
        let key = owner.insert(1);

        assert_eq!(*key.read(), 1);
    }

    works_test::<UnsyncStorage>();
    works_test::<SyncStorage>();
}

#[test]
fn insert_while_reading() {
    fn insert_while_reading_test<S: Storage<String> + Storage<&'static i32>>() {
        let owner = S::owner();
        let key;
        {
            let data: String = "hello world".to_string();
            key = owner.insert(data);
        }
        let value = key.read();
        owner.insert(&1);
        assert_eq!(*value, "hello world");
    }

    insert_while_reading_test::<UnsyncStorage>();
    insert_while_reading_test::<SyncStorage>();
}

#[test]
#[should_panic]
fn panics() {
    fn panics_test<S: Storage<i32>>() {
        let owner = S::owner();

        let key = owner.insert(1);
        drop(owner);

        assert_eq!(*key.read(), 1);
    }

    panics_test::<UnsyncStorage>();
    panics_test::<SyncStorage>();
}

#[test]
fn fuzz() {
    fn maybe_owner_scope<S: Storage<String>>(
        valid_keys: &mut Vec<GenerationalBox<String, S>>,
        invalid_keys: &mut Vec<GenerationalBox<String, S>>,
        path: &mut Vec<u8>,
    ) {
        let branch_cutoff = 5;
        let children = if path.len() < branch_cutoff {
            rand::random::<u8>() % 4
        } else {
            rand::random::<u8>() % 2
        };

        for i in 0..children {
            let owner = S::owner();
            let key = owner.insert(format!("hello world {path:?}"));
            println!("created new box {key:?}");
            valid_keys.push(key);
            path.push(i);
            // read all keys
            println!("{:?}", path);
            for key in valid_keys.iter() {
                println!("reading {key:?}");
                let value = key.read();
                println!("{:?}", &*value);
                assert!(value.starts_with("hello world"));
            }
            for key in invalid_keys.iter() {
                println!("reading invalid {key:?}");
                assert!(key.try_read().is_err());
            }
            maybe_owner_scope(valid_keys, invalid_keys, path);
            let invalid = valid_keys.pop().unwrap();
            println!("popping {invalid:?}");
            invalid_keys.push(invalid);
            path.pop();
        }
    }

    for _ in 0..10 {
        maybe_owner_scope::<UnsyncStorage>(&mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }

    for _ in 0..10 {
        maybe_owner_scope::<SyncStorage>(&mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }
}
