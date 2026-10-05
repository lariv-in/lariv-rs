use lariv_core::db::migration_sql::exec_sql;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    r#"
CREATE TYPE hr_job_posting_question AS (
    job_posting_question_id uuid,
    display_text text,
    question_type jsonb,
    required boolean,
    description text
)
"#,
    r#"
CREATE TYPE hr_job_posting_answer AS (
    question hr_job_posting_question,
    answer jsonb
)
"#,
    "ALTER TABLE hr_job_forms ADD COLUMN questions hr_job_posting_question[] NOT NULL DEFAULT '{}'",
    r#"
UPDATE hr_job_forms jf
SET questions = COALESCE((
    SELECT ARRAY(
        SELECT ROW(
            (q->>'form_question_id')::uuid,
            COALESCE(q->>'display_text', ''),
            COALESCE(q->'question_type', 'null'::jsonb),
            COALESCE((q->>'required')::boolean, false),
            COALESCE(q->>'description', '')
        )::hr_job_posting_question
        FROM forms f
        CROSS JOIN LATERAL jsonb_array_elements(
            CASE WHEN jsonb_typeof(f.questions) = 'array' THEN f.questions ELSE '[]'::jsonb END
        ) AS q
        WHERE f.id = jf.form_id
    )
), '{}'::hr_job_posting_question[])
"#,
    "ALTER TABLE hr_job_forms DROP CONSTRAINT fk_hr_job_forms_form_id",
    "DROP INDEX idx_hr_job_forms_form_id",
    "ALTER TABLE hr_job_forms DROP COLUMN form_id",
    "ALTER TABLE hr_applicants ADD COLUMN answers hr_job_posting_answer[] NOT NULL DEFAULT '{}'",
    r#"
UPDATE hr_applicants a
SET answers = COALESCE((
    SELECT ARRAY(
        SELECT ROW(
            ROW(
                (q->>'form_question_id')::uuid,
                COALESCE(q->>'display_text', ''),
                COALESCE(q->'question_type', 'null'::jsonb),
                COALESCE((q->>'required')::boolean, false),
                COALESCE(q->>'description', '')
            )::hr_job_posting_question,
            ans.value
        )::hr_job_posting_answer
        FROM form_responses fr
        JOIN forms f ON f.id = fr.form_id
        CROSS JOIN LATERAL jsonb_each(
            CASE WHEN jsonb_typeof(fr.answers) = 'object' THEN fr.answers ELSE '{}'::jsonb END
        ) AS ans(key, value)
        JOIN LATERAL jsonb_array_elements(
            CASE WHEN jsonb_typeof(f.questions) = 'array' THEN f.questions ELSE '[]'::jsonb END
        ) AS q ON (q->>'form_question_id') = ans.key
        WHERE fr.id = a.form_response_id
    )
), '{}'::hr_job_posting_answer[])
WHERE a.form_response_id IS NOT NULL
"#,
    "ALTER TABLE hr_applicants DROP CONSTRAINT fk_hr_applicants_form_response_id",
    "DROP INDEX idx_hr_applicants_form_response_id",
    "ALTER TABLE hr_applicants DROP COLUMN form_response_id",
];

const DOWN: &[&str] = &[
    "ALTER TABLE hr_applicants ADD COLUMN form_response_id bigint NULL",
    "CREATE INDEX idx_hr_applicants_form_response_id ON hr_applicants (form_response_id)",
    r#"
ALTER TABLE hr_applicants
    ADD CONSTRAINT fk_hr_applicants_form_response_id
    FOREIGN KEY (form_response_id) REFERENCES form_responses (id) ON DELETE SET NULL
"#,
    "ALTER TABLE hr_applicants DROP COLUMN answers",
    "ALTER TABLE hr_job_forms ADD COLUMN form_id bigint NULL",
    "CREATE INDEX idx_hr_job_forms_form_id ON hr_job_forms (form_id)",
    r#"
ALTER TABLE hr_job_forms
    ADD CONSTRAINT fk_hr_job_forms_form_id
    FOREIGN KEY (form_id) REFERENCES forms (id) ON DELETE RESTRICT
"#,
    "ALTER TABLE hr_job_forms DROP COLUMN questions",
    "DROP TYPE hr_job_posting_answer",
    "DROP TYPE hr_job_posting_question",
];

async fn exec_each(manager: &SchemaManager<'_>, statements: &[&str]) -> Result<(), DbErr> {
    for sql in statements {
        exec_sql(manager, sql).await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_each(manager, UP).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_each(manager, DOWN).await
    }
}
