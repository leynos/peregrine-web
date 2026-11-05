//! A bounded loopback Hyper fixture shared by the network witnesses.
//!
//! The fixture binds port zero, accepts exactly one connection, and drives the
//! same adaptation and invocation functions as the synthetic body cases. Both
//! directions are bounded before they accumulate, and the owned server task is
//! joined through [`Harness::shutdown`], whose result the tests propagate, so a
//! shutdown timeout or a panicked server cannot hide behind a passing test.
//!
//! This module has one consumer, `network_body_spike`, which uses every item,
//! so it needs no `dead_code` allowance. The other fixture modules are included
//! by several binaries and carry one; see `docs/developers-guide.md`.

use std::{
    convert::Infallible,
    error::Error,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
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
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

use crate::{
    spike_components::{CatalogueResource, DependencyState},
    spike_frames::MAX_BODY_BYTES,
};

/// Deadline for one loopback exchange or shutdown; a bound, not a timing claim.
pub const EXCHANGE_DEADLINE: Duration = Duration::from_secs(15);

/// Reported when the fixture server was still running at its deadline.
const STOPPED_AT_DEADLINE: &str = "the fixture server overran its deadline and was stopped";

/// The outcome of the most recent request handled by the fixture server.
#[derive(Default)]
pub struct Observation {
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
    pub fn take(&self) -> Option<InvocationOutcome> {
        match self.outcome.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }
}

/// A loopback Hyper server scoped to one test.
pub struct Harness {
    /// The port the fixture server bound.
    pub port: u16,
    /// The server task, aborted when the harness is dropped early.
    task: Option<JoinHandle<()>>,
}

impl Harness {
    /// Binds port zero and spawns the one-connection fixture server.
    pub async fn start(
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

    /// Joins the server task within the deadline, reaping it if it overruns.
    ///
    /// A completed task's panic is reported rather than discarded, and an
    /// aborted task is awaited so the fixture leaves nothing detached behind.
    pub async fn shutdown(mut self) -> Result<(), Box<dyn Error>> {
        let Some(mut task) = self.task.take() else {
            return Ok(());
        };
        match tokio::time::timeout(EXCHANGE_DEADLINE, &mut task).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(failure)) => Err(format!("the fixture server task failed: {failure}").into()),
            Err(_) => {
                task.abort();
                Err(stopped_outcome(task.await).into())
            }
        }
    }
}

/// Describes what stopping the fixture server at its deadline left behind.
///
/// Aborting a task that finishes anyway is still a deadline overrun, so the
/// cancelled and completed arms report the same reason. Only an abort that
/// surfaces a different panic adds detail.
fn stopped_outcome(joined: Result<(), tokio::task::JoinError>) -> String {
    match joined {
        Err(failure) if failure.is_cancelled() => STOPPED_AT_DEADLINE.to_owned(),
        Err(failure) => format!("the aborted fixture server task failed: {failure}"),
        Ok(()) => STOPPED_AT_DEADLINE.to_owned(),
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        // Abort is a fallback for an early return, not evidence of cleanup: the
        // tests call `shutdown` and propagate its result.
        if let Some(task) = self.task.take() {
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
pub async fn invoke_body(
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

/// Reads a response, failing as soon as it exceeds the fixture bound.
async fn read_bounded(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut response = Vec::new();
    let mut chunk = [0_u8; 512];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(response);
        }
        if response.len() + read > MAX_BODY_BYTES {
            return Err(format!(
                "the response exceeded the fixture bound of {MAX_BODY_BYTES} bytes"
            )
            .into());
        }
        let received = chunk
            .get(..read)
            .ok_or("a short read must lie inside the fixture buffer")?;
        response.extend_from_slice(received);
    }
}

/// Runs one fixture interaction under the exchange deadline.
///
/// The deadline is a failure bound, not a timing claim, so an elapsed deadline
/// becomes an error naming the interaction rather than a hang.
async fn within_deadline<T, F>(what: &str, interaction: F) -> Result<T, Box<dyn Error>>
where
    F: Future<Output = Result<T, Box<dyn Error>>>,
{
    tokio::time::timeout(EXCHANGE_DEADLINE, interaction)
        .await
        .unwrap_or_else(|_| Err(format!("the {what} exceeded its deadline").into()))
}

/// Writes a complete request and reads the bounded response.
pub async fn exchange(port: u16, request: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let interaction = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(request).await?;
        stream.flush().await?;
        read_bounded(&mut stream).await
    };
    within_deadline("loopback exchange", interaction).await
}

/// Writes a request head and a truncated body, then half-closes the write side.
pub async fn exchange_truncated(
    port: u16,
    head: &[u8],
    partial: &[u8],
) -> Result<(), Box<dyn Error>> {
    let interaction = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(head).await?;
        stream.write_all(partial).await?;
        stream.flush().await?;
        // The write-half close is what tells the server the body ended early.
        stream.shutdown().await?;
        let _closed = read_bounded(&mut stream).await?;
        Ok::<(), Box<dyn Error>>(())
    };
    within_deadline("truncated exchange", interaction).await
}

/// The resource, runner, and registration one network fixture serves.
pub type FixtureParts = (
    Arc<CatalogueResource>,
    Arc<ResourceRunner>,
    Arc<RegisteredResource>,
);

/// Builds a public catalogue resource and the registration the fixture serves.
pub fn fixture() -> Result<FixtureParts, Box<dyn Error>> {
    let dependency = DependencyState::new("catalogue", "catalogue-record");
    let resource = CatalogueResource::new("catalogue", Policy::Public, dependency);
    let runner = Arc::new(ResourceRunner::new());
    // Bind the clone before the call so the unsizing coercion happens on the
    // value rather than being pushed back into `Arc::clone`'s type parameter.
    let retained: Arc<CatalogueResource> = Arc::clone(&resource);
    let registered = Arc::new(runner.register(retained, Method::Get)?);
    Ok((resource, runner, registered))
}

/// Returns the rendered response, replacing invalid UTF-8 lossily.
///
/// The conversion substitutes U+FFFD for invalid byte sequences rather than
/// rejecting them, so a witness that asserts on the rendered text also proves
/// the fixture produced valid UTF-8 where it matters.
pub fn render(response: &[u8]) -> String { String::from_utf8_lossy(response).into_owned() }

/// Turns a failed expectation into the error a `Result`-returning test returns.
///
/// The network tests propagate setup and exchange errors, so a failed
/// expectation is reported the same way rather than by unwinding.
pub fn require(condition: bool, describe: impl FnOnce() -> String) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(describe().into())
    }
}
