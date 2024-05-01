#!/bin/bash
set -euo pipefail

SCRIPT_NAME="${BASH_SOURCE[0]}"
SCRIPT_PATH=$(dirname "$(realpath -s "${BASH_SOURCE[0]}")")
VERSION=${VERSION:-latest}
DOCKER_COMPOSE_FILE="docker-compose.yml"
HOST="localhost"
GRAFANA_DASHBOARD_JSON="grafana-provisioning/dashboards/tps.json"

check_command() {
    command -v "$1" >/dev/null 2>&1 && { $1 --version >/dev/null 2>&1; }
}

if check_command "docker compose"; then
    DOCKER_COMPOSE="docker compose"
elif check_command "docker-compose"; then
    DOCKER_COMPOSE="docker-compose"
else
    echo "Neither 'docker compose' nor 'docker-compose' could be found or are functional on your system."
    exit 1
fi

function echoerr() { echo "$@" 1>&2; }

function usage {
  cat << EOF
Script to generate PNG graphs out of CSV formatted data from smart-bench via ephemeral Grafana+InfluxDB environment

Usage: ${SCRIPT_NAME} ARGS

ARGS
 -p, --panel-id       (Required) ID of the panel within Grafana dashboard to render as PNG
 -c, --csv-data       (Required) CSV formatted output of smart-bench
 -o, --output         (Required) Path to file where output PNG image will be stored
 -h, --help           Print this help message

EXAMPLE
${SCRIPT_NAME} --panel-id=2 --csv-data=benchmark-result.csv --output=tps.png
EOF
}

function parse_args {
  function needs_arg {
    if [ -z "${OPTARG}" ]; then
      echoerr "No arg for --${OPT} option"
      exit 2
    fi
  }

  # shellcheck disable=SC2214
  while getopts p:c:o:h-: OPT; do
    # support long options: https://stackoverflow.com/a/28466267/519360
    if [ "$OPT" = "-" ]; then   # long option: reformulate OPT and OPTARG
      OPT="${OPTARG%%=*}"       # extract long option name
      OPTARG="${OPTARG#"$OPT"}"   # extract long option argument (may be empty)
      OPTARG="${OPTARG#=}"      # if long option argument, remove assigning `=`
    fi
    case "$OPT" in
      p | panel-id)             needs_arg && PANEL_ID="${OPTARG}";;
      c | csv-data)             needs_arg && CSV_DATA="${OPTARG}";;
      o | output)               needs_arg && OUTPUT="${OPTARG}";;
      h | help )                usage; exit 0;;
      ??* )                     echoerr "Illegal option --$OPT"; exit 2;;  # bad long option
      ? )                       exit 2 ;;  # bad short option (error reported via getopts)
    esac
  done
  shift $((OPTIND-1)) # remove parsed options and args from $@ list

  [ -n "${PANEL_ID-}" ] || {
    echoerr "missing -p/--panel-id arg"
    usage
    exit 2
  }
  [ -n "${CSV_DATA-}" ] || {
    echoerr "missing -c/--csv-data arg"
    usage
    exit 2
  }
  [ -n "${OUTPUT-}" ] || {
    echoerr "missing -o/--output arg"
    usage
    exit 2
  }
}

retry_command() {
  local max_retries="$1"
  local retry_interval="$2"

  # Shift to remove the first two parameters (max_retries and retry_interval)
  shift 2
  local command=("$@")

  for ((i=1; i<=max_retries; i++)); do
    if "${command[@]}"; then
      return 0
    else
      echo "Retrying in $retry_interval seconds (Attempt $i/$max_retries)..."
      sleep "$retry_interval"
    fi
  done

  return 1
}

wait_for_service() {
  local host="$1"
  local port="$2"
  local max_retries=5
  local retry_interval=1

  echo "Waiting for TCP service at $host:$port..."
  retry_command "$max_retries" "$retry_interval" nc -z "$host" "$port"
}

check_influxdb() {
  local max_retries=5
  local retry_interval=1

  wait_for_service "${HOST}" "${INFLUXDB_PORT}"
  echo "Checking InfluxDB is responsive"
  retry_command "$max_retries" "$retry_interval" curl -s -o /dev/null -w "%{http_code}" "${HOST}:${INFLUXDB_PORT}/ping"
}

check_grafana() {
  local max_retries=5
  local retry_interval=5

  wait_for_service "${HOST}" "${RENDERER_PORT}"
  wait_for_service "${HOST}" "${GRAFANA_PORT}"
  echo "Checking Grafana is responsive"
  retry_command "$max_retries" "$retry_interval" curl -s -o /dev/null -w "%{http_code}" "${HOST}:${GRAFANA_PORT}/api/health"
}

wait_for_containers() {
  local max_retries=5
  local retry_interval=1

  echo "Waiting for all Docker containers to be running..."
  retry_command "$max_retries" "$retry_interval" ${DOCKER_COMPOSE} -f "$DOCKER_COMPOSE_FILE" \
	  ps --services --filter "status=running" | grep -qvx " "
}

start_containers() {
  echo "Starting docker containers"
  ${DOCKER_COMPOSE} -f "$DOCKER_COMPOSE_FILE" up -d
  wait_for_containers
}

stop_containers() {
  echo "Stopping docker containers"
  ${DOCKER_COMPOSE} -f "$DOCKER_COMPOSE_FILE" down
}

print_containers_logs() {
  echo "Container logs -- START"
  ${DOCKER_COMPOSE} -f "$DOCKER_COMPOSE_FILE" logs
  echo "Container logs -- END"
}

convert_csv_to_line_protocol() {
  local csv_file=$1
  local lp_file=$2

  awk -F, 'function trim(field){
     gsub(/^ +| +$/,"", field);
     return field
   };
   function escape(field){
     gsub(/ /,"\\ ", field);
     return field
   }{
   printf "%s,platform=\"%s\",parachain_ver=\"%s\",contract_type=\"%s\",contract_compiler_ver=\"%s\" tx_per_sec=%s %s\n",
     "tps",
     trim($2),
     trim($3),
     trim($4),
     escape(trim($6)),
     trim($5),
     trim($1)
   }' "${csv_file}" > "${lp_file}"
}

populate_influxdb() {
  local csv_file=$1
  local tmpdir=$2
  echo "Populating InfluxDB with data"

  temp_file="${tmpdir}/line_proto.txt"
  convert_csv_to_line_protocol "${csv_file}" "${temp_file}"

  echo "Excerpt from data to be uploaded"
  tail -10 "${temp_file}"

  check_influxdb
  curl -i -XPOST "http://${HOST}:${INFLUXDB_PORT}/api/v2/write?org=${INFLUXDB_ORG}&bucket=${INFLUXDB_BUCKET}&precision=s" \
	  -H "Authorization: Token ${INFLUXDB_TOKEN}" \
	  --data-binary "@${temp_file}"

  echo "Data population complete."
}

create_grafana_snapshot() {
  echo "Creating Grafana snapshot..."
  local dashboard_id=$1
  local panel_id=$2
  local csv_file=$3
  local output_png=$4
  local tmpdir=$5

  sort -k1,1 "${csv_file}" > "${tmpdir}/sorted_csv"
  # extend with 000 as grafana requires nanosecond precision
  timestamp_start="$(head -1 "${tmpdir}/sorted_csv" | cut -f1 -d"," )000"
  timestamp_end="$(tail -1 "${tmpdir}/sorted_csv" | cut -f1 -d"," )000"

  check_grafana
  curl -u"${GRAFANA_USERNAME}:${GRAFANA_PASSWORD}" \
    "${HOST}:${GRAFANA_PORT}/render/d-solo/${dashboard_id}?orgId=1&from=${timestamp_start}&to=${timestamp_end}&panelId=${panel_id}&width=1000&height=500" \
    -o "${output_png}"
  echo "Grafana snapshot created."
}

cleanup() {
  [ -z "${tmpdir-}" ] || rm -rf -- "${tmpdir}"
  stop_containers
  echo "Workspace cleanup completed successfully."
}

parse_args "$@"
source .env

trap 'cleanup' EXIT
tmpdir="$(mktemp -d)"

start_containers
populate_influxdb "${CSV_DATA}" "${tmpdir}"

dasboard_id=$(jq -r '.uid' "${GRAFANA_DASHBOARD_JSON}")
dashboard_title=$(jq -r '.title' "${GRAFANA_DASHBOARD_JSON}")
create_grafana_snapshot "${dasboard_id}/${dashboard_title}" "${PANEL_ID}" "${CSV_DATA}" "${OUTPUT}" "${tmpdir}"

# check if file seem to contain png data
if file "${OUTPUT}" | grep -q PNG; then
  exit 0
else
  print_containers_logs
  echoerr "Output file doesn't seem to be a PNG image"
  echo "File dump -- START"
  hexdump -C "${OUTPUT}"
  echo "File dump -- END"
  exit 1
fi
