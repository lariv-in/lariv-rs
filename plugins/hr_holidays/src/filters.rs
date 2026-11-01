//! Catalog for holiday table filters.

use lariv_core::list_filters::{FilterField, date_column, text_column};

use crate::entities::holiday;

pub fn holiday_fields() -> Vec<FilterField> {
    vec![
        text_column("title", "Title", holiday::Column::Title),
        date_column("date", "Date", holiday::Column::Date),
    ]
}
