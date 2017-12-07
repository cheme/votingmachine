#!/bin/bash
STRIPLE_BASE=../base.data ; export STRIPLE_BASE
RUST_BACKTRACE=1; export RUST_BACKTRACE 

ix=$1
efd=3
fifo=fifo$ix
voteconf=./peers/conf$1.json
bootstrap=./peers_bootstrap/conf$1.json
EXE="../target/debug/voting -C $voteconf -B $bootstrap"
echo $EXE
rm -f ${fifo}
mkfifo ${fifo}
exec 3<> ${fifo}
cp ../refs.data .
expect=(
  "vote_file, vote_key, quit?"
  "file path?"
  "your vote ?"
  )
answers=(
  "vote_file"
  "../vote.json"
  "yes"
  )
opt_1="vote_file"
opt_2="../vote.json"
opt_3="yes"
state=0
while IFS= read -d $'\0' -n 1 a ; do
    str+="${a}"

    if [ "${str}" = "${expect[$state]}" ] ; then
        echo "!!! found: ${str}"
        echo "${answers[$state]}" >&3
        unset str
        ((state++))
        
    fi

    if [ "$a" = $'\n' ] ; then
        echo "$ix : ${str}"
        unset str
    fi
done < <($EXE <${fifo})

rm ${fifo}
