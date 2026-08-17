.PHONY: fmt-check clippy test check

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

check: fmt-check clippy test
