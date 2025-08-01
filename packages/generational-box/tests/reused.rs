//! This test needs to be in its own file such that it doesn't share
//! an address space with the other tests.
//!
//! That will cause random failures on CI.

use generational_box::{Storage, SyncStorage, UnsyncStorage};

#[test]
fn reused() {
    fn reused_test<S: Storage<i32> + 'static>() {
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
