pub struct Store<T> {
    user_data: T,
}

/// Select just a small bit of the
fn use_selector() {}

/*

// borrow a closure so we can copy the reference
let dispatch = use_dispatch::<UserData>(ctx);


dispatch(|| UserData::Logout)

dispatch()

*/
fn use_dispatch() {}

mod tests {

    struct UserData {}

    // static SelectLoggedIn: FC<T> = |_| {};

    /*

    // Merge the various stores into a single context
    // This auto generates the appropriate selectors, reducing the need to wrap the app in excess providers
    let all = combine_stores(vec![store1, store2]);

    <Redux store={all}>

    </Redux>
    */
}
