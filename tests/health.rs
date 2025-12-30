use axum_ecommerce_api::routes::health::health_check;

#[tokio::test]
async fn health_check_returns_ok() {
    let response = health_check().await;
    assert_eq!(response.0.message, "Health check");

    let data = response.0.data.expect("health data");
    assert_eq!(data.status, "ok");
}
