use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PRODUCT_KIND_GOODS: &str = "Goods";
pub const PRODUCT_KIND_SERVICES: &str = "Services";

/// Goods vs services row in the unified product catalog.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "product_kind")]
pub enum ProductKind {
    #[default]
    #[sea_orm(string_value = "Goods")]
    Goods,
    #[sea_orm(string_value = "Services")]
    Services,
}

impl ProductKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Goods => PRODUCT_KIND_GOODS,
            Self::Services => PRODUCT_KIND_SERVICES,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Goods => "Goods",
            Self::Services => "Services",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            PRODUCT_KIND_GOODS => Some(Self::Goods),
            PRODUCT_KIND_SERVICES => Some(Self::Services),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (PRODUCT_KIND_GOODS, "Goods"),
            (PRODUCT_KIND_SERVICES, "Services"),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub product_kind: ProductKind,
    pub remarks: Option<String>,
    pub name: String,
    /// HSN for goods, SAC for services.
    pub hsn_code: Option<i64>,
    /// JSON object of variable name to type (`length`, `weight`, …).
    pub variables: String,
    /// Rune expression for the sales price of one product before tax. Invoice quantity multiplies it.
    pub sales_price_formula: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::product_tax::Entity")]
    ProductTaxes,
}

impl Related<super::product_tax::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductTaxes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
