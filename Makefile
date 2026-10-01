# `make fmt` and `make check-fmt` call mdtablefix directly. `--git` selects the
# Markdown files Git tracks and `--include-untracked` adds the untracked files
# Git does not ignore, so a new document is formatted before it is staged.
# Both modes need mdtablefix 0.6.0 or later; CI pins the version at its
# install-mdtablefix step.
MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences

.PHONY: help all clean test act-validation act-contract-smoke whitaker-driver-integration build release package coverage \
  lint lint-clippy lint-whitaker fmt check-fmt markdownlint spelling nixie \
  audit rust-audit install-build-tools check-build-tools check-coverage-tools

SHELL := bash


TARGET ?= libperegrine-web.rlib

USER_WHITAKER := $(HOME)/.local/bin/whitaker
USER_BIN_PATH := $(HOME)/.cargo/bin:$(HOME)/.local/bin:$(HOME)/.bun/bin
CARGO ?= cargo
BUILD_JOBS ?=
BUILD_TOOLS_PREFIX ?= $(HOME)/.local
CHECK_BUILD_TOOLS ?= scripts/check-build-tools.sh
export BUILD_TOOLS_PREFIX
export PATH := $(BUILD_TOOLS_PREFIX)/bin:$(PATH)
POLONIUS_FLAGS ?=
RUST_FLAGS ?=
RUST_FLAGS := -D warnings $(RUST_FLAGS)
# The build standard: every `rustflags` source in `.cargo/config.toml` carries
# the parallel frontend, and the x86_64 GNU Linux source adds `mold`. Assigning `RUSTFLAGS`
# replaces those sources outright, so the gate targets restate the flags here.
# Development recipes add those flags to inherited RUSTFLAGS, including the CI
# setup-rust value. Coverage, release, and packaging take neither. The checked toolchain
# components come from rust-toolchain.toml. Native Cargo routes through the
# Clang wrapper, which puts pinned ld.mold first; coverage checks its lld linker.
DEV_THREADS_FLAGS ?= -Zthreads=8
BUILD_HOST_OS := $(shell uname -s)
# Only the native x86_64 GNU Linux route has a pinned mold linker.
BUILD_HOST_ARCH := $(shell uname -m)
BUILD_HOST_TRIPLE := $(shell rustup show 2>/dev/null | sed -n 's/^Default host: //p')
DEV_LINKER_FLAGS ?= $(if $(and $(filter Linux,$(BUILD_HOST_OS)),$(filter x86_64,$(BUILD_HOST_ARCH)),$(filter x86_64-unknown-linux-gnu,$(BUILD_HOST_TRIPLE))),-C link-arg=-fuse-ld=mold)
DEV_RUST_FLAGS ?= $(RUST_FLAGS) $(POLONIUS_FLAGS) $(DEV_THREADS_FLAGS) $(DEV_LINKER_FLAGS)
RELEASE_RUST_FLAGS ?= $(RUST_FLAGS) $(POLONIUS_FLAGS)
PACKAGE_FLAGS ?=
RUSTDOC_FLAGS ?=
RUSTDOC_FLAGS := --cfg docsrs -D warnings $(POLONIUS_FLAGS) $(RUSTDOC_FLAGS)
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
TEST_CMD := $(if $(shell $(CARGO) nextest --version 2>/dev/null),nextest run,test)
WITH_ACT ?= 0
ACT ?= act
ACT_RUNNER_IMAGE ?= catthehacker/ubuntu:act-latest
ACT_GIT_COMMON_DIR := $(shell git rev-parse --path-format=absolute --git-common-dir)
ACT_GITHUB_TOKEN ?= $(or $(GITHUB_TOKEN),$(GH_TOKEN))
COVERAGE_LINKER_FLAGS ?= -fuse-ld=lld
COVERAGE_RUST_FLAGS ?= $(RUST_FLAGS) $(POLONIUS_FLAGS) -C link-arg=$(COVERAGE_LINKER_FLAGS)
# RUSTFLAGS displaces configured flags, while the dev profile backend needs its
# own override so coverage runs LLVM instrumentation instead of Cranelift.
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie
TYPOS_CONFIG_BUILDER = uv tool run --from \
	"git+https://github.com/leynos/typos-config-builder.git@v0.1.3" \
	typos-config-builder
WHITAKER ?= $(or $(shell command -v whitaker 2>/dev/null),$(wildcard $(USER_WHITAKER)),whitaker)

build: target/debug/$(TARGET) ## Build debug binary
release: ## Build the production release artefact with LLVM and the platform linker
	$(PRODUCTION_ENV) $(CARGO) build $(BUILD_JOBS) --release

package: ## Build and verify a publishable Cargo package with LLVM
	$(PRODUCTION_ENV) $(CARGO) package --locked $(PACKAGE_FLAGS)

all: ## Perform every commit gate sequentially, even with make -j
	+$(MAKE) check-fmt
	+$(MAKE) lint
	+$(MAKE) test
	+$(MAKE) spelling

clean: ## Remove build artefacts
	$(CARGO) clean
	rm -f .typos-oxendict-base.json .typos-oxendict-base.toml

test: check-build-tools ## Run tests with warnings treated as errors
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) $(TEST_CMD) $(TEST_FLAGS) $(BUILD_JOBS)
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) test --doc --workspace --all-features
	if [ "$(WITH_ACT)" = "1" ]; then $(MAKE) act-validation; fi

act-validation: ## Run the CI workflow through Act after outer Cargo tests pass
	@GITHUB_TOKEN="$(ACT_GITHUB_TOKEN)" $(ACT) pull_request --bind \
		--container-options "--volume $(ACT_GIT_COMMON_DIR):$(ACT_GIT_COMMON_DIR):ro" \
		--secret GITHUB_TOKEN \
		--env ACT=true \
		--platform "ubuntu-latest=$(ACT_RUNNER_IMAGE)" \
		--workflows .github/workflows/ci.yml --job build-test

act-contract-smoke: check-build-tools ## Check Act step routing with local action and command fixtures
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) test --test act_workflow smoke::real_act_runs_derived_workflow_and_propagates_failure -- --ignored --exact

whitaker-driver-integration: ## Check cold then warm Dylint driver construction
	@set -euo pipefail; mkdir -p target; \
		cache_dir="$$(mktemp -d "$(CURDIR)/target/whitaker-driver.XXXXXX")"; \
		trap 'rm -rf "$$cache_dir"' EXIT; \
		DYLINT_DRIVER_PATH="$$cache_dir" $(MAKE) lint-whitaker; \
		DYLINT_DRIVER_PATH="$$cache_dir" $(MAKE) lint-whitaker

target/debug/$(TARGET): check-build-tools ## Build debug binary
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) build $(BUILD_JOBS)

# Cargo packages the repository config and verifies with the dev profile. Both
# profiles therefore select LLVM; the native linker override bypasses the
# development wrapper even for release host/build-script units. An inherited
# encoded flags value would outrank RUSTFLAGS, so remove it explicitly.
PRODUCTION_ENV = env -u CARGO_ENCODED_RUSTFLAGS CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm CARGO_PROFILE_RELEASE_CODEGEN_BACKEND=llvm RUSTFLAGS="$(RELEASE_RUST_FLAGS)"

coverage: check-coverage-tools ## Generate lcov coverage with lld for llvm-tools compatibility
	@echo "coverage linker flags: $(COVERAGE_LINKER_FLAGS)"
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang \
		CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm \
		RUSTFLAGS="$(COVERAGE_RUST_FLAGS)" \
		CFLAGS="$(COVERAGE_LINKER_FLAGS)" \
		LDFLAGS="$(COVERAGE_LINKER_FLAGS)" \
		$(CARGO) llvm-cov --lcov --output-path lcov.info $(TEST_FLAGS)

lint: ## Run rustdoc, Clippy, then Whitaker sequentially
	+$(MAKE) lint-clippy
	+$(MAKE) lint-whitaker

lint-clippy: check-build-tools ## Run rustdoc and Clippy with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) doc --no-deps
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) clippy $(CLIPPY_FLAGS)

WHITAKER_CLEAN_ENV = env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
	-u CARGO_PROFILE_DEV_CODEGEN_BACKEND -u CARGO_BUILD_TARGET \
	-u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CFLAGS -u LDFLAGS

lint-whitaker: ## Run Whitaker with clean driver inputs and repository Cargo defaults
	@$(WHITAKER_CLEAN_ENV) $(CHECK_BUILD_TOOLS)
	@echo "Whitaker binary: $(WHITAKER)"
	$(WHITAKER_CLEAN_ENV) PATH="$(USER_BIN_PATH):$(PATH)" DYLINT_RUSTFLAGS="$(RUST_FLAGS) $(POLONIUS_FLAGS)" $(WHITAKER) --all -- $(CARGO_FLAGS)

typecheck: check-build-tools ## Type-check without building
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(DEV_RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS)

install-build-tools: ## Install pinned `mold` and the repository toolchain
	@scripts/install-build-tools.sh

check-build-tools: ## Check pinned development build prerequisites
	@case " $(CARGO_FLAGS) $(TEST_FLAGS) $(BUILD_JOBS) " in *" --target"*) \
		echo "build-tools: --target is outside the supported native Make build" >&2; exit 1;; \
	esac; $(CHECK_BUILD_TOOLS)

check-coverage-tools: ## Check coverage linker prerequisites
	@case " $(TEST_FLAGS) " in *" --target"*) \
		echo "build-tools: --target is outside the supported native Make coverage" >&2; exit 1;; \
	esac; $(CHECK_BUILD_TOOLS) --coverage

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	$(MDTABLEFIX) --in-place $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)
	$(MDLINT) --fix "**/*.md"

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check
	$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'
	+$(MAKE) spelling
spelling: ## Enforce en-GB-oxendict spelling in Markdown prose
	@if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then \
		echo "make spelling needs a Git repository: the gate enumerates tracked files with git ls-files. Run: git init && git add -A" >&2; \
		exit 1; \
	fi
	@if [ -z "$$(git ls-files)" ]; then \
		echo "make spelling found no tracked files: the gate enumerates tracked files with git ls-files. Run: git add -A" >&2; \
		exit 1; \
	fi
# The managed session injects GitHub URL rewrites through an identity helper.
# The public pinned tool and local git ls-files input need no injected config.
	env -u GIT_CONFIG_COUNT $(TYPOS_CONFIG_BUILDER) gate

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

audit: rust-audit ## Audit dependencies for known vulnerabilities

rust-audit: ## Audit the Rust workspace for known vulnerabilities
	set -eo pipefail; \
	manifest_list=$$(mktemp); \
	trap 'rm -f "$$manifest_list"' EXIT; \
	printf "Audit metadata phase: deriving workspace manifests\n"; \
	$(CARGO) metadata --no-deps --format-version 1 | python3 -c 'import json, sys; metadata = json.load(sys.stdin); members = set(metadata["workspace_members"]); print(metadata["workspace_root"]); [print(package["manifest_path"]) for package in metadata["packages"] if package["id"] in members]' > "$$manifest_list"; \
	workspace_root=$$(sed -n '1p' "$$manifest_list"); \
	audit_flags=(); \
	for advisory in $$CARGO_AUDIT_IGNORES; do \
		audit_flags+=(--ignore "$$advisory"); \
	done; \
	printf "Auditing Rust workspace %s\n" "$$workspace_root"; \
	sed -n '2,$$p' "$$manifest_list" | while IFS= read -r manifest; do \
		manifest_dir=$$(dirname "$$manifest"); \
		printf "Workspace Rust manifest %s\n" "$$manifest_dir/Cargo.toml"; \
	done; \
	printf "Audit execution phase: running cargo audit\n"; \
	printf "Audit failures may indicate RustSec advisories, cargo metadata errors, or documented ignores that need CARGO_AUDIT_IGNORES entries.\n"; \
	(cd "$$workspace_root" && $(CARGO) audit "$${audit_flags[@]}")

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
