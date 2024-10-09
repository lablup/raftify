#!/bin/bash
# TODO: Consider finding a better method than running the script.
# On macOS, unlike Linux, loopback addresses other than 127.0.0.1 are not assigned by default, which may cause integration tests to fail. 
# In that case, please run the following script first.

# TODO: Support other octets as well, not just the last octet.
for i in {1..254}; do
    if [[ "$(uname)" == "Darwin" ]]; then
        sudo ifconfig lo0 alias 127.0.0.$i up
    elif [[ "$(uname)" == "Linux" ]]; then
        sudo ifconfig lo:0 127.0.0.$i netmask 255.0.0.0 up
    else
        echo "Unsupported OS"
        exit 1
    fi
done
