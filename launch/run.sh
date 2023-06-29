#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="${BASH_SOURCE[0]}"
SCRIPT_PATH=$(dirname "$(realpath -s "${BASH_SOURCE[0]}")")
VERSION=${VERSION:-latest}
NAME=smart-bench
IMAGE="${NAME}:${VERSION}"
BINARIES_DIR=""
CONTRACTS_DIR=""
CONFIGS_DIR=""

function echoerr() { echo "$@" 1>&2; }

function usage {
  cat << EOF
Usage: ${SCRIPT_NAME} OPTION -- ARGUMENTS_TO_SMART_BENCH

OPTION
 -b, --binaries-dir   (Optional) Path to directory that contains binaries to mount into the container (eg. polkadot, zombienet, moonbeam)
                      List of binaries being used depends on config provided. Default set of binaries is available within the image
 -t, --contracts-dir  (Optional) Path to directory that contains compiled smart contracts. Default set of compiled smart contracts is available within the image
 -u, --configs-dir    (Optional) Path to directory that contains zombienet config files. Default set of configs files is available within the image
 -h, --help           Print this help message

ARGUMENTS_TO_SMART_BENCH
  smart-bench specific parameters (NOTE: do not provide --url param as it is managed by this tool)

EXAMPLES
${SCRIPT_NAME} -- evm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} -- ink-wasm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} -- sol-wasm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} --binaries-dir=./bin -- sol-wasm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} --contracts-dir=../contracts -- sol-wasm erc20 --instance-count 1 --call-count 10
${SCRIPT_NAME} --configs-dir=./configs -- sol-wasm erc20 --instance-count 1 --call-count 10

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
      [ -z "${BINARIES_DIR}" ] || BINARIES_DIR=$(realpath -qe "${BINARIES_DIR}") || {
         echoerr "BINARIES_DIR path=[${BINARIES_DIR}] doesn't exist"
         exit 2
      }
      [ -z "${CONTRACTS_DIR}" ] || CONTRACTS_DIR=$(realpath -qe "${CONTRACTS_DIR}") || {
         echoerr "CONTRACTS_DIR path=[${CONTRACTS_DIR}] doesn't exist"
         exit 2
      }
      [ -z "${CONFIGS_DIR}" ] || CONFIGS_DIR=$(realpath -qe "${CONFIGS_DIR}") || {
         echoerr "CONTRACTS_DIR path=[${CONTRACTS_DIR}] doesn't exist"
         exit 2
      }
  }

  # shellcheck disable=SC2214
  while getopts b:c:t:u:h:-: OPT; do
    # support long options: https://stackoverflow.com/a/28466267/519360
    if [ "$OPT" = "-" ]; then   # long option: reformulate OPT and OPTARG
      OPT="${OPTARG%%=*}"       # extract long option name
      OPTARG="${OPTARG#"$OPT"}"   # extract long option argument (may be empty)
      OPTARG="${OPTARG#=}"      # if long option argument, remove assigning `=`
    fi
    case "$OPT" in
      b | binaries-dir)         BINARIES_DIR="${OPTARG}";;
      t | contracts-dir)        CONTRACTS_DIR="${OPTARG}";;
      u | configs-dir)          CONFIGS_DIR="${OPTARG}";;
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

container_dir="/usr/local"
container_zombienet_configs="${container_dir}/smart-bench/config"
container_contracts="${container_dir}/smart-bench/contracts"
container_binaries="${container_dir}/smart-bench/bin"

volume_args=""
if [ -n "${CONFIGS_DIR}" ]; then
  volume_args="${volume_args} -v ${CONFIGS_DIR}:${container_zombienet_configs}"
fi

if [ -n "${CONTRACTS_DIR}" ]; then
  volume_args="${volume_args} -v ${CONTRACTS_DIR}:${container_contracts}"
fi

if [ -n "${BINARIES_DIR}" ]; then
  volume_args="${volume_args} -v ${BINARIES_DIR}:${container_binaries}"
fi

# shellcheck disable=SC2086
(set -x; docker run --rm -it --init \
  ${volume_args} \
  "${IMAGE}" \
  "${OTHERARGS[@]}"
)
