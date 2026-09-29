"""Synthetic scale data over the disposable strict SSH lab; never a production daemon listing."""
import json
import subprocess
import sys
import time


def handle(args, operation, env):
    if operation == ['ps','--all','--no-trunc','--format','{{json .}}']:
        ids=[args.owned[-1],*args.owned[:-1]]
        for i in range(1000):
            print(json.dumps({'ID':ids[i] if i<len(ids) else format(10000+i,'064x'),'Names':f'pressure-workload-{i:04}','Image':'synthetic/scale','State':'running','Status':'Up (synthetic summary)','Ports':'','Labels':'','CreatedAt':'2026-09-29 00:00:00 +0000 UTC'}))
        return True
    if len(operation)==5 and operation[:4]==['inspect','--type','container','--'] and operation[4] in args.owned:
        with (args.control/'pressure-inspect-reads').open('a') as count: count.write('read\n')
        data=json.loads(subprocess.run(['/usr/bin/docker','--config',args.config,'--host','unix:///var/run/docker.sock',*operation],env=env,check=True,capture_output=True,text=True,timeout=15).stdout)
        data[0]['Config']['Env']=[f'SYNTHETIC_{n:04}='+('x'*480) for n in range(1024)]
        data[0]['Config']['Labels']={f'synthetic-{n:04}':'y'*480 for n in range(1024)}
        output=json.dumps(data)
        (args.control/'pressure-inspect-bytes').write_text(str(len(output.encode())))
        print(output)
        return True
    if operation[:5]==['events','--filter','type=container','--format','{{json .}}']:
        options=operation[5:]
        if options and not (len(options)==2 and options[0]=='--since' and options[1].isdigit()): return False
        start=time.time_ns();counter=0;deadline=time.monotonic()+200
        try:
            while time.monotonic()<deadline:
                for _ in range(500):
                    counter+=1
                    print(json.dumps({'Type':'container','Action':'start','Actor':{'ID':args.owned[-1],'Attributes':{'token':'synthetic-only'}},'timeNano':start+counter}),flush=False)
                sys.stdout.flush();time.sleep(.1)
        except BrokenPipeError: pass
        return True
    return False
