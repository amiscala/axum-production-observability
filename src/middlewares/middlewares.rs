use crate::log;
use axum::http::header;
use axum::middleware::Next;
use axum::{extract::Request, response::Response};
use http_body_util::BodyExt;
use opentelemetry::trace::Status;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::Context;
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use std::collections::HashMap;
use tracing::event;
use tracing::{error_span, info_span, Instrument, Span};
use tracing_core::Level;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let request_span = make_request_span(&request);
    request_span.set_parent(extract_context(request.headers()));
    let (parts, body) = request.into_parts();
    let body_bytes = match body.collect().await {
        Ok(content) => content.to_bytes(),
        Err(e) => {
            // Maybe should implement tracing::Value, but then it would be necessary to create a wrapper for the Axum error type just for this line of code, not sure what would be
            // the benefits
            let error = format!("{:?}", e);
            error_span!("Error while trying to read body bytes {:?}", error);
            axum::body::Bytes::new()
        }
    };

    let string_body =
        String::from_utf8(body_bytes.clone().into()).unwrap_or_else(|e| format!("{:?}", e));
    log!(Level::INFO, "Received Request", span: &request_span,request_body: string_body);

    // rebuilds the request
    let request = Request::from_parts(parts, axum::body::Body::from(body_bytes));

    map_and_log_headers(
        &request_span,
        request.headers(),
        opentelemetry_semantic_conventions::trace::HTTP_REQUEST_HEADER,
    );
    let mut response = next.run(request).instrument(request_span.clone()).await;
    let ctx = request_span.context();
    inject_context(&ctx, response.headers_mut());
    map_and_log_headers(
        &request_span,
        response.headers(),
        opentelemetry_semantic_conventions::trace::HTTP_RESPONSE_HEADER,
    );
    match response.status().as_u16() {
        200..=399 => request_span.set_status(Status::Ok),
        400..=599 => request_span.set_status(Status::error("See trace spans for the error")),
        _ => request_span.set_status(Status::error("Unknown HTTP Status Code")),
    }
    let (response_parts, response_body) = response.into_parts();
    let response_body_bytes = match response_body.collect().await {
        Ok(content) => content.to_bytes(),
        Err(e) => {
            // Maybe should implement tracing::Value, but then it would be necessary to create a wrapper for the Axum error type just for this line of code, not sure what would be
            // the benefits
            let error = format!("{:?}", e);
            error_span!("Error while trying to read response body bytes {:?}", error);
            axum::body::Bytes::new()
        }
    };

    let response_body_string = String::from_utf8(response_body_bytes.clone().into())
        .unwrap_or_else(|e| format!("{:?}", e));
    log!(Level::INFO, "Received Request", span: &request_span,response_body: response_body_string);

    // log!(Level::INFO, "Response sent", response=&response_body_string);
    let response =
        Response::from_parts(response_parts, axum::body::Body::from(response_body_bytes));
    response
}

fn make_request_span(request: &Request) -> Span {
    let method = request.method().as_str();
    let uri = request.uri();
    let path = uri.path();
    let scheme = uri.scheme_str().unwrap_or("http");
    let query = uri.query().unwrap_or("");
    let (server_address, port) = if let Some(host_header) = request.headers().get("Host") {
        match host_header.to_str() {
            Ok(host_and_port) => match host_and_port.split_once(":") {
                Some((host, port)) => (host, port),
                None => {
                    let default_port = match scheme {
                        "http" => "80",
                        "https" => "443",
                        _ => "",
                    };
                    (host_and_port, default_port)
                }
            },
            Err(_e) => ("", ""),
        }
    } else {
        ("", "")
    };

    let query_and_path = match uri.path_and_query() {
        Some(val) => val.as_str(),
        None => "",
    };
    let formated_port = if port.is_empty() {
        "".to_string()
    } else {
        format!(":{}", port)
    };
    let full_uri = format!(
        "{}://{}{}{}",
        scheme, server_address, &formated_port, query_and_path
    );
    let real_ip_header_value = match request.headers().get("X-Real-Ip") {
        Some(x_real_ip) => x_real_ip.to_str().unwrap_or_else(|_e| "UNKNOWN"),
        None => "NOT_SENT",
    };
    let user_agent = match request.headers().get("User-Agent") {
        Some(user_agent) => user_agent.to_str().unwrap_or_else(|_e| "UNKNOWN"),
        None => "NOT_SENT",
    };

    // do something with `request`...
    let request_span = info_span!(
        "RequestSummary",
        service_name = env!("CARGO_PKG_NAME"),
        "http.request.method" = method,
        "server.address" = server_address,
        "server.port" = port,
        "url.scheme" = scheme,
        "url.full" = full_uri,
        "user_agent.original" = user_agent,
        "url.query" = query,
        "client.address" = real_ip_header_value,
        "http.request.path" = path,
    );
    request_span
}

fn inject_context(context: &Context, headers: &mut axum::http::HeaderMap) {
    let mut injector = HeaderInjector(headers);
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, &mut injector);
    });
}

fn extract_context(headers: &axum::http::HeaderMap) -> Context {
    let extractor = HeaderExtractor(headers);
    opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

fn map_and_log_headers(
    span: &Span,
    headers: &axum::http::HeaderMap<axum::http::HeaderValue>,
    header_name_prefix: &str,
) {
    let mut headers_dict: HashMap<String, Vec<String>> = HashMap::new();
    for (header_name, header_value) in headers {
        if let Ok(header_value_str) = header_value.to_str() {
            // Means it found the header value on the dict, which means it already has an entry so we will add a new one to it.
            if header_name == header::AUTHORIZATION.as_str() {
                if let Some(pos) = header_value_str.rfind(".") {
                    let (redacted_jwt, _) = header_value_str.split_at(pos + 1);
                    let jwt_to_be_logged = format!("{}REDACTED", redacted_jwt);
                    headers_dict.insert(header_name.to_string(), vec![jwt_to_be_logged]);
                } else {
                    continue;
                }
            } else {
                headers_dict.insert(header_name.to_string(), vec![header_value_str.to_string()]);
            }
        }
    }

    for header_vals in headers_dict {
        let header_values = header_vals.1.join(",");
        let string_header_name = format!("{}.{}", header_name_prefix, header_vals.0);
        span.set_attribute(string_header_name, header_values);
    }
}
