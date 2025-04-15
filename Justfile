ui *args:
    cargo run -p r-lanui -- {{args}}

generate-dot-env:
    echo "PRIVATE_KEY_FILE=\"\${HOME}/.ssh/id_rsa\"" > .env
    echo "PUBLIC_KEY_FILE=\"\${HOME}/.ssh/id_rsa.pub\"" >> .env
    echo "PUBLIC_KEY=\"$(cat ${HOME}/.ssh/id_rsa.pub)\"" >> .env

scan *args:
    cargo run -p r-lanscan -- {{args}}

up *args:
    docker compose up --build -d {{args}}

down *args:
    docker compose down --remove-orphans {{args}}

logs *args:
    docker compose logs -f {{args}}
