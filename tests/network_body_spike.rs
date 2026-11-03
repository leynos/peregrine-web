//! Loopback evidence that Hyper's real `Incoming` traverses the spike adapter.
//!
//! The fixture in `tests/support/spike/network.rs` binds port zero, accepts
//! exactly one connection, and drives the same adaptation and invocation
//! functions as the synthetic body cases. Raw HTTP/1.1 bytes are written so the
//! fixture controls chunk framing and the half-close that truncates a request
//! body. Deadlines are failure bounds, not timing assertions, and every test
//! joins the fixture-owned task through `Harness::shutdown`, propagating its
//! result so a shutdown timeout or a panicked server cannot pass silently.

#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/network.rs"]
mod spike_network;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use std::{
    error::Error,
    sync::{Arc, atomic::AtomicUsize},
};

use bytes::Bytes;
use http::HeaderMap;
use peregrine_web::spike::{
    BodyError,
    InvocationOutcome,
    Method,
    RequestView,
    ResourceError,
    adapt_body,
};
use spike_components::OwnedResource;
use spike_network::{
    Harness,
    Observation,
    exchange,
    exchange_truncated,
    fixture,
    invoke_body,
    render,
    require,
};
use spike_synthetic::{FrameStep, SyntheticBody};

#[tokio::test]
async fn synthetic_and_network_requests_share_one_invocation_path() -> Result<(), Box<dyn Error>> {
    let (resource, runner, registered) = fixture()?;
    let headers = HeaderMap::new();
    let body = adapt_body(SyntheticBody::new(
        vec![FrameStep::Data(Bytes::from_static(b"hello"))],
        None,
        Arc::new(AtomicUsize::new(0)),
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
    harness.shutdown().await?;
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
    harness.shutdown().await?;
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
    harness.shutdown().await?;
    exchange?;

    let recorded = observation
        .take()
        .ok_or("the fixture server must record one outcome")?;
    let InvocationOutcome::Failed(ResourceError::Body { source }) = recorded else {
        return Err(
            format!("an incomplete body must surface as a body failure, got {recorded:?}").into(),
        );
    };
    match *source {
        BodyError::Upstream { .. } => {}
    }
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
