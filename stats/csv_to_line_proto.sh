#!/bin/bash

trim_spaces() {
    awk '{$1=$1};1'
}

convert_csv_to_line_protocol() {
    local csv_file=$1
    local measurement="tps"

    cat "$csv_file" | while IFS=',' read -r timestamp platform parachain_ver contract_type tx_per_sec contract_compiler_ver; do
        timestamp=$(echo $timestamp | trim_spaces)
        platform=$(echo $platform | trim_spaces)
        parachain_ver=$(echo $parachain_ver | trim_spaces)
        contract_type=$(echo $contract_type | trim_spaces)
        tx_per_sec=$(echo $tx_per_sec | trim_spaces)
	contract_compiler_ver=$(printf %q "$(echo $contract_compiler_ver | trim_spaces)")

	line_protocol="${measurement},platform=${platform},parachain_ver=${parachain_ver},contract_type=${contract_type},contract_compiler_ver=\"${contract_compiler_ver}\" tx_per_sec=${tx_per_sec} ${timestamp}"
        echo "$line_protocol"
    done
}

convert_csv_to_line_protocol "$1"
