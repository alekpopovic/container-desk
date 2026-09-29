#!/usr/bin/python3
"""035 isolated-Engine read gate. Never installed or used by the application.
A one-shot, exact labelled fixture removal models a container disappearing during inspect.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
args=sys.argv[1:]
operation=args[2:] if args[:2]==['--host','unix:///var/run/docker.sock'] else args
listing='{"name":{{json .Name}},"driver":{{json .Driver}},"scope":{{json .Scope}}}'
mounts='{"id":{{json .Id}},"name":{{json .Name}},"state":{{json .State.Status}},"mounts":{{json .Mounts}}}'
name=lambda value:bool(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,254}',value))
ident=lambda value:bool(re.fullmatch(r'[a-f0-9]{64}',value))
allowed=(len(operation)==3 and operation[0] in ('version','info') and operation[1]=='--format') or (len(operation)==4 and operation[:3]==['context','inspect','--format']) or operation in (['compose','version'],['compose','version','--format','json'])
allowed=allowed or operation==['volume','ls','--format',listing]
allowed=allowed or (len(operation)==4 and operation[:3]==['volume','inspect','--'] and name(operation[3]))
allowed=allowed or (len(operation)==6 and operation[:5]==['ps','--all','--quiet','--no-trunc','--filter'] and operation[5].startswith('volume=') and name(operation[5][7:]))
is_mounts=len(operation)>=7 and operation[:6]==['inspect','--type','container','--format',mounts,'--'] and len(operation[6:])<=64 and len(set(operation[6:]))==len(operation[6:]) and all(map(ident,operation[6:]))
allowed=allowed or is_mounts
if not allowed:sys.exit(126)
with Path('/tmp/volume-read-commands.jsonl').open('a') as log:log.write(json.dumps(operation)+'\n')
race=Path('/tmp/volume-race-id')
if is_mounts and race.exists():
    target=race.read_text().strip()
    if not ident(target) or target not in operation[6:]:sys.exit(126)
    label=subprocess.run(['/usr/bin/docker','inspect','--type','container','--format','{{index .Config.Labels "dev.containerdesk.fixture"}}','--',target],check=True,capture_output=True,text=True,timeout=10).stdout.strip()
    if label!='035-race':sys.exit(126)
    subprocess.run(['/usr/bin/docker','rm','--',target],check=True,stdout=subprocess.DEVNULL,timeout=10)
    race.unlink()
os.execv('/usr/bin/docker',['docker',*args])
