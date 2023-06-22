#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="${BASH_SOURCE[0]}"
SCRIPT_PATH=$(dirname "$(realpath -s "${BASH_SOURCE[0]}")")
THIS_GIT_REPOSITORY_ROOT=$(git rev-parse --show-toplevel)
VERSION=$(grep "^version" "${THIS_GIT_REPOSITORY_ROOT}/Cargo.toml" | grep -Eo "([0-9\.]+)")
NAME=$(grep "^name" "${THIS_GIT_REPOSITORY_ROOT}/Cargo.toml" | tr -d ' ' | tr -d '"' | cut -f2 -d'=')
IMAGE="${NAME}:v${VERSION}"

function echoerr() { echo "$@" 1>&2; }

function usage {
  cat << EOF
Usage: ${SCRIPT_NAME} OPTION -- ARGUMENTS_TO_SMART_BENCH

OPTION
 -b, --binaries-dir   Path to directory that contains all required binaries (eg. polkadot, zombienet, moonbeam)
                      List of required binaries depends on config provided
 -t, --contracts-dir  Path to directory that contains compiled smart contracts
 -c, --config         Path to zombienet config file
 -h, --help           Print this help message

ARGUMENTS_TO_SMART_BENCH
  smart-bench specific parameters (NOTE: do not provide --url param as it is managed by this tool)

EXAMPLES
${SCRIPT_NAME} --binaries-dir="bin/" --contracts-dir="../contracts" --config="configs/network_native_moonbeam.toml" -- evm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} --binaries-dir="bin/" --contracts-dir="../contracts" --config="configs/network_native_ink.toml" -- ink-wasm erc20 --instance-count 1 --call-count 10


EOF
}

function parse_args {
  function needs_arg {
    if [ -z "${OPTARG}" ]; then
      echoerr "No arg for --${OPT} option"
      exit 2
    fi
  }

  function check_args {
      BINARIES_DIR=$(realpath -qe "${BINARIES_DIR}") || {
         echoerr "BINARIES_DIR path=[${BINARIES_DIR}] doesn't exist"
         exit 2
      }
      CONTRACTS_DIR=$(realpath -qe "${CONTRACTS_DIR}") || {
         echoerr "CONTRACTS_DIR path=[${CONTRACTS_DIR}] doesn't exist"
         exit 2
      }
      CONFIG=$(realpath -qe "${CONFIG}") || {
         echoerr "CONFIG path=[${CONFIG}] doesn't exist"
         exit 2
      }
  }

  # shellcheck disable=SC2214
  while getopts b:c:t:h:-: OPT; do
    # support long options: https://stackoverflow.com/a/28466267/519360
    if [ "$OPT" = "-" ]; then   # long option: reformulate OPT and OPTARG
      OPT="${OPTARG%%=*}"       # extract long option name
      OPTARG="${OPTARG#"$OPT"}"   # extract long option argument (may be empty)
      OPTARG="${OPTARG#=}"      # if long option argument, remove assigning `=`
    fi
    case "$OPT" in
      b | binaries-dir)         needs_arg; BINARIES_DIR="${OPTARG}";;
      t | contracts-dir)        needs_arg; CONTRACTS_DIR="${OPTARG}";;
      c | config)               needs_arg; CONFIG="${OPTARG}";;
      h | help )                usage; exit 0;;
      ??* )                     echoerr "Illegal option --$OPT"; exit 2;;  # bad long option
      ? )                       exit 2 ;;  # bad short option (error reported via getopts)
    esac
  done
  shift $((OPTIND-1)) # remove parsed options and args from $@ list
  check_args
  OTHERARGS=("$@")
}

parse_args "$@"

(cd "${THIS_GIT_REPOSITORY_ROOT}" &&
   DOCKER_BUILDKIT=1 docker build \
     --build-arg DOCKERFILE_DIR="$(realpath --relative-to="${THIS_GIT_REPOSITORY_ROOT}" "${SCRIPT_PATH}")" \
     -f "${SCRIPT_PATH}/smart_bench.Dockerfile" -t "${IMAGE}" .
)

container_dir="/usr/local/"
container_zombienet_config="${container_dir}/etc/config.toml"
container_contracts="${container_dir}/etc/contracts"

volume_args="-v ${CONFIG}:${container_zombienet_config}"
volume_args="${volume_args} -v ${CONTRACTS_DIR}:${container_contracts}"
for file in "${BINARIES_DIR}"/*; do
    volume_args="${volume_args} -v ${file}:${container_dir}/bin/$(basename "${file}")"
done

# shellcheck disable=SC2086
docker run --rm -it \
  ${volume_args} \
  "${IMAGE}" \
  "${container_zombienet_config}" \
  "${container_contracts}" \
  "${OTHERARGS[@]}"
