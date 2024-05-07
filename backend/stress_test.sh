#!/bin/sh

# Loop over the requests and send each one in the background
for request in $(seq 1 10000)
do
  echo "result,-0.00108793180413" | nc 127.0.0.1 7878 &
done

# Wait for all background jobs to finish
wait
