use generational_box::{Storage, SyncStorage, UnsyncStorage};

#[test]
fn reference_counting() {
    fn reference_counting<S: Storage<String>>() {
        let data = String::from("hello world");
        let outer_owner = S::owner();
        let reference;
        {
            // create an owner
            let owner = S::owner();
            // insert data into the store
            let original = owner.insert_rc(data);
            reference = outer_owner.reference(original);
            // The reference should point to the value immediately
            assert_eq!(&*reference.read(), "hello world");
            // Original is dropped
        }
        // The reference should still point to the value
        assert_eq!(&*reference.read(), "hello world");
    }

    reference_counting::<UnsyncStorage>();
    reference_counting::<SyncStorage>();
}
