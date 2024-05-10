#!/bin/python3

import json
import sys

if len(sys.argv) != 2:
    exit("Usage: {} <data.json path>".format(sys.argv[0]))

f = open(sys.argv[1])
tree = json.load(f)
cull = []

del tree['extraImages']
del tree['points']
del tree['nodes']['root']

classes = {}
for v in tree['classes']:
    classes[v['name']] = {
        'base_str': v['base_str'],
        'base_dex': v['base_dex'],
        'base_int': v['base_int'],
    }
tree['classes'] = classes

sprites = {}
for (k,v) in tree['sprites'].items():
    if '0.3835' not in v:
        continue
    sprites[k] = v["0.3835"]
    # Remove URL, ?XXXXX and extension
    new_filename = sprites[k]['filename'].split('/')[-1].split('?')[0].split('.')[0]
    sprites[k]['filename'] = new_filename + ".dds"
tree['sprites'] = sprites

# 'nodes' in groups string to int
for v in tree['groups'].values():
    nodes = []
    for node in v['nodes']:
        nodes.append(int(node))
    v['nodes'] = nodes

# 'out' nodes string to int
for node in tree['nodes'].values():
    if 'out' not in node:
        continue
    out = []
    for n in node['out']:
        out.append(int(n))
    node['out'] = out

# 'out' nodes string to int
for node in tree['nodes'].values():
    if 'in' not in node:
        continue
    nodes_in = []
    for n in node['in']:
        nodes_in.append(int(n))
    node['in'] = nodes_in

print(json.dumps(tree,indent=2))

