#!/bin/bash
rm -rf testcmdout
mkdir testcmdout
cp ./base.data ./testcmdout
cd testcmdout
PATH=$PATH:..
#genbasestriples
STRIPLE_BASE=./base.data ; export STRIPLE_BASE
# check some of this base
striple check -i ./base.data -x 4 --fromfile ./base.data -x 1
striple check -i ./base.data -x 2 --fromfile ./base.data -x 2
striple check -i ./base.data -x 1 --fromfile ./base.data -x 1
peerkind=$( echo "Voting manchine peer" | base64 -w 0 )
striple create --kindfile ./base.data -x 8 --fromfile ./base.data -x 1 --aboutfile ./base.data -x 4 --content ${peerkind} -o ./refs.data -c NoCipher
striple disp -i refs.data
