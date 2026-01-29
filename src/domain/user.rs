pub type UserId = i64;

#[derive(Debug)]
pub struct User {
    id: UserId,
}

impl User {
    pub fn new(id: UserId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> UserId {
        self.id
    }
}
