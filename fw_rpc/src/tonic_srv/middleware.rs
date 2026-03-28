pub fn rpc_trace_intercept(
    mut req: tonic::Request<()>,
) -> Result<tonic::Request<()>, tonic::Status> {
    let cur_span = tracing::Span::current();
    tracing::info!("cur_span:{:#?}", cur_span);

    // req.metadata_mut()
    //     .insert("x-req-id", req_id.parse().unwrap());
    // req.metadata_mut()
    //     .insert("x-action", action.parse().unwrap());

    Ok(req)
}
