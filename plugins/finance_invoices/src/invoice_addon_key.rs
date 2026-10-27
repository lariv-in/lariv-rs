//! Shared identity for invoice hub/PDF deployment addons.

/// Which invoice header an addon should load data for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InvoiceAddonKey {
    Draft(i64),
    Posted(i64),
}

impl InvoiceAddonKey {
    pub fn id(self) -> i64 {
        match self {
            Self::Draft(id) | Self::Posted(id) => id,
        }
    }

    pub fn is_draft(self) -> bool {
        matches!(self, Self::Draft(_))
    }

    pub fn is_posted(self) -> bool {
        matches!(self, Self::Posted(_))
    }
}
