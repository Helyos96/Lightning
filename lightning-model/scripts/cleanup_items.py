#!/bin/python

import json
import sys

if len(sys.argv) != 2:
    exit("Usage: {} <base_items.json path>".format(sys.argv[0]))

f = open(sys.argv[1])
base_items = json.load(f)
out = {}

for v in base_items.values():
    out[v["name"]] = v

print(json.dumps(out,indent=2))

