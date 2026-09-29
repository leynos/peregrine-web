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
        directory.write(
            "rustup",
            "#!/bin/sh\ncase \"$1 $2\" in\n  'toolchain list') cat \"$FAKE_TOOLCHAINS\" ;;\n  \
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
            "#!/bin/sh\nprintf 'mold %s (compatible with GNU ld)\\n' \"$FAKE_MOLD_VERSION\"\n",
        )?;
        directory.symlink("mold", "bin/ld.mold")?;
        directory.write(
            "clang",
            "#!/bin/sh\nselected=${FAKE_LINKER_PATH:-$BUILD_TOOLS_PREFIX/bin/ld.mold}\ncase \" $* \
             \" in\n  *' -print-prog-name=ld.mold '*) printf '%s\\n' \"$selected\" ;;\n  *' -### \
             '*) printf ' \"%s\" \"--hash-style=gnu\"\\n' \"$selected\" >&2 ;;\n  *) exit 0 \
             ;;\nesac\n",
        )?;
        directory.write("ld.lld", "#!/bin/sh\nexit 0\n")?;
        for executable in ["rustup", "bin/mold", "clang", "ld.lld"] {
            directory.set_permissions(executable, Permissions::from_mode(0o700))?;
        }
        let fixture = Self {
            parent,
            directory,
            path: parent_path.join(&name),
            name,
        };
        fixture.write_components(None)?;
        Ok(fixture)
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
        let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("bash");
        command.arg(root.join("scripts/check-build-tools.sh"));
        if coverage {
            command.arg("--coverage");
        }
        command
            .env("PATH", format!("{}:/usr/bin:/bin", self.path))
            .env("FAKE_COMPONENTS", self.path.join("components").as_str())
            .env("FAKE_TOOLCHAINS", self.path.join("toolchains").as_str())
            .env("FAKE_MOLD_VERSION", "2.41.0")
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
        if let Some(variable) = missing {
            command.env(variable, "missing-build-tool");
        }
        if let Some((variable, value)) = override_var {
            command.env(variable, value);
        }
        command.output()
    }
}

impl Drop for ToolFixture {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}
