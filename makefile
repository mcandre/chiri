.POSIX:
.SILENT:
.PHONY: \
	all \
	audit \
	build \
	cargo-check \
	clean \
	clean-cargo \
	clean-examples \
	clean-go \
	clean-rust \
	clean-shell \
	clippy \
	doc \
	install \
	itest \
	itest-go \
	itest-rust \
	itest-shell \
	lint \
	publish \
	rustfmt \
	test \
	uninstall
.IGNORE: \
	clean \
	clean-cargo \
	clean-example

VERSION!=cargo metadata --format-version 1 --no-deps | jq -r ".packages[0].version"

all: build

audit: cargo-audit

build:
	cargo build --release

cargo-audit:
	cargo audit

cargo-check:
	cargo check

clean: clean-cargo clean-examples

clean-cargo:
	cargo clean

clean-examples: clean-go clean-shell

clean-go:
	sh -c "cd examples/go && chiri -c"

clean-rust:
	sh -c "cd examples/rust && chiri -c"

clean-shell:
	sh -c "cd examples/shell && chiri -c"

clippy:
	cargo clippy

doc:
	cargo doc

install:
	cargo install --force --path .

itest: itest-go itest-rust itest-shell

itest-go:
	sh -c "cd examples/go && chiri && tree .chiri/artifacts"

itest-rust:
	sh -c "cd examples/rust && chiri && tree .chiri/artifacts"

itest-shell:
	sh -c "cd examples/shell && chiri && tree .chiri/artifacts"

lint: \
	cargo-check \
	clippy \
	doc \
	rustfmt

publish:
	cargo publish

rustfmt:
	cargo fmt

test:
	cargo test

uninstall:
	cargo uninstall chiri
