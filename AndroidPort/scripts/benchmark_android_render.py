"""Measure an installed Android renderer with isolated saves; restore normal app mode."""
from pathlib import Path
import argparse,csv,hashlib,io,json,statistics,subprocess,time
ROOT=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--label',default='android-render')
parser.add_argument('--trials',type=int,default=2,choices=range(1,7))
parser.add_argument('--workers',type=int,default=2,choices=[2,4])
parser.add_argument('--serial',required=True)
parser.add_argument('--extra-x',type=int,default=136)
parser.add_argument('--extra-y',type=int,default=176)
parser.add_argument('--frames',type=int,default=600)
parser.add_argument('--state',type=Path,default=ROOT/'target/pc-performance/reference.state')
parser.add_argument('--input',type=Path)
parser.add_argument('--snapshot-frame',type=int,default=-1)
args=parser.parse_args()
if not args.label or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for c in args.label):
 parser.error('label must contain only letters, digits, hyphens or underscores')
OUT=ROOT/'AndroidPort/target'/args.label;OUT.mkdir(parents=True,exist_ok=True)
worker_trials=[args.workers]*args.trials
extra_x,extra_y=args.extra_x,args.extra_y
frame_count=args.frames;snapshot_frame=args.snapshot_frame
if not 600<=frame_count<=30000:parser.error('frames must be 600..30000')
ADB=['C:/platform-tools/adb.exe','-s',args.serial];PKG='com.fftarecomp.port'
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
def exists(path):
 return subprocess.run(ADB+['shell','run-as',PKG,'test','-f',path],capture_output=True).returncode==0
old={p:(call('exec-out','run-as',PKG,'cat','files/'+p) if exists('files/'+p) else None) for p in ['benchmark.input','benchmark.state']}
for p,b in old.items():
 if b is not None:(OUT/('original-'+p)).write_bytes(b)
state=args.state.read_bytes()
replay=args.input.read_bytes() if args.input else b'gba-input v1\n0 3ff\n60 3df\n180 3ff\n240 3ef\n360 3ff\n420 3df\n540 3ff\n'
summaries=[]
try:
 for trial,workers in enumerate(worker_trials):
  tag=f'{trial+1}-{workers}-workers';print('Starting',tag,flush=True)
  call('shell','am','force-stop',PKG)
  put('files/benchmark.state',state);put('files/benchmark.input',replay)
  call('shell','run-as',PKG,'rm','-f','cache/benchmark.csv','cache/benchmark-hashes.txt')
  args=['shell','am','start','-n',PKG+'/.MainActivity']
  for key,value,kind in [('ffta_benchmark','true','--ez'),('ffta_benchmark_state','true','--ez'),('ffta_benchmark_frames',str(frame_count),'--ei'),('ffta_benchmark_snapshot_frame',str(snapshot_frame),'--ei'),('ffta_benchmark_clock','false','--ez'),('ffta_benchmark_metrics','false','--ez'),('ffta_benchmark_fill','true','--ez'),('ffta_benchmark_profile_ui','true','--ez'),('ffta_benchmark_fast_cpu','true','--ez'),('ffta_benchmark_zoom_workers',str(workers),'--ei'),('ffta_benchmark_extra_x',str(extra_x),'--ei'),('ffta_benchmark_extra_y',str(extra_y),'--ei')]:args += [kind,key,value]
  call(*args);started=time.monotonic();captured=False
  while time.monotonic()-started<240:
   rows=list(csv.DictReader(io.StringIO(call('shell','run-as',PKG,'cat','cache/benchmark.csv',check=False).decode(errors='replace'))))
   if rows and not captured:
    pid=call('shell','pidof',PKG).decode().strip()
    (OUT/(tag+'-threads.txt')).write_bytes(call('shell','ps','-T','-p',pid))
    captured=True
   if rows and int(rows[-1]['frame'])>=frame_count:break
   if time.monotonic()-started>20 and not call('shell','pidof',PKG,check=False).strip():raise RuntimeError('App exited; inspect logcat')
   time.sleep(1)
  else:raise RuntimeError('Benchmark timed out')
  files={p:call('exec-out','run-as',PKG,'cat','cache/'+p) for p in ['benchmark.csv','benchmark-camera.csv','benchmark-hashes.txt']}
  for p,data in files.items():(OUT/(tag+'-'+p)).write_bytes(data)
  if snapshot_frame>=0:(OUT/(tag+'-snapshot.state')).write_bytes(call('exec-out','run-as',PKG,'cat','files/benchmark.state'))
  (OUT/(tag+'-screen.png')).write_bytes(call('exec-out','screencap','-p'))
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
 for p,data in old.items():
  if data is not None:put('files/'+p,data)
  else:call('shell','run-as',PKG,'rm','-f','files/'+p)
 call('shell','rm','-f','/data/local/tmp/ffta-benchmark-transfer',check=False)
 print(call('shell','am','start','-n',PKG+'/.MainActivity').decode(),flush=True)
