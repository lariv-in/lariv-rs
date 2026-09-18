//! Dynamic answer editor driven by parent form questions (Alpine + hidden JSON).

use maud::{Markup, html};

use crate::components::attrs::escape_attr;

fn alpine_methods() -> &'static str {
    r#"parseQuestions(raw) {
	if (!raw) return [];
	try {
		const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
		return Array.isArray(parsed) ? parsed : [];
	} catch (e) { return []; }
},
loadQuestions(raw) {
	this.questions = this.parseQuestions(raw);
	this.initAnswers();
},
initAnswers() {
	const existing = this.parseAnswersMap(this.defaults);
	this.answerRows = (this.questions || []).map((q) => {
		const id = q.form_question_id;
		const qt = q.question_type;
		const kind = typeof qt === 'string' ? qt : (qt && Object.keys(qt)[0]) || 'ShortText';
		const payload = typeof qt === 'object' && qt ? qt[kind] : null;
		const prev = existing[id] || null;
		const mcqCustom = this.readMcqWithCustom(prev, payload);
		const scaleSpec = kind === 'LinearScale' && payload ? payload : null;
		const scaleStart = scaleSpec ? (parseInt(scaleSpec.start, 10) || 1) : 1;
		const linearScale = this.readAnswer(prev, 'LinearScale');
		return {
			id,
			display_text: q.display_text || '',
			required: !!q.required,
			kind,
			payload,
			short_text: this.readAnswer(prev, 'ShortText') || '',
			long_text: this.readAnswer(prev, 'LongText') || '',
			number_val: this.readAnswer(prev, 'Number') ?? '',
			mcq_text: this.readAnswer(prev, 'MCQText') || this.readAnswer(prev, 'Dropdown') || '',
			mcq_pick: mcqCustom.pick,
			mcq_custom: mcqCustom.custom,
			checkboxes: this.readCheckboxes(prev, payload),
			linear_scale: linearScale == null || linearScale === '' ? scaleStart : linearScale,
			rating: this.readAnswer(prev, 'Rating') ?? '',
			grid_selections: this.readGridSelections(prev, 'MCQGrid'),
			grid_checks: this.readGridChecks(prev, 'TickBoxGrid'),
			date_val: this.readAnswer(prev, 'Date') || '',
			time_val: this.readAnswer(prev, 'Time') || '',
		};
	});
},
parseAnswersMap(raw) {
	if (!raw) return {};
	try {
		const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
		return parsed && typeof parsed === 'object' ? parsed : {};
	} catch (e) { return {}; }
},
readAnswer(prev, kind) {
	if (!prev || typeof prev !== 'object') return null;
	if (prev[kind] !== undefined) return prev[kind];
	return null;
},
asArray(v) {
	if (Array.isArray(v)) return v;
	if (v && typeof v === 'object') return Object.values(v);
	return [];
},
setMcqText(row, opt) {
	row.mcq_text = opt;
	this.syncHiddenField();
},
setMcqPick(row, opt) {
	row.mcq_pick = opt;
	this.syncHiddenField();
},
readMcqWithCustom(prev, payload) {
	const val = this.readAnswer(prev, 'MCQWithCustom') || '';
	const opts = this.asArray(payload);
	if (!val) return { pick: '', custom: '' };
	if (opts.includes(val)) return { pick: val, custom: '' };
	return { pick: '__custom__', custom: val };
},
readGridSelections(prev, kind) {
	const v = this.readAnswer(prev, kind);
	if (v && v.selections && typeof v.selections === 'object') return { ...v.selections };
	if (v && v.row && v.col) return { [v.row]: v.col };
	return {};
},
readGridChecks(prev, kind) {
	const v = this.readAnswer(prev, kind);
	if (v && v.selections && typeof v.selections === 'object') {
		const out = {};
		for (const [row, cols] of Object.entries(v.selections)) {
			out[row] = Array.isArray(cols) ? cols.slice() : [];
		}
		return out;
	}
	if (v && Array.isArray(v.rows) && Array.isArray(v.cols)) {
		const out = {};
		for (let i = 0; i < v.rows.length; i++) {
			const row = v.rows[i];
			const col = v.cols[i];
			if (!row || !col) continue;
			if (!out[row]) out[row] = [];
			if (!out[row].includes(col)) out[row].push(col);
		}
		return out;
	}
	return {};
},
readCheckboxes(prev, payload) {
	const v = this.readAnswer(prev, 'Checkboxes');
	if (Array.isArray(v)) return v.slice();
	return [];
},
toggleCheckbox(row, opt) {
	if (!Array.isArray(row.checkboxes)) row.checkboxes = [];
	const i = row.checkboxes.indexOf(opt);
	if (i >= 0) row.checkboxes.splice(i, 1);
	else row.checkboxes.push(opt);
	this.syncHiddenField();
},
gridMcqChecked(row, rowLabel, colLabel) {
	return (row.grid_selections || {})[rowLabel] === colLabel;
},
gridSelectMcq(row, rowLabel, colLabel) {
	if (!row.grid_selections) row.grid_selections = {};
	row.grid_selections[rowLabel] = colLabel;
	this.syncHiddenField();
},
gridTickChecked(row, rowLabel, colLabel) {
	const cols = (row.grid_checks || {})[rowLabel];
	return Array.isArray(cols) && cols.includes(colLabel);
},
gridToggleTick(row, rowLabel, colLabel) {
	if (!row.grid_checks) row.grid_checks = {};
	if (!Array.isArray(row.grid_checks[rowLabel])) row.grid_checks[rowLabel] = [];
	const cols = row.grid_checks[rowLabel];
	const i = cols.indexOf(colLabel);
	if (i >= 0) cols.splice(i, 1);
	else cols.push(colLabel);
	this.syncHiddenField();
},
hasText(value) {
	return String(value ?? '').trim() !== '';
},
buildAnswers() {
	const out = {};
	for (const row of this.answerRows || []) {
		const id = row.id;
		let val = null;
		switch (row.kind) {
			case 'ShortText':
				if (this.hasText(row.short_text)) val = { ShortText: String(row.short_text).trim() };
				break;
			case 'LongText':
				if (this.hasText(row.long_text)) val = { LongText: String(row.long_text).trim() };
				break;
			case 'Number': {
				const raw = String(row.number_val ?? '').trim();
				if (raw !== '') val = { Number: parseFloat(raw) || 0 };
				break;
			}
			case 'MCQText':
				if (this.hasText(row.mcq_text)) val = { MCQText: String(row.mcq_text).trim() };
				break;
			case 'MCQWithCustom': {
				const ans = row.mcq_pick === '__custom__'
					? (row.mcq_custom || '')
					: (row.mcq_pick || '');
				if (this.hasText(ans)) val = { MCQWithCustom: String(ans).trim() };
				break;
			}
			case 'Dropdown':
				if (this.hasText(row.mcq_text)) val = { Dropdown: String(row.mcq_text).trim() };
				break;
			case 'Checkboxes':
				if (Array.isArray(row.checkboxes) && row.checkboxes.length > 0) {
					val = { Checkboxes: row.checkboxes };
				}
				break;
			case 'LinearScale':
				val = { LinearScale: parseInt(row.linear_scale, 10) || 0 };
				break;
			case 'Rating':
				if (row.rating !== '' && row.rating != null) {
					val = { Rating: parseInt(row.rating, 10) || 0 };
				}
				break;
			case 'MCQGrid':
				if (row.grid_selections && Object.keys(row.grid_selections).length > 0) {
					val = { MCQGrid: { selections: row.grid_selections } };
				}
				break;
			case 'TickBoxGrid':
				if (row.grid_checks && Object.keys(row.grid_checks).length > 0) {
					val = { TickBoxGrid: { selections: row.grid_checks } };
				}
				break;
			case 'Date':
				if (this.hasText(row.date_val)) val = { Date: String(row.date_val).trim() };
				break;
			case 'Time':
				if (this.hasText(row.time_val)) val = { Time: String(row.time_val).trim() };
				break;
		}
		if (val) out[id] = val;
	}
	return out;
},
scaleStart(row) {
	const spec = row.payload || {};
	return parseInt(spec.start, 10) || 1;
},
scaleEnd(row) {
	const spec = row.payload || {};
	return parseInt(spec.end, 10) || 5;
},
scaleStartLabel(row) {
	const spec = row.payload || {};
	return spec.start_label || String(this.scaleStart(row));
},
scaleEndLabel(row) {
	const spec = row.payload || {};
	return spec.end_label || String(this.scaleEnd(row));
},
ratingOptions(row) {
	const spec = row.payload || {};
	const max = parseInt(spec.range, 10) || 5;
	const opts = [];
	for (let i = 1; i <= max; i++) opts.push(i);
	return opts;
},
ratingIconName(row) {
	const marker = (row.payload && row.payload.marker) || 'Star';
	const map = { Star: 'star', Heart: 'heart', Like: 'hand-thumb-up' };
	return map[marker] || 'star';
},
ratingIconStyle(row, n) {
	const name = this.ratingIconName(row);
	const filled = parseInt(row.rating, 10) >= n;
	const set = filled ? 'heroicons-solid' : 'heroicons';
	return `--heroicon-url: url('https://api.iconify.design/${set}/${name}.svg')`;
},
optionList(row) {
	return this.asArray(row.payload);
},
gridRows(row) {
	const payload = row.payload || {};
	return this.asArray(payload.rows);
},
gridCols(row) {
	const payload = row.payload || {};
	return this.asArray(payload.cols);
},
syncHiddenField(fieldName) {
	const name = fieldName || this.answersFieldName || '';
	if (!name) return '';
	let json = '{}';
	try {
		json = JSON.stringify(this.buildAnswers());
	} catch (e) {
		return '';
	}
	this.hidden_answers = json;
	return json;
},
pushAnswersToHtmxBody(ev, fieldName) {
	const json = this.syncHiddenField(fieldName);
	if (!json) return;
	const name = fieldName || this.answersFieldName || '';
	if (!name) return;
	const ctx = ev.detail && (ev.detail.ctx || ev.detail);
	const body = ctx && ctx.request && ctx.request.body;
	if (body && typeof body.set === 'function') {
		body.set(name, json);
	}
},
wireAnswersForm(fieldName) {
	if (this._answersFormWired) return;
	this._answersFormWired = true;
	this.answersFieldName = fieldName;
	const form = this.$el.closest('form');
	if (!form) return;
	const sync = () => this.syncHiddenField(fieldName);
	const push = (ev) => this.pushAnswersToHtmxBody(ev, fieldName);
	for (const evtName of ['htmx:config:request', 'htmx:before:request']) {
		form.addEventListener(evtName, push);
	}
	form.addEventListener('submit', () => { sync(); }, true);
	this.$watch('answerRows', () => { sync(); }, { deep: true });
	sync();
	if (window.htmx) window.htmx.process(form);
	if (window.htmx) window.htmx.process(this.$el);
},
onFkSelect(ev) {
	const d = ev && ev.detail;
	if (!d || d.id == null) return;
	const target = String(d.target_input || d.name || '').toLowerCase();
	if (!target.includes('form')) return;
	const fid = parseInt(d.id, 10);
	if (!fid) return;
	const qs = (this.catalog && this.catalog[fid]) ? this.catalog[fid] : [];
	this.loadQuestions(qs);
}"#
}

/// Parse user/submitted JSON and re-serialize for safe embedding in Alpine `x-data`.
fn alpine_json_literal(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .and_then(|v| serde_json::to_string(&v))
        .unwrap_or_else(|_| fallback.to_string())
}

pub struct InputFormAnswers<'a> {
    pub name: &'a str,
    pub defaults: &'a str,
    pub questions_json: &'a str,
    pub forms_catalog_json: &'a str,
    pub classes: &'a str,
}

impl Default for InputFormAnswers<'_> {
    fn default() -> Self {
        Self {
            name: "answers_json",
            defaults: "{}",
            questions_json: "[]",
            forms_catalog_json: "{}",
            classes: "w-full",
        }
    }
}

/// Render the dynamic answer editor.
pub fn input_form_answers(opts: InputFormAnswers<'_>) -> Markup {
    let defaults_json = alpine_json_literal(opts.defaults, "{}");
    let questions_json = alpine_json_literal(opts.questions_json, "[]");
    let catalog_json = alpine_json_literal(opts.forms_catalog_json, "{}");

    let hidden_initial =
        serde_json::to_string(&alpine_json_literal(opts.defaults, "{}")).unwrap_or_else(|_| {
            "\"{}\"".into()
        });
    let alpine_data = format!(
        "{{ questions: [], answerRows: [], hidden_answers: {hidden_initial}, defaults: {defaults_json}, catalog: {catalog_json}, {methods} }}",
        methods = alpine_methods().trim_end_matches(',')
    );

    let name_escaped = escape_attr(opts.name);
    let field_name_js =
        serde_json::to_string(opts.name).unwrap_or_else(|_| "\"AnswersJson\"".into());
    let init_js = format!(
        r#"
(function () {{
	const d = Alpine.$data($el);
	if (!d) return;
	d.loadQuestions({questions_json});
	d.wireAnswersForm({field_name_js});
}})();
$nextTick(() => {{ if (window.htmx) window.htmx.process($el); }});
document.addEventListener('fk-select', (ev) => {{
	const d = Alpine.$data($el);
	if (d && d.onFkSelect) d.onFkSelect(ev);
}});"#,
        field_name_js = field_name_js,
    );

    html! {
        div class=(opts.classes) data-form-answers="" x-data=(alpine_data) x-init=(init_js) {
            input type="hidden" name=(name_escaped) data-form-answers-hidden="" x-model="hidden_answers" {}
            div class="flex flex-col gap-3" {
                template x-if="answerRows.length === 0" {
                    p class="text-sm opacity-60" { "Select a form to edit answers." }
                }
                template x-for="row in answerRows" x-bind:key="row.id" {
                    div class="border border-base-300 rounded-lg p-3 flex flex-col gap-2" {
                        div class="font-medium text-sm" {
                            span x-text="row.display_text" {}
                            span x-show="row.required" class="text-error ml-1" { "*" }
                        }
                        template x-if="row.kind === 'ShortText'" {
                            div {
                                input class="input input-bordered input-sm w-full" type="text" name="" x-model="row.short_text" {}
                            }
                        }
                        template x-if="row.kind === 'LongText'" {
                            div {
                                textarea class="textarea textarea-bordered w-full" rows="3" name="" x-model="row.long_text" {}
                            }
                        }
                        template x-if="row.kind === 'Number'" {
                            div {
                                input class="input input-bordered input-sm w-full" type="number" name="" x-model="row.number_val" {}
                            }
                        }
                        template x-if="row.kind === 'MCQText'" {
                            div class="flex flex-col gap-1" {
                                template x-for="(opt, i) in optionList(row)" x-bind:key="row.id + '-mcq-' + i" {
                                    label class="label cursor-pointer justify-start gap-2 py-1" {
                                        input type="radio" class="radio radio-sm" name=""
                                            x-bind:value="opt"
                                            x-bind:checked="row.mcq_text === opt"
                                            x-on:change="setMcqText(row, opt)" {}
                                        span class="label-text" x-text="opt" {}
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'MCQWithCustom'" {
                            div class="flex flex-col gap-1" {
                                template x-for="(opt, i) in optionList(row)" x-bind:key="row.id + '-mcq-custom-' + i" {
                                    label class="label cursor-pointer justify-start gap-2 py-1" {
                                        input type="radio" class="radio radio-sm" name=""
                                            x-bind:value="opt"
                                            x-bind:checked="row.mcq_pick === opt"
                                            x-on:change="setMcqPick(row, opt)" {}
                                        span class="label-text" x-text="opt" {}
                                    }
                                }
                                label class="label cursor-pointer justify-start gap-2 py-1" {
                                    input type="radio" class="radio radio-sm" name=""
                                        value="__custom__"
                                        x-bind:checked="row.mcq_pick === '__custom__'"
                                        x-on:change="setMcqPick(row, '__custom__')" {}
                                    span class="label-text" { "Other" }
                                }
                                div x-show="row.mcq_pick === '__custom__'" class="pl-8" {
                                    input class="input input-bordered input-sm w-full" type="text" name=""
                                        placeholder="Your answer"
                                        x-model="row.mcq_custom" {}
                                }
                            }
                        }
                        template x-if="row.kind === 'Dropdown'" {
                            div {
                                select class="select select-bordered select-sm w-full" name="" x-model="row.mcq_text" {
                                    option value="" { "—" }
                                    template x-for="(opt, i) in optionList(row)" x-bind:key="row.id + '-dd-' + i" {
                                        option x-bind:value="opt" x-text="opt" {}
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'Checkboxes'" {
                            div class="flex flex-col gap-1" {
                                template x-for="(opt, i) in optionList(row)" x-bind:key="row.id + '-cb-' + i" {
                                    label class="label cursor-pointer justify-start gap-2 py-1" {
                                        input type="checkbox" class="checkbox checkbox-sm" name=""
                                            x-bind:checked="(row.checkboxes || []).includes(opt)"
                                            x-on:change="toggleCheckbox(row, opt)" {}
                                        span class="label-text" x-text="opt" {}
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'LinearScale'" {
                            div class="flex flex-col gap-2" {
                                div class="flex items-center gap-3" {
                                    span class="text-xs opacity-70 shrink-0" x-text="scaleStartLabel(row)" {}
                                    input type="range" class="range range-sm range-primary flex-1" name=""
                                        x-bind:min="scaleStart(row)"
                                        x-bind:max="scaleEnd(row)"
                                        step="1"
                                        x-model="row.linear_scale" {}
                                    span class="text-xs opacity-70 shrink-0" x-text="scaleEndLabel(row)" {}
                                }
                                div class="text-sm text-center opacity-80" x-text="row.linear_scale" {}
                            }
                        }
                        template x-if="row.kind === 'Rating'" {
                            div class="flex flex-wrap gap-1" {
                                template x-for="n in ratingOptions(row)" x-bind:key="row.id + '-rating-' + n" {
                                    button type="button" class="btn btn-ghost btn-sm p-1 min-h-0 h-auto"
                                        x-bind:class="parseInt(row.rating, 10) >= n ? 'text-warning' : 'opacity-40'"
                                        x-on:click="row.rating = n" {
                                        span class="heroicon heroicon-lg"
                                            x-bind:style="ratingIconStyle(row, n)" {}
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'MCQGrid'" {
                            div class="overflow-x-auto" {
                                table class="table table-sm table-zebra" {
                                    thead {
                                        tr {
                                            th {}
                                            template x-for="(col, ci) in gridCols(row)" x-bind:key="row.id + '-col-' + ci" {
                                                th class="text-center text-xs font-medium" x-text="col" {}
                                            }
                                        }
                                    }
                                    tbody {
                                        template x-for="(gridRow, ri) in gridRows(row)" x-bind:key="row.id + '-row-' + ri" {
                                            tr {
                                                td class="font-medium text-sm whitespace-nowrap" x-text="gridRow" {}
                                                template x-for="(col, ci) in gridCols(row)" x-bind:key="row.id + '-cell-' + ri + '-' + ci" {
                                                    td class="text-center" {
                                                        input type="radio" class="radio radio-sm" name=""
                                                            x-bind:checked="gridMcqChecked(row, gridRow, col)"
                                                            x-on:change="gridSelectMcq(row, gridRow, col)" {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'TickBoxGrid'" {
                            div class="overflow-x-auto" {
                                table class="table table-sm table-zebra" {
                                    thead {
                                        tr {
                                            th {}
                                            template x-for="(col, ci) in gridCols(row)" x-bind:key="row.id + '-tcol-' + ci" {
                                                th class="text-center text-xs font-medium" x-text="col" {}
                                            }
                                        }
                                    }
                                    tbody {
                                        template x-for="(gridRow, ri) in gridRows(row)" x-bind:key="row.id + '-trow-' + ri" {
                                            tr {
                                                td class="font-medium text-sm whitespace-nowrap" x-text="gridRow" {}
                                                template x-for="(col, ci) in gridCols(row)" x-bind:key="row.id + '-tcell-' + ri + '-' + ci" {
                                                    td class="text-center" {
                                                        input type="checkbox" class="checkbox checkbox-sm" name=""
                                                            x-bind:checked="gridTickChecked(row, gridRow, col)"
                                                            x-on:change="gridToggleTick(row, gridRow, col)" {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        template x-if="row.kind === 'Date'" {
                            div {
                                input class="input input-bordered input-sm w-full" type="date" name="" x-model="row.date_val" {}
                            }
                        }
                        template x-if="row.kind === 'Time'" {
                            div {
                                input class="input input-bordered input-sm w-full" type="time" name="" x-model="row.time_val" {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Field wrapper for templates.
pub fn field_form_answers(opts: InputFormAnswers<'_>) -> Markup {
    html! {
        div class="form-control w-full" {
            label class="label" { span class="label-text" { "Answers" } }
            (input_form_answers(opts))
        }
    }
}
