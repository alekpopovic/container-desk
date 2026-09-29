"""037 disposable Compose setup/oracle. Never part of the application runtime."""
import json
from pathlib import Path
import subprocess
from ssh_auth import BASE


def setup(root, docker, env):
    directory=root/"remote project's directory"
    directory.mkdir()
    base=directory/"base file's.yml"
    override=directory/'ordered override.yml'
    missing=directory/'missing env override.yml'
    (directory/'required.env').write_text('CHECKPOINT_TOKEN=synthetic-compose-037-private\n')
    service={'image':BASE,'command':['/bin/sh','-c','while true; do sleep 1; done'],'network_mode':'none','read_only':True,'cap_drop':['ALL'],'security_opt':['no-new-privileges'],'pids_limit':32,'mem_limit':'32m','env_file':['required.env'],'environment':{'ORDER_MARKER':'base'}}
    base.write_text(json.dumps({'services':{'web':service,'worker':service}}))
    override.write_text(json.dumps({'services':{'web':{'environment':{'ORDER_MARKER':'override-v1'}}}}))
    missing.write_text(json.dumps({'services':{'web':{'env_file':['absent-required.env']}}}))
    project='cd037_'+root.name.rsplit('-',1)[-1].replace('_','').lower()
    config={'projectName':project,'workingDirectory':str(directory),'configFiles':[str(base),str(override)]}
    prefix=['compose','--ansi','never','--progress','quiet','--parallel','1','--profile','*','--project-directory',str(directory)]
    owned=[]
    # Capture created IDs even on setup failure so the caller can clean exact owned resources.
    try:
        for name in [project,project+'_unrelated']:
            args=docker+prefix+['--project-name',name,'--file',str(base),'--file',str(override)]
            try:
                subprocess.run(args+['up','--detach','--no-build','--pull','never'],cwd=directory,env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,timeout=45)
            finally:
                ids=subprocess.run(args+['ps','--all','--quiet'],cwd=directory,env=env,check=True,capture_output=True,text=True,timeout=10).stdout.split()
                owned.extend(ids)
            assert len(ids)==2
        details=json.loads(subprocess.run(docker+['inspect','--type','container','--',*owned],env=env,check=True,capture_output=True,text=True,timeout=10).stdout)
        manifest={'configuration':config,'missingOverride':str(missing),'override':str(override),'ids':owned[:2],'unrelatedIds':owned[2:],'startedAt':{row['Id']:row['State']['StartedAt'] for row in details},'allowedProjects':[project,project+'_wrong'], 'allowedFiles':[str(base),str(override),str(missing)]}
        (root/'compose-project.json').write_text(json.dumps(manifest))
        return owned,manifest
    except BaseException:
        for ident in owned:subprocess.run(docker+['rm','-f','--',ident],env=env,check=True,stdout=subprocess.DEVNULL,timeout=10)
        raise


def verify(root,docker,env,manifest,has_gui):
    all_ids=manifest['ids']+manifest['unrelatedIds']
    details=json.loads(subprocess.run(docker+['inspect','--type','container','--',*all_ids],env=env,check=True,capture_output=True,text=True,timeout=10).stdout)
    for row in details:
        assert row['State']['Running']
        if row['Id'] in manifest['ids']:assert row['State']['StartedAt']!=manifest['startedAt'][row['Id']]
        else:assert row['State']['StartedAt']==manifest['startedAt'][row['Id']], 'Unrelated project must not be restarted'
    commands=[json.loads(line) for line in (root/'compose-action-commands.jsonl').read_text().splitlines()]
    expected=[['restart','--no-deps','--timeout','1','--','web','worker'],['stop','--timeout','1','--','web','worker'],['start','--','web','worker']]
    if has_gui:expected.insert(0,expected[0])
    assert commands==expected, 'Only explicit GUI/backend actions are admitted; no replay or extra commands' 
    print(f'PASS independent Compose oracle: {len(commands)} explicit lifecycle command(s); both owned services running with newer start times; both unrelated project services retain original start times; no deployment/build/pull/down command admitted by app gate.',flush=True)
