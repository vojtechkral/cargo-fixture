use std::io;

use cargo_fixture::FixtureClient;
use tokio::task::JoinHandle;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();

    // Spin up a simple HTTP server and share its listening port number
    // with tests via an environment variables.
    let (handle, port) = spawn_server().await.unwrap();
    eprintln!("HTTP server running on port {port}...");
    fixture
        .set_env_var("HTTP_PORT", port.to_string())
        .await
        .unwrap();

    // Tell the fixture we're ready to run tests.
    // This will return when cargo test call is complete.
    let success = fixture.ready().await.unwrap();
    eprintln!("Tests finished, success: {success}");

    // Wrap up.
    eprintln!("Shutting down HTTP server...");
    handle.abort();
    match handle.await {
        Err(err) if !err.is_cancelled() => {
            panic!("Http server error: {}", err)
        }
        _ => {}
    }
}

/// A very simple hyper-based server.
async fn spawn_server() -> io::Result<(JoinHandle<()>, u16)> {
    use std::convert::Infallible;

    use hyper::body;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    async fn service(_: Request<body::Incoming>) -> Result<Response<String>, Infallible> {
        Ok(Response::new("OK".into()))
    }

    let handle = tokio::spawn(async move {
        while let Ok((stream, _addr)) = listener.accept().await {
            let io = TokioIo::new(stream);
            tokio::task::spawn(async move {
                http1::Builder::new()
                    .serve_connection(io, service_fn(service))
                    .await
                    .unwrap();
            });
        }
    });

    Ok((handle, port))
}
