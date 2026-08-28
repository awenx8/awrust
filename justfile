help:
    @echo ""

env:
    pnpm add -DE @biomejs/biome rumdl

fmt:
    biome format --write .
    rumdl fmt .
    cargo fmt --all

lint: fmt
    biome format .
    rumdl fmt --check .
    cargo check --workspace --all-targets 2>&1
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

examples:
    @for d in crates/*/examples/*.rs; do \
        name=$$$$(basename $$$$d .rs); \
        crate=$$$$(basename $$$$(dirname $$$$(dirname $$$$d))); \
        echo "\n▶ Running $$$$crate::$$$$name ..."; \
        if [ "$$$$name" = "hot_reload" ]; then \
            timeout 5 cargo run -p $$$$crate --example $$$$name --all-features || true; \
        else \
            cargo run -p $$$$crate --example $$$$name --all-features; \
        fi; \
    done

verify: fmt lint test examples
    @echo "Verify successful"
