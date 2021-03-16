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

struct Context {
    data: String,
    logged_in: bool,
}

// "static" selectors automatically get memoized
static SelectUserName: Selector<&str> = |root: Context| root.data.as_str();
static SelectUserName: Selector<bool> = |root: Context| root.data.logged_in;

fn main() {
    /*
    use_context is very unsafe! It essentially exposes your data in an unsafecell where &mut T and &T can exist at the same time. It's up to *you* the library implmenetor to make this safe.

    We provide a redux-style system and a recoil-style system that are both saf

    */

    // connect itsy bits of state together
    let context = use_create_context(
        ctx,
        ContextBuilder::new()
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .with_root(|| Context {})
            .build(),
    );

    let g: HashMap<TypeId, Box<dyn Any>> = HashMap::new();
}
