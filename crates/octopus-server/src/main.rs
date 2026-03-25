#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("listener should bind");

    axum::serve(listener, octopus_server::build_app())
        .await
        .expect("server should run");
}
