package com.fftarecomp.port
import android.app.Activity
import android.content.Context
import java.io.File
/** Debug-only deterministic replay. Never uses the user's normal save file. */
class GameBenchmark private constructor(context:Context,val frames:Int,val paced:Boolean,val view:ViewConfig){
    private val characterMetrics=(context as Activity).intent.getBooleanExtra("ffta_benchmark_metrics",true)
    private val profileUi=(context as Activity).intent.getBooleanExtra("ffta_benchmark_profile_ui",false)
    private val uiTimings=LongArray(7)
    val audioFrames=(context as Activity).intent.getIntExtra("ffta_benchmark_audio_frames",0).coerceIn(0,120)
    var audioDone=false;private set
    private var audioCount=0
    private var audioStart=0L
    private val audioReport=File(context.cacheDir,"benchmark-normal-audio.csv")
    fun beginAudio(){if(audioStart==0L)audioStart=System.nanoTime()}
    fun recordAudio(output:NormalAudioOutput){
        audioCount++
        if(audioCount>=audioFrames){
            val elapsed=System.nanoTime()-audioStart
            audioReport.writeText("batches,speed,elapsed_ms,written_samples,played_pairs,underruns\n"+
                "$audioCount,4,${elapsed/1e6},${output.writtenSamples},${output.playedPairs},${output.underruns}\n")
            audioDone=true
        }
    }
    private val points:List<Pair<Long,Int>>
    private val report=File(context.cacheDir,"benchmark.csv")
    private val checkpoints=File(context.cacheDir,"benchmark-hashes.txt")
    private val camera=File(context.cacheDir,"benchmark-camera.csv").bufferedWriter()
    private val characters=File(context.cacheDir,"benchmark-characters.csv").bufferedWriter()
    private val loopReport=File(context.cacheDir,"benchmark-loop.csv").bufferedWriter().apply { write("frame,publication_ms,audio_ms,measurement_ms,save_ms,diagnostics_ms,total_ms\n") }
    private var loopClosed=false
    fun recordLoop(publication:Long,audio:Long,measurement:Long,save:Long,diagnostics:Long,total:Long){
        if(loopClosed)return
        loopReport.write("${frame-1},${publication/1e6},${audio/1e6},${measurement/1e6},${save/1e6},${diagnostics/1e6},${total/1e6}\n")
        if(frame%120==0)loopReport.flush()
        if(frame==frames){loopReport.close();loopClosed=true}
    }
    private val snapshotFrame=(context as Activity).intent.getIntExtra("ffta_benchmark_snapshot_frame",-1)
    private var frameSnapshot=false
    private var nameSnapshot=false
    private val times=ArrayList<Long>(120)
    private var cursor=0
    var frame=0;private set
    init{
        val lines=File(context.filesDir,"benchmark.input").readLines()
        require(lines.firstOrNull()?.trim()=="gba-input v1"){"Invalid benchmark input"}
        points=lines.drop(1).filter{it.isNotBlank()}.map{
            val fields=it.trim().split(Regex("\\s+"))
            fields[0].toLong() to fields[1].toInt(16)
        }.sortedBy{it.first}
        require(points.isNotEmpty()){"Benchmark input contains no events"}
        characters.write("frame,roster_valid,present,drawn,ids,drawn_ids,duplicates,missing_positions\n")
        camera.write("frame,hofs,vofs,keys,frame_ms,audio_underruns"+
            (if(profileUi)",ui_capture_ms,ui_setup_ms,ui_export_ms,ui_rows,ui_active,raster_ms,zoom_ms" else "")+"\n")
        report.writeText("frame,mean_ms,p95_ms,max_ms\n")
        checkpoints.writeText("Debug replay; isolated benchmark.sav\n")
    }
    fun keys():Int{
        if(characterMetrics && frame==frames-1201)NativeBridge.gbaCharacterMetrics(true)
        while(cursor<points.size && points[cursor].first<=frame)cursor++
        return if(cursor==0)0x3ff else points[cursor-1].second
    }
    fun record(nanos:Long,audioUnderruns:Int){
        if(snapshotFrame<0 && !nameSnapshot && NativeBridge.gbaNameEntry()){NativeBridge.gbaBenchmarkSnapshot(false);nameSnapshot=true}
        if(!frameSnapshot && snapshotFrame>=0 && frame>=snapshotFrame){
            check(NativeBridge.gbaBenchmarkSnapshot(false)>0){NativeBridge.gbaStatus()};frameSnapshot=true
        }
        val position=NativeBridge.gbaCamera()
        if(profileUi)NativeBridge.gbaUiTimings(uiTimings)
        camera.write(listOf(frame,if(position<0)-1 else position and 511,
            if(position<0)-1 else (position ushr 9) and 511,keys(),nanos/1e6,audioUnderruns).joinToString(",")+
            (if(profileUi)",${uiTimings[0]/1e6},${uiTimings[1]/1e6},${uiTimings[2]/1e6},${uiTimings[3]},${uiTimings[4]},${uiTimings[5]/1e6},${uiTimings[6]/1e6}" else "")+"\n")
        if(characterMetrics && frame>=frames-1201)characters.write(NativeBridge.gbaCharacters()+"\n")
        times.add(nanos);frame++
        if(times.size>=120||frame==frames){
            val sorted=times.sorted()
            report.appendText(listOf(frame,times.average()/1e6,sorted[((sorted.size-1)*.95).toInt()]/1e6,sorted.last()/1e6).joinToString(",")+"\n")
            times.clear();camera.flush()
        }
        if(frame==600||frame==frames-1200||frame==frames-600||frame==frames){
            checkpoints.appendText("Replay frame "+frame+"\n"+NativeBridge.gbaDiagnostics())
        }
        if(frame==frames){camera.close();characters.close();NativeBridge.gbaCharacterMetrics(false);checkpoints.appendText("COMPLETE\n")}
    }
    companion object{
        fun enabled(context:Context)=BuildConfig.DEBUG &&
            (context as? Activity)?.intent?.getBooleanExtra("ffta_benchmark",false)==true
        // Debug-only interactive save tests share no normal slots or SRAM.
        fun isolatedSaves(context:Context)=enabled(context) || (BuildConfig.DEBUG &&
            (context as? Activity)?.intent?.getBooleanExtra("ffta_save_sandbox",false)==true)
        fun preferFastCpu(context:Context)=!enabled(context) ||
            (context as? Activity)?.intent?.getBooleanExtra("ffta_benchmark_fast_cpu",false)==true
        fun create(context:Context):GameBenchmark?{
            if(!enabled(context))return null
            val intent=(context as Activity).intent
            if(intent.getBooleanExtra("ffta_benchmark_state",false))check(NativeBridge.gbaBenchmarkSnapshot(true)>0){NativeBridge.gbaStatus()}
            NativeBridge.gbaUiProfile(intent.getBooleanExtra("ffta_benchmark_profile_ui",false),
                intent.getBooleanExtra("ffta_benchmark_reference_ui",false))
            NativeBridge.gbaReferenceFrame(intent.getBooleanExtra("ffta_benchmark_reference_frame",false))
            NativeBridge.gbaReferenceDialogue(intent.getBooleanExtra("ffta_benchmark_reference_dialogue",false))
            NativeBridge.gbaParallelZoom(intent.getBooleanExtra("ffta_benchmark_parallel",true))
            NativeBridge.gbaZoomWorkers(intent.getIntExtra("ffta_benchmark_zoom_workers",0))
            return GameBenchmark(context,intent.getIntExtra("ffta_benchmark_frames",9600).coerceIn(600,30000),
                intent.getBooleanExtra("ffta_benchmark_clock",false),
                ViewConfig(intent.getIntExtra("ffta_benchmark_extra_x",8).coerceIn(0,648),
                    intent.getIntExtra("ffta_benchmark_extra_y",48).coerceIn(0,320),intent.getBooleanExtra("ffta_benchmark_fill",false)))
        }
    }
}
