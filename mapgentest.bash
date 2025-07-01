#!/bin/bash

touch output.txt
for (( i=0; i < 256; ++i ))
do
    cargo run -- maptest -s $i || echo "DEBUG FAILURE AT $i" >> output.txt
    cargo run --release -- maptest -s $i || echo "RELEASE FAILURE AT $i" >> output.txt
done

echo "Failures:"
cat output.txt
