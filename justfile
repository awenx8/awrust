help:
    @echo ""

setup:
    @just -f vendor/awenv/justfile setup
    @just -f vendor/awdocker/justfile up postgres
    @if [ ! -f .env ] && [ -f .env.example ]; then cp .env.example .env && echo ">> created .env from .env.example"; fi
    @echo ">> setup done: env ready, postgres started"

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
        name=$(basename $d .rs); \
        crate=$(basename $(dirname $(dirname $d))); \
        echo "\n▶ Running $crate::$name ..."; \
        case "$name" in \
            hot_reload|graceful_shutdown) timeout 5 cargo run -p $crate --example $name --all-features || true ;; \
            mysql_connect|postgres_connect|redis_connect) echo "⏭ Skipping $crate::$name (requires a live database)" ;; \
            *) timeout 60 cargo run -p $crate --example $name --all-features || true ;; \
        esac; \
    done

verify: fmt lint test examples
    @echo "Verify successful"
