//! Fake pinned build tools used by the preflight integration tests.

use std::{
    error::Error,
    fmt::Write as _,
    io,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Permissions, PermissionsExt},
    fs_utf8::Dir,
};

type Read<T> = Result<T, Box<dyn Error>>;
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
const TOOLCHAIN_PIN: &str = concat!(
    "[toolchain]\n",
    "channel = \"nightly-2026-08-27\"\n",
    "components = [\n",
    "  \"clippy\",\n",
    "  \"llvm-tools-preview\",\n",
    "  \"rustc-codegen-cranelift-preview\",\n",
    "  \"rust-analyzer\",\n",
    "  \"rustfmt\",\n",
    "]\n",
);

/// Owns fake build tools in a private scratch directory.
pub(super) struct ToolFixture {
    parent: Dir,
    pub(super) directory: Dir,
    name: String,
    pub(super) path: Utf8PathBuf,
}

impl ToolFixture {
    pub(super) fn new() -> Read<Self> {
        let parent_path = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?;
        let parent = Dir::open_ambient_dir(&parent_path, ambient_authority())?;
        let name = format!(
            "peregrine-build-tools-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        let directory = parent.open_dir(&name)?;
        directory.write("trace", "")?;
        directory.write("version", "2.41.0\n")?;

        directory.write("toolchain.toml", TOOLCHAIN_PIN)?;
        directory.write(
            "rustup",
            "#!/bin/sh\nprintf 'rustup:%s\\n' \"$*\" >> \"$PREFLIGHT_TRACE\"\ncase \"$1 $2\" in\n  'toolchain list') cat \"$FAKE_TOOLCHAINS\" ;;\n  \
             'component list') cat \"$FAKE_COMPONENTS\" ;;\n  'show ') printf '%s\\n' 'Default \
             host: x86_64-unknown-linux-gnu' ;;\n  *) exit 1 ;;\nesac\n",
        )?;
        directory.write(
            "toolchains",
            "nightly-2026-08-27-x86_64-unknown-linux-gnu (default)\n",
        )?;
        directory.create_dir("bin")?;
        directory.write(
            "bin/mold",
            concat!(
                "#!/bin/sh\n",
                "printf 'mold:%s\\n' \"$*\" >> \"$PREFLIGHT_TRACE\"\n",
                "printf 'mold %s (compatible with GNU ld)\\n' ",
                "\"$FAKE_MOLD_VERSION\"\n",
            ),
        )?;
        directory.symlink("mold", "bin/ld.mold")?;
        directory.write(
            "clang",
            concat!(
                "#!/bin/sh\n",
                "printf 'clang:%s\\n' \"$*\" >> \"$PREFLIGHT_TRACE\"\n",
                "selected=${FAKE_LINKER_PATH:-$BUILD_TOOLS_PREFIX/bin/ld.mold}\n",
                "case \" $* \" in\n",
                "  *' -print-prog-name=ld.mold '*) printf '%s\\n' ",
                "\"$selected\" ;;\n",
                "  *' -### '*) printf ' \"%s\" \"--hash-style=gnu\"\\n' ",
                "\"$selected\" >&2 ;;\n",
                "  *) exit 0 ;;\n",
                "esac\n",
            ),
        )?;
        directory.write(
            "ld.lld",
            "#!/bin/sh\nprintf 'ld.lld:%s\\n' \"$*\" >> \"$PREFLIGHT_TRACE\"\nexit 0\n",
        )?;
        for executable in ["rustup", "bin/mold", "clang", "ld.lld"] {
            directory.set_permissions(executable, Permissions::from_mode(0o700))?;
        }
        let fixture = Self {
            parent,
            directory,
            path: parent_path.join(&name),
            name,
        };
        fixture.write_components(None).map(|()| fixture)
    }

    pub(super) fn write_components(&self, omitted: Option<&str>) -> Read<()> {
        let root = Dir::open_ambient_dir(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
            ambient_authority(),
        )?;
        let source = root.read_to_string("rust-toolchain.toml")?;
        let document: toml::Value = toml::from_str(&source)?;
        let components = document
            .get("toolchain")
            .and_then(|toolchain| toolchain.get("components"))
            .and_then(toml::Value::as_array)
            .ok_or("toolchain components must be an array")?;
        let mut contents = String::new();
        for component in components
            .iter()
            .filter_map(toml::Value::as_str)
            .filter(|component| Some(*component) != omitted)
        {
            let alias = component.strip_suffix("-preview").unwrap_or(component);
            writeln!(&mut contents, "{alias}-x86_64-unknown-linux-gnu")?;
        }
        self.directory.write("components", contents)?;
        Ok(())
    }

    pub(super) fn check(
        &self,
        coverage: bool,
        missing: Option<&str>,
        override_var: Option<(&str, &str)>,
    ) -> io::Result<Output> {
        let mut command = self.configured_command();
        if coverage {
            command.arg("--coverage");
        }
        if let Some(variable) = missing {
            command.env(variable, "missing-build-tool");
        }
        if let Some((variable, value)) = override_var {
            command.env(variable, value);
        }
        command.output()
    }

    pub(super) fn check_argument(&self, argument: &str) -> io::Result<Output> {
        self.configured_command().arg(argument).output()
    }

    fn configured_command(&self) -> Command {
        let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("bash");
        command
            .arg(root.join("scripts/check-build-tools.sh"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.path))
            .env("FAKE_COMPONENTS", self.path.join("components").as_str())
            .env("FAKE_TOOLCHAINS", self.path.join("toolchains").as_str())
            .env("FAKE_MOLD_VERSION", "2.41.0")
            .env("PREFLIGHT_TRACE", self.path.join("trace").as_str())
            .env("MOLD_VERSION_FILE", self.path.join("version").as_str())
            .env(
                "RUST_TOOLCHAIN_FILE",
                self.path.join("toolchain.toml").as_str(),
            )
            .env("BUILD_TOOLS_PREFIX", self.path.as_str())
            .env("CLANG_COMMAND", self.path.join("clang").as_str())
            .env("LLD_COMMAND", self.path.join("ld.lld").as_str());
        for variable in [
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "CARGO_BUILD_TARGET",
            "CARGO_ENCODED_RUSTFLAGS",
        ] {
            command.env_remove(variable);
        }
        command
    }

    pub(super) fn set_mold_version_pin(&self, pin: Option<&str>) -> Read<()> {
        pin.map_or_else(
            || self.directory.remove_file("version"),
            |value| self.directory.write("version", value),
        )?;
        Ok(())
    }

    pub(super) fn set_toolchain_pin(&self, contents: &str) -> Read<()> {
        self.directory.write("toolchain.toml", contents)?;
        Ok(())
    }

    pub(super) fn trace(&self) -> Read<String> { Ok(self.directory.read_to_string("trace")?) }
}

impl Drop for ToolFixture {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}
