//! SeaORM entities for users and roles.
pub mod role;
pub mod user;

pub use role::Entity as RoleEntity;
pub use role::Model as Role;
pub use user::Entity as UserEntity;
pub use user::Model as User;
