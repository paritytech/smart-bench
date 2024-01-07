#!/bin/bash
set -e

DOCKER_COMPOSE_FILE="docker-compose.yml"
GRAFANA_URL="localhost:3000"
INFLUXDB_URL="localhost:8086"
GRAFANA_USERNAME="admin"
GRAFANA_PASSWORD="admin"
GRAFANA_DASHBOARD="e2f936b3-fffc-443a-837b-e719581a031d/transactions-dashboard"

TMPDIR="$(mktemp -d)"
trap 'rm -rf -- "$TMPDIR"' EXIT

wait_for_service() {
    local host="$1"
    local port="$2"
    local max_attempts=30
    local wait_interval=5

    echo "Waiting for TCP service at $host:$port..."

    for ((i=0; i<$max_attempts; i++)); do
        if nc -z "$host" "$port" >/dev/null 2>&1; then
            echo "TCP service is up and running!"
            return 0
        fi
        sleep "$wait_interval"
    done

    echo "Error: Unable to connect to TCP service at $host:$port after $((max_attempts * wait_interval)) seconds."
    exit 1
}

# Function to wait for all containers to be running
wait_for_containers() {
    local max_attempts=30
    local wait_interval=5

    echo "Waiting for all Docker containers to be running..."

    for ((i=0; i<$max_attempts; i++)); do
        if docker-compose -f "$DOCKER_COMPOSE_FILE" ps --services --filter "status=running" | grep -qvx " "; then
            echo "All Docker containers are running!"
            return 0
        fi
        sleep "$wait_interval"
    done

    echo "Error: Not all Docker containers are running after $((max_attempts * wait_interval)) seconds."
    exit 1
}

# Function to populate InfluxDB database with data from CSV file
populate_influxdb() {
    local csv_file=$1
    echo "Populating InfluxDB with data"

    temp_file="${TMPDIR}/line_proto.txt"
    ./csv_to_line_proto.sh "${csv_file}" > "${temp_file}"
    echo "Excerpt from data to be uploaded"
    tail -10 "${temp_file}"

    wait_for_service "$(echo ${INFLUXDB_URL} | cut -f1 -d':')" "$(echo ${INFLUXDB_URL} | cut -f2 -d':')"
    sleep 5

    curl -i -XPOST "http://${INFLUXDB_URL}/api/v2/write?org=${INFLUXDB_ORG}&bucket=${INFLUXDB_BUCKET}&precision=s" -H "Authorization: Token ${INFLUXDB_TOKEN}" --data-binary "@${temp_file}"

    echo "Data population complete."
}

# Function to create a Grafana snapshot
create_grafana_snapshot() {
    echo "Creating Grafana snapshot..."
    local csv_file=$1
    local output_png=$2

    timestamp_start="$(cat benchmark-result.csv | cut -f1 -d',' | sort | head -1)000"
    timestamp_end="$(cat benchmark-result.csv | cut -f1 -d',' | sort | tail -1)000"


    curl -u"${GRAFANA_USERNAME}:${GRAFANA_PASSWORD}" \
	    "${GRAFANA_URL}/render/d-solo/${GRAFANA_DASHBOARD}?orgId=1&from=${timestamp_start}&to=${timestamp_end}&panelId=2&width=1000&height=500" \
	    -o "${output_png}"
    echo "Grafana snapshot created."
}

source .env

docker-compose -f "$DOCKER_COMPOSE_FILE" up -d

wait_for_containers

populate_influxdb "$1"

create_grafana_snapshot "$1" "$2"

docker-compose -f "$DOCKER_COMPOSE_FILE" down

echo "Workspace cleanup completed successfully."

