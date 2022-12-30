#!/bin/python3

import json

f = open("skilltree-export/data.json")
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

for v in tree['groups'].values():
    nodes = []
    for node in v['nodes']:
        nodes.append(int(node))
    v['nodes'] = nodes


print(json.dumps(tree,indent=2))

