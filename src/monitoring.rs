use lazy_static::lazy_static;
use prometheus::{IntCounter, Registry};
use std::result::Result;
use warp::{Filter, Rejection, Reply};

lazy_static! {
    pub static ref INCOMING_REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").expect("metric can be created");
    pub static ref REGISTRY: Registry = Registry::new();
}

pub async fn web_main() {
    register_custom_metrics();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    let status_route = warp::path!("status").and_then(status_handler);

    warp::serve(metrics_route.or(status_route))
        .run(([0, 0, 0, 0], 9090))
        .await;
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("collector can be registered");
}

async fn status_handler() -> Result<impl Reply, Rejection> {
    Ok("ok")
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        eprintln!("could not encode custom metrics: {}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        eprintln!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}
