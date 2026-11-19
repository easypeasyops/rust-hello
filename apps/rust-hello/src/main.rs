// The entry point: binds the ports and turns SIGTERM into a graceful shutdown. Left out of
// coverage; scripts/smoke.sh proves it, in the image.
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let metrics_port = std::env::var("METRICS_PORT").unwrap_or_else(|_| "9100".into());
    let app = TcpListener::bind("0.0.0.0:8000").await?;
    let metrics = TcpListener::bind(format!("0.0.0.0:{metrics_port}")).await?;
    // Kubernetes sends SIGTERM, then waits 30 s: finish the requests in hand, then exit.
    let mut term = signal(SignalKind::terminate())?;
    println!("listening on :8000, metrics on :{metrics_port}");
    app::serve(app, metrics, async move {
        term.recv().await;
        println!("SIGTERM: finishing the requests in hand");
    })
    .await?;
    println!("graceful shutdown complete");
    Ok(())
}
