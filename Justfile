default:
    @just --list

generate-dot-env:
    echo "PRIVATE_KEY_FILE=\"${HOME}/.ssh/id_rsa\"" > .env
    echo "PUBLIC_KEY_FILE=\"${HOME}/.ssh/id_rsa.pub\"" >> .env
    echo "PUBLIC_KEY=\"$(cat ${HOME}/.ssh/id_rsa.pub)\"" >> .env

term *args:
    cargo run -p r-lanterm -- {{args}}

scan *args:
    cargo run -p r-lancli -- {{args}}

up *args: generate-dot-env && exec-workspace
    docker compose up --build -d {{args}}

exec-workspace:
    docker compose exec workspace /workspace/target/debug/r-lanterm

down *args:
    docker compose down --remove-orphans {{args}}

logs *args:
    docker compose logs -f {{args}}

test *args:
    cargo test {{args}}

test-report *args:
    cargo llvm-cov {{args}}
