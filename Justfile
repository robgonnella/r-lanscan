default:
    @just --list

ui *args:
    cargo run -p r-lanui -- {{args}}

generate-dot-env:
    echo "PRIVATE_KEY_FILE=\"${HOME}/.ssh/id_rsa\"" > .env
    echo "PUBLIC_KEY_FILE=\"${HOME}/.ssh/id_rsa.pub\"" >> .env
    echo "PUBLIC_KEY=\"$(cat ${HOME}/.ssh/id_rsa.pub)\"" >> .env

scan *args:
    cargo run -p r-lanscan -- {{args}}

up *args: generate-dot-env && exec-workspace
    docker compose up --build -d {{args}}

exec-workspace:
    docker compose exec workspace /workspace/target/debug/r-lanui

down *args:
    docker compose down --remove-orphans {{args}}

logs *args:
    docker compose logs -f {{args}}

test *args:
    cargo test {{args}}

test-report *args:
    cargo llvm-cov {{args}}
