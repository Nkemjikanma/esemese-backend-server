#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
	echo >&2 "Error: sqlx is not installed."
	echo >&2 "Use:"
	echo >&2 " cargo install sqlx-cli \
		--no-default-features --features rustls,postgres"
	echo >&2 "to install it."
	exit 1
fi

DB_PORT="${DB_PORT:=5432}"
DB_ADMIN_USERNAME="${DB_ADMIN_USERNAME:=nkemjika_admin}"
DB_ADMIN_PASSWORD="${DB_ADMIN_PASSWORD:=password}"
DB_NAME="${DB_NAME:=esemese_db}"

if [[ -z "${SKIP_DOCKER}" ]]; then
	docker compose up -d --wait postgres
fi

>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now"

DATABASE_URL=postgres://${DB_ADMIN_USERNAME}:${DB_ADMIN_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go."
