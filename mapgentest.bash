#!/bin/bash

touch output
NUM_FAILS=0
FAILS=""
for (( i=0; i < 256; ++i ))
do
    echo "$i:" >> output.txt
    cargo run --profile="release-with-debug" -- maptest -s $i
    if [ $? -eq 0 ]; then
	echo "$i: success" >> output
    else
	echo "$i: FAILURE" >> output
	NUM_FAILS=$(($NUM_FAILS+1))
	FAILS="$FAIL:$i"
    fi
done

if [ $NUM_FAILS -eq 0 ]; then
    echo "No failures"
    rm output
else
    echo "$NUM_FAILS Failures:"
    echo "$FAILS"
fi
