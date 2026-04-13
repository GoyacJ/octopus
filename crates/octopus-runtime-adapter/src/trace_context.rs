use super::*;

pub(crate) fn runtime_trace_context(
    session_id: &str,
    parent_run_id: Option<String>,
) -> RuntimeTraceContext {
    RuntimeTraceContext {
        session_id: session_id.to_string(),
        trace_id: format!("trace-{}", Uuid::new_v4()),
        turn_id: format!("turn-{}", Uuid::new_v4()),
        parent_run_id,
    }
}
