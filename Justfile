default:
    @just --list

generate-dot-env:
    echo "PRIVATE_KEY_FILE=\"${HOME}/.ssh/id_rsa\"" > .env
    echo "PUBLIC_KEY_FILE=\"${HOME}/.ssh/id_rsa.pub\"" >> .env
    echo "PUBLIC_KEY=\"$(cat ${HOME}/.ssh/id_rsa.pub)\"" >> .env

term *args:
    sudo -E cargo run -p r-lanterm -- {{args}}

scan *args:
    sudo -E cargo run -p r-lancli -- {{args}}

up *args: generate-dot-env
    docker compose up --build -d {{args}}

exec-workspace: up
    docker compose exec workspace sh

exec-workspace-term: up
    docker compose exec workspace /workspace/target/debug/r-lanterm

down *args:
    docker compose down --remove-orphans {{args}}

logs *args:
    docker compose logs -f {{args}}

test *args:
    cargo test {{args}}

test-report *args:
    cargo llvm-cov --ignore-filename-regex "(_test.rs$)|(_tests.rs$)" {{args}}

lint *args:
    cargo clippy --all-targets --all-features {{args}}
