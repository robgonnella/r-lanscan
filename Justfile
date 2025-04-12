ui *args:
    cargo run -p r-lanui -- {{args}}

scan *args:
    cargo run -p r-lanscan -- {{args}}

up *args:
    docker compose up --build -d {{args}}

down *args:
    docker compose down --remove-orphans {{args}}

logs *args:
    docker compose logs -f {{args}}
