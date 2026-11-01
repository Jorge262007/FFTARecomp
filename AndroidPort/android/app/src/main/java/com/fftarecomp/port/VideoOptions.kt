package com.fftarecomp.port

/** Display-only settings; never affect guest state or recompilation. */
data class VideoOptions(
    val screen:Int=0,val brightness:Float=1f,val darken:Float=-1f,
    val response:Int=0,val grid:Int=0,val scaling:Int=1,val vsync:Boolean=true,
    val fullscreen:Boolean=true
)
