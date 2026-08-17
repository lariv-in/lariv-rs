use crate::export::{ExportCapability, ExportRegistrar, ExportTable};

#[derive(Clone, Copy, Default)]
pub struct ExportHook;

impl ExportRegistrar for ExportHook {
    fn register_export(self, export: ExportCapability) -> ExportCapability {
        export
            .register(ExportTable::new(
                "roles",
                "Role",
                vec![
                    "id".into(),
                    "created_at".into(),
                    "updated_at".into(),
                    "name".into(),
                ],
            ))
            .register(
                ExportTable::new(
                    "users",
                    "User",
                    vec![
                        "id".into(),
                        "created_at".into(),
                        "updated_at".into(),
                        "name".into(),
                        "email".into(),
                        "phone".into(),
                        "is_superuser".into(),
                        "role_id".into(),
                        "timezone".into(),
                    ],
                )
                .with_deps(vec!["roles".into()]),
            )
    }
}
