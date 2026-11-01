//! Loopback evidence that Hyper's real `Incoming` traverses the spike adapter.
//!
//! The fixture binds port zero, accepts exactly one connection, and drives the
//! same adaptation and invocation functions as the synthetic body cases. Raw
//! HTTP/1.1 bytes are written so the fixture controls chunk framing and the
//! half-close that truncates a request body. Deadlines are failure bounds, not
//! timing assertions, and every owned task is joined or aborted before the test
//! finishes.

#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use std::{
    convert::Infallible,
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use peregrine_web::spike::{
    ErasedBody,
    InvocationOutcome,
    Method,
    Policy,
    RegisteredResource,
    RequestView,
    ResourceRunner,
    adapt_body,
};
use spike_components::{CatalogueResource, DependencyState, OwnedResource};
use spike_synthetic::{FrameStep, SyntheticBody};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// Deadline for one loopback exchange or shutdown; a bound, not a timing claim.
const EXCHANGE_DEADLINE: Duration = Duration::from_secs(15);

/// Maximum request or response payload this fixture handles.
const MAX_PAYLOAD: usize = 4096;

/// The outcome of the most recent request handled by the fixture server.
#[derive(Default)]
struct Observation {
    /// The recorded outcome, consumed by the test once the exchange completes.
    outcome: Mutex<Option<InvocationOutcome>>,
}

impl Observation {
    /// Records the outcome of one handled request.
    fn record(&self, outcome: InvocationOutcome) {
        match self.outcome.lock() {
            Ok(mut guard) => *guard = Some(outcome),
            Err(poisoned) => *poisoned.into_inner() = Some(outcome),
        }
    }

    /// Takes the recorded outcome, leaving the slot empty.
    fn take(&self) -> Option<InvocationOutcome> {
        match self.outcome.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }
}

/// A loopback Hyper server scoped to one test.
struct Harness {
    /// The port the fixture server bound.
    port: u16,
    /// The server task, aborted when the harness is dropped early.
    task: Option<JoinHandle<()>>,
}

impl Harness {
    /// Binds port zero and spawns the one-connection fixture server.
    async fn start(
        runner: Arc<ResourceRunner>,
        registered: Arc<RegisteredResource>,
        observation: Arc<Observation>,
    ) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let task = tokio::spawn(serve_one_connection(
            listener,
            runner,
            registered,
            observation,
        ));
        Ok(Self {
            port,
            task: Some(task),
        })
    }

    /// Joins the server task within the deadline, aborting it if it overruns.
    async fn shutdown(mut self) {
        if let Some(mut task) = self.task.take()
            && tokio::time::timeout(EXCHANGE_DEADLINE, &mut task)
                .await
                .is_err()
        {
            task.abort();
        }
    }
}

/// Accepts one connection and serves requests on it until the peer closes.
async fn serve_one_connection(
    listener: TcpListener,
    runner: Arc<ResourceRunner>,
    registered: Arc<RegisteredResource>,
    observation: Arc<Observation>,
) {
    let Ok((stream, _peer)) = listener.accept().await else {
        return;
    };
    let service = service_fn(move |request: Request<Incoming>| {
        serve_request(
            Arc::clone(&runner),
            Arc::clone(&registered),
            Arc::clone(&observation),
            request,
        )
    });
    let io = TokioIo::new(stream);
    let _served = http1::Builder::new().serve_connection(io, service).await;
}

/// Handles one request and records the outcome the test inspects.
async fn serve_request(
    runner: Arc<ResourceRunner>,
    registered: Arc<RegisteredResource>,
    observation: Arc<Observation>,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let outcome = handle(&runner, &registered, request).await;
    let response = response_for(&outcome);
    observation.record(outcome);
    Ok(response)
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// Adapts a network body once and invokes the registered resource.
async fn handle(
    runner: &ResourceRunner,
    registered: &RegisteredResource,
    request: Request<Incoming>,
) -> InvocationOutcome {
    let (parts, body) = request.into_parts();
    let headers = parts.headers;
    // The fixture only issues GET; `unwrap_or` keeps an unexpected extension
    // method from masking a body failure behind a dispatch error.
    let frozen = Method::from_http(&parts.method).unwrap_or(Method::Get);
    let view = RequestView::new(frozen, parts.uri.path(), &headers, None);
    invoke_body(runner, registered, view, adapt_body(body)).await
}

/// Invokes the registered resource over an already-adapted body.
///
/// Both the synthetic and the network cases call this function, so a passing
/// network test also witnesses the same adaptation path.
async fn invoke_body(
    runner: &ResourceRunner,
    registered: &RegisteredResource,
    view: RequestView<'_>,
    owned_body: ErasedBody,
) -> InvocationOutcome {
    let mut body = owned_body;
    runner.invoke(registered, view, &mut body).await
}

/// Builds the fixture server's response for one invocation outcome.
fn response_for(outcome: &InvocationOutcome) -> Response<Full<Bytes>> {
    let responded = outcome.responded();
    let body = responded.map_or_else(Bytes::new, |draft| draft.body().clone());
    let mut response = Response::new(Full::new(body));
    *response.status_mut() = if responded.is_some() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    response
}

/// Writes a complete request and reads the bounded response.
async fn exchange(port: u16, request: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let exchange = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(request).await?;
        stream.flush().await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<Vec<u8>, std::io::Error>(response)
    };
    match tokio::time::timeout(EXCHANGE_DEADLINE, exchange).await {
        Ok(Ok(response)) if response.len() <= MAX_PAYLOAD => Ok(response),
        Ok(Ok(response)) => Err(format!(
            "the response exceeded the fixture bound: {} bytes",
            response.len()
        )
        .into()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("the loopback exchange exceeded its deadline".into()),
    }
}

/// Writes a request head and a truncated body, then half-closes the write side.
async fn exchange_truncated(port: u16, head: &[u8], partial: &[u8]) -> Result<(), Box<dyn Error>> {
    let exchange = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(head).await?;
        stream.write_all(partial).await?;
        stream.flush().await?;
        // The write-half close is what tells the server the body ended early.
        stream.shutdown().await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<(), std::io::Error>(())
    };
    match tokio::time::timeout(EXCHANGE_DEADLINE, exchange).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("the truncated exchange exceeded its deadline".into()),
    }
}

/// The resource, runner, and registration one network fixture serves.
type FixtureParts = (
    Arc<CatalogueResource>,
    Arc<ResourceRunner>,
    Arc<RegisteredResource>,
);

/// Builds a public catalogue resource and the registration the fixture serves.
fn fixture() -> Result<FixtureParts, Box<dyn Error>> {
    let dependency = DependencyState::new("catalogue", "catalogue-record");
    let resource = CatalogueResource::new("catalogue", Policy::Public, dependency);
    let runner = Arc::new(ResourceRunner::new());
    // Bind the clone before the call so the unsizing coercion happens on the
    // value rather than being pushed back into `Arc::clone`'s type parameter.
    let retained: Arc<CatalogueResource> = Arc::clone(&resource);
    let registered = Arc::new(runner.register(retained, Method::Get)?);
    Ok((resource, runner, registered))
}

/// Returns the rendered response, rejecting non-UTF-8 fixture output.
fn render(response: &[u8]) -> String { String::from_utf8_lossy(response).into_owned() }

/// Turns a failed expectation into the error a `Result`-returning test returns.
///
/// The network tests propagate setup and exchange errors, so a failed
/// expectation is reported the same way rather than by unwinding.
fn require(condition: bool, describe: impl FnOnce() -> String) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(describe().into())
    }
}

#[tokio::test]
async fn synthetic_and_network_requests_share_one_invocation_path() -> Result<(), Box<dyn Error>> {
    let (resource, runner, registered) = fixture()?;
    let headers = HeaderMap::new();
    let body = adapt_body(SyntheticBody::new(
        vec![FrameStep::Data(Bytes::from_static(b"hello"))],
        None,
        Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    ));
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);

    let synthetic = invoke_body(&runner, &registered, view, body).await;

    let synthetic_body = synthetic
        .responded()
        .map(|draft| draft.body().clone())
        .ok_or("the synthetic invocation must respond")?;
    require(
        synthetic_body == Bytes::from_static(b"catalogue-record /catalogue 5"),
        || format!("the synthetic body must reach the injected dependency, got {synthetic_body:?}"),
    )?;

    let observation = Arc::new(Observation::default());
    let harness = Harness::start(
        Arc::clone(&runner),
        Arc::clone(&registered),
        Arc::clone(&observation),
    )
    .await?;
    let request = concat!(
        "GET /catalogue HTTP/1.1\r\n",
        "Host: localhost\r\n",
        "Content-Length: 5\r\n",
        "Connection: close\r\n",
        "\r\n",
        "hello",
    );
    let raw_response = exchange(harness.port, request.as_bytes()).await;
    harness.shutdown().await;
    let response = render(&raw_response?);

    require(response.starts_with("HTTP/1.1 200 OK"), || {
        format!("the fixture server must answer with a 200 status line, got: {response}")
    })?;
    require(response.contains("catalogue-record /catalogue 5"), || {
        format!("the network body must reach the same dependency, got: {response}")
    })?;
    let recorded = observation
        .take()
        .ok_or("the fixture server must record one outcome")?;
    require(recorded.responded().is_some(), || {
        format!("the network invocation must respond, got {recorded:?}")
    })?;
    require(resource.state().responder_calls() == 2, || {
        format!(
            "one responder call per invocation path, got {}",
            resource.state().responder_calls()
        )
    })?;
    require(resource.dependency().calls() == 2, || {
        format!(
            "one dependency consultation per invocation path, got {}",
            resource.dependency().calls()
        )
    })?;
    Ok(())
}

#[tokio::test]
async fn chunked_request_trailers_reach_the_resource() -> Result<(), Box<dyn Error>> {
    let (resource, runner, registered) = fixture()?;
    let observation = Arc::new(Observation::default());
    let harness = Harness::start(runner, registered, Arc::clone(&observation)).await?;
    let request = concat!(
        "GET /catalogue HTTP/1.1\r\n",
        "Host: localhost\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Connection: close\r\n",
        "\r\n",
        "5\r\nhello\r\n",
        "0\r\n",
        "x-checksum: abc123\r\n",
        "\r\n",
    );

    let raw_response = exchange(harness.port, request.as_bytes()).await;
    harness.shutdown().await;
    let response = render(&raw_response?);

    require(response.contains("catalogue-record /catalogue 5"), || {
        format!("the chunked body must reach the dependency, got: {response}")
    })?;
    let trailers = resource.state().trailers();
    require(
        trailers == vec![("x-checksum".to_owned(), b"abc123".to_vec())],
        || format!("the request trailers must survive adaptation unchanged, got {trailers:?}"),
    )?;
    require(resource.state().responder_calls() == 1, || {
        format!(
            "the responder must run once for one request, got {}",
            resource.state().responder_calls()
        )
    })?;
    Ok(())
}

#[tokio::test]
async fn truncated_content_length_surfaces_the_body_error() -> Result<(), Box<dyn Error>> {
    let (resource, runner, registered) = fixture()?;
    let observation = Arc::new(Observation::default());
    let harness = Harness::start(runner, registered, Arc::clone(&observation)).await?;
    let head = concat!(
        "GET /catalogue HTTP/1.1\r\n",
        "Host: localhost\r\n",
        "Content-Length: 10\r\n",
        "Connection: close\r\n",
        "\r\n",
    );

    let exchange = exchange_truncated(harness.port, head.as_bytes(), b"hello").await;
    harness.shutdown().await;
    exchange?;

    let recorded = observation
        .take()
        .ok_or("the fixture server must record one outcome")?;
    require(matches!(recorded, InvocationOutcome::Failed(_)), || {
        format!("an incomplete body must surface as a failed invocation, got {recorded:?}")
    })?;
    require(resource.state().responder_calls() == 1, || {
        format!(
            "the responder must start before the body error surfaces, got {}",
            resource.state().responder_calls()
        )
    })?;
    require(resource.dependency().calls() == 0, || {
        format!(
            "a failed body drain must not consult the dependency, got {}",
            resource.dependency().calls()
        )
    })?;
    Ok(())
}
