use crate::traq::TraqClient;
use std::time::Duration;
use tokio::{
    sync::mpsc,
    time::{Instant, sleep_until},
};
use uuid::Uuid;

pub(super) const MAX_PER_JOB: u32 = 10_000;
const MAX_PER_REQUEST: u32 = 100;
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(50);

pub(super) struct Job {
    pub(super) message_id: Uuid,
    pub(super) stamp_id: Uuid,
    pub(super) requester_channel: Uuid,
    pub(super) count: u32,
}

pub(super) async fn run(client: &TraqClient, mut queue: mpsc::Receiver<Job>) {
    let mut next_request = Instant::now();
    while let Some(job) = queue.recv().await {
        let mut remaining = job.count;
        while remaining > 0 {
            let batch = remaining.min(MAX_PER_REQUEST);
            sleep_until(next_request).await;
            let result = client.add_stamp(job.message_id, job.stamp_id, batch).await;
            next_request = Instant::now() + MIN_REQUEST_INTERVAL;

            if let Err(error) = result {
                let confirmed = job.count - remaining;
                tracing::warn!(%error, message_id = %job.message_id, confirmed, "Stamp job stopped");
                let reply = format!(
                    "スタンプの連打を途中で終了しました（確認済み {confirmed} / {} 回）。",
                    job.count
                );
                if let Err(error) = client.reply(job.requester_channel, &reply).await {
                    tracing::warn!(%error, "Could not report stamp job failure");
                }
                break;
            }
            remaining -= batch;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
    use serde::Deserialize;
    use std::sync::{Arc, Mutex};

    #[derive(Deserialize)]
    struct StampRequest {
        count: u32,
    }

    #[tokio::test]
    async fn spaces_out_requests_across_jobs() {
        type Requests = Arc<Mutex<Vec<(Instant, u32)>>>;

        async fn stamp(
            State(requests): State<Requests>,
            Json(request): Json<StampRequest>,
        ) -> StatusCode {
            requests
                .lock()
                .unwrap()
                .push((Instant::now(), request.count));
            StatusCode::NO_CONTENT
        }

        let requests = Requests::default();
        let router = Router::new()
            .route("/api/v3/messages/{message}/stamps/{stamp}", post(stamp))
            .with_state(requests.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let server = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });

        let (tx, rx) = mpsc::channel(2);
        for count in [1, 150] {
            tx.try_send(Job {
                message_id: Uuid::now_v7(),
                stamp_id: Uuid::now_v7(),
                requester_channel: Uuid::now_v7(),
                count,
            })
            .unwrap();
        }
        drop(tx);
        let client = TraqClient::new(origin, "test-token", Uuid::now_v7()).unwrap();
        run(&client, rx).await;

        let records = requests.lock().unwrap();
        assert_eq!(
            records.iter().map(|&(_, count)| count).collect::<Vec<_>>(),
            [1, 100, 50]
        );
        for pair in records.windows(2) {
            assert!(pair[1].0 - pair[0].0 >= MIN_REQUEST_INTERVAL);
        }
        server.abort();
    }
}
