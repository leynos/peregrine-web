//! Behavioural contracts for the isolated build-tools installer.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

/// Child executors and pin fixtures for the installer integration tests.
#[path = "support/install_build_tools.rs"]
mod support;

use std::{
    io::{self, Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

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
        output.status.success(),
        "valid pins and a verified archive must install: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-machine,rustup-show,curl,tar,rustup-install",
        "the installer must verify the host, archive, and toolchain in order"
    );
    assert!(
        fixture
            .trace_contains("--connect-timeout 2 --speed-limit 1 --speed-time 2")
            .expect("inspect curl arguments"),
        "curl must receive bounded connection and stalled-transfer settings"
    );
    assert!(
        fixture
            .trace_contains("--strip-components=1")
            .expect("inspect tar arguments"),
        "tar must strip the release archive root"
    );
    assert!(
        fixture
            .trace_contains("--component clippy --component rustfmt")
            .expect("inspect rustup arguments"),
        "rustup must receive the pinned toolchain components"
    );
    assert!(
        stderr.contains("downloading mold-2.41.0-x86_64-linux.tar.gz"),
        "the installer should identify the artifact in its download diagnostic"
    );
    assert!(
        !stderr.contains("fixture-url-secret"),
        "the configurable release URL must not appear in diagnostics"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "successful installation must remove its temporary download directory"
    );
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

    assert!(
        output.status.success(),
        "real curl and tar should install the checksummed loopback archive: {stderr}"
    );
    assert_eq!(
        fixture
            .extracted_payload()
            .expect("read the extracted mold payload"),
        "fixture mold executable\n",
        "real tar must extract the expected file after removing the archive root"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-machine,rustup-show,rustup-install",
        "the loopback route must keep toolchain installation under the controlled rustup fixture"
    );
    assert_eq!(
        fixture
            .scratch_entries()
            .expect("inspect installer scratch cleanup"),
        0,
        "successful real download and extraction must remove temporary archive state"
    );
}

/// Serves one checked installer URL and archive over a loopback connection.
fn serve_archive(listener: &TcpListener, archive: &[u8]) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(connection) => break connection,
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
    };
    stream.set_nonblocking(false)?;
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
            break;
        }
    }
    if !request.starts_with(b"GET /v2.41.0/mold-2.41.0-x86_64-linux.tar.gz HTTP/") {
        return Err(io::Error::other(
            "curl did not request the pinned mold release path",
        ));
    }
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

        assert_eq!(
            output.status.code(),
            Some(1),
            "an invalid checksum record must use the installer's controlled failure status"
        );
        assert!(
            stderr.contains(diagnostic),
            "the checksum failure should explain {diagnostic}: {stderr}"
        );
        assert_eq!(
            fixture
                .command_names()
                .expect("read installer command order"),
            "uname-system,uname-machine,rustup-show,curl",
            "checksum rejection must stop before unpacking and toolchain installation"
        );
        assert_eq!(
            fixture.scratch_entries().expect("inspect scratch cleanup"),
            0,
            "checksum rejection must remove the downloaded archive"
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

    assert_eq!(
        output.status.code(),
        Some(1),
        "download failure must use the installer's controlled failure status"
    );
    assert!(
        stderr.contains("failed to download mold-2.41.0-x86_64-linux.tar.gz"),
        "download failure must retain its controlled artifact diagnostic: {stderr}"
    );
    assert!(
        !stderr.contains("fixture-curl-secret") && !stderr.contains("fixture-url-secret"),
        "downloader output and URL credentials must stay out of diagnostics: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-machine,rustup-show,curl",
        "download failure must stop before unpacking and toolchain installation"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "download failure must remove the temporary directory"
    );
}

/// Stops after a failed archive extraction and removes its temporary download.
#[test]
fn unpack_failure_stops_before_toolchain_install_and_cleans_scratch() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Linux", "x86_64", Some("tar"))
        .expect("run installer with a failing extractor");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(1),
        "archive extraction failure must use the controlled failure status"
    );
    assert!(
        stderr.contains("failed to unpack mold-2.41.0-x86_64-linux.tar.gz"),
        "archive extraction failure must identify the controlled failure: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-machine,rustup-show,curl,tar",
        "unpack failure must stop before installing the Rust toolchain"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "unpack failure must remove the temporary directory"
    );
}

/// Reports rustup failure after archive verification and extraction.
#[test]
fn toolchain_failure_follows_unpack_and_cleans_scratch() {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    let output = fixture
        .run("Linux", "x86_64", Some("rustup"))
        .expect("run installer with failing rustup installation");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(1),
        "toolchain installation failure must use the controlled failure status"
    );
    assert!(
        stderr.contains("failed to install toolchain nightly-2030-01-01"),
        "toolchain failure must identify the pinned toolchain: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-machine,rustup-show,curl,tar,rustup-install",
        "the installer must unpack mold before attempting the toolchain install"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "toolchain failure must remove the temporary download directory"
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
        output.status.success(),
        "a non-native host should retain its platform linker: {stderr}"
    );
    assert_eq!(
        fixture
            .command_names()
            .expect("read installer command order"),
        "uname-system,uname-system-machine,rustup-install",
        "non-native hosts must skip rustup host discovery, download, and extraction"
    );
    assert!(
        stderr.contains("skipping on Darwin arm64"),
        "the installer should explain why mold was skipped: {stderr}"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "the non-native route must not create temporary download state"
    );
}
