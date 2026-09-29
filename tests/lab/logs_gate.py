"""Fixture-only forced SSH command: canonical POSIX argv, read probes, and owned log IDs.
This helper is never installed or required by ContainerDesk.
"""
import argparse
import os
import shlex
import sys
import time
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--mvp', action='store_true')
parser.add_argument('--terminal', action='store_true')
parser.add_argument('--compose-actions', action='store_true')
parser.add_argument('--networks', action='store_true')
parser.add_argument('--batch', action='store_true')
parser.add_argument('--management', action='store_true')
parser.add_argument('--config', required=True)
parser.add_argument('--control', type=Path)
parser.add_argument('--owned', nargs='+', required=True)
args = parser.parse_args()
original = os.environ.get('SSH_ORIGINAL_COMMAND', '')
try:
    words = shlex.split(original)
except ValueError:
    sys.exit(126)
compose_metadata=None
compound_files=None
if args.compose_actions and args.control:
    import json
    compose_metadata=json.loads((args.control/'compose-project.json').read_text())
if words and words[0]=='cd' and compose_metadata:
    if len(words)<4 or words[1]!=compose_metadata['configuration']['workingDirectory'] or words[2]!='&&':sys.exit(126)
    directory=words[1];remaining=words[3:];compound_files=[]
    while remaining[:2]==['test','-f']:
        if len(remaining)<8 or remaining[3:6]!=['&&','test','-r'] or remaining[6]!=remaining[2] or remaining[7]!='&&' or remaining[2] not in compose_metadata['allowedFiles']:sys.exit(126)
        compound_files.append(remaining[2]);remaining=remaining[8:]
    if not compound_files or len(compound_files)>8 or len(set(compound_files))!=len(compound_files):sys.exit(126)
    prefix='cd '+"'"+directory.replace("'","'\\''")+"'"
    for file in compound_files:
        quoted="'"+file.replace("'","'\\''")+"'"
        prefix+=' && test -f '+quoted+' && test -r '+quoted
    words=remaining
else:prefix=None
if not words or words.pop(0)!='exec':sys.exit(126)
canonical='exec '+' '.join("'"+word.replace("'","'\\''")+"'" for word in words)
if prefix:canonical=prefix+' && '+canonical
if canonical!=original:sys.exit(126)
env = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'DOCKER_CONFIG': args.config, 'DOCKER_HOST': 'unix:///var/run/docker.sock'}
allowed = words == ['printf', 'containerdesk-access-ok']
if words and words[0] in ('docker', '/usr/bin/docker'):
    operation = words[1:]
    if operation[:2] == ['--host', 'unix:///var/run/docker.sock']:
        operation = operation[2:]
    if args.terminal and operation[:1]==['exec']:
        if len(operation)!=8 or operation[:6]!=['exec','--interactive','--tty','--user','1000:1000','--'] or operation[6] not in args.owned[2:] or operation[7] not in ('/bin/sh','/bin/bash') or not os.isatty(0):sys.exit(126)
        import json
        with (args.control/'pty-execs.jsonl').open('a') as trace: trace.write(json.dumps({'id':operation[6],'shell':operation[7],'tty':True,'user':'1000:1000'})+'\n')
        env['TERM']='xterm-256color'
        os.execve('/usr/bin/docker',['docker','--config',args.config,'--host','unix:///var/run/docker.sock']+operation,env)
    if args.terminal and os.isatty(0):sys.exit(126)
    allowed = (len(operation) == 3 and operation[0] in ('version', 'info') and operation[1] == '--format') or (len(operation) == 4 and operation[:3] == ['context', 'inspect', '--format']) or operation in (['compose', 'version'], ['compose', 'version', '--format', 'json'])
    if compose_metadata:
        if operation==['compose','ls','--all','--format','json']:
            allowed=True
            words.extend(['--filter','name='+compose_metadata['configuration']['projectName']])
            original='exec '+' '.join(shlex.quote(word) for word in words)
        template='{"id":{{json .Id}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},"service":{{json (index .Config.Labels "com.docker.compose.service")}},"hash":{{json (index .Config.Labels "com.docker.compose.config-hash")}},"oneoff":{{json (index .Config.Labels "com.docker.compose.oneoff")}}}'
        if len(operation)>=7 and operation[:6]==['inspect','--type','container','--format',template,'--']:
            allowed=1<=len(operation[6:])<=20 and len(set(operation[6:]))==len(operation[6:]) and all(ident in compose_metadata['ids'] for ident in operation[6:])
        if compound_files:
            prefix=['compose','--ansi','never','--progress','quiet','--parallel','1','--profile','*','--project-directory',compose_metadata['configuration']['workingDirectory'],'--project-name']
            if operation[:len(prefix)]!=prefix or len(operation)<=len(prefix) or operation[len(prefix)] not in compose_metadata['allowedProjects']:sys.exit(126)
            project=operation[len(prefix)];offset=len(prefix)+1
            file_args=[]
            for file in compound_files:file_args.extend(['--file',file])
            if operation[offset:offset+len(file_args)]!=file_args:sys.exit(126)
            tail=operation[offset+len(file_args):]
            allowed=tail in (['config','--quiet'],['config','--hash','*'],['ps','--all','--quiet','--orphans=false'])
            if project==compose_metadata['configuration']['projectName'] and compound_files==compose_metadata['configuration']['configFiles']:
                valid=(tail==['start','--','web','worker']) or (len(tail)==6 and tail[0]=='stop' and tail[1]=='--timeout' and tail[2].isdigit() and 1<=int(tail[2])<=120 and tail[3:]==['--','web','worker']) or (len(tail)==7 and tail[:3]==['restart','--no-deps','--timeout'] and tail[3].isdigit() and 1<=int(tail[3])<=120 and tail[4:]==['--','web','worker'])
                if valid:
                    allowed=True
                    with (args.control/'compose-action-commands.jsonl').open('a') as trace:trace.write(json.dumps(tail)+'\n')
            if not allowed:sys.exit(126)
    if args.networks and args.control:
        import json
        network=(args.control/'owned-network-id').read_text().strip()
        template='{"id":{{json .ID}},"name":{{json .Name}},"driver":{{json .Driver}},"scope":{{json .Scope}},"internal":{{json .Internal}},"ipv6":{{json .IPv6}}}'
        if operation==['network','ls','--no-trunc','--format',template]:
            allowed=True
            words.extend(['--filter','id='+network])
            original='exec '+' '.join(shlex.quote(word) for word in words)
        if operation==['network','inspect','--',network]:allowed=True
        if operation and operation[0]=='network':
            if not allowed:sys.exit(126)
            with (args.control/'network-commands.jsonl').open('a') as trace:trace.write(json.dumps(operation)+'\n')
    if args.management and args.control and (args.control / 'mutation-identity-drift').exists() and len(operation) == 3 and operation[:2] == ['info', '--format']:
        print('{"id":"controlled-drift-032","os":"linux","security":[]}')
        sys.exit(0)
    if args.management and operation and operation[0] in ('start', 'stop', 'restart', 'rm'):
        target = operation[-1]
        admitted = target in args.owned if args.batch else target == args.owned[-1]
        valid = admitted and ((operation == ['start', '--', target]) or (args.batch and operation == ['rm', '--', target]) or (len(operation) == 5 and operation[0] in ('stop', 'restart') and operation[1] == '-t' and operation[2].isdigit() and 1 <= int(operation[2]) <= 120 and operation[3:] == ['--', target]))
        if not valid or not args.control: sys.exit(126)
        with (args.control / 'mutation-count').open('a') as counter: counter.write(operation[0] + '\n')
        if args.batch and target == args.owned[1] and (args.control / 'deny-batch-target').exists():
            print('Permission denied by owned test gate', file=sys.stderr)
            sys.exit(1)
        completed = subprocess.run(['/bin/sh', '-c', original], env=env, timeout=140)
        if (args.control / 'hold-mutation').exists():
            (args.control / 'mutation-dispatched').write_text(operation[0])
            deadline = time.monotonic() + 40
            while (args.control / 'hold-mutation').exists() and time.monotonic() < deadline: time.sleep(.02)
        sys.exit(completed.returncode)
    if len(operation) == 7 and operation[:6] == ['stats', '--no-stream', '--no-trunc', '--format', '{{json .}}', '--'] and operation[6] in args.owned:
        allowed = True
        if args.control and (args.control / 'remove-during-stats').exists():
            (args.control / 'stats-dispatch').write_text(operation[6])
            deadline = time.monotonic() + 10
            while not (args.control / 'stats-removed').exists() and time.monotonic() < deadline:
                time.sleep(.02)
            if not (args.control / 'stats-removed').exists(): sys.exit(124)
    if len(operation) == 7 and operation[:4] == ['inspect', '--type', 'container', '--format'] and operation[4] == '{"state":{{json .State.Status}},"running":{{json .State.Running}}}' and operation[5] == '--' and operation[6] in args.owned:
        allowed = True
    if len(operation) == 7 and operation[:4] == ['inspect', '--type', 'container', '--format'] and operation[4] == '{"running":{{json .State.Running}},"startedAt":{{json .State.StartedAt}}}' and operation[5] == '--' and operation[6] in args.owned:
        allowed = True
    if operation[:3] == ['logs', '--follow', '--timestamps']:
        operation = [operation[0]] + operation[2:]
    if operation[:2] == ['logs', '--timestamps'] and operation[-2:-1] == ['--'] and operation[-1] in args.owned:
        options = operation[2:-2]
        allowed = len(options) in (2, 4, 6) and options[0] == '--tail'
        for index in range(0, len(options), 2):
            allowed = allowed and options[index] in ('--tail', '--since', '--until') and all(c in '0123456789.' for c in options[index + 1])
    if operation[:5] == ['events', '--filter', 'type=container', '--format', '{{json .}}']:
        options = operation[5:]
        allowed = not options or (len(options) == 2 and options[0] == '--since' and options[1] and all(c in '0123456789.' for c in options[1]))
        if allowed:
            for ident in args.owned: words.extend(['--filter', 'container=' + ident])
            original = 'exec ' + ' '.join(shlex.quote(word) for word in words)
    # Project listing is deliberately not admitted: 030 exercises honest label fallback.
    # Real Compose-plugin listing on a wholly isolated Engine is covered by 029.
    if (args.mvp or args.compose_actions) and operation == ['ps', '--all', '--quiet', '--no-trunc', '--filter', 'label=com.docker.compose.project']:
        allowed = True
        for ident in args.owned: words.extend(['--filter', 'id=' + ident])
        original = 'exec ' + ' '.join(shlex.quote(word) for word in words)
    if (args.mvp or args.compose_actions) and len(operation) >= 7 and operation[:4] == ['inspect', '--type', 'container', '--format'] and operation[4] == '{"id":{{json .Id}},"name":{{json .Name}},"state":{{json .State.Status}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},"service":{{json (index .Config.Labels "com.docker.compose.service")}},"configFiles":{{json (index .Config.Labels "com.docker.compose.project.config_files")}},"workingDir":{{json (index .Config.Labels "com.docker.compose.project.working_dir")}}}' and operation[5] == '--':
        allowed = 1 <= len(operation[6:]) <= 64 and len(set(operation[6:])) == len(operation[6:]) and all(ident in args.owned for ident in operation[6:])
    if operation == ['ps', '--all', '--no-trunc', '--format', '{{json .}}']:
        allowed = True
        if args.control and (args.control / 'delay-list').exists():
            (args.control / ('list-started-' + str(os.getpid()))).write_text('owned read started')
            deadline = time.monotonic() + 15
            while (args.control / 'delay-list').exists() and time.monotonic() < deadline: time.sleep(.02)
            if (args.control / 'delay-list').exists(): sys.exit(124)
        for ident in args.owned: words.extend(['--filter', 'id=' + ident])
        original = 'exec ' + ' '.join("'" + word.replace("'", "'\\''") + "'" for word in words)
    if len(operation) == 5 and operation[:4] == ['inspect', '--type', 'container', '--'] and operation[4] in args.owned:
        allowed = True
if not allowed:
    sys.exit(126)
# Exercise the real remote POSIX parser after checking the canonical escaped arguments.
os.execve('/bin/sh', ['sh', '-c', original], env)
