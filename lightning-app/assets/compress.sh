#!/bin/sh

for file in ./*
do
	compressonatorcli -fd BC7 "$file" "${file%.*}.dds"
done

