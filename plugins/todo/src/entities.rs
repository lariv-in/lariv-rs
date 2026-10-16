//! SeaORM entities for personal to-do pages and the items on them.
pub mod todo;
pub mod todo_page;

pub use todo::Entity as TodoEntity;
pub use todo::Model as Todo;
pub use todo_page::Entity as TodoPageEntity;
pub use todo_page::Model as TodoPage;
