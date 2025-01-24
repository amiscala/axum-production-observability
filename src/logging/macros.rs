#[macro_export] macro_rules! log {
    ($level:expr,$message:expr,$($v:ident = $t:expr),*) => {

        let mut log_body: HashMap<&str, String> = HashMap::new();
        $(
            log_body.insert(&stringify!($v),$t.handleInput);
        )*
        log_body.insert("message", $message.to_string());
        let trace_id = tracing::Span::current().context().span().context().span().span_context().trace_id().to_string();
        let span_id = tracing::Span::current().context().span().context().span().span_context().span_id().to_string();
        event!(
            target: "log",
            $level,
            trace_id,
            span_id,
            "{}",
            serde_json::to_string(&log_body).unwrap()
        );
    };
    ($level:expr,$message:expr,span: $span:expr,response_body: $body:expr) =>{
        let trace_id = $span.context().span().span_context().trace_id().to_string();
        let span_id = $span.context().span().span_context().span_id().to_string();
        let mut log_body: HashMap<&str, String> = HashMap::new();
        log_body.insert("response_body",$body);
        log_body.insert("message", $message.to_string());
        event!(
            target: "log",
            $level,
            trace_id,
            span_id,
            "{}",
            serde_json::to_string(&log_body).unwrap()
        );
    };
    ($level:expr,$message:expr,span: $span:expr,request_body: $body:expr) => {
        let trace_id = $span.context().span().span_context().trace_id().to_string();
        let span_id = $span.context().span().span_context().span_id().to_string();
        let mut log_body: HashMap<&str, String> = HashMap::new();
        log_body.insert("request_body",$body);
        log_body.insert("message", $message.to_string());
        event!(
            target: "log",
            $level,
            trace_id,
            span_id,
            "{}",
            serde_json::to_string(&log_body).unwrap()
        );
    };
}
