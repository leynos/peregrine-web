//! Behavioural contracts for the isolated build-tools installer.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

#[path = "support/install_build_tool_pins.rs"]
mod pin_contracts;
/// Child executors and pin fixtures for the installer integration tests.
#[path = "support/install_build_tools.rs"]
mod support;

use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

use rstest::rstest;
use support::{ChecksumRecord, InstallerFixture};

/// Installs a verified archive before installing the pinned Rust toolchain.
#[test]
fn native_install_orders_download_verification_unpack_and_toolchain() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Linux", "x86_64", None)
        .expect("run the real installer with controlled tools");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success()
            && stderr.contains("downloading mold-2.41.0-x86_64-linux.tar.gz")
            && !stderr.contains("fixture-url-secret"),
        "valid pins must install with a safe artifact diagnostic: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "rustup-show,uname-system,uname-machine,curl,tar,rustup-install",
        "the installer must verify the host, archive, and toolchain in order"
    );
    let expected_trace = fixture
        .trace_contains("--connect-timeout 2 --speed-limit 1 --speed-time 2")
        .expect("inspect curl arguments")
        && fixture
            .trace_contains("--strip-components=1")
            .expect("inspect tar arguments")
        && fixture
            .trace_contains("--component clippy --component rustfmt")
            .expect("inspect rustup arguments");
    let scratch_is_clean = fixture.scratch_entries().expect("inspect scratch cleanup") == 0;
    assert!(
        expected_trace && scratch_is_clean,
        "install must bound curl, strip the archive root, install pinned components, and clean \
         scratch"
    );
}

#[test]
fn unknown_rustup_host_stops_before_skipping_mold_or_installing_toolchain() {
    for failure in ["rustup-show", "rustup-show-empty"] {
        let fixture = InstallerFixture::new().expect("create installer fixture");
        let output = fixture
            .run("Linux", "x86_64", Some(failure))
            .expect("run installer with unavailable rustup host metadata");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            !output.status.success()
                && stderr.contains("cannot determine the rustup default host; check rustup show"),
            "unavailable rustup host metadata must fail with an actionable diagnostic: \
             mode={failure}; status={:?}; stderr={stderr}",
            output.status.code()
        );
        assert_eq!(
            fixture
                .command_names()
                .expect("read installer command order"),
            "rustup-show",
            "host discovery failure must stop before mold download or toolchain installation"
        );
        assert_eq!(
            fixture.scratch_entries().expect("inspect scratch cleanup"),
            0,
            "host discovery failure must not leave installer scratch directories"
        );
    }
}

/// Exercises the real downloader, checksum verifier, and extractor over loopback.
#[test]
fn real_curl_and_tar_install_archive_from_local_http_server() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    fixture
        .create_real_archive()
        .expect("create the local mold archive");
    fixture
        .set_checksum(ChecksumRecord::Correct)
        .expect("write the archive checksum");
    let archive = fixture
        .archive_bytes()
        .expect("read the local mold archive");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local archive server");
    let address = listener
        .local_addr()
        .expect("read the local archive server address");
    let server = thread::spawn(move || serve_archive(&listener, &archive));

    let base_url = format!("http://{address}");
    let output = fixture
        .run_with_real_download(&base_url)
        .expect("run installer against the local archive server");
    server
        .join()
        .expect("local HTTP fixture thread must complete")
        .expect("local HTTP fixture must serve the archive");
    let stderr = String::from_utf8_lossy(&output.stderr);

    let extracted_payload = fixture
        .extracted_payload()
        .expect("read the extracted mold payload");
    assert!(
        output.status.success() && extracted_payload == "fixture mold executable\n",
        "real curl and tar must install the checksummed payload: {stderr}; \
         payload={extracted_payload:?}"
    );
    let command_names = fixture
        .command_names()
        .expect("read installer command order");
    let scratch_entries = fixture
        .scratch_entries()
        .expect("inspect installer scratch cleanup");
    assert!(
        command_names == "rustup-show,uname-system,uname-machine,rustup-install"
            && scratch_entries == 0,
        "loopback install must use controlled rustup and clean scratch: commands={command_names}; \
         scratch_entries={scratch_entries}"
    );
}

/// Serves one checked installer URL and archive over a loopback connection.
fn serve_archive(listener: &TcpListener, archive: &[u8]) -> io::Result<()> {
    let mut stream = accept_archive_connection(listener)?;
    stream.set_nonblocking(false)?;
    let request = read_archive_request(&mut stream)?;
    validate_archive_request(&request)?;
    write_archive_response(&mut stream, archive)
}

/// Accepts curl's local connection within the server's fixed deadline.
fn accept_archive_connection(listener: &TcpListener) -> io::Result<TcpStream> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return Ok(stream),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "curl did not connect to the local archive server before its deadline",
                ));
            }
            Err(error) => return Err(error),
        }
    }
}

/// Reads one complete HTTP request header from the loopback curl connection.
fn read_archive_request(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let received = stream.read(&mut buffer)?;
        if received == 0 {
            return Err(io::Error::other("curl closed before sending its request"));
        }
        let bytes = buffer.get(..received).ok_or_else(|| {
            io::Error::other("local archive server read beyond its request buffer")
        })?;
        request.extend_from_slice(bytes);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
    }
}

/// Checks curl requested the exact pinned artifact path.
fn validate_archive_request(request: &[u8]) -> io::Result<()> {
    if !request.starts_with(b"GET /v2.41.0/mold-2.41.0-x86_64-linux.tar.gz HTTP/") {
        return Err(io::Error::other(
            "curl did not request the pinned mold release path",
        ));
    }
    Ok(())
}

/// Sends the archive as a bounded HTTP/1.1 response.
fn write_archive_response(stream: &mut TcpStream, archive: &[u8]) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        archive.len()
    )?;
    stream.write_all(archive)?;
    stream.flush()
}

/// Rejects missing, ambiguous, and incorrect archive checksums before install.
#[test]
fn invalid_checksum_records_fail_before_unpacking_or_toolchain_install() {
    for (record, diagnostic) in [
        (ChecksumRecord::Missing, "no checksum recorded"),
        (ChecksumRecord::Duplicate, "2 checksums recorded"),
        (ChecksumRecord::Wrong, "checksum mismatch"),
    ] {
        let fixture = InstallerFixture::new().expect("create installer fixture");
        fixture
            .set_checksum(record)
            .expect("write the selected checksum fixture");
        let output = fixture
            .run("Linux", "x86_64", None)
            .expect("run installer with an invalid checksum fixture");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.code() == Some(1) && stderr.contains(diagnostic),
            "invalid checksum must fail with {diagnostic}: status={:?}; stderr={stderr}",
            output.status.code()
        );
        let command_names = fixture
            .command_names()
            .expect("read installer command order");
        let scratch_entries = fixture.scratch_entries().expect("inspect scratch cleanup");
        assert!(
            command_names == "rustup-show,uname-system,uname-machine,curl" && scratch_entries == 0,
            "checksum rejection must stop before unpacking and clean the archive: \
             commands={command_names}; scratch_entries={scratch_entries}"
        );
    }
}

/// Hides downloader details while preserving the controlled failure report.
#[test]
fn download_failure_hides_downloader_details_and_cleans_scratch() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Linux", "x86_64", Some("curl"))
        .expect("run installer with a failing downloader");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.code() == Some(1)
            && stderr.contains("failed to download mold-2.41.0-x86_64-linux.tar.gz")
            && !stderr.contains("fixture-curl-secret")
            && !stderr.contains("fixture-url-secret"),
        "download failure must be controlled and redact downloader details: status={:?}; \
         stderr={stderr}",
        output.status.code()
    );
    let command_names = fixture
        .command_names()
        .expect("read installer command order");
    let scratch_entries = fixture.scratch_entries().expect("inspect scratch cleanup");
    assert!(
        command_names == "rustup-show,uname-system,uname-machine,curl" && scratch_entries == 0,
        "download failure must stop before unpacking and clean scratch: commands={command_names}; \
         scratch_entries={scratch_entries}"
    );
}

/// Stops at the failed installer stage and removes its temporary download.
#[rstest]
#[case::unpack(
    "tar",
    "failed to unpack mold-2.41.0-x86_64-linux.tar.gz",
    "rustup-show,uname-system,uname-machine,curl,tar",
    "archive extraction failure"
)]
#[case::toolchain(
    "rustup",
    "failed to install toolchain nightly-2030-01-01",
    "rustup-show,uname-system,uname-machine,curl,tar,rustup-install",
    "toolchain installation failure"
)]
fn installer_failure_stops_at_the_failed_stage(
    #[case] failing_command: &str,
    #[case] diagnostic: &str,
    #[case] expected_commands: &str,
    #[case] failure_description: &str,
) {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Linux", "x86_64", Some(failing_command))
        .expect("run installer with a failing stage");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.code() == Some(1) && stderr.contains(diagnostic),
        "{failure_description} must stop with its controlled diagnostic: status={:?}; \
         stderr={stderr}",
        output.status.code()
    );
    let command_names = fixture
        .command_names()
        .expect("read installer command order");
    let scratch_entries = fixture.scratch_entries().expect("inspect scratch cleanup");
    assert!(
        command_names == expected_commands && scratch_entries == 0,
        "{failure_description} must stop and clean scratch: commands={command_names}; \
         scratch_entries={scratch_entries}"
    );
}

/// Skips mold on a non-native host while still installing the toolchain.
#[test]
fn non_native_host_skips_mold_but_installs_the_pinned_toolchain() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Darwin", "arm64", None)
        .expect("run installer with a non-native host report");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success() && stderr.contains("skipping on Darwin arm64"),
        "a non-native host must skip mold and report why: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "rustup-show,uname-system,uname-system-machine,rustup-install",
        "non-native hosts must skip the mold download and extraction"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "the non-native route must not create temporary download state"
    );
}
