#!/bin/python

import json

f = open("zao-repoe/RePoE/data/base_items.json")
base_items = json.load(f)
out = {}

for v in base_items.values():
    out[v["name"]] = v

print(json.dumps(out,indent=2))

