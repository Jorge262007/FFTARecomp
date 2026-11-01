package com.fftarecomp.port

import android.opengl.GLES20.*
import android.opengl.EGL14
import java.nio.ByteBuffer
import java.nio.ByteOrder

/** One display pass for world and UI. No CPU-sized copies or per-pixel Kotlin work. */
internal class VideoPass {
    private var program=0
    private val textures=IntArray(2)
    private val fbos=IntArray(2)
    private var palette=0
    private var width=0
    private var height=0
    private var current=0
    private var history=false
    private var enabled=false
    private var options=VideoOptions()
    private var paletteOptions:VideoOptions?=null
    private val paletteBytes=ByteBuffer.allocateDirect(32768*4).order(ByteOrder.nativeOrder())
    private val vertices=ByteBuffer.allocateDirect(32).order(ByteOrder.nativeOrder()).asFloatBuffer().apply{
        put(floatArrayOf(-1f,-1f,1f,-1f,-1f,1f,1f,1f));position(0)
    }
    fun reset(){program=0;palette=0;width=0;height=0;history=false;paletteOptions=null}
    private fun texture(id:Int){
        glBindTexture(GL_TEXTURE_2D,id)
        glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MIN_FILTER,GL_NEAREST)
        glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MAG_FILTER,GL_NEAREST)
        glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_S,GL_CLAMP_TO_EDGE)
        glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_T,GL_CLAMP_TO_EDGE)
    }
    private fun shader(type:Int,text:String):Int{
        val id=glCreateShader(type);glShaderSource(id,text);glCompileShader(id)
        val ok=IntArray(1);glGetShaderiv(id,GL_COMPILE_STATUS,ok,0)
        check(ok[0]!=0){glGetShaderInfoLog(id)};return id
    }
    private fun create(){
        program=glCreateProgram()
        val v=shader(GL_VERTEX_SHADER,"attribute vec2 position; varying vec2 uv;void main(){gl_Position=vec4(position,0.,1.);uv=position*.5+.5;}")
        val f=shader(GL_FRAGMENT_SHADER,FRAGMENT)
        glAttachShader(program,v);glAttachShader(program,f);glLinkProgram(program)
        val ok=IntArray(1);glGetProgramiv(program,GL_LINK_STATUS,ok,0)
        check(ok[0]!=0){glGetProgramInfoLog(program)};glDeleteShader(v);glDeleteShader(f)
        glGenTextures(2,textures,0);glGenFramebuffers(2,fbos,0)
        val ids=IntArray(1);glGenTextures(1,ids,0);palette=ids[0]
    }
    fun begin(w:Int,h:Int,value:VideoOptions){
        options=value
        enabled=value.screen!=0 || value.brightness!=1f || value.response!=0 || value.grid!=0 || value.scaling!=1
        if(w<=0 || h<=0)return
        if(program==0)create()
        EGL14.eglSwapInterval(EGL14.eglGetCurrentDisplay(),if(value.vsync)1 else 0)
        if(!enabled){glBindFramebuffer(GL_FRAMEBUFFER,0);history=false;return}
        if(width!=w || height!=h){
            width=w;height=h;history=false
            for(i in 0..1){
                texture(textures[i]);glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA,w,h,0,GL_RGBA,GL_UNSIGNED_BYTE,null)
                glBindFramebuffer(GL_FRAMEBUFFER,fbos[i]);glFramebufferTexture2D(GL_FRAMEBUFFER,GL_COLOR_ATTACHMENT0,GL_TEXTURE_2D,textures[i],0)
                check(glCheckFramebufferStatus(GL_FRAMEBUFFER)==GL_FRAMEBUFFER_COMPLETE){"Display framebuffer is incomplete"}
            }
        }
        val old=paletteOptions
        if(old==null || old.screen!=value.screen || old.brightness!=value.brightness || old.darken!=value.darken){
            paletteBytes.position(0)
            check(NativeBridge.gbaVideoPalette(paletteBytes,value.screen,value.darken,value.brightness)>0){"Display palette failed"}
            texture(palette);paletteBytes.position(0)
            glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA,128,256,0,GL_RGBA,GL_UNSIGNED_BYTE,paletteBytes)
            paletteOptions=value;history=false
        }
        current=1-current
        glActiveTexture(GL_TEXTURE0)
        glBindFramebuffer(GL_FRAMEBUFFER,fbos[current])
    }
    fun finish(w:Int,h:Int,sourceWidth:Int,sourceHeight:Int){
        if(!enabled)return
        glBindFramebuffer(GL_FRAMEBUFFER,0);glViewport(0,0,w,h)
        glUseProgram(program)
        glActiveTexture(GL_TEXTURE0);glBindTexture(GL_TEXTURE_2D,textures[current])
        glActiveTexture(GL_TEXTURE1);glBindTexture(GL_TEXTURE_2D,textures[if(history)1-current else current])
        glActiveTexture(GL_TEXTURE2);glBindTexture(GL_TEXTURE_2D,palette)
        glUniform1i(glGetUniformLocation(program,"image"),0)
        glUniform1i(glGetUniformLocation(program,"previous"),1)
        glUniform1i(glGetUniformLocation(program,"palette"),2)
        glUniform2f(glGetUniformLocation(program,"screen"),w.toFloat(),h.toFloat())
        glUniform2f(glGetUniformLocation(program,"source"),sourceWidth.toFloat(),sourceHeight.toFloat())
        glUniform1f(glGetUniformLocation(program,"response"),options.response.toFloat())
        val grid=when(options.grid){1->if(options.screen==0)0f else .08f;2->.10f;3->.23f;else->0f}
        glUniform1f(glGetUniformLocation(program,"grid"),grid)
        glUniform1f(glGetUniformLocation(program,"scaler"),options.scaling.toFloat())
        val pos=glGetAttribLocation(program,"position");glEnableVertexAttribArray(pos);vertices.position(0)
        glVertexAttribPointer(pos,2,GL_FLOAT,false,0,vertices);glDrawArrays(GL_TRIANGLE_STRIP,0,4)
        glActiveTexture(GL_TEXTURE0);history=true
    }
    companion object {
        private val FRAGMENT="""
            precision highp float;
            varying vec2 uv;uniform sampler2D image;uniform sampler2D previous;uniform sampler2D palette;
            uniform vec2 screen;uniform vec2 source;uniform float response;uniform float grid;uniform float scaler;
            vec3 sampleColor(vec2 p){return texture2D(image,p).rgb;}
            void main(){
                vec2 delta=1./source;vec3 color=sampleColor(uv);
                if(scaler==0. || scaler==2.){
                    vec2 f=fract(uv*source);
                    vec2 o=(step(vec2(.5),f)-.5)/screen;
                    vec3 filtered=(sampleColor(uv+vec2(o.x,0.))+sampleColor(uv+vec2(0.,o.y))+sampleColor(uv+o))/3.;
                    color=mix(color,filtered,scaler==2.?.18:.5);
                }else if(scaler==3.){
                    vec3 b=sampleColor(uv-vec2(0.,delta.y));vec3 d=sampleColor(uv-vec2(delta.x,0.));
                    vec3 f=sampleColor(uv+vec2(delta.x,0.));vec3 h=sampleColor(uv+vec2(0.,delta.y));
                    vec2 q=fract(uv*source);
                    if(distance(b,h)>.01 && distance(d,f)>.01){
                        vec3 a=q.y<.5?b:h;vec3 c=q.x<.5?d:f;
                        if(distance(a,c)<.01)color=a;
                    }
                }
                if(response>0.){
                    vec3 old=texture2D(previous,uv).rgb;
                    float weight=response==1.?.25:(response==2.?(distance(color,old)<.22?.18:0.):.42);
                    color=mix(color,old,weight);
                }
                vec3 raw=floor(clamp(color,0.,1.)*31.+.5);
                float code=raw.r+raw.g*32.+raw.b*1024.;
                vec2 at=vec2((mod(code,128.)+.5)/128.,(floor(code/128.)+.5)/256.);
                color=texture2D(palette,at).rgb;
                vec2 cell=fract(uv*source);
                float edge=1.-step(.13,min(cell.x,cell.y));
                color*=1.-edge*grid;
                gl_FragColor=vec4(color,1.);
            }
        """.trimIndent()
    }
}
