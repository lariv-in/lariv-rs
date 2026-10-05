use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PRODUCT_TYPE_GOODS: &str = "Goods";
pub const PRODUCT_TYPE_SERVICES: &str = "Services";
pub const PRODUCT_TYPE_BOTH: &str = "Both";

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "product_type")]
pub enum ProductType {
    #[sea_orm(string_value = "Goods")]
    Goods,
    #[sea_orm(string_value = "Services")]
    Services,
    #[sea_orm(string_value = "Both")]
    Both,
}

impl ProductType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Goods => PRODUCT_TYPE_GOODS,
            Self::Services => PRODUCT_TYPE_SERVICES,
            Self::Both => PRODUCT_TYPE_BOTH,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            PRODUCT_TYPE_GOODS => Some(Self::Goods),
            PRODUCT_TYPE_SERVICES => Some(Self::Services),
            PRODUCT_TYPE_BOTH => Some(Self::Both),
            _ => None,
        }
    }
}

impl fmt::Display for ProductType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProductType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid ProductType: {s:?}"))
    }
}

impl Default for ProductType {
    fn default() -> Self {
        Self::Goods
    }
}

impl From<ProductType> for String {
    fn from(v: ProductType) -> Self {
        v.as_str().into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub product_type: ProductType,
    pub reference: Option<String>,
    pub remarks: Option<String>,
    pub name: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub base_cost: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub sales_price: Decimal,
    pub hsn_code: i64,
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
