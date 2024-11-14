use uuid::Uuid;

pub async fn health_check() -> () {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Health check",
        %request_id
    );
    {
        let _ = request_span.enter();
    }
    {
        let _ = request_span.enter();
    }
}