"""Actual release WebKitGTK resource measurements with explicitly synthetic scale sources."""
import base64
import json
import os
from pathlib import Path
import statistics
import subprocess
import time


def process_sample(app_pid):
    processes={}
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit(): continue
        try:
            data={row.split(':',1)[0]:row.split(':',1)[1].strip() for row in (entry/'status').read_text().splitlines() if ':' in row}
            processes[int(entry.name)]=(int(data['PPid']),data['Name'],int(data.get('VmRSS','0 kB').split()[0]),data['State'])
        except (OSError,ValueError,KeyError): continue
    owned={app_pid}
    # ControlPersist masters fork away from the app. Only match paths leased by this exact app PID.
    runtime_paths=[]
    try:
        for fd in Path('/proc',str(app_pid),'fd').iterdir():
            try:
                target=Path(os.readlink(fd))
                if target.name=='lease' and str(target).startswith(f'/tmp/containerdesk-{os.getuid()}/'):
                    runtime_paths.append(str(target.parent)+'/')
            except OSError: pass
    except OSError: pass
    for pid, info in processes.items():
        if info[1]!='ssh': continue
        try:
            arguments=Path('/proc',str(pid),'cmdline').read_bytes().decode(errors='replace')
            if any(path in arguments for path in runtime_paths): owned.add(pid)
        except OSError: pass
    while True:
        more={pid for pid, info in processes.items() if info[0] in owned}
        if more<=owned: break
        owned|=more
    pss=0; missing=0
    for pid in owned:
        try:
            values=[int(line.split()[1]) for line in Path('/proc',str(pid),'smaps_rollup').read_text().splitlines() if line.startswith('Pss:')]
            pss+=sum(values)
        except OSError: missing+=1
    identities=[]
    for pid in owned:
        if pid in processes and processes[pid][1]=='ssh':
            try: identities.append((pid,Path('/proc',str(pid),'stat').read_text().rsplit(') ',1)[1].split()[19]))
            except OSError: pass
    return {'sshIdentities':identities,'rssKiB':sum(processes[pid][2] for pid in owned if pid in processes),'pssKiB':pss,'pssUnavailable':missing,'processCount':len(owned),'sshChildren':sum(processes[pid][1]=='ssh' for pid in owned if pid in processes),'zombies':sum(processes[pid][3].startswith('Z') for pid in owned if pid in processes)}


def verify(root,artifacts,script,command,click,button,fill,wait,live_id,window,xdotool,env):
    app_pid=int(subprocess.run([str(xdotool),'getwindowpid',window],env=env,check=True,capture_output=True,text=True).stdout)
    def search(query):
        subprocess.run([str(xdotool),'key','--clearmodifiers','ctrl+f'],env=env,check=True,timeout=10)
        wait('return document.activeElement?.matches("[data-shortcut=search]")')
        subprocess.run([str(xdotool),'key','--clearmodifiers','ctrl+a','BackSpace'],env=env,check=True,timeout=10)
        wait('return document.querySelector("[data-shortcut=search]")?.value===""')
        subprocess.run([str(xdotool),'type','--clearmodifiers','--delay','25','--',query],env=env,check=True,timeout=10)
    baseline=process_sample(app_pid)
    baseline.pop('sshIdentities')
    observed_ssh=set()
    assert baseline['sshChildren']==0
    button('Connect saved host')
    wait('return document.body.innerText.includes("Ready · SSH session")')
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Containers"]')
    wait('return document.querySelector(".container-table")?.getAttribute("aria-rowcount")==="1001"')
    initial_rows=script('return document.querySelectorAll("[data-container-id]").length')
    assert initial_rows<=24
    latencies=[]
    for query in ['pressure-workload-0999','pressure-workload-0500','pressure-workload-0000']:
        begin=time.monotonic();search(query)
        wait('return document.querySelectorAll("[data-container-id]").length===1')
        latencies.append((time.monotonic()-begin)*1000)
    assert max(latencies)<1500, 'Native filter exceeded the measured responsiveness gate'
    begin=time.monotonic();click(f'//tr[@data-container-id="{live_id}"]//button')
    click('//button[@role="tab" and normalize-space(.)="Environment"]')
    wait('return Array.from(document.querySelectorAll("dt")).filter(e=>e.textContent.startsWith("SYNTHETIC_")).length===1024')
    inspect_ms=(time.monotonic()-begin)*1000
    time.sleep(3)
    assert script('return document.querySelector(".inspect-detail [role=tab][aria-selected=true]")?.textContent==="Environment"'), 'Inventory events reset the selected inspect tab'
    assert not script('return document.body.innerText.includes("xxxxxxxxxxxxxxxxxxxx") || document.body.innerText.includes("yyyyyyyyyyyyyyyyyyyy")')
    (artifacts/'native-large-inspect.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    click('//button[@role="tab" and normalize-space(.)="Overview"]')
    button('Start logs')
    wait('return document.querySelector(".live-logs")?.innerText.includes("Following")')
    wait('return document.querySelector(".live-logs")?.innerText.includes("1.00 MiB")')
    script('window.__pressureFrames=[];window.__pressureLast=performance.now();window.__pressureActive=true;function frame(t){if(!window.__pressureActive)return;window.__pressureFrames.push(t-window.__pressureLast);if(window.__pressureFrames.length>10000)window.__pressureFrames.shift();window.__pressureLast=t;requestAnimationFrame(frame)}requestAnimationFrame(frame)')
    samples=[];begin=time.monotonic()
    for _ in range(60):
        time.sleep(2)
        sample=process_sample(app_pid);observed_ssh.update(sample.pop('sshIdentities'));sample['seconds']=time.monotonic()-begin
        metrics=script('const text=document.querySelector(".live-logs")?.innerText??"";const m=text.match(/Retained: ([0-9]+) lines · ([0-9]+) bytes/);return {lines:m?Number(m[1]):null,bytes:m?Number(m[2]):null,logDomRows:document.querySelectorAll("[data-log-key]").length,containerDomRows:document.querySelectorAll("[data-container-id]").length,gap:text.includes("by view retention"),eventStatus:document.querySelector(".event-status")?.textContent??""}')
        assert metrics['lines'] is not None and metrics['lines']<=8000 and metrics['bytes']<=1024*1024
        assert metrics['logDomRows']<=32 and metrics['containerDomRows']<=24
        sample.update(metrics);samples.append(sample)
        if len(samples)%10==0:
            (artifacts/"native-pressure-samples.json").write_text(json.dumps(samples,indent=2)+"\n")
            print(f'Pressure sample {len(samples)}/60: RSS {sample["rssKiB"]/1024:.1f} MiB; owned SSH {sample["sshChildren"]}; retained {sample["lines"]} lines.',flush=True)
    frames=script('window.__pressureActive=false;return window.__pressureFrames.sort((a,b)=>a-b)')
    def percentile(fraction):return frames[min(len(frames)-1,int(len(frames)*fraction))] if frames else None
    (artifacts/'native-sustained-pressure.png').write_bytes(base64.b64decode(command('GET','/screenshot'),validate=True))
    # Refresh/filter while both source streams remain active; never replay input or mutations.
    begin=time.monotonic();search('pressure-workload-0999')
    wait('return document.querySelectorAll("[data-container-id]").length===1 && document.querySelector("[data-container-id]")?.textContent.includes("pressure-workload-0999")')
    stressed_filter=(time.monotonic()-begin)*1000
    click('//nav[@aria-label="Resources"]//a[normalize-space(.)="Hosts"]')
    button('Disconnect saved host')
    wait('return document.body.innerText.includes("Disconnected · SSH session")')
    deadline=time.monotonic()+10
    while True:
        after=process_sample(app_pid)
        after.pop('sshIdentities')
        surviving=[]
        for pid,start in observed_ssh:
            try:
                if Path('/proc',str(pid),'stat').read_text().rsplit(') ',1)[1].split()[19]==start: surviving.append(pid)
            except OSError: pass
        if after['sshChildren']==baseline['sshChildren'] and after['zombies']==0 and not surviving: break
        assert time.monotonic()<deadline,'Owned SSH children did not return to baseline'
        time.sleep(.2)
    rss=[s['rssKiB']/1024 for s in samples]
    growth=statistics.median(rss[-15:])-statistics.median(rss[15:30])
    pss_missing=max(s['pssUnavailable'] for s in samples)
    hardware={'cpuModel':next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')),'unknown'),'cpuAffinity':len(os.sched_getaffinity(0)),'memoryKiB':int(Path('/proc/meminfo').read_text().splitlines()[0].split()[1])}
    result={'workload':'1000 synthetic summaries; 1024 masked env + 1024 masked labels over actual strict SSH; actual Docker log producer ~50000 lines/s; synthetic event producer ~5000 events/s','hardware':hardware,'configuredLimits':{'logLines':8000,'logBytes':1024*1024,'statsHistory':60,'activeHosts':1,'concurrentJobs':4},'baseline':baseline,'initialRenderedContainerRows':initial_rows,'filterRoundtripMs':latencies,'largeInspectRoundtripMs':inspect_ms,'largeInspectInputBytes':int((root/'pressure-inspect-bytes').read_text()),'largeInspectReads':len((root/'pressure-inspect-reads').read_text().splitlines()),'observedSshIdentitiesReaped':len(observed_ssh),'stressedFilterRoundtripMs':stressed_filter,'frameIntervalMs':{'p50':percentile(.5),'p95':percentile(.95),'p99':percentile(.99),'max':max(frames) if frames else None},'samples':samples,'afterDisconnect':after,'plateau':{'metric':'sum app/descendant/leased-master RSS (shared pages counted more than once)','medianGrowthMiBLast30sVs30to60s':growth,'maxRssMiB':max(rss),'pssUnavailableMax':pss_missing,'acceptanceGrowthMiB':32}}
    (artifacts/'native-pressure.json').write_text(json.dumps(result,indent=2)+'\n')
    assert growth<=32, f'Sustained RSS did not plateau: median growth {growth:.2f} MiB'
    assert stressed_filter<1500
    assert any(s['gap'] for s in samples)
    print(f'PASS native pressure: 1000 summaries, {result["largeInspectInputBytes"]} inspect bytes; 120s stream retention; RSS median growth {growth:.2f} MiB; stressed filter {stressed_filter:.1f} ms; SSH children {after["sshChildren"]} after cancellation.',flush=True)
