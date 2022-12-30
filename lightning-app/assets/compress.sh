#!/bin/sh

for file in ./*
do
	compressonator -fd BC7 "$file" "${file%.*}.dds"
done

