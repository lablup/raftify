#!/bin/bash
# TODO: Test this script

for i in {1..50000}
do
    curl -XGET "http://localhost:8001/put/$i/1"
done
