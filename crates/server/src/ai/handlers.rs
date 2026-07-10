use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::{self, Stream, StreamExt};
use std::convert::Infallible;

use crate::ai::backend::LlmBackend;
use crate::ai::dto::{AiRequest, AiResponse, AiScenario};
use crate::state::AppState;

pub async fn suggest_handler(
    State(state): State<AppState>,
    Json(req): Json<AiRequest>,
) -> Json<AiResponse> {
    let backend = LlmBackend::from_config(&state.config.llm);
    let suggestions = backend.suggest(&req.scenario, &req.context).await.unwrap_or_default();
    let summary = format!("{} 分析完成，共 {} 条建议", req.scenario, suggestions.len());

    Json(AiResponse {
        scenario: req.scenario.to_string(),
        suggestions,
        summary,
    })
}

pub async fn stream_handler(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<StreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let scenario = match query.scenario.as_deref() {
        Some("process_design") => AiScenario::ProcessDesign,
        Some("gap_analysis") => AiScenario::GapAnalysis,
        Some("redundancy_identification") => AiScenario::RedundancyIdentification,
        _ => AiScenario::CapabilityDecomposition,
    };

    let backend = LlmBackend::from_config(&state.config.llm);
    let suggestions = backend.suggest(&scenario, &serde_json::json!({})).await.unwrap_or_default();
    let scenario_str = scenario.to_string();

    let suggestion_stream = stream::iter(suggestions.into_iter().enumerate().map(move |(i, s)| {
        let data = serde_json::to_string(&s).unwrap_or_default();
        Ok(Event::default().event("suggestion").id(i.to_string()).data(data))
    }));

    let done_stream = stream::once(async move {
        Ok(Event::default()
            .event("done")
            .data(format!("{{\"scenario\":\"{}\"}}", scenario_str)))
    });

    let stream = suggestion_stream.chain(done_stream);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, serde::Deserialize)]
pub struct StreamQuery {
    pub scenario: Option<String>,
}
