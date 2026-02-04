pub type UserId = i64;

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    name: String,
}

impl User {
    pub fn new(id: UserId, name: String) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
