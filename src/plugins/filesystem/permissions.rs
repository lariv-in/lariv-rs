//! Unix-style access bits for a VNode.
//!
//! User, Role, and Other are exclusive: the owner uses only User bits, a principal
//! whose role name matches the node's `role` uses only Role bits, and everyone else
//! uses Other bits. All bits are an extra grant that applies to every principal.
//! A null owner never matches User. A null role never matches Role.

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, TryGetable};
use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct NodePermissions: i32 {
        const USER_READ = 1 << 0;
        const USER_WRITE = 1 << 1;
        const USER_EXECUTE = 1 << 2;
        const ROLE_READ = 1 << 3;
        const ROLE_WRITE = 1 << 4;
        const ROLE_EXECUTE = 1 << 5;
        const OTHER_READ = 1 << 6;
        const OTHER_WRITE = 1 << 7;
        const OTHER_EXECUTE = 1 << 8;
        const ALL_READ = 1 << 9;
        const ALL_WRITE = 1 << 10;
        const ALL_EXECUTE = 1 << 11;
    }
}

/// One access right. View, change, and open on the Permissions page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRight {
    Read,
    Write,
    Execute,
}

/// Principal for an access check. Anonymous actors match only Other and Anyone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessActor {
    pub user_id: Option<i64>,
    pub role: Option<String>,
}

impl AccessActor {
    pub fn anonymous() -> Self {
        Self {
            user_id: None,
            role: None,
        }
    }

    pub fn from_auth(auth: &crate::plugins::users::state::AuthContext) -> Self {
        let role = auth.user.role.trim();
        Self {
            user_id: Some(auth.user.id),
            role: if role.is_empty() {
                None
            } else {
                Some(role.to_string())
            },
        }
    }
}

impl NodePermissions {
    /// New file: user read/write, role read, other read. All is empty.
    pub fn for_file() -> Self {
        Self::USER_READ | Self::USER_WRITE | Self::ROLE_READ | Self::OTHER_READ
    }

    /// New directory: user read/write/execute, role read/execute, other read/execute.
    pub fn for_directory() -> Self {
        Self::USER_READ
            | Self::USER_WRITE
            | Self::USER_EXECUTE
            | Self::ROLE_READ
            | Self::ROLE_EXECUTE
            | Self::OTHER_READ
            | Self::OTHER_EXECUTE
    }

    /// Existing rows: user, role, and other can view, change, and open. All is empty.
    pub fn legacy() -> Self {
        Self::USER_READ
            | Self::USER_WRITE
            | Self::USER_EXECUTE
            | Self::ROLE_READ
            | Self::ROLE_WRITE
            | Self::ROLE_EXECUTE
            | Self::OTHER_READ
            | Self::OTHER_WRITE
            | Self::OTHER_EXECUTE
    }

    pub fn for_kind(is_directory: bool) -> Self {
        if is_directory {
            Self::for_directory()
        } else {
            Self::for_file()
        }
    }

    /// `actor_in_group` is the actor's role name matching the node's role, not the owner's role.
    pub fn allows(
        self,
        right: NodeRight,
        actor_is_owner: bool,
        actor_in_group: bool,
        actor_role: Option<&str>,
    ) -> bool {
        if actor_role.is_some_and(crate::plugins::users::roles::Superuser::matches) {
            return true;
        }
        if self.contains(Self::all_flag(right)) {
            return true;
        }
        let class_flag = if actor_is_owner {
            Self::user_flag(right)
        } else if actor_in_group {
            Self::role_flag(right)
        } else {
            Self::other_flag(right)
        };
        self.contains(class_flag)
    }

    pub fn from_access(
        owner_view: bool,
        owner_change: bool,
        owner_open: bool,
        role_view: bool,
        role_change: bool,
        role_open: bool,
        other_view: bool,
        other_change: bool,
        other_open: bool,
        anyone_view: bool,
        anyone_change: bool,
        anyone_open: bool,
    ) -> Self {
        let mut permissions = Self::empty();
        if owner_view {
            permissions |= Self::USER_READ;
        }
        if owner_change {
            permissions |= Self::USER_WRITE;
        }
        if owner_open {
            permissions |= Self::USER_EXECUTE;
        }
        if role_view {
            permissions |= Self::ROLE_READ;
        }
        if role_change {
            permissions |= Self::ROLE_WRITE;
        }
        if role_open {
            permissions |= Self::ROLE_EXECUTE;
        }
        if other_view {
            permissions |= Self::OTHER_READ;
        }
        if other_change {
            permissions |= Self::OTHER_WRITE;
        }
        if other_open {
            permissions |= Self::OTHER_EXECUTE;
        }
        if anyone_view {
            permissions |= Self::ALL_READ;
        }
        if anyone_change {
            permissions |= Self::ALL_WRITE;
        }
        if anyone_open {
            permissions |= Self::ALL_EXECUTE;
        }
        permissions
    }

    pub fn owner_view(self) -> bool {
        self.contains(Self::USER_READ)
    }
    pub fn owner_change(self) -> bool {
        self.contains(Self::USER_WRITE)
    }
    pub fn owner_open(self) -> bool {
        self.contains(Self::USER_EXECUTE)
    }
    pub fn role_view(self) -> bool {
        self.contains(Self::ROLE_READ)
    }
    pub fn role_change(self) -> bool {
        self.contains(Self::ROLE_WRITE)
    }
    pub fn role_open(self) -> bool {
        self.contains(Self::ROLE_EXECUTE)
    }
    pub fn other_view(self) -> bool {
        self.contains(Self::OTHER_READ)
    }
    pub fn other_change(self) -> bool {
        self.contains(Self::OTHER_WRITE)
    }
    pub fn other_open(self) -> bool {
        self.contains(Self::OTHER_EXECUTE)
    }
    pub fn anyone_view(self) -> bool {
        self.contains(Self::ALL_READ)
    }
    pub fn anyone_change(self) -> bool {
        self.contains(Self::ALL_WRITE)
    }
    pub fn anyone_open(self) -> bool {
        self.contains(Self::ALL_EXECUTE)
    }

    fn user_flag(right: NodeRight) -> Self {
        match right {
            NodeRight::Read => Self::USER_READ,
            NodeRight::Write => Self::USER_WRITE,
            NodeRight::Execute => Self::USER_EXECUTE,
        }
    }

    fn role_flag(right: NodeRight) -> Self {
        match right {
            NodeRight::Read => Self::ROLE_READ,
            NodeRight::Write => Self::ROLE_WRITE,
            NodeRight::Execute => Self::ROLE_EXECUTE,
        }
    }

    fn other_flag(right: NodeRight) -> Self {
        match right {
            NodeRight::Read => Self::OTHER_READ,
            NodeRight::Write => Self::OTHER_WRITE,
            NodeRight::Execute => Self::OTHER_EXECUTE,
        }
    }

    fn all_flag(right: NodeRight) -> Self {
        match right {
            NodeRight::Read => Self::ALL_READ,
            NodeRight::Write => Self::ALL_WRITE,
            NodeRight::Execute => Self::ALL_EXECUTE,
        }
    }
}

impl From<NodePermissions> for Value {
    fn from(source: NodePermissions) -> Self {
        Value::Int(Some(source.bits()))
    }
}

impl TryGetable for NodePermissions {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        match i32::try_get_by(res, idx) {
            Ok(bits) => Ok(Self::from_bits_retain(bits)),
            Err(err @ TryGetError::Null(_)) => Err(err),
            Err(_) => {
                let bits = i64::try_get_by(res, idx)?;
                let bits = <i32 as TryFrom<i64>>::try_from(bits).map_err(|_| {
                    TryGetError::DbErr(DbErr::Type(
                        "NodePermissions value does not fit in i32".into(),
                    ))
                })?;
                Ok(Self::from_bits_retain(bits))
            }
        }
    }
}

impl ValueType for NodePermissions {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        let bits = match v {
            Value::Int(Some(n)) => n,
            Value::BigInt(Some(n)) => {
                <i32 as TryFrom<i64>>::try_from(n).map_err(|_| ValueTypeErr)?
            }
            Value::SmallInt(Some(n)) => i32::from(n),
            Value::TinyInt(Some(n)) => i32::from(n),
            _ => return Err(ValueTypeErr),
        };
        Ok(Self::from_bits_retain(bits))
    }

    fn type_name() -> String {
        "NodePermissions".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::Int
    }

    fn column_type() -> ColumnType {
        ColumnType::Integer
    }
}

impl Nullable for NodePermissions {
    fn null() -> Value {
        Value::Int(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_round_trip_keeps_known_and_unknown_bits() {
        let permissions = NodePermissions::for_file();
        let restored =
            <NodePermissions as ValueType>::try_from(Value::from(permissions)).expect("value");
        assert_eq!(restored, permissions);

        let with_unknown =
            NodePermissions::from_bits_retain(NodePermissions::USER_READ.bits() | (1 << 20));
        let restored =
            <NodePermissions as ValueType>::try_from(Value::from(with_unknown)).expect("value");
        assert!(restored.contains(NodePermissions::USER_READ));
        assert_eq!(restored.bits() & (1 << 20), 1 << 20);
    }

    #[test]
    fn file_directory_and_legacy_masks() {
        let file = NodePermissions::for_file();
        assert!(file.contains(
            NodePermissions::USER_READ
                | NodePermissions::USER_WRITE
                | NodePermissions::ROLE_READ
                | NodePermissions::OTHER_READ
        ));
        assert!(!file.intersects(
            NodePermissions::USER_EXECUTE
                | NodePermissions::ROLE_WRITE
                | NodePermissions::ROLE_EXECUTE
                | NodePermissions::OTHER_WRITE
                | NodePermissions::OTHER_EXECUTE
                | NodePermissions::ALL_READ
                | NodePermissions::ALL_WRITE
                | NodePermissions::ALL_EXECUTE
        ));

        let dir = NodePermissions::for_directory();
        assert!(dir.contains(
            NodePermissions::USER_READ
                | NodePermissions::USER_WRITE
                | NodePermissions::USER_EXECUTE
                | NodePermissions::ROLE_READ
                | NodePermissions::ROLE_EXECUTE
                | NodePermissions::OTHER_READ
                | NodePermissions::OTHER_EXECUTE
        ));
        assert!(!dir.contains(NodePermissions::ROLE_WRITE));
        assert!(!dir.contains(NodePermissions::OTHER_WRITE));
        assert!(!dir.intersects(
            NodePermissions::ALL_READ | NodePermissions::ALL_WRITE | NodePermissions::ALL_EXECUTE
        ));

        let legacy = NodePermissions::legacy();
        assert!(legacy.contains(
            NodePermissions::USER_READ
                | NodePermissions::USER_WRITE
                | NodePermissions::USER_EXECUTE
                | NodePermissions::ROLE_READ
                | NodePermissions::ROLE_WRITE
                | NodePermissions::ROLE_EXECUTE
                | NodePermissions::OTHER_READ
                | NodePermissions::OTHER_WRITE
                | NodePermissions::OTHER_EXECUTE
        ));
        assert!(!legacy.intersects(
            NodePermissions::ALL_READ | NodePermissions::ALL_WRITE | NodePermissions::ALL_EXECUTE
        ));
    }

    #[test]
    fn allows_matches_exclusive_class_then_all_and_superuser() {
        let user_read = NodePermissions::USER_READ;
        assert!(user_read.allows(NodeRight::Read, true, false, None));
        assert!(!user_read.allows(NodeRight::Read, false, true, None));
        assert!(!user_read.allows(NodeRight::Read, false, false, None));

        let role_read = NodePermissions::ROLE_READ;
        assert!(role_read.allows(NodeRight::Read, false, true, None));
        assert!(!role_read.allows(NodeRight::Read, true, true, None));
        assert!(!role_read.allows(NodeRight::Read, false, false, None));

        let other_read = NodePermissions::OTHER_READ;
        assert!(other_read.allows(NodeRight::Read, false, false, None));
        assert!(!other_read.allows(NodeRight::Read, true, false, None));

        let all_read = NodePermissions::ALL_READ;
        assert!(all_read.allows(NodeRight::Read, false, false, None));
        assert!(all_read.allows(NodeRight::Read, true, false, None));

        assert!(NodePermissions::empty().allows(
            NodeRight::Write,
            false,
            false,
            Some(crate::plugins::users::roles::Superuser::NAME)
        ));
        assert!(!NodePermissions::empty().allows(NodeRight::Read, false, false, None));
    }

    #[test]
    fn null_owner_and_null_role_only_match_other_or_all() {
        let permissions = NodePermissions::USER_READ | NodePermissions::ROLE_READ;
        assert!(!permissions.allows(NodeRight::Read, false, false, None));
        assert!((permissions | NodePermissions::OTHER_READ).allows(
            NodeRight::Read,
            false,
            false,
            None
        ));
        assert!((permissions | NodePermissions::ALL_READ).allows(
            NodeRight::Read,
            false,
            false,
            None
        ));
    }
}
