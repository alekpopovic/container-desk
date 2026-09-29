import os,subprocess,select
read,write=os.pipe()
with open('/tmp/containerdesk-028-xvfb.log','w') as log:
 server=subprocess.Popen(['/tmp/containerdesk-025-tools/xvfb/usr/bin/Xvfb','-displayfd',str(write),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write,),stdout=log,stderr=log)
 os.close(write)
 try:
  assert select.select([read],[],[],10)[0], 'Xvfb display startup timed out'
  display=os.read(read,32).decode().strip()
  assert display.isdigit(), 'No Xvfb display number'
  env={**os.environ,'DISPLAY':':'+display,'GDK_BACKEND':'x11','LIBGL_ALWAYS_SOFTWARE':'1'}
  env.pop('WAYLAND_DISPLAY',None)
  result=subprocess.run(['python3','tests/lab/logs.py','--sshd-root','/tmp/containerdesk-025-tools/openssh','--stream','--stats','--native-driver','/tmp/containerdesk-025-tools/bin/tauri-driver','--webkit-driver','/tmp/containerdesk-025-tools/webkit/usr/bin/WebKitWebDriver','--native-artifacts','docs/verification/028-native','--focus-xdotool','/tmp/containerdesk-025-tools/xdotool/usr/bin/xdotool'],env=env)
  raise SystemExit(result.returncode)
 finally:
  os.close(read)
  server.terminate()
  server.wait(timeout=10)
