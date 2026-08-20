.PHONY: fmt-check clippy test check

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

check: fmt-check clippy test

# AI Cockpit start arguments are consumed by the included Makefile.ai target.
AI_START_FLAGS = $(if $(AI_START_CONCURRENCY_BOUNDARY),--concurrency-boundary '$(AI_START_CONCURRENCY_BOUNDARY)',) $(if $(AI_START_CALIBRATION_CORRECTIVE),--calibration-corrective '$(AI_START_CALIBRATION_CORRECTIVE)',)

include Makefile.ai
