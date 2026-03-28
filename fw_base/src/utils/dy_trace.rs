use tokio::task_local;

task_local! {
    static CURRENT_ACTION: String;
}

#[inline]
pub fn trace_with_action(action: &str) -> tracing::Span {
    let span = tracing::Span::current();
    // span.record("action", "pull_simple_info");

    // "pull_simple_info" => pull_simple_info
    span.record("action", &tracing::field::display(action));

    span
}
