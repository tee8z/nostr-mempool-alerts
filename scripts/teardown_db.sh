#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed"
    echo >&2 "Use:"
    echo >&2 "  cargo install --version='~0.6' sqlx-cli \
    --no-default-features --features rustls,postgres"
    echo >&2 "to install it"
    exit 1
fi


DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB=nostrmempoolalerts}"
DB_PORT="${POSTGRES_PORT=15420}"

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and runnning on port ${DB_PORT}!"

dropdb --if-exists -h localhost -p ${DB_PORT} -U ${DB_USER} ${DB_NAME}

>&2 echo "Postgres has been dropped!"

if [[ -z "${SKIP_DOCKER}" ]]
then 
    docker stop nostr_mempool_alerts_db
    docker rm nostr_mempool_alerts_db
fi