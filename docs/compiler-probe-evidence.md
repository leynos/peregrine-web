# Compiler probe evidence

These eight complete, isolated Rust programs preserve the evidence for the
[ownership experiment](polonius-ownership-experiment.md). They are compiler
fixtures, not Peregrine APIs or a completed framework prototype. The cache
programs exercise failure, retry, and hit behaviour; the phase-view fixture is
compile-only and does not establish executor or `Send` compatibility.

Copy each Rust block into the filename given by its heading. For each file,
compile with all four combinations below; run the successful `cache_old`,
`cache_polonius`, `map_old`, and `map_polonius` binaries to check their
assertions. The final two fixtures deliberately fail. Expected results follow
the sources.

```sh
rustc +nightly-2026-08-27 --edition=2024 -Dwarnings \
  -Zpolonius=off -Znext-solver=no cache_old.rs -o cache_old
./cache_old
```

Replace the borrow-checker flag with `off` or `next` and the solver flag with
`no` or `globally`, always selecting both explicitly. Diagnostic wording is
unstable; compare acceptance and the error categories recorded below. The
fixtures have been formatted for readability without changing their behaviour.

## 1. `cache_old.rs`

```rust
//! Fallible lazy request cache with a branch-before-borrow baseline.
#[derive(Debug, PartialEq)]
pub struct DecodeError;
pub struct RequestCache {
    decoded: Option<String>,
}
impl RequestCache {
    pub fn decoded(
        &mut self,
        decode: impl FnOnce() -> Result<String, DecodeError>,
    ) -> Result<&str, DecodeError> {
        if self.decoded.is_none() {
            self.decoded = Some(decode()?);
        }
        self.decoded.as_deref().ok_or(DecodeError)
    }
}
fn main() {
    let mut cache = RequestCache { decoded: None };
    assert_eq!(
        cache.decoded(|| Err(DecodeError)),
        Err(DecodeError),
        "decode errors must propagate"
    );
    assert_eq!(
        cache.decoded(|| Ok("value".into())),
        Ok("value"),
        "successful retry must populate cache"
    );
    assert_eq!(
        cache.decoded(|| Err(DecodeError)),
        Ok("value"),
        "cache hit must borrow existing value"
    );
}
```

## 2. `cache_polonius.rs`

```rust
//! Fallible lazy request cache using an early returned borrow.
#[derive(Debug, PartialEq)]
pub struct DecodeError;
pub struct RequestCache {
    decoded: Option<String>,
}
impl RequestCache {
    pub fn decoded(
        &mut self,
        decode: impl FnOnce() -> Result<String, DecodeError>,
    ) -> Result<&str, DecodeError> {
        if let Some(value) = self.decoded.as_deref() {
            return Ok(value);
        }
        Ok(self.decoded.insert(decode()?).as_str())
    }
}
fn main() {
    let mut cache = RequestCache { decoded: None };
    assert_eq!(
        cache.decoded(|| Err(DecodeError)),
        Err(DecodeError),
        "decode errors must propagate"
    );
    assert_eq!(
        cache.decoded(|| Ok("value".into())),
        Ok("value"),
        "successful retry must populate cache"
    );
    assert_eq!(
        cache.decoded(|| Err(DecodeError)),
        Ok("value"),
        "cache hit must borrow existing value"
    );
}
```

## 3. `map_old.rs`

```rust
//! Entry-based fallible lazy lookup; the lookup key is owned on hits and misses.
use std::collections::{HashMap, hash_map::Entry};
#[derive(Debug, PartialEq)]
pub struct DecodeError;
pub fn decoded<'a>(
    cache: &'a mut HashMap<String, String>,
    key: &str,
    decode: impl FnOnce() -> Result<String, DecodeError>,
) -> Result<&'a mut String, DecodeError> {
    match cache.entry(key.to_owned()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => Ok(entry.insert(decode()?)),
    }
}
fn main() {
    let mut cache = HashMap::new();
    let mut calls = 0;
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Err(DecodeError)
        }),
        Err(DecodeError),
        "decode error must propagate"
    );
    assert!(cache.is_empty(), "failed decode must leave cache empty");
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Ok("value".into())
        })
        .map(|v| v.as_str()),
        Ok("value"),
        "retry must initialize cache"
    );
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Err(DecodeError)
        })
        .map(|v| v.as_str()),
        Ok("value"),
        "hit must return existing borrow"
    );
    assert_eq!(calls, 2, "decoder must only run on misses");
}
```

## 4. `map_polonius.rs`

```rust
//! Borrow the existing value before allocating a miss-path key.
use std::collections::{HashMap, hash_map::Entry};
#[derive(Debug, PartialEq)]
pub struct DecodeError;
pub fn decoded<'a>(
    cache: &'a mut HashMap<String, String>,
    key: &str,
    decode: impl FnOnce() -> Result<String, DecodeError>,
) -> Result<&'a mut String, DecodeError> {
    if let Some(value) = cache.get_mut(key) {
        return Ok(value);
    }
    match cache.entry(key.to_owned()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => Ok(entry.insert(decode()?)),
    }
}
fn main() {
    let mut cache = HashMap::new();
    let mut calls = 0;
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Err(DecodeError)
        }),
        Err(DecodeError),
        "decode error must propagate"
    );
    assert!(cache.is_empty(), "failed decode must leave cache empty");
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Ok("value".into())
        })
        .map(|v| v.as_str()),
        Ok("value"),
        "retry must initialize cache"
    );
    assert_eq!(
        decoded(&mut cache, "body", || {
            calls += 1;
            Err(DecodeError)
        })
        .map(|v| v.as_str()),
        Ok("value"),
        "hit must return existing borrow"
    );
    assert_eq!(calls, 2, "decoder must only run on misses");
}
```

## 5. `map_old_no_hit_allocation.rs`

```rust
//! Avoid hit-path key allocation with a second hit-path lookup.
use std::collections::{HashMap, hash_map::Entry};
#[derive(Debug, PartialEq)]
pub struct DecodeError;
pub fn decoded<'a>(
    cache: &'a mut HashMap<String, String>,
    key: &str,
    decode: impl FnOnce() -> Result<String, DecodeError>,
) -> Result<Option<&'a mut String>, DecodeError> {
    if cache.contains_key(key) {
        return Ok(cache.get_mut(key));
    }
    match cache.entry(key.to_owned()) {
        Entry::Occupied(entry) => Ok(Some(entry.into_mut())),
        Entry::Vacant(entry) => Ok(Some(entry.insert(decode()?))),
    }
}
fn main() {}
```

## 6. `phase_split.rs`

```rust
//! A resource responder borrows disjoint request fields across await.
struct Request {
    tenant: String,
    body: Option<String>,
    response: String,
}
struct ResponderView<'a> {
    tenant: &'a str,
    body: &'a mut Option<String>,
    response: &'a mut String,
}
struct Resource;
impl Resource {
    async fn on_post(&self, view: ResponderView<'_>) {
        let body = async { view.body.take().unwrap_or_default() }.await;
        view.response.push_str(view.tenant);
        view.response.push_str(&body);
    }
}
async fn dispatch(resource: &Resource, request: &mut Request) {
    resource
        .on_post(ResponderView {
            tenant: &request.tenant,
            body: &mut request.body,
            response: &mut request.response,
        })
        .await;
}
fn main() {
    let mut request = Request {
        tenant: "tenant".into(),
        body: None,
        response: String::new(),
    };
    let resource = Resource;
    let _future = dispatch(&resource, &mut request);
}
```

## 7. `async_dyn.rs`

```rust
//! Native async methods are not dyn compatible.
trait Resource {
    async fn on_get(&self);
}
fn dispatch(_: &dyn Resource) {}
fn main() {}
```

## 8. `alias.rs`

```rust
//! Simultaneous mutable borrows remain forbidden.
fn main() {
    let mut response = String::new();
    let first = &mut response;
    let second = &mut response;
    first.push('a');
    second.push('b');
}
```

## 9. Recorded results

Observed 2026-09-20; compiler identity and all 32 outcomes are preserved below.
These results establish the isolated claims only, not performance or framework
correctness.

```text
rustc 1.100.0-nightly (bff8e12ff 2026-08-26)
binary: rustc
commit-hash: bff8e12ff5e6bcd53dfb1dbccdcec80a60a856ed
commit-date: 2026-08-26
host: x86_64-unknown-linux-gnu
release: 1.100.0-nightly
LLVM version: 23.1.0

cache_old | off | no | PASS; runtime assertions PASS
cache_old | off | globally | PASS; runtime assertions PASS
cache_old | next | no | PASS; runtime assertions PASS
cache_old | next | globally | PASS; runtime assertions PASS
cache_polonius | off | no | FAIL E0502
cache_polonius | off | globally | FAIL E0502
cache_polonius | next | no | PASS; runtime assertions PASS
cache_polonius | next | globally | PASS; runtime assertions PASS
map_old | off | no | PASS; runtime assertions PASS
map_old | off | globally | PASS; runtime assertions PASS
map_old | next | no | PASS; runtime assertions PASS
map_old | next | globally | PASS; runtime assertions PASS
map_polonius | off | no | FAIL E0499
map_polonius | off | globally | FAIL E0499
map_polonius | next | no | PASS; runtime assertions PASS
map_polonius | next | globally | PASS; runtime assertions PASS
map_old_no_hit_allocation | off | no | PASS
map_old_no_hit_allocation | off | globally | PASS
map_old_no_hit_allocation | next | no | PASS
map_old_no_hit_allocation | next | globally | PASS
phase_split | off | no | PASS
phase_split | off | globally | PASS
phase_split | next | no | PASS
phase_split | next | globally | PASS
async_dyn | off | no | FAIL E0038
async_dyn | off | globally | FAIL E0038
async_dyn | next | no | FAIL E0038
async_dyn | next | globally | FAIL E0038
alias | off | no | FAIL E0499
alias | off | globally | FAIL E0499
alias | next | no | FAIL E0499
alias | next | globally | FAIL E0499
```
