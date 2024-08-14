use generational_box::{Storage, SyncStorage, UnsyncStorage};

#[test]
fn reference_counting() {
    fn reference_counting<S: Storage<String>>() {
        let data = String::from("hello world");
        {
            // create an owner
            let owner = S::owner();
            // insert data into the store
            let original = owner.insert(data);
            let reference = original.reference();
            assert_eq!(
                reference.try_read().as_deref().unwrap(),
                &"hello world".to_string()
            );
        }
    }

    reference_counting::<UnsyncStorage>();
    reference_counting::<SyncStorage>();
}
