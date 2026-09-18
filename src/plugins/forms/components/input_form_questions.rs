//! Visual question builder (Alpine + hidden JSON).

use maud::{Markup, html};

use crate::components::attrs::escape_attr;
use crate::components::input::single_choice_combobox_alpine_shell;

const QUESTION_TYPES: &[(&str, &str)] = &[
    ("ShortText", "Short text"),
    ("LongText", "Long text"),
    ("Number", "Number"),
    ("MCQText", "Multiple choice"),
    ("MCQWithCustom", "Multiple choice (custom)"),
    ("Checkboxes", "Checkboxes"),
    ("Dropdown", "Dropdown"),
    ("LinearScale", "Linear scale"),
    ("Rating", "Rating"),
    ("MCQGrid", "MCQ grid"),
    ("TickBoxGrid", "Tick-box grid"),
    ("Date", "Date"),
    ("Time", "Time"),
];

const RATING_MARKERS: &[(&str, &str)] = &[
    ("Star", "Star"),
    ("Heart", "Heart"),
    ("Like", "Like"),
];

fn type_picker_alpine_factory(question_types_json: &str) -> String {
    format!(
        r#"questionTypes: {question_types_json}.map(([key, label]) => ({{ key, label }})),
typeLabel(kind) {{
	const choice = this.questionTypes.find((item) => item.key === kind);
	return choice ? choice.label : (kind || '');
}},
typePickerData(row) {{
	const parent = this;
	return {{
		choices: parent.questionTypes,
		row,
		value: row.type_kind,
		query: parent.typeLabel(row.type_kind),
		error: '',
		open: false,
		highlight: 0,
		placeholder: 'Search type…',
		labelFor(key) {{
			const choice = this.choices.find((item) => item.key === key);
			return choice ? choice.label : key;
		}},
		filtered() {{
			const q = String(this.query || '').trim().toLowerCase();
			return this.choices.filter((choice) => {{
				if (!q) return true;
				return choice.label.toLowerCase().includes(q) || choice.key.toLowerCase().includes(q);
			}});
		}},
		unknownError(q) {{
			return 'No listed option matches ' + JSON.stringify(q) + '.';
		}},
		syncError() {{
			const q = String(this.query || '').trim();
			this.error = q && !this.filtered().length ? this.unknownError(q) : '';
		}},
		select(choice) {{
			if (!choice) return;
			this.row.type_kind = choice.key;
			this.value = choice.key;
			this.query = choice.label;
			this.error = '';
			this.highlight = 0;
			this.open = false;
			parent.onTypeChange(this.row);
		}},
		commitQuery() {{
			const q = String(this.query || '').trim();
			if (!q) {{
				this.error = '';
				return true;
			}}
			const options = this.filtered();
			if (options.length) {{
				const idx = Math.max(0, Math.min(this.highlight, options.length - 1));
				this.select(options[idx]);
				return true;
			}}
			this.error = this.unknownError(q);
			this.open = false;
			return false;
		}},
		onFocusOut(event) {{
			if (this.$el.contains(event.relatedTarget)) return;
			this.open = false;
			const q = String(this.query || '').trim();
			if (!q || this.choices.some((choice) => choice.label === q)) {{
				this.query = this.labelFor(this.row.type_kind);
				this.error = '';
				return;
			}}
			if (this.filtered().length) {{
				this.query = this.labelFor(this.row.type_kind);
				this.error = '';
				return;
			}}
			this.error = this.unknownError(q);
		}},
		move(delta) {{
			const options = this.filtered();
			if (!options.length) {{
				this.open = true;
				return;
			}}
			this.open = true;
			const next = this.highlight + delta;
			this.highlight = (next + options.length) % options.length;
		}},
		onKey(event) {{
			if (event.key === 'ArrowDown') {{
				event.preventDefault();
				this.move(1);
			}} else if (event.key === 'ArrowUp') {{
				event.preventDefault();
				this.move(-1);
			}} else if (event.key === 'Enter') {{
				event.preventDefault();
				this.commitQuery();
			}} else if (event.key === 'Escape') {{
				this.open = false;
				this.query = this.labelFor(this.row.type_kind);
				this.error = '';
			}}
		}},
	}};
}},"#
    )
}

fn alpine_methods() -> &'static str {
    r#"newId() {
	if (typeof crypto !== 'undefined' && crypto.randomUUID) return crypto.randomUUID();
	return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
		const r = Math.random() * 16 | 0;
		const v = c === 'x' ? r : (r & 0x3 | 0x8);
		return v.toString(16);
	});
},
defaultRow() {
	return {
		form_question_id: this.newId(),
		display_text: 'New question',
		required: false,
		description: '',
		type_kind: 'ShortText',
		options: ['Option 1'],
		number_start: '',
		number_end: '',
		scale_start: 1,
		scale_end: 5,
		scale_start_label: '',
		scale_end_label: '',
		rating_range: 5,
		rating_marker: 'Star',
		grid_rows: ['Row 1'],
		grid_cols: ['Column 1'],
	};
},
addQuestion() {
	this.questions.push(this.defaultRow());
},
removeQuestion(idx) {
	if (!Array.isArray(this.questions)) return;
	this.questions.splice(idx, 1);
},
moveQuestion(idx, dir) {
	const j = idx + dir;
	if (j < 0 || j >= this.questions.length) return;
	const tmp = this.questions[idx];
	this.questions[idx] = this.questions[j];
	this.questions[j] = tmp;
},
addOption(q) {
	if (!Array.isArray(q.options)) q.options = [];
	q.options.push('New option');
},
removeOption(q, idx) {
	if (!Array.isArray(q.options) || q.options.length <= 1) return;
	q.options.splice(idx, 1);
},
addGridRow(q) {
	if (!Array.isArray(q.grid_rows)) q.grid_rows = [];
	q.grid_rows.push('New row');
},
removeGridRow(q, idx) {
	if (!Array.isArray(q.grid_rows) || q.grid_rows.length <= 1) return;
	q.grid_rows.splice(idx, 1);
},
addGridCol(q) {
	if (!Array.isArray(q.grid_cols)) q.grid_cols = [];
	q.grid_cols.push('New column');
},
removeGridCol(q, idx) {
	if (!Array.isArray(q.grid_cols) || q.grid_cols.length <= 1) return;
	q.grid_cols.splice(idx, 1);
},
onTypeChange(q) {
	if (['MCQText','MCQWithCustom','Checkboxes','Dropdown'].includes(q.type_kind)) {
		if (!Array.isArray(q.options) || q.options.length === 0) q.options = ['Option 1'];
	}
	if (['MCQGrid','TickBoxGrid'].includes(q.type_kind)) {
		if (!Array.isArray(q.grid_rows) || q.grid_rows.length === 0) q.grid_rows = ['Row 1'];
		if (!Array.isArray(q.grid_cols) || q.grid_cols.length === 0) q.grid_cols = ['Column 1'];
	}
},
buildQuestionType(q) {
	const kind = q.type_kind || 'ShortText';
	if (kind === 'ShortText' || kind === 'LongText' || kind === 'Date' || kind === 'Time') return kind;
	if (kind === 'Number') {
		const start = q.number_start === '' || q.number_start == null ? null : parseFloat(String(q.number_start));
		const end = q.number_end === '' || q.number_end == null ? null : parseFloat(String(q.number_end));
		return { Number: { start: isNaN(start) ? null : start, end: isNaN(end) ? null : end } };
	}
	if (['MCQText','MCQWithCustom','Checkboxes','Dropdown'].includes(kind)) {
		return { [kind]: Array.isArray(q.options) ? q.options : [] };
	}
	if (kind === 'LinearScale') {
		return { LinearScale: {
			start: parseInt(q.scale_start, 10) || 1,
			end: parseInt(q.scale_end, 10) || 5,
			start_label: q.scale_start_label || null,
			end_label: q.scale_end_label || null,
		}};
	}
	if (kind === 'Rating') {
		return { Rating: { range: parseInt(q.rating_range, 10) || 5, marker: q.rating_marker || 'Star' } };
	}
	if (kind === 'MCQGrid' || kind === 'TickBoxGrid') {
		return { [kind]: { rows: q.grid_rows || [], cols: q.grid_cols || [] } };
	}
	return 'ShortText';
},
toEditorRow(q) {
	const qt = q.question_type;
	let type_kind = 'ShortText';
	let row = this.defaultRow();
	row.form_question_id = q.form_question_id || this.newId();
	row.display_text = q.display_text || '';
	row.required = !!q.required;
	row.description = q.description || '';
	if (typeof qt === 'string') {
		type_kind = qt;
	} else if (qt && typeof qt === 'object') {
		type_kind = Object.keys(qt)[0] || 'ShortText';
		const payload = qt[type_kind];
		if (type_kind === 'Number' && payload) {
			row.number_start = payload.start ?? '';
			row.number_end = payload.end ?? '';
		}
		if (['MCQText','MCQWithCustom','Checkboxes','Dropdown'].includes(type_kind) && Array.isArray(payload)) {
			row.options = payload.length ? payload.slice() : ['Option 1'];
		}
		if (type_kind === 'LinearScale' && payload) {
			row.scale_start = payload.start ?? 1;
			row.scale_end = payload.end ?? 5;
			row.scale_start_label = payload.start_label || '';
			row.scale_end_label = payload.end_label || '';
		}
		if (type_kind === 'Rating' && payload) {
			row.rating_range = payload.range ?? 5;
			row.rating_marker = payload.marker || 'Star';
		}
		if ((type_kind === 'MCQGrid' || type_kind === 'TickBoxGrid') && payload) {
			row.grid_rows = Array.isArray(payload.rows) && payload.rows.length ? payload.rows.slice() : ['Row 1'];
			row.grid_cols = Array.isArray(payload.cols) && payload.cols.length ? payload.cols.slice() : ['Column 1'];
		}
	}
	row.type_kind = type_kind;
	return row;
},
toSerdeQuestions() {
	return (this.questions || []).map((q) => ({
		form_question_id: q.form_question_id,
		display_text: q.display_text || '',
		question_type: this.buildQuestionType(q),
		required: !!q.required,
		description: (q.description || '').trim() ? q.description : null,
	}));
}"#
}

pub struct InputFormQuestions<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub classes: &'a str,
}

impl Default for InputFormQuestions<'_> {
    fn default() -> Self {
        Self {
            name: "questions_json",
            defaults: "[]",
            classes: "w-full",
        }
    }
}

/// Render the question builder widget.
pub fn input_form_questions(opts: InputFormQuestions<'_>) -> Markup {
    let defaults = if opts.defaults.trim().is_empty() {
        "[]".to_string()
    } else {
        opts.defaults.trim().to_string()
    };

    let question_types_json =
        serde_json::to_string(QUESTION_TYPES).unwrap_or_else(|_| "[]".into());
    let alpine_data = format!(
        "{{ questions: [], {type_picker} {methods} }}",
        type_picker = type_picker_alpine_factory(&question_types_json),
        methods = alpine_methods().trim_end_matches(',')
    );

    let name_escaped = escape_attr(opts.name);
    let init_js = format!(
        r#"
(function () {{
	const d = Alpine.$data($el);
	if (!d) return;
	let raw = {defaults};
	if (!Array.isArray(raw)) raw = [];
	if (raw.length && raw[0].question_type !== undefined) {{
		d.questions = raw.map((q) => d.toEditorRow(q));
	}} else {{
		d.questions = raw;
	}}
	if (!Array.isArray(d.questions) || d.questions.length === 0) {{
		d.questions = [d.defaultRow()];
	}}
}})();
$el.closest('form').addEventListener('submit', (ev) => {{
	const d = Alpine.$data($el);
	if (!d) return;
	const h = $el.querySelector('input[type="hidden"][name={name_q}]');
	if (!h) return;
	h.value = JSON.stringify(d.toSerdeQuestions());
}}, true);"#,
        name_q = serde_json::to_string(opts.name).unwrap_or_else(|_| "\"questions_json\"".into()),
    );

    html! {
        div class=(opts.classes) x-data=(alpine_data) x-init=(init_js) {
            input type="hidden" name=(name_escaped) value="" {}
            div class="flex flex-col gap-3" {
                template x-for="(q, idx) in questions" x-bind:key="q.form_question_id" {
                    div class="border border-base-300 rounded-lg p-3 flex flex-col gap-2" {
                        div class="flex items-center justify-between gap-2" {
                            div class="flex items-center gap-3" {
                                span class="text-xs font-medium opacity-70" x-text="'#' + (idx + 1)" {}
                                label class="label cursor-pointer gap-1 py-0" {
                                    input type="checkbox" class="checkbox checkbox-sm" x-model="q.required" {}
                                    span class="label-text text-xs" { "Required" }
                                }
                            }
                            div class="flex gap-1" {
                                button type="button" class="btn btn-ghost btn-xs" x-on:click="moveQuestion(idx, -1)" { "↑" }
                                button type="button" class="btn btn-ghost btn-xs" x-on:click="moveQuestion(idx, 1)" { "↓" }
                                button type="button" class="btn btn-ghost btn-xs text-error" x-on:click="removeQuestion(idx)" { "✕" }
                            }
                        }
                        input class="input input-bordered input-sm w-full" type="text" placeholder="Question text" x-model="q.display_text" {}
                        div class="min-w-[12rem]" x-data="typePickerData(q)" {
                            (single_choice_combobox_alpine_shell(true, "", ""))
                        }
                        input class="input input-bordered input-sm w-full" type="text" placeholder="Description (optional)" x-model="q.description" {}
                        div x-show="['MCQText','MCQWithCustom','Checkboxes','Dropdown'].includes(q.type_kind)" class="flex flex-col gap-1" {
                            template x-for="(opt, oi) in q.options" x-bind:key="oi" {
                                div class="flex gap-1 items-center" {
                                    input class="input input-bordered input-sm flex-1" type="text" x-model="q.options[oi]" {}
                                    button type="button" class="btn btn-ghost btn-sm btn-square text-error shrink-0" x-on:click="removeOption(q, oi)" aria-label="Remove option" { "✕" }
                                }
                            }
                            button type="button" class="btn btn-ghost btn-xs self-start" x-on:click="addOption(q)" { "+ Option" }
                        }
                        div x-show="q.type_kind === 'Number'" class="flex gap-2" {
                            input class="input input-bordered input-sm w-24" type="number" placeholder="Min" x-model="q.number_start" {}
                            input class="input input-bordered input-sm w-24" type="number" placeholder="Max" x-model="q.number_end" {}
                        }
                        div x-show="q.type_kind === 'LinearScale'" class="flex flex-wrap gap-2" {
                            input class="input input-bordered input-sm w-20" type="number" placeholder="Start" x-model="q.scale_start" {}
                            input class="input input-bordered input-sm w-20" type="number" placeholder="End" x-model="q.scale_end" {}
                            input class="input input-bordered input-sm flex-1" type="text" placeholder="Start label" x-model="q.scale_start_label" {}
                            input class="input input-bordered input-sm flex-1" type="text" placeholder="End label" x-model="q.scale_end_label" {}
                        }
                        div x-show="q.type_kind === 'Rating'" class="flex gap-2" {
                            input class="input input-bordered input-sm w-20" type="number" placeholder="Max" x-model="q.rating_range" {}
                            select class="select select-bordered select-sm" x-model="q.rating_marker" {
                                @for (value, label) in RATING_MARKERS {
                                    option value=(value) { (label) }
                                }
                            }
                        }
                        div x-show="['MCQGrid','TickBoxGrid'].includes(q.type_kind)" class="grid grid-cols-1 md:grid-cols-2 gap-2" {
                            div {
                                div class="text-xs font-medium mb-1" { "Rows" }
                                template x-for="(row, ri) in q.grid_rows" x-bind:key="'r'+ri" {
                                    div class="flex gap-1 mb-1" {
                                        input class="input input-bordered input-sm flex-1" type="text" x-model="q.grid_rows[ri]" {}
                                        button type="button" class="btn btn-ghost btn-xs" x-on:click="removeGridRow(q, ri)" { "✕" }
                                    }
                                }
                                button type="button" class="btn btn-ghost btn-xs" x-on:click="addGridRow(q)" { "+ Row" }
                            }
                            div {
                                div class="text-xs font-medium mb-1" { "Columns" }
                                template x-for="(col, ci) in q.grid_cols" x-bind:key="'c'+ci" {
                                    div class="flex gap-1 mb-1" {
                                        input class="input input-bordered input-sm flex-1" type="text" x-model="q.grid_cols[ci]" {}
                                        button type="button" class="btn btn-ghost btn-xs" x-on:click="removeGridCol(q, ci)" { "✕" }
                                    }
                                }
                                button type="button" class="btn btn-ghost btn-xs" x-on:click="addGridCol(q)" { "+ Column" }
                            }
                        }
                    }
                }
                button type="button" class="btn btn-outline btn-sm self-start" x-on:click="addQuestion()" { "+ Add question" }
            }
        }
    }
}

/// Field wrapper for templates that need explicit rendering.
pub fn field_form_questions(opts: InputFormQuestions<'_>) -> Markup {
    html! {
        div class="form-control w-full" {
            label class="label" { span class="label-text" { "Questions" } }
            (input_form_questions(opts))
        }
    }
}
