//! An owned response body cannot capture a borrowed request path.

use bytes::Bytes;
use peregrine_web::spike::RequestView;

/// Builds a body that must outlive the borrowed request view.
fn body_for(view: RequestView<'_>) -> Bytes { Bytes::from_static(view.path().as_bytes()) }

fn main() {}