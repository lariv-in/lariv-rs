use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        FieldText, ObjectList, ShellChrome, TableColumnHeader, TableRow, column_sort_url,
        data_table_list_refresh, field_text, row_attr_select, sort_indicator,
    },
    picker::RenderPickerSelect,
    template::RenderTemplate,
};

use crate::plugins::finance_accounts::keys::{SourceDocSelectModalKey, SourceDocSelectTableKey};

use super::common::render_picker_pagination;

#[derive(Clone)]
pub struct SourceDocRow {
    pub id: i64,
    pub source_doc_type: String,
    pub source_doc_id: i64,
    pub label: String,
}

#[derive(Generic)]
pub struct SourceDocSelectPage {
    pub docs: ObjectList<SourceDocRow>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<SourceDocSelectTableKey, SourceDocSelectModalKey> for SourceDocSelectPage {
    fn render_table(&self) -> Markup {
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let reference_sort = column_sort_url(&self.path_and_query, "Reference", &self.sort);
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let reference_label = format!("Reference{}", sort_indicator(&self.sort, "Reference"));
        let headers = [
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Reference",
                label: &reference_label,
                sort_url: Some(&reference_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Label",
                label: "Label",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .docs
            .items
            .iter()
            .map(|d| TableRow {
                attrs: row_attr_select(&self.target_input, &d.id.to_string(), &d.label),
                cells: vec![
                    field_text(FieldText { value: &d.source_doc_type, classes: "" }),
                    field_text(FieldText {
                        value: &d.source_doc_id.to_string(),
                        classes: "",
                    }),
                    field_text(FieldText { value: &d.label, classes: "" }),
                ],
            })
            .collect();
        let pagination = render_picker_pagination::<SourceDocSelectModalKey>(
            &self.path_and_query,
            self.docs.number,
            self.docs.num_pages,
        );
        data_table_list_refresh::<SourceDocSelectTableKey>(
            "Select Source Document",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for SourceDocSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
