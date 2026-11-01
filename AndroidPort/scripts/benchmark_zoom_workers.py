"""Compare isolated debug replays with two/four zoom workers; restore normal app mode."""
from pathlib import Path
import csv,hashlib,io,json,statistics,subprocess,time
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'AndroidPort/target/note10-workers-warm';OUT.mkdir(parents=True,exist_ok=True)
ADB=['C:/platform-tools/adb.exe','-s','R58M78B8F0M'];PKG='com.fftarecomp.port'
def call(*args,check=True):
 r=subprocess.run(ADB+list(args),capture_output=True)
 if check and r.returncode:raise RuntimeError(r.stderr.decode(errors='replace'))
 return r.stdout
def put(path,data):
 transfer=OUT/'transfer.bin';transfer.write_bytes(data)
 subprocess.run(ADB+['push',str(transfer),'/data/local/tmp/ffta-benchmark-transfer'],capture_output=True,check=True)
 call('shell','run-as',PKG,'cp','/data/local/tmp/ffta-benchmark-transfer',path)
 actual=call('shell','run-as',PKG,'sha256sum',path).decode().split()[0]
 assert actual==hashlib.sha256(data).hexdigest(),'Transfer checksum mismatch'
normal=call('exec-out','run-as',PKG,'cat','files/ffta.sav')
(OUT/'normal-save-backup.sav').write_bytes(normal)
old={p:call('exec-out','run-as',PKG,'cat','files/'+p) for p in ['benchmark.input','benchmark.state']}
for p,b in old.items():(OUT/('original-'+p)).write_bytes(b)
state=(ROOT/'target/pc-performance/reference.state').read_bytes()
replay=b'gba-input v1\n0 3ff\n60 3df\n180 3ff\n240 3ef\n360 3ff\n420 3df\n540 3ff\n'
summaries=[]
try:
 for trial,workers in enumerate([4,2,4]):
  tag=f'{trial+1}-{workers}-workers';print('Starting',tag,flush=True)
  call('shell','am','force-stop',PKG)
  put('files/benchmark.state',state);put('files/benchmark.input',replay)
  call('shell','run-as',PKG,'rm','-f','cache/benchmark.csv','cache/benchmark-hashes.txt')
  args=['shell','am','start','-n',PKG+'/.MainActivity']
  for key,value,kind in [('ffta_benchmark','true','--ez'),('ffta_benchmark_state','true','--ez'),('ffta_benchmark_frames','600','--ei'),('ffta_benchmark_clock','false','--ez'),('ffta_benchmark_metrics','false','--ez'),('ffta_benchmark_fill','true','--ez'),('ffta_benchmark_profile_ui','true','--ez'),('ffta_benchmark_fast_cpu','true','--ez'),('ffta_benchmark_zoom_workers',str(workers),'--ei'),('ffta_benchmark_extra_x','218','--ei'),('ffta_benchmark_extra_y','80','--ei')]:args += [kind,key,value]
  call(*args);started=time.monotonic();captured=False
  while time.monotonic()-started<240:
   rows=list(csv.DictReader(io.StringIO(call('shell','run-as',PKG,'cat','cache/benchmark.csv',check=False).decode(errors='replace'))))
   if rows and not captured:
    pid=call('shell','pidof',PKG).decode().strip()
    (OUT/(tag+'-threads.txt')).write_bytes(call('shell','ps','-T','-p',pid))
    captured=True
   if rows and int(rows[-1]['frame'])>=600:break
   if time.monotonic()-started>20 and not call('shell','pidof',PKG,check=False).strip():raise RuntimeError('App exited; inspect logcat')
   time.sleep(1)
  else:raise RuntimeError('Benchmark timed out')
  files={p:call('exec-out','run-as',PKG,'cat','cache/'+p) for p in ['benchmark.csv','benchmark-camera.csv','benchmark-hashes.txt']}
  for p,data in files.items():(OUT/(tag+'-'+p)).write_bytes(data)
  samples=list(csv.DictReader(io.StringIO(files['benchmark-camera.csv'].decode())))
  samples=[r for r in samples if int(r['frame'])>=120]
  summary={'workers':workers,'trial':trial+1,'samples':len(samples)}
  for field in ['frame_ms','raster_ms','zoom_ms','ui_setup_ms']:
   values=[float(r[field]) for r in samples];summary[field]=statistics.mean(values)
  summary['camera_changes']=sum((a['hofs'],a['vofs'])!=(b['hofs'],b['vofs']) for a,b in zip(samples,samples[1:]))
  summaries.append(summary);print(json.dumps(summary),flush=True)
 (OUT/'results.json').write_text(json.dumps(summaries,indent=2))
 assert call('exec-out','run-as',PKG,'cat','files/ffta.sav')==normal,'Normal SRAM changed'
 print('Normal SRAM unchanged:',hashlib.sha256(normal).hexdigest(),flush=True)
finally:
 call('shell','am','force-stop',PKG,check=False)
 for p,data in old.items():put('files/'+p,data)
 print(call('shell','am','start','-n',PKG+'/.MainActivity').decode(),flush=True)
