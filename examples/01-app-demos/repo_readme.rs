use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};

// ---------- one / maybe / get traits ----------

pub trait ReadOne {
    type Target: ?Sized;
    fn read(&self) -> Ref<'_, Self::Target>;
}

pub trait WriteOne: ReadOne {
    fn write(&self) -> RefMut<'_, Self::Target>;
}

pub trait ReadMaybe {
    type Target: ?Sized;
    fn read_opt(&self) -> Option<Ref<'_, Self::Target>>;
}

pub trait WriteMaybe: ReadMaybe {
    fn write_opt(&self) -> Option<RefMut<'_, Self::Target>>;
}

pub trait GetOne {
    type Value;
    fn get(&self) -> Self::Value;
}

pub trait GetMaybe {
    type Value;
    fn get_opt(&self) -> Option<Self::Value>;
}

// ---------- many traits ----------

pub trait Many {
    type Item;

    fn iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_>;
}

pub trait ManyGet {
    type Value;

    fn iter_values(&self) -> Box<dyn Iterator<Item = Self::Value> + '_>;
}

// ---------- signal wrapper ----------

#[derive(Clone)]
pub struct Signal<A> {
    access: A,
}

impl<T> Signal<RwRoot<T>> {
    pub fn new(value: T) -> Self {
        Self {
            access: RwRoot {
                cell: Rc::new(RefCell::new(value)),
            },
        }
    }
}

impl<A> Signal<A> {
    pub fn field<S: ?Sized, U: ?Sized>(
        self,
        get: fn(&S) -> &U,
        get_mut: fn(&mut S) -> &mut U,
    ) -> Signal<Field<A, S, U>> {
        Signal {
            access: Field {
                parent: self.access,
                get,
                get_mut,
            },
        }
    }

    pub fn some<T>(self) -> Signal<SomeOpt<A, T>> {
        Signal {
            access: SomeOpt {
                parent: self.access,
                _marker: PhantomData,
            },
        }
    }

    pub fn get_map<S: ?Sized, U>(self, get: fn(&S) -> U) -> Signal<GetMap<A, S, U>> {
        Signal {
            access: GetMap {
                parent: self.access,
                get,
            },
        }
    }

    pub fn each_vec<T>(self) -> Signal<EachVec<A, T>> {
        Signal {
            access: EachVec {
                parent: self.access,
                _marker: PhantomData,
            },
        }
    }

    pub fn map_items<Q, U>(self, f: fn(Q) -> U) -> Signal<MapMany<A, Q, U>> {
        Signal {
            access: MapMany {
                parent: self.access,
                f,
                _marker: PhantomData,
            },
        }
    }
}

// one
impl<A> Signal<A>
where
    A: ReadOne,
{
    pub fn read(&self) -> Ref<'_, A::Target> {
        self.access.read()
    }
}

impl<A> Signal<A>
where
    A: WriteOne,
{
    pub fn write(&self) -> RefMut<'_, A::Target> {
        self.access.write()
    }
}

// maybe
impl<A> Signal<A>
where
    A: ReadMaybe,
{
    pub fn read_opt(&self) -> Option<Ref<'_, A::Target>> {
        self.access.read_opt()
    }
}

impl<A> Signal<A>
where
    A: WriteMaybe,
{
    pub fn write_opt(&self) -> Option<RefMut<'_, A::Target>> {
        self.access.write_opt()
    }
}

// get
impl<A> Signal<A>
where
    A: GetOne,
{
    pub fn get(&self) -> A::Value {
        self.access.get()
    }
}

impl<A> Signal<A>
where
    A: GetMaybe,
{
    pub fn get_opt(&self) -> Option<A::Value> {
        self.access.get_opt()
    }
}

// many
impl<A> Signal<A>
where
    A: Many,
{
    pub fn iter(&self) -> Box<dyn Iterator<Item = A::Item> + '_> {
        self.access.iter()
    }
}

impl<A> Signal<A>
where
    A: ManyGet,
{
    pub fn iter_values(&self) -> Box<dyn Iterator<Item = A::Value> + '_> {
        self.access.iter_values()
    }
}

// ---------- base rw root ----------

pub struct RwRoot<T> {
    cell: Rc<RefCell<T>>,
}

impl<T> Clone for RwRoot<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T> ReadOne for RwRoot<T> {
    type Target = T;

    fn read(&self) -> Ref<'_, T> {
        self.cell.borrow()
    }
}

impl<T> WriteOne for RwRoot<T> {
    fn write(&self) -> RefMut<'_, T> {
        self.cell.borrow_mut()
    }
}

// ---------- field ----------

pub struct Field<A, S: ?Sized, U: ?Sized> {
    parent: A,
    get: fn(&S) -> &U,
    get_mut: fn(&mut S) -> &mut U,
}

impl<A, S: ?Sized, U: ?Sized> Clone for Field<A, S, U>
where
    A: Clone,
{
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            get: self.get,
            get_mut: self.get_mut,
        }
    }
}

impl<A, S: ?Sized, U: ?Sized> ReadOne for Field<A, S, U>
where
    A: ReadOne<Target = S>,
{
    type Target = U;

    fn read(&self) -> Ref<'_, U> {
        Ref::map(self.parent.read(), self.get)
    }
}

impl<A, S: ?Sized, U: ?Sized> WriteOne for Field<A, S, U>
where
    A: WriteOne<Target = S>,
{
    fn write(&self) -> RefMut<'_, U> {
        RefMut::map(self.parent.write(), self.get_mut)
    }
}

impl<A, S: ?Sized, U: ?Sized> ReadMaybe for Field<A, S, U>
where
    A: ReadMaybe<Target = S>,
{
    type Target = U;

    fn read_opt(&self) -> Option<Ref<'_, U>> {
        self.parent.read_opt().map(|r| Ref::map(r, self.get))
    }
}

impl<A, S: ?Sized, U: ?Sized> WriteMaybe for Field<A, S, U>
where
    A: WriteMaybe<Target = S>,
{
    fn write_opt(&self) -> Option<RefMut<'_, U>> {
        self.parent
            .write_opt()
            .map(|r| RefMut::map(r, self.get_mut))
    }
}

// ---------- some prism ----------

#[derive(Clone)]
pub struct SomeOpt<A, T> {
    parent: A,
    _marker: PhantomData<fn() -> T>,
}

impl<A, T> ReadMaybe for SomeOpt<A, T>
where
    A: ReadOne<Target = Option<T>>,
{
    type Target = T;

    fn read_opt(&self) -> Option<Ref<'_, T>> {
        Ref::filter_map(self.parent.read(), |o| o.as_ref()).ok()
    }
}

impl<A, T> WriteMaybe for SomeOpt<A, T>
where
    A: WriteOne<Target = Option<T>>,
{
    fn write_opt(&self) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(self.parent.write(), |o| o.as_mut()).ok()
    }
}

// ---------- getter ----------

#[derive(Clone)]
pub struct GetMap<A, S: ?Sized, U> {
    parent: A,
    get: fn(&S) -> U,
}

impl<A, S: ?Sized, U> GetOne for GetMap<A, S, U>
where
    A: ReadOne<Target = S>,
{
    type Value = U;

    fn get(&self) -> U {
        (self.get)(&*self.parent.read())
    }
}

impl<A, S: ?Sized, U> GetMaybe for GetMap<A, S, U>
where
    A: ReadMaybe<Target = S>,
{
    type Value = U;

    fn get_opt(&self) -> Option<U> {
        self.parent.read_opt().map(|r| (self.get)(&*r))
    }
}

// ---------- many over Vec<T> ----------

#[derive(Clone)]
pub struct EachVec<A, T> {
    parent: A,
    _marker: PhantomData<fn() -> T>,
}

#[derive(Clone)]
pub struct VecIndex<A, T> {
    parent: A,
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<A, T> Many for EachVec<A, T>
where
    A: Clone + ReadOne<Target = Vec<T>> + WriteOne<Target = Vec<T>> + 'static,
    T: 'static,
{
    type Item = Signal<VecIndex<A, T>>;

    fn iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_> {
        let len = self.parent.read().len();
        let parent = self.parent.clone();

        Box::new((0..len).map(move |index| Signal {
            access: VecIndex {
                parent: parent.clone(),
                index,
                _marker: PhantomData,
            },
        }))
    }
}

impl<A, T> ReadOne for VecIndex<A, T>
where
    A: ReadOne<Target = Vec<T>>,
{
    type Target = T;

    fn read(&self) -> Ref<'_, T> {
        Ref::map(self.parent.read(), |v| &v[self.index])
    }
}

impl<A, T> WriteOne for VecIndex<A, T>
where
    A: WriteOne<Target = Vec<T>>,
{
    fn write(&self) -> RefMut<'_, T> {
        RefMut::map(self.parent.write(), |v| &mut v[self.index])
    }
}

// ---------- map over many ----------

#[derive(Clone)]
pub struct MapMany<A, Q, U> {
    parent: A,
    f: fn(Q) -> U,
    _marker: PhantomData<fn() -> (Q, U)>,
}

impl<A, Q, U> ManyGet for MapMany<A, Q, U>
where
    A: Many<Item = Q>,
    U: 'static,
    Q: 'static,
{
    type Value = U;

    fn iter_values(&self) -> Box<dyn Iterator<Item = U> + '_> {
        let f = self.f;
        Box::new(self.parent.iter().map(f))
    }
}

// ---------- demo domain ----------

#[derive(Debug)]
struct App {
    user: Option<User>,
    todos: Vec<Todo>,
}

#[derive(Debug)]
struct User {
    active: bool,
}

#[derive(Debug)]
struct Todo {
    done: bool,
    title: String,
}

fn app_user(a: &App) -> &Option<User> {
    &a.user
}

fn app_user_mut(a: &mut App) -> &mut Option<User> {
    &mut a.user
}

fn user_active(u: &User) -> &bool {
    &u.active
}

fn user_active_mut(u: &mut User) -> &mut bool {
    &mut u.active
}

fn app_todos(a: &App) -> &Vec<Todo> {
    &a.todos
}

fn app_todos_mut(a: &mut App) -> &mut Vec<Todo> {
    &mut a.todos
}

fn todo_done(t: &Todo) -> &bool {
    &t.done
}

fn todo_done_mut(t: &mut Todo) -> &mut bool {
    &mut t.done
}

fn todo_title(t: &Todo) -> &String {
    &t.title
}

fn todo_title_mut(t: &mut Todo) -> &mut String {
    &mut t.title
}

fn bool_not(v: &bool) -> bool {
    !*v
}

fn title_len(v: Signal<VecIndex<Field<RwRoot<App>, App, Vec<Todo>>, Todo>>) -> usize {
    v.field(todo_title, todo_title_mut).read().len()
}

// ---------- demo ----------

fn main() {
    let app = Signal::new(App {
        user: Some(User { active: true }),
        todos: vec![
            Todo {
                done: false,
                title: "write code".into(),
            },
            Todo {
                done: true,
                title: "ship it".into(),
            },
        ],
    });

    // One<Rw<_>> -> Maybe<Rw<_>> -> Maybe<Get<_>>
    let user_is_inactive = app
        .clone()
        .field(app_user, app_user_mut)
        .some()
        .field(user_active, user_active_mut)
        .get_map(bool_not);

    assert_eq!(user_is_inactive.get_opt(), Some(false));

    // One<Rw<Vec<T>>> -> Many<Rw<T>>
    let todos = app
        .clone()
        .field(app_todos, app_todos_mut)
        .each_vec::<Todo>();

    for todo in todos.iter() {
        todo.field(todo_done, todo_done_mut)
            .write()
            .clone_from(&true);
    }

    assert_eq!(app.read().todos[0].done, true);
    assert_eq!(app.read().todos[1].done, true);

    // Many<Rw<T>> -> Many<Get<U>>
    let title_lengths = todos.map_items(title_len);
    let lengths: Vec<_> = title_lengths.iter_values().collect();
    assert_eq!(lengths, vec![10, 7]);

    // base still feels RefCell-like
    let root_todos = app.field(app_todos, app_todos_mut);
    root_todos.write().push(Todo {
        done: false,
        title: "test".into(),
    });

    assert_eq!(root_todos.read().len(), 3);
}
