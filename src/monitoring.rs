use lazy_static::lazy_static;
use prometheus::{IntCounter, Registry};
use std::result::Result;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use warp::{Filter, Rejection, Reply};

lazy_static! {
    pub static ref INCOMING_REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").expect("metric can be created");
    pub static ref PROCESSED_ITEMS: IntCounter =
        IntCounter::new("processed_items", "Processed Items").expect("metric can be created");
    pub static ref REGISTRY: Registry = Registry::new();
}

#[cfg(test)]
lazy_static! {
    static ref METRICS_REGISTERED: AtomicBool = AtomicBool::new(false);
}

#[cfg(test)]
fn register_custom_metrics() {
    if METRICS_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        REGISTRY
            .register(Box::new(INCOMING_REQUESTS.clone()))
            .expect("collector can be registered");
        REGISTRY
            .register(Box::new(PROCESSED_ITEMS.clone()))
            .expect("collector can be registered");
    }
}

#[cfg(not(test))]
fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(PROCESSED_ITEMS.clone()))
        .expect("collector can be registered");
}

#[cfg(not(tarpaulin_include))]
pub async fn web_main() {
    register_custom_metrics();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    let status_route = warp::path!("status").and_then(status_handler);

    warp::serve(metrics_route.or(status_route))
        .run(([0, 0, 0, 0], 9090))
        .await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Client, StatusCode};
    use warp::Filter;

    // Start a Warp server for testing
    async fn setup() -> String {
        register_custom_metrics();

        let metrics_route = warp::path!("metrics").and_then(metrics_handler);
        let status_route = warp::path!("status").and_then(status_handler);

        let routes = metrics_route.or(status_route);
        let (addr, server) = warp::serve(routes).bind_ephemeral(([127, 0, 0, 1], 0)); // Bind to a random port

        tokio::spawn(async move {
            server.await;
        }); // Spawn the server in a background task

        format!("http://{}", addr) // Return the address
    }

    #[tokio::test]
    async fn test_status_handler() {
        let base_url = setup().await;
        println!("{}", base_url);
        let client = Client::builder().no_proxy().build().unwrap();

        let response = client
            .get(format!("{}/status", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "ok");
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        let base_url = setup().await;
        let client = Client::builder().no_proxy().build().unwrap();

        let response = client
            .get(format!("{}/metrics", base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        assert!(response
            .text()
            .await
            .unwrap()
            .contains("incoming_requests 0"));

        INCOMING_REQUESTS.inc();
        let response = client
            .get(format!("{}/metrics", base_url))
            .send()
            .await
            .unwrap();

        assert!(response
            .text()
            .await
            .unwrap()
            .contains("incoming_requests 1"));
    }
}
