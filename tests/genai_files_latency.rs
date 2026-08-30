//! Latency / payload tests for Gemini Files API vs inline PDF attachments.
//!
//! Offline tests always run and quantify request JSON size for a ~265 KiB PDF.
//! Live tests are `#[ignore]` and need `GOOGLE_API_KEY` / `GEMINI_API_KEY`.
//!
//! ```text
//! cargo test --test genai_files_latency -- --nocapture
//! cargo test --test genai_files_latency -- --ignored --nocapture
//!
//! # optional real PDF (defaults to a synthetic 265 KiB PDF):
//! LARIV_LATENCY_PDF=/path/to/file.pdf cargo test --test genai_files_latency -- --ignored --nocapture
//! ```

#![cfg(feature = "cap-llm")]

use std::path::PathBuf;
use std::time::Instant;

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use lariv_rs::genai::{
    ASSISTANT_SYSTEM_PROMPT, Blob, Content, FileData, FunctionDeclaration, GenaiClient, Part, Role,
};
use lariv_rs::plugins::llm_assistant::config::{CHAT_MAX_OUTPUT_TOKENS, DEFAULT_CHAT_MODEL};

/// Matches the user-reported attachment size (~265 KiB).
const TARGET_PDF_BYTES: usize = 265 * 1024;
const TOOL_ROUNDS: usize = 5;

fn synthetic_pdf(target_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(target_len);
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    out.extend_from_slice(b"1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n");
    out.extend_from_slice(b"2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n");
    out.extend_from_slice(
        b"3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>endobj\n",
    );
    out.extend_from_slice(b"trailer<< /Root 1 0 R >>\nstartxref\n0\n%%EOF\n");
    while out.len() < target_len {
        let remaining = target_len - out.len();
        let chunk =
            b"%..............................................................................\n";
        out.extend_from_slice(&chunk[..remaining.min(chunk.len())]);
    }
    out
}

fn load_latency_pdf() -> (Vec<u8>, String) {
    if let Ok(path) = std::env::var("LARIV_LATENCY_PDF") {
        let path = PathBuf::from(path);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read LARIV_LATENCY_PDF={}: {e}", path.display()));
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment.pdf")
            .to_string();
        return (bytes, name);
    }
    (synthetic_pdf(TARGET_PDF_BYTES), "latency-265k.pdf".into())
}

fn inline_user_content(name: &str, bytes: &[u8]) -> Content {
    Content {
        role: Role::User,
        parts: vec![
            Part {
                text: Some("Incoming email with PDF attachment. Summarize it briefly.".into()),
                ..Default::default()
            },
            Part {
                inline_data: Some(Blob {
                    mime_type: "application/pdf".into(),
                    data: B64.encode(bytes),
                }),
                display_name: name.to_string(),
                ..Default::default()
            },
        ],
    }
}

fn file_data_user_content(name: &str, file_uri: &str) -> Content {
    Content {
        role: Role::User,
        parts: vec![
            Part {
                text: Some("Incoming email with PDF attachment. Summarize it briefly.".into()),
                ..Default::default()
            },
            Part {
                file_data: Some(FileData {
                    file_uri: file_uri.to_string(),
                    mime_type: "application/pdf".into(),
                }),
                display_name: name.to_string(),
                ..Default::default()
            },
        ],
    }
}

fn fake_tool_round_pair(round: usize) -> (Content, Content) {
    let model = Content {
        role: Role::Model,
        parts: vec![Part {
            function_call: Some(lariv_rs::genai::FunctionCall {
                id: format!("call-{round}"),
                name: "list_skills".into(),
                args: Some(serde_json::json!({})),
                ..Default::default()
            }),
            ..Default::default()
        }],
    };
    let tool = Content {
        role: Role::User,
        parts: vec![Part {
            function_response: Some(lariv_rs::genai::FunctionResponse {
                function_response_id: format!("call-{round}"),
                name: "list_skills".into(),
                response: Some(serde_json::json!({ "skills": [] })),
                ..Default::default()
            }),
            ..Default::default()
        }],
    };
    (model, tool)
}

fn sample_tool_decls() -> Vec<FunctionDeclaration> {
    vec![FunctionDeclaration {
        name: "list_skills".into(),
        description: "List skills".into(),
        parameters: Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
    }]
}

fn request_len(contents: Vec<Content>) -> usize {
    GenaiClient::generate_request_json_len(contents, CHAT_MAX_OUTPUT_TOKENS, &sample_tool_decls())
        .expect("serialize request")
}

#[test]
fn offline_265k_pdf_file_data_request_stays_small_across_tool_rounds() {
    let (pdf, name) = load_latency_pdf();
    assert!(
        pdf.len() >= 200 * 1024,
        "expected ~265KiB fixture, got {} bytes",
        pdf.len()
    );

    let inline_user = inline_user_content(&name, &pdf);
    let file_user = file_data_user_content(
        &name,
        "https://generativelanguage.googleapis.com/v1beta/files/latency-test-uri",
    );

    let mut inline_history = vec![inline_user];
    let mut file_history = vec![file_user];
    let mut inline_total = 0usize;
    let mut file_total = 0usize;

    eprintln!("offline payload probe: pdf_bytes={} name={name}", pdf.len());
    eprintln!("system_prompt_chars={}", ASSISTANT_SYSTEM_PROMPT.len());

    for round in 1..=TOOL_ROUNDS {
        let inline_len = request_len(inline_history.clone());
        let file_len = request_len(file_history.clone());
        inline_total = inline_total.saturating_add(inline_len);
        file_total = file_total.saturating_add(file_len);
        eprintln!(
            "round {round}: inline_json={inline_len} file_data_json={file_len} ratio={:.1}x",
            inline_len as f64 / file_len.max(1) as f64
        );

        if round == 1 {
            assert!(
                inline_len > 250_000,
                "inline request should embed ~265KiB PDF as base64; got {inline_len}"
            );
            assert!(
                file_len < 20_000,
                "file_data request should stay small; got {file_len}"
            );
        }

        let (model, tool) = fake_tool_round_pair(round);
        inline_history.push(model.clone());
        inline_history.push(tool.clone());
        file_history.push(model);
        file_history.push(tool);
    }

    eprintln!("cumulative inline_json={inline_total} file_data_json={file_total}");
    assert!(
        inline_total > file_total.saturating_mul(10),
        "inline cumulative payload should dwarf file_data across {TOOL_ROUNDS} rounds \
         (inline={inline_total}, file_data={file_total})"
    );
}

#[test]
fn offline_reports_bytes_reuploaded_per_tool_round_with_inline() {
    let (pdf, name) = load_latency_pdf();
    let inline_user = inline_user_content(&name, &pdf);
    let round1 = request_len(vec![inline_user.clone()]);
    let mut hist = vec![inline_user];
    let (model, tool) = fake_tool_round_pair(1);
    hist.push(model);
    hist.push(tool);
    let round2 = request_len(hist);
    // The PDF base64 remains in history, so round 2 is at least as large as round 1.
    assert!(
        round2 >= round1,
        "tool round must re-send prior inline PDF (round1={round1}, round2={round2})"
    );
    eprintln!(
        "inline re-send: round1={round1} round2={round2} delta={}",
        round2 - round1
    );
}

#[test]
fn offline_elided_followup_drops_pdf_from_request() {
    use lariv_rs::plugins::llm_assistant::content::elide_attachment_parts_for_api;

    let (pdf, name) = load_latency_pdf();
    let file_user = file_data_user_content(
        &name,
        "https://generativelanguage.googleapis.com/v1beta/files/latency-test-uri",
    );
    let mut hist = vec![file_user];
    let (model, tool) = fake_tool_round_pair(1);
    hist.push(model);
    hist.push(tool);

    let with_file = request_len(hist.clone());
    elide_attachment_parts_for_api(&mut hist);
    let elided = request_len(hist);
    eprintln!("follow-up with file_uri still present={with_file} after elide={elided}");
    assert!(
        elided < with_file,
        "eliding attachments must shrink follow-up requests"
    );
    // Elided follow-ups should not keep paying for multimodal PDF context size.
    assert!(
        elided < 8_000,
        "elided follow-up should be tiny text; got {elided} (pdf was {} bytes)",
        pdf.len()
    );
}

fn require_api_key() -> String {
    std::env::var("GOOGLE_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .unwrap_or_default()
}

#[tokio::test]
#[ignore = "live Gemini latency probe; run with --ignored --nocapture"]
async fn live_265k_pdf_upload_and_tool_round_latency() {
    let api_key = require_api_key();
    assert!(
        !api_key.trim().is_empty(),
        "set GOOGLE_API_KEY or GEMINI_API_KEY for live latency test"
    );

    let (pdf, name) = load_latency_pdf();
    let model = std::env::var("LARIV_LATENCY_MODEL").unwrap_or_else(|_| DEFAULT_CHAT_MODEL.into());
    let client = GenaiClient::new(api_key, model.clone());
    let decls = sample_tool_decls();

    eprintln!(
        "live latency probe: model={model} pdf_bytes={} name={name}",
        pdf.len()
    );

    let upload_started = Instant::now();
    let (uploaded, timing) = client
        .upload_file_timed(&name, "application/pdf", &pdf)
        .await
        .unwrap_or_else(|e| panic!("upload_file_timed failed: {e}"));
    eprintln!(
        "upload: total_ms={} start_ms={} bytes_ms={} poll_ms={} uri={}",
        upload_started.elapsed().as_millis(),
        timing.start_ms,
        timing.bytes_ms,
        timing.poll_ms,
        uploaded.uri
    );

    let mut history = vec![file_data_user_content(&name, &uploaded.uri)];

    for round in 1..=3 {
        let req_len =
            GenaiClient::generate_request_json_len(history.clone(), CHAT_MAX_OUTPUT_TOKENS, &decls)
                .expect("request len");
        let started = Instant::now();
        let model_content = client
            .stream_generate_content(history.clone(), CHAT_MAX_OUTPUT_TOKENS, &decls, |_| {})
            .await
            .unwrap_or_else(|e| panic!("stream round {round} failed: {e}"));
        let elapsed = started.elapsed().as_millis();
        let has_fc = model_content
            .parts
            .iter()
            .any(|p| p.function_call.is_some());
        eprintln!(
            "generate round {round}: {elapsed}ms request_json={req_len} parts={} has_function_call={has_fc}",
            model_content.parts.len()
        );

        history.push(model_content);
        if has_fc {
            // Mirror the assistant tool loop: append a tiny function response and continue.
            let (model_stub, tool) = fake_tool_round_pair(round);
            let _ = model_stub;
            history.push(tool);
        } else {
            // Force another round with the PDF still in history (follow-up user turn).
            history.push(Content::text(
                Role::User,
                format!("Follow-up {round}: list one key detail from the PDF."),
            ));
        }
    }

    eprintln!(
        "interpretation: if upload is fast but every generate round stays slow while \
         request_json stays small, Gemini is re-processing the PDF each round via file_uri \
         (payload fix alone will not remove that latency)."
    );
}
