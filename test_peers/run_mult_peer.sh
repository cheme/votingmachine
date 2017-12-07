#!/bin/bash

# static as expected conf are not created automatically
# TODO could call example init latter
nb=5

for ix in {1..5}
do
        ./run_peer.sh $ix &
done
read
PID=`ps -eaf | grep voting | grep -v grep | awk '{print $2}'`
if [[ "" !=  "$PID" ]]; then
  echo "killing $PID"
  kill -9 $PID
fi
