use generational_box::{Storage, SyncStorage, UnsyncStorage};

#[test]
fn reference_counting() {
    fn reference_counting<S: Storage<String> + 'static>() {
        let data = String::from("hello world");
        let reference;
        {
            let outer_owner = S::owner();
            {
                // create an owner
                let owner = S::owner();
                // insert data into the store
                let original = owner.insert_rc(data);
                reference = outer_owner.insert_reference(original).unwrap();
                // The reference should point to the value immediately
                assert_eq!(&*reference.read(), "hello world");
                // Original is dropped
            }
            // The reference should still point to the value
            assert_eq!(&*reference.read(), "hello world");
        }
        // Now that all references are dropped, the value should be dropped
        assert!(reference.try_read().is_err());
    }

    reference_counting::<UnsyncStorage>();
    reference_counting::<SyncStorage>();
}

#[test]
fn move_reference_in_place() {
    fn move_reference_in_place<S: Storage<String> + 'static>() {
        let data1 = String::from("hello world");
        let data2 = String::from("hello world 2");

        // create an owner
        let original_owner = S::owner();
        // insert data into the store
        let original = original_owner.insert_rc(data1.clone());
        let reference = original_owner.insert_reference(original).unwrap();
        // The reference should point to the original value
        assert_eq!(&*reference.read(), &data1);

        let new_owner = S::owner();
        // Move the reference in place
        let new = new_owner.insert_rc(data2.clone());
        reference.point_to(new).unwrap();
        // The reference should point to the new value
        assert_eq!(&*reference.read(), &data2);

        // make sure both got dropped
        drop(original_owner);
        drop(new_owner);
        assert!(original.try_read().is_err());
        assert!(new.try_read().is_err());
    }

    move_reference_in_place::<UnsyncStorage>();
    move_reference_in_place::<SyncStorage>();
}
