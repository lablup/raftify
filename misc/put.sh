#!/bin/bash

# 200번 반복
for i in {1..200}
do
  curl -XGET "http://localhost:8001/put/1/A"
done