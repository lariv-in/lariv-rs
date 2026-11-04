//! Weight stored as a decimal magnitude plus a display unit.
//!
//! Kilograms are the conversion pivot. Display units are milligrams, grams,
//! kilograms, tonnes, and pounds (international avoirdupois).

use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

/// Display / input unit for a weight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeightUnit {
    Milligram,
    Gram,
    #[default]
    Kilogram,
    Tonne,
    Pound,
}

impl WeightUnit {
    /// Units offered by a weight selector, in selector order.
    pub const ALL: [WeightUnit; 5] = [
        Self::Milligram,
        Self::Gram,
        Self::Kilogram,
        Self::Tonne,
        Self::Pound,
    ];

    /// Short selector label (`mg`, `g`, `kg`, `t`, `lb`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Milligram => "mg",
            Self::Gram => "g",
            Self::Kilogram => "kg",
            Self::Tonne => "t",
            Self::Pound => "lb",
        }
    }

    /// Kilograms in one of this unit. The pound is the international avoirdupois pound.
    pub fn kg_per_unit(self) -> Decimal {
        match self {
            Self::Milligram => Decimal::new(1, 6),
            Self::Gram => Decimal::new(1, 3),
            Self::Kilogram => Decimal::ONE,
            Self::Tonne => Decimal::from(1000),
            Self::Pound => Decimal::new(45_359_237, 8),
        }
    }
}

impl fmt::Display for WeightUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WeightUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_weight_unit(s).ok_or_else(|| format!("unknown weight unit {s:?}"))
    }
}

/// Parse a unit token (`mg`, `g`, `kg`, `t`, `lb`, and common names).
pub fn parse_weight_unit(s: &str) -> Option<WeightUnit> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mg" | "milligram" | "milligrams" => Some(WeightUnit::Milligram),
        "g" | "gram" | "grams" => Some(WeightUnit::Gram),
        "kg" | "kilogram" | "kilograms" => Some(WeightUnit::Kilogram),
        "t" | "tonne" | "tonnes" | "metric ton" | "metric tons" => Some(WeightUnit::Tonne),
        "lb" | "lbs" | "pound" | "pounds" => Some(WeightUnit::Pound),
        _ => None,
    }
}

/// Convert `qty` in `unit` to kilograms.
pub fn to_kg(qty: Decimal, unit: WeightUnit) -> Decimal {
    qty * unit.kg_per_unit()
}

/// Convert kilograms into `unit`.
pub fn from_kg(kg: Decimal, unit: WeightUnit) -> Decimal {
    kg / unit.kg_per_unit()
}
