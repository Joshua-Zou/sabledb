use crate::server::{ServerOptions, Telemetry};
use crate::SableError;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, RwLock as StdRwLock};

pub struct MetricsServer;

impl MetricsServer {
    /// Spawn a dedicated OS thread that serves Prometheus metrics over HTTP.
    /// Returns immediately after spawning the thread.
    pub fn run(
        address: String,
        telemetry: Arc<StdRwLock<Telemetry>>,
    ) -> Result<(), SableError> {
        let listener = TcpListener::bind(&address)?;
        tracing::info!("Prometheus metrics endpoint listening on {}", address);

        std::thread::Builder::new()
            .name("metrics-server".into())
            .spawn(move || {
                Self::serve(listener, telemetry);
            })?;
        Ok(())
    }

    fn serve(listener: TcpListener, telemetry: Arc<StdRwLock<Telemetry>>) {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    // Read the request (we don't need to parse it, just drain it)
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);

                    let body = match telemetry.read() {
                        Ok(t) => t.to_prometheus(),
                        Err(_) => "# error reading telemetry\n".to_string(),
                    };

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body,
                    );

                    if let Err(e) = stream.write_all(response.as_bytes()) {
                        tracing::debug!("metrics endpoint write error: {:?}", e);
                    }
                }
                Err(e) => {
                    crate::error_with_throttling!(300, "metrics endpoint accept error: {:?}", e);
                }
            }
        }
    }
}
