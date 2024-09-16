use uuid::Uuid;

#[derive(PartialEq)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}
