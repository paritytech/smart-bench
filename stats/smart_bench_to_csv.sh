#!/bin/bash

set -euo pipefail

SCRIPT_NAME="${BASH_SOURCE[0]}"
SCRIPT_PATH=$(dirname "$(realpath -s "${BASH_SOURCE[0]}")")
TIMESTAMP=$(date +%s)

if [ -p /dev/stdin ]; then
  PROCESS_LIST=$(ps -u)
  CONTRACT_TYPE=$(echo "${PROCESS_LIST}" | grep -v grep | grep -oE "erc20|flipper|incrementer|erc721|erc1155|odd-product|triangle-number|storage-read|storage-write|storage-read-write")
  PLATFORM=$(echo "${PROCESS_LIST}" | grep -v grep | grep -oE "ink-wasm|sol-wasm|evm")

  echo "${CONTRACT_TYPE}"
  echo "${PLATFORM}"
  STATS=$(</dev/stdin)
else
  echo "No input was found on stdin, skipping!"
fi

function echoerr() { echo "$@" 1>&2; }

function usage {
  cat << EOF
Script to translate smart bench stdout data into csv formatted benchmarking results and append to given output file
Script expects data to be piped from smart-bench application into this script

Usage: smart-bench ..... | ${SCRIPT_NAME} ARGS

ARGS
 -o, --csv-output     (Required) CSV formatted output of smart-bench
 -t, --timestamp      (Optional) Timestamp to use for benchmark results - if not provided, current time is used
 -h, --help           Print this help message

EXAMPLE
cargo run --release -- evm flipper --instance-count 1 --call-count 1500 --url ws://localhost:9988 | ${SCRIPT_NAME} --csv-output=benchmark-result.csv --timestamp=1714515934
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
  while getopts o:t:h-: OPT; do
    # support long options: https://stackoverflow.com/a/28466267/519360
    if [ "$OPT" = "-" ]; then   # long option: reformulate OPT and OPTARG
      OPT="${OPTARG%%=*}"       # extract long option name
      OPTARG="${OPTARG#"$OPT"}"   # extract long option argument (may be empty)
      OPTARG="${OPTARG#=}"      # if long option argument, remove assigning `=`
    fi
    case "$OPT" in
      o | csv-output)           needs_arg && CSV_OUTPUT="${OPTARG}";;
      t | timestamp )           needs_arg && TIMESTAMP="${OPTARG}";;
      h | help )                usage; exit 0;;
      ??* )                     echoerr "Illegal option --$OPT"; exit 2;;  # bad long option
      ? )                       exit 2 ;;  # bad short option (error reported via getopts)
    esac
  done
  shift $((OPTIND-1)) # remove parsed options and args from $@ list

  [ -n "${CSV_OUTPUT-}" ] || {
    echoerr "missing -c/--csv-output arg"
    echoerr ""
    usage
    exit 2
  }
}

parse_args "$@"


blocks=$(echo ${STATS} | grep -o 'Total Blocks: [0-9]*' | awk '{print $3}')
extrinsics=$(echo ${STATS} | grep -o 'Total Extrinsics: [0-9]*' | awk '{print $3}')
tps=$(echo ${STATS} | grep -o 'sTPS: [0-9]*' | awk '{print $2}')

echo "${TIMESTAMP}, ${PLATFORM}, not-supp, ${CONTRACT_TYPE}, ${tps}, not-supp, not-supp" >> "${CSV_OUTPUT}"
