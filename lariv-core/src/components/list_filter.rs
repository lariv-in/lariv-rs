//! Shared list-filter fields (page size) and the dynamic filter builder.
//!
//! Use [`with_list_filter_common`] in list/picker filter forms and
//! [`page_size_only_filter_form`] when the panel has no entity filters.
//! Table funnels that pick fields at runtime use [`list_filter_builder`].

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::button::{ButtonClear, ButtonSubmit, button_clear, button_submit};
use crate::components::container::container_row;
use crate::components::form::{FormOpts, form};
use crate::components::input::{
    InputDate, InputDatetime, InputDuration, InputForeignKey, InputManyToMany, InputNumber,
    InputSelect, InputSelectOption, InputSingleChoiceCombobox, InputText, InputTime,
    ManyToManyItem, input_date, input_datetime, input_duration, input_foreign_key,
    input_many_to_many, input_number, input_select, input_single_choice_combobox, input_text,
    input_time,
};
use crate::components::swap::{SwapKey, form_hx_get_route};
use crate::components::table::{clamp_page_size, page_size_choice_pairs};
use crate::components::text::icon;
use crate::html_form::{CsrfToken, FormCtx, HtmlForm, html_form, widgets::Select};
use crate::http::{FragmentGet, RouteUrl};
use crate::list_filters::{CmpOp, FilterClause, FilterField, FilterRowError, FilterWidget, JoinOp};

/// Common fields rendered on every list/picker filter form.
#[html_form]
pub struct ListFilterCommonForm {
    #[form(
        label = "Page size",
        widget = Select,
        choices = "page_size",
        required,
        name = "page_size"
    )]
    pub page_size: String,
}

/// Render the shared page-size Select for the current value.
pub fn list_filter_common_inputs(page_size: u32) -> Markup {
    let page_size = clamp_page_size(Some(page_size));
    let choices = page_size_choice_pairs();
    let value = page_size.to_string();
    ListFilterCommonForm::render_inputs(
        &FormCtx::form::<ListFilterCommonForm>(CsrfToken::current())
            .value(ListFilterCommonFormField::PageSize, value.as_str())
            .choices(ListFilterCommonFormField::PageSize, &choices),
    )
}

/// Append shared list-filter fields after entity-specific filter inputs.
pub fn with_list_filter_common(entity_inputs: Markup, page_size: u32) -> Markup {
    html! {
        (entity_inputs)
        (list_filter_common_inputs(page_size))
    }
}

fn filter_form_actions(apply_label: &str) -> Markup {
    html! {
        (container_row("flex gap-2", html! {
            (button_submit(ButtonSubmit { label: apply_label, ..Default::default() }))
            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
        }))
    }
}

/// Page-size-only filter panel for list views that have no other filters.
///
/// `extra_inputs` can carry hidden fields (e.g. `tab`) that must survive Apply.
pub fn page_size_only_filter_form_with_extras<K, R>(page_size: u32, extra_inputs: Markup) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    form(
        &CsrfToken::current(),
        FormOpts {
            attrs: form_hx_get_route::<K, R>(R::default()),
            inputs: html! {
                (list_filter_common_inputs(page_size))
                (extra_inputs)
            },
            actions: filter_form_actions("Apply Filters"),
            ..Default::default()
        },
    )
}

/// Page-size-only filter panel for list views that have no other filters.
pub fn page_size_only_filter_form<K, R>(page_size: u32) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    page_size_only_filter_form_with_extras::<K, R>(page_size, Markup::default())
}

/// Page-size-only filter panel for FK picker modals (`hx-push-url=false`).
pub fn page_size_only_picker_filter_form<K, R>(page_size: u32) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    form(
        &CsrfToken::current(),
        FormOpts {
            attrs: form_hx_get_route::<K, R>(R::default()).set("hx-push-url", "false"),
            inputs: list_filter_common_inputs(page_size),
            actions: filter_form_actions("Apply"),
            ..Default::default()
        },
    )
}

/// One rendered filter row, including values the widgets cannot recover from the query alone.
#[derive(Clone, Debug)]
pub struct FilterClauseView {
    pub clause: FilterClause,
    pub error: String,
    /// Label shown in a foreign-key control. Empty when the row is not a foreign key.
    pub fk_display: String,
    /// Chips shown in a many-to-many control.
    pub m2m_items: Vec<ManyToManyItem>,
}

impl FilterClauseView {
    /// Pair parsed clauses with apply errors. Displays and chips start empty.
    pub fn from_clauses(clauses: &[FilterClause], errors: &[FilterRowError]) -> Vec<Self> {
        clauses
            .iter()
            .map(|clause| {
                let error = errors
                    .iter()
                    .find(|err| err.index == clause.index)
                    .map(|err| err.message.clone())
                    .unwrap_or_default();
                Self {
                    clause: clause.clone(),
                    error,
                    fk_display: String::new(),
                    m2m_items: Vec::new(),
                }
            })
            .collect()
    }
}

/// Dynamic filter panel: add, reorder, and combine catalog fields.
pub struct ListFilterBuilder<'a> {
    pub fields: &'a [FilterField],
    pub rows: &'a [FilterClauseView],
    pub page_size: u32,
    pub sort: &'a str,
    pub attrs: HtmlAttrs,
    /// Submit button label. Empty uses "Apply Filters".
    pub apply_label: &'a str,
    /// Hidden inputs that must survive Apply (`tab`, `target_input`, …).
    /// Do not include `sort` or `page_size`; the builder already submits those.
    pub extra: Markup,
}

/// GET form whose rows submit `filter.{i}.*` in visual order.
pub fn list_filter_builder(opts: ListFilterBuilder<'_>) -> Markup {
    let apply_label = if opts.apply_label.is_empty() {
        "Apply Filters"
    } else {
        opts.apply_label
    };
    let kinds = operator_kinds_json(opts.fields);
    let alpine = filter_builder_alpine(&kinds);
    let field_choices: Vec<(String, String)> = opts
        .fields
        .iter()
        .map(|field| (field.key().to_string(), field.label().to_string()))
        .collect();
    let attrs = opts.attrs.set("x-data", alpine);
    form(
        &CsrfToken::current(),
        FormOpts {
            attrs,
            inputs: html! {
                div x-ref="rows" data-filter-rows="" class="flex flex-col" {
                    @for row in opts.rows {
                        (filter_row(opts.fields, &field_choices, row, false))
                    }
                }
                @if !opts.fields.is_empty() {
                    button type="button" class="btn btn-outline btn-sm mt-2 self-start" "@click"="addRow()" {
                        "Add filter"
                    }
                }
                (list_filter_common_inputs(opts.page_size))
                input type="hidden" name="sort" value=(opts.sort);
                (opts.extra)
            },
            actions: html! {
                (container_row("flex gap-2", html! {
                    (button_submit(ButtonSubmit { label: apply_label, ..Default::default() }))
                    button type="button" class="btn btn-ghost" "@click"="clearRows()" { "Clear" }
                }))
            },
            suffix: html! {
                (new_row_template(opts.fields, &field_choices))
                @for field in opts.fields {
                    template data-filter-widget=(field.key()) {
                        (value_widget(field, "filter.__i__.value", &[], "", &[]))
                    }
                }
            },
            ..Default::default()
        },
    )
}

fn operator_kinds_json(fields: &[FilterField]) -> String {
    let mut out = String::from("{");
    let mut first = true;
    for field in fields.iter().filter(|field| field.has_operator()) {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&serde_json::to_string(field.key()).unwrap_or_else(|_| "\"\"".into()));
        out.push_str(":true");
    }
    out.push('}');
    out
}

fn filter_builder_alpine(kinds_json: &str) -> String {
    format!(
        r#"{{
            kinds: {kinds_json},
            rowContainer() {{
                return this.$refs.rows || this.$root.querySelector('[data-filter-rows]');
            }},
            addRow() {{
                const tpl = this.$refs.newRow;
                const container = this.rowContainer();
                if (!tpl || !container) return;
                container.appendChild(tpl.content.cloneNode(true));
                const row = container.lastElementChild;
                if (window.Alpine && row) window.Alpine.initTree(row);
                this.reindex(container);
            }},
            clearRows() {{
                const container = this.rowContainer();
                if (container) container.replaceChildren();
            }},
            rowOf(event) {{
                return event.target.closest('[data-filter-row]');
            }},
            rows(container) {{
                const root = container || (this.$refs.rows || this.$root.querySelector('[data-filter-rows]'));
                if (!root) return [];
                return [...root.children].filter((el) => el.matches && el.matches('[data-filter-row]'));
            }},
            indexOf(row) {{
                return this.rows().indexOf(row);
            }},
            reindex(container) {{
                this.rows(container).forEach((row, i) => {{
                    row.querySelectorAll('[name]').forEach((el) => {{
                        const name = el.getAttribute('name') || '';
                        if (name.indexOf('filter.') === 0) {{
                            el.setAttribute('name', name.replace(/^filter\.(?:\d+|__i__)\./, 'filter.' + i + '.'));
                        }}
                    }});
                    const join = row.querySelector('[data-filter-join]');
                    if (!join) return;
                    const first = i === 0;
                    join.classList.toggle('hidden', first);
                    if (first) join.setAttribute('hidden', '');
                    else join.removeAttribute('hidden');
                }});
            }},
            removeRow(event) {{
                const row = this.rowOf(event);
                if (!row) return;
                const container = row.parentElement;
                row.remove();
                this.reindex(container);
            }},
            move(event, delta) {{
                const row = this.rowOf(event);
                if (!row) return;
                const container = row.parentElement;
                const rows = this.rows(container);
                const i = rows.indexOf(row);
                const j = i + delta;
                if (i < 0 || j < 0 || j >= rows.length) return;
                if (delta < 0) container.insertBefore(row, rows[j]);
                else container.insertBefore(rows[j], row);
                this.reindex(container);
            }},
            widgetTemplate(key) {{
                const nodes = this.$root.querySelectorAll('template[data-filter-widget]');
                for (const el of nodes) {{
                    if (el.getAttribute('data-filter-widget') === key) return el;
                }}
                return null;
            }},
            onField(event) {{
                const row = event.target.closest('[data-filter-row]');
                if (!row) return;
                const key = event.detail && event.detail.key != null ? String(event.detail.key) : '';
                if ((row.dataset.field || '') === key) return;
                row.dataset.field = key;
                const op = row.querySelector('[data-filter-op]');
                if (op) {{
                    const show = !!this.kinds[key];
                    op.classList.toggle('hidden', !show);
                    if (show) op.removeAttribute('hidden');
                    else op.setAttribute('hidden', '');
                }}
                const slot = row.querySelector('[data-filter-value]');
                if (!slot) return;
                slot.replaceChildren();
                const tpl = this.widgetTemplate(key);
                if (!tpl) return;
                const i = this.indexOf(row);
                slot.innerHTML = tpl.innerHTML.replaceAll('__i__', String(i < 0 ? 0 : i));
                if (window.Alpine) window.Alpine.initTree(slot);
            }}
        }}"#,
        kinds_json = kinds_json,
    )
}

fn filter_row(
    fields: &[FilterField],
    field_choices: &[(String, String)],
    row: &FilterClauseView,
    template: bool,
) -> Markup {
    let index = if template {
        "__i__".to_string()
    } else {
        row.clause.index.to_string()
    };
    let field_key = if template {
        ""
    } else {
        row.clause.field.as_str()
    };
    let selected = fields.iter().find(|field| field.key() == field_key);
    let has_op = selected.is_some_and(FilterField::has_operator);
    let join_name = format!("filter.{index}.join");
    let field_name = format!("filter.{index}.field");
    let op_name = format!("filter.{index}.op");
    let value_name = format!("filter.{index}.value");
    let join_class = if template || row.clause.index == 0 {
        "w-full hidden"
    } else {
        "w-full"
    };
    let join_hidden = if template || row.clause.index == 0 {
        " hidden"
    } else {
        ""
    };
    let op = if template { CmpOp::Eq } else { row.clause.op };
    let op_class = if has_op {
        "shrink-0"
    } else {
        "shrink-0 hidden"
    };
    let op_hidden = if has_op { "" } else { " hidden" };
    let gt_checked = if op == CmpOp::Gt { " checked" } else { "" };
    let eq_checked = if op == CmpOp::Eq { " checked" } else { "" };
    let lt_checked = if op == CmpOp::Lt { " checked" } else { "" };
    let and_checked = if row.clause.join != JoinOp::Or {
        " checked"
    } else {
        ""
    };
    let or_checked = if row.clause.join == JoinOp::Or {
        " checked"
    } else {
        ""
    };
    let value_body = match selected {
        Some(field) if !template => value_widget(
            field,
            &value_name,
            &row.clause.values,
            &row.fk_display,
            &row.m2m_items,
        ),
        _ => Markup::default(),
    };
    html! {
        (PreEscaped(format!(
            r#"<div data-filter-row data-field="{field}" class="flex flex-col gap-2 border-b border-base-200 py-2">"#,
            field = escape_attr(field_key),
        )))
        (PreEscaped(format!(
            r#"<div data-filter-join class="{join_class}"{join_hidden}><div class="join" role="radiogroup" aria-label="Combine with previous filter"><input type="radio" class="join-item btn btn-sm" name="{name}" value="and" aria-label="AND"{and_checked}><input type="radio" class="join-item btn btn-sm" name="{name}" value="or" aria-label="OR"{or_checked}></div></div><div class="flex flex-wrap items-center gap-2">"#,
            join_class = join_class,
            join_hidden = join_hidden,
            name = escape_attr(&join_name),
            and_checked = and_checked,
            or_checked = or_checked,
        )))
        div class="min-w-40 flex-1" "@single-choice"="onField($event)" {
            (input_single_choice_combobox(InputSingleChoiceCombobox {
                name: &field_name,
                choices: field_choices,
                value: field_key,
                placeholder: "Field",
                classes: "my-0",
                ..Default::default()
            }))
        }
        (PreEscaped(format!(
            r#"<div data-filter-op class="{op_class}"{op_hidden}><div class="join" role="radiogroup" aria-label="Comparison"><input type="radio" class="join-item btn btn-sm" name="{name}" value="gt" aria-label="&gt;" title="Greater than"{gt_checked}><input type="radio" class="join-item btn btn-sm" name="{name}" value="eq" aria-label="=" title="Equals"{eq_checked}><input type="radio" class="join-item btn btn-sm" name="{name}" value="lt" aria-label="&lt;" title="Less than"{lt_checked}></div></div>"#,
            op_class = op_class,
            op_hidden = op_hidden,
            name = escape_attr(&op_name),
            gt_checked = gt_checked,
            eq_checked = eq_checked,
            lt_checked = lt_checked,
        )))
        div data-filter-value="" class="min-w-40 flex-1" {
            (value_body)
        }
        div class="ml-auto flex shrink-0 flex-col items-center gap-1" {
            button type="button" class="btn btn-ghost btn-xs" aria-label="Move up" "@click"="move($event, -1)" {
                (icon("chevron-up", ""))
            }
            button type="button" class="btn btn-ghost btn-xs" aria-label="Move down" "@click"="move($event, 1)" {
                (icon("chevron-down", ""))
            }
            button type="button" class="btn btn-ghost btn-xs" aria-label="Remove filter" "@click"="removeRow($event)" {
                (icon("x-mark", ""))
            }
        }
        (PreEscaped("</div>"))
        @if !row.error.is_empty() {
            p class="text-error text-sm basis-full" { (row.error.as_str()) }
        }
        (PreEscaped("</div>"))
    }
}

fn new_row_template(fields: &[FilterField], field_choices: &[(String, String)]) -> Markup {
    let blank = FilterClauseView {
        clause: FilterClause {
            index: 0,
            join: JoinOp::And,
            field: String::new(),
            op: CmpOp::Eq,
            values: Vec::new(),
        },
        error: String::new(),
        fk_display: String::new(),
        m2m_items: Vec::new(),
    };
    html! {
        template x-ref="newRow" {
            (filter_row(fields, field_choices, &blank, true))
        }
    }
}

fn value_widget(
    field: &FilterField,
    name: &str,
    values: &[String],
    fk_display: &str,
    m2m_items: &[ManyToManyItem],
) -> Markup {
    let value = values.first().map(String::as_str).unwrap_or("");
    match field.widget() {
        FilterWidget::Text => input_text(InputText {
            name,
            value,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::Number => input_number(InputNumber {
            name,
            value,
            classes: "my-0",
            attrs: HtmlAttrs::new().set("step", "1"),
            ..Default::default()
        }),
        FilterWidget::Decimal => input_number(InputNumber {
            name,
            value,
            classes: "my-0",
            attrs: HtmlAttrs::new().set("step", "any"),
            ..Default::default()
        }),
        FilterWidget::Date => input_date(InputDate {
            label: "",
            name,
            value,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::Time => input_time(InputTime {
            label: "",
            name,
            value,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::Datetime => input_datetime(InputDatetime {
            label: "",
            name,
            value,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::Duration => input_duration(InputDuration {
            label: "",
            name,
            value,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::ForeignKey {
            url,
            placeholder,
            search_key,
        } => input_foreign_key(InputForeignKey {
            name,
            value,
            display: fk_display,
            placeholder,
            url,
            uid: name,
            search_key,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::ManyToMany { url, placeholder } => input_many_to_many(InputManyToMany {
            name,
            items: m2m_items,
            placeholder,
            url,
            classes: "my-0",
            ..Default::default()
        }),
        FilterWidget::Choice => {
            let mut options = Vec::with_capacity(field.choices().len() + 1);
            options.push(InputSelectOption {
                value: "",
                label: "Any",
                selected: value.is_empty(),
            });
            for (key, label) in field.choices() {
                options.push(InputSelectOption {
                    value: key,
                    label,
                    selected: key == value,
                });
            }
            input_select(InputSelect {
                name,
                options: &options,
                classes: "my-0",
                ..Default::default()
            })
        }
        FilterWidget::Bool => {
            let options = [
                InputSelectOption {
                    value: "",
                    label: "Any",
                    selected: value.is_empty(),
                },
                InputSelectOption {
                    value: "1",
                    label: "Yes",
                    selected: value == "1" || value.eq_ignore_ascii_case("true"),
                },
                InputSelectOption {
                    value: "0",
                    label: "No",
                    selected: value == "0" || value.eq_ignore_ascii_case("false"),
                },
            ];
            input_select(InputSelect {
                name,
                options: &options,
                classes: "my-0",
                ..Default::default()
            })
        }
    }
}

/// Parse errors for a filter submit that only swaps `.data-table-body`.
///
/// Place this inside that region. The filter panel itself shows the same
/// messages when the whole page renders.
pub fn filter_error_banner(errors: &[FilterRowError]) -> Markup {
    if errors.is_empty() {
        return Markup::default();
    }
    html! {
        div class="alert alert-error text-sm mb-2" role="alert" {
            ul class="list-disc pl-4" {
                @for err in errors {
                    li { (err.message.as_str()) }
                }
            }
        }
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;
    use crate::list_filters::{FilterWidget, custom};
    use sea_orm::sea_query::Condition;

    fn field(key: &'static str, label: &'static str, widget: FilterWidget) -> FilterField {
        custom(
            key,
            label,
            widget,
            Vec::new(),
            false,
            |_clause, _backend| Ok(Condition::all()),
        )
    }

    #[test]
    fn builder_renders_active_row_templates_and_operators() {
        let fields = [
            field("title", "Title", FilterWidget::Text),
            field("priority", "Priority", FilterWidget::Number),
        ];
        let rows = [FilterClauseView {
            clause: FilterClause {
                index: 0,
                join: JoinOp::And,
                field: "title".into(),
                op: CmpOp::Eq,
                values: vec!["Hello".into()],
            },
            error: String::new(),
            fk_display: String::new(),
            m2m_items: Vec::new(),
        }];
        let html = list_filter_builder(ListFilterBuilder {
            fields: &fields,
            rows: &rows,
            page_size: 12,
            sort: "Title ASC",
            attrs: HtmlAttrs::new().set("method", "GET"),
            apply_label: "Apply Filters",
            extra: Markup::default(),
        })
        .into_string();
        assert!(html.contains(r#"name="filter.0.field""#), "{html}");
        assert!(html.contains("Hello"), "{html}");
        assert!(html.contains("Add filter"), "{html}");
        assert!(html.contains(r#"aria-label="AND""#), "{html}");
        assert!(html.contains(r#"aria-label="OR""#), "{html}");
        assert!(html.contains(r#"type="radio""#), "{html}");
        assert!(html.contains("join-item"), "{html}");
        assert!(html.contains(r#"aria-label="&gt;""#), "{html}");
        assert!(html.contains(r#"aria-label="=""#), "{html}");
        assert!(html.contains(r#"aria-label="&lt;""#), "{html}");
        assert!(html.contains(r#"value="gt""#), "{html}");
        assert!(html.contains(r#"data-filter-widget="priority""#), "{html}");
        assert!(html.contains("filter.__i__.value"), "{html}");
        assert!(html.contains(r#"name="sort" value="Title ASC""#), "{html}");
        assert!(html.contains(r#"name="page_size""#), "{html}");
        assert!(html.contains("clearRows()"), "{html}");
        assert!(html.contains("Move up"), "{html}");
        assert!(html.contains(r#"data-filter-join class="w-full hidden""#), "{html}");
        assert!(html.contains("classList.toggle('hidden'"), "{html}");
    }
}
