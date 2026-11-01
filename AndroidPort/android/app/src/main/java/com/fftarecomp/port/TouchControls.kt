package com.fftarecomp.port

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import kotlin.math.roundToInt

private val TouchInk=Color(0xFF302519)
private val TouchPaper=Color(0xFFF6EBD0)
private val TouchGold=Color(0xFFB68B35)

private fun dpadPetalPoints(side:Float):List<Offset>{
    val points=mutableListOf<Offset>()
    fun point(x:Float,y:Float)=Offset(x*side,y*side)
    fun curve(start:Offset,c1:Offset,c2:Offset,end:Offset){
        for(step in 1..24){
            val t=step/24f;val u=1f-t
            points+=Offset(u*u*u*start.x+3*u*u*t*c1.x+3*u*t*t*c2.x+t*t*t*end.x,
                u*u*u*start.y+3*u*u*t*c1.y+3*u*t*t*c2.y+t*t*t*end.y)
        }
    }
    val tip=point(.50f,.45f);points+=tip
    curve(tip,point(.43f,.40f),point(.37f,.35f),point(.365f,.27f))
    points+=point(.365f,.055f)
    curve(points.last(),point(.365f,.025f),point(.385f,0f),point(.415f,0f))
    points+=point(.585f,0f)
    curve(points.last(),point(.615f,0f),point(.635f,.025f),point(.635f,.055f))
    points+=point(.635f,.27f)
    curve(points.last(),point(.63f,.35f),point(.57f,.40f),tip)
    return points
}

private fun dpadPetalPath(side:Float)=Path().apply{
    val points=dpadPetalPoints(side)
    moveTo(points.first().x,points.first().y)
    points.drop(1).forEach{lineTo(it.x,it.y)}
    close()
}

private fun dpadArrowPath(side:Float)=Path().apply{
    moveTo(.50f*side,.12f*side)
    lineTo(.415f*side,.235f*side)
    lineTo(.585f*side,.235f*side)
    close()
}

private fun polygonContains(points:List<Offset>,point:Offset):Boolean{
    var inside=false;var previous=points.last()
    for(current in points){
        if((current.y>point.y)!=(previous.y>point.y) &&
            point.x<(previous.x-current.x)*(point.y-current.y)/(previous.y-current.y)+current.x)inside=!inside
        previous=current
    }
    return inside
}

private fun dpadDirectionAt(point:Offset,width:Float,height:Float):Int{
    val side=minOf(width,height)
    if(side<=0f)return 0
    val originX=(width-side)/2f;val originY=(height-side)/2f
    val local=Offset(point.x-originX,point.y-originY)
    val points=dpadPetalPoints(side)
    for(index in 0..3){
        val dx=local.x-side/2f;val dy=local.y-side/2f
        val radians=Math.toRadians((-index*90).toDouble())
        val rotated=Offset(side/2f+(dx*kotlin.math.cos(radians)-dy*kotlin.math.sin(radians)).toFloat(),
            side/2f+(dx*kotlin.math.sin(radians)+dy*kotlin.math.cos(radians)).toFloat())
        if(polygonContains(points,rotated))return when(index){0->64;1->16;2->128;else->32}
    }
    return 0
}

@Composable
fun TouchscreenSettings(settings:GameSettings,onEdit:()->Unit){
    OutlinedButton(onClick=onEdit,modifier=Modifier.fillMaxWidth()){Text("Edit layout")}
    Row(verticalAlignment=Alignment.CenterVertically){
        Switch(settings.controls,{settings.controls=it;settings.save()});Text("Touch controls")
    }
    Row(verticalAlignment=Alignment.CenterVertically){
        Switch(settings.touch.vibration,{settings.touch.vibration=it;settings.touch.save()})
        Text("Vibration feedback")
    }
    Row(verticalAlignment=Alignment.CenterVertically){
        Switch(settings.touch.experimentalClicks,{settings.touch.experimentalClicks=it;settings.touch.save()})
        Text("Touch selection")
    }
    Text("Opacity")
    Slider(settings.opacity,{settings.opacity=it},valueRange=0.2f..1f,onValueChangeFinished={settings.save()})
    Text("Size")
    Slider(settings.controlSize,{settings.controlSize=it},valueRange=0.75f..1.25f,onValueChangeFinished={settings.save()})
    Text("Move controls and choose which buttons appear in Edit layout. Android Back always opens Pause.",fontSize=12.sp)
}

@Composable
fun TouchOverlay(settings:GameSettings,input:GameInput,editing:Boolean,onPause:()->Unit,
    onSave:()->Unit={},onCancel:()->Unit={}){
    val density=LocalDensity.current
    val view=LocalView.current
    val haptic by rememberUpdatedState(settings.touch.vibration)
    val feedback:()->Unit={if(haptic)view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY);Unit}
    var selected by remember{mutableStateOf<TouchControl?>(null)}
    var showButtons by remember{mutableStateOf(false)}
    var panelY by remember{mutableFloatStateOf(0f)}
    Box(Modifier.fillMaxSize()){
        if(editing)Box(Modifier.fillMaxSize().background(Color(0x4014231C)))
        BoxWithConstraints(Modifier.fillMaxSize().safeDrawingPadding().padding(8.dp)){
            val canvasWidth=with(density){maxWidth.toPx()}
            val canvasHeight=with(density){maxHeight.toPx()}
            val rowWidths=TouchGeometry.bottomRow.map{it.width*settings.touch.layout.positions.getValue(it).scale*density.density}
            val rowAnchor=settings.touch.layout.positions.getValue(TouchControl.SELECT)
            val rowAnchorHeight=TouchControl.SELECT.height*rowAnchor.scale*density.density
            val rowCenterY=TouchGeometry.rowCenterY(canvasHeight,rowAnchorHeight,rowAnchor.y)
            TouchControl.entries.forEach{control->
                val pose=settings.touch.layout.positions.getValue(control)
                val utility=control in listOf(TouchControl.PAUSE,TouchControl.FAST_TOGGLE,TouchControl.ZOOM_IN,TouchControl.ZOOM_OUT)
                if(pose.visible&&(editing||utility||settings.controls)){
                    val scale=pose.scale*(if(control==TouchControl.A||control==TouchControl.B||control==TouchControl.DPAD)settings.controlSize else 1f)
                    val w=(control.width*scale).dp;val h=(control.height*scale).dp
                    val availableX=(canvasWidth-with(density){w.toPx()}).coerceAtLeast(0f)
                    val availableY=(canvasHeight-with(density){h.toPx()}).coerceAtLeast(0f)
                    val rowIndex=TouchGeometry.bottomRow.indexOf(control)
                    val alignedX=if(pose.centeredRow && rowIndex>=0)TouchGeometry.rowLeft(rowIndex,canvasWidth,rowWidths,12*density.density)else pose.x*availableX
                    val draggable=if(editing)Modifier.pointerInput(control,availableX,availableY){
                        detectDragGestures(onDragStart={selected=control},onDrag={change,amount->
                            change.consume()
                            if(rowIndex>=0&&pose.centeredRow){
                                val movedCenter=(rowCenterY+amount.y).coerceIn(0f,canvasHeight)
                                TouchGeometry.bottomRow.forEach{rowControl->
                                    val rowPose=settings.touch.layout.positions.getValue(rowControl)
                                    val rowHeight=rowControl.height*rowPose.scale*density.density
                                    val rowAvailable=(canvasHeight-rowHeight).coerceAtLeast(0f)
                                    val top=TouchGeometry.rowItemTop(canvasHeight,rowHeight,movedCenter)
                                    settings.touch.layout.positions[rowControl]=rowPose.copy(
                                        y=if(rowAvailable>0f)top/rowAvailable else 0f,centeredRow=true)
                                }
                            }else{
                                val current=settings.touch.layout.positions.getValue(control)
                                settings.touch.layout.positions[control]=current.move(amount.x,amount.y,availableX,availableY)
                            }
                        })
                    }else Modifier
                    val alignedY=if(rowIndex>=0&&pose.centeredRow){
                        val itemHeight=with(density){h.toPx()}
                        TouchGeometry.rowItemTop(canvasHeight,itemHeight,rowCenterY)
                    }else pose.y*availableY
                    val modifier=Modifier.offset{IntOffset(alignedX.roundToInt(),alignedY.roundToInt())}
                        .size(w,h).then(draggable)
                        .then(if(editing)Modifier.pointerInput(control){detectTapGestures{selected=control}}else Modifier)
                        .then(if(editing&&selected==control)Modifier.border(2.dp,TouchGold,RoundedCornerShape(12.dp))else Modifier)
                        .semantics{contentDescription=if(editing)"Move "+control.label else control.label}
                    val label=when(control){
                        TouchControl.FAST_TOGGLE->if(settings.fastForward)speedLabel(settings.multiplier)+" ON" else ">>"
                        TouchControl.FAST_HOLD->"Hold >>"
                        else->control.label
                    }
                    if(control==TouchControl.DPAD){
                        TouchDirectionPad(modifier,settings,input,editing,feedback)
                    }else{
                        val onPressed:(Boolean)->Unit={down->
                            when{
                                control.mask!=0->input.set("touch:"+control.name,if(down)control.mask else 0)
                                control==TouchControl.FAST_HOLD->settings.holdFast=down
                                down->when(control){
                                    TouchControl.PAUSE->onPause()
                                    TouchControl.FAST_TOGGLE->{
                                        if(!settings.fastForward)settings.fastForward=true
                                        else {
                                            val next=FAST_SPEEDS.indexOf(settings.multiplier)+1
                                            if(next in FAST_SPEEDS.indices)settings.multiplier=FAST_SPEEDS[next]
                                            else {settings.fastForward=false;settings.multiplier=FAST_SPEEDS.first()}
                                        }
                                        settings.save()
                                    }
                                    TouchControl.ZOOM_IN->settings.zoomBy(-1)
                                    TouchControl.ZOOM_OUT->settings.zoomBy(1)
                                    else->Unit
                                }
                            }
                        }
                        DisposableEffect(control,editing){
                            onDispose{
                                if(control.mask!=0)input.set("touch:"+control.name,0)
                                if(control==TouchControl.FAST_HOLD)settings.holdFast=false
                            }
                        }
                        TouchKey(label,modifier,settings.opacity,editing,feedback,onPressed,
                            control==TouchControl.ZOOM_IN||control==TouchControl.ZOOM_OUT,
                            control==TouchControl.FAST_TOGGLE&&settings.fastForward)
                    }
                }
            }
            if(editing){
                Column(Modifier.align(Alignment.Center).offset{IntOffset(0,panelY.roundToInt())}.widthIn(max=440.dp)
                    .background(TouchPaper,RoundedCornerShape(12.dp)).border(1.dp,TouchGold,RoundedCornerShape(12.dp))
                    .padding(8.dp),horizontalAlignment=Alignment.CenterHorizontally){
                    Text("Edit touchscreen",Modifier.pointerInput(canvasHeight){
                        detectDragGestures{change,amount->
                            change.consume()
                            val limit=(canvasHeight/2-with(density){64.dp.toPx()}).coerceAtLeast(0f)
                            panelY=(panelY+amount.y).coerceIn(-limit,limit)
                        }
                    },color=TouchInk,fontWeight=FontWeight.Bold,fontSize=16.sp)
                    Text("Paused - drag controls or this title",color=TouchInk,fontSize=11.sp)
                    if(selected!=null){
                        val control=selected!!
                        val pose=settings.touch.layout.positions.getValue(control)
                        Text(control.label+" size: "+(pose.scale*100).roundToInt()+"%",color=TouchInk,fontSize=12.sp)
                        Row(Modifier.widthIn(max=280.dp),verticalAlignment=Alignment.CenterVertically){
                            Slider(pose.scale,{settings.touch.layout.positions[control]=pose.copy(scale=it).bounded()},
                                valueRange=.5f..2f,modifier=Modifier.weight(1f))
                            TextButton(onClick={settings.touch.layout.positions[control]=pose.copy(scale=1f)}){Text("100%")}
                        }
                    }else Text("Tap a button to change its size",color=TouchInk,fontSize=11.sp)
                    Row(horizontalArrangement=Arrangement.spacedBy(4.dp)){
                        TextButton(onClick={showButtons=true}){Text("Buttons")}
                        TextButton(onClick={settings.touch.layout.reset()}){Text("Default")}
                        TextButton(onClick=onCancel){Text("Cancel")}
                        Button(onClick=onSave,contentPadding=PaddingValues(horizontal=12.dp)){Text("Save")}
                    }
                }
            }
        }
        if(showButtons){
            Dialog(onDismissRequest={showButtons=false}){
                Column(Modifier.heightIn(max=320.dp).background(TouchPaper,RoundedCornerShape(16.dp))
                    .padding(16.dp)){
                    Text("Buttons",fontSize=21.sp,color=TouchInk)
                    Text("Tap a name to change its size",fontSize=12.sp,color=TouchInk)
                    Column(Modifier.weight(1f,fill=false).verticalScroll(rememberScrollState())){
                    TouchControl.entries.forEach{control->
                        val pose=settings.touch.layout.positions.getValue(control)
                        Row(verticalAlignment=Alignment.CenterVertically){
                            Checkbox(pose.visible,{settings.touch.layout.positions[control]=pose.copy(visible=it)})
                            TextButton(onClick={selected=control;showButtons=false}){Text(control.label,color=TouchInk)}
                        }
                    }
                    Row(verticalAlignment=Alignment.CenterVertically){
                        Switch(settings.touch.vibration,{settings.touch.vibration=it})
                        Text("Vibration feedback",color=TouchInk)
                    }
                    }
                    Button(onClick={showButtons=false},modifier=Modifier.fillMaxWidth()){Text("Done")}
                }
            }
        }
    }
}

@Composable
private fun TouchKey(label:String,modifier:Modifier,opacity:Float,editing:Boolean,
    feedback:()->Unit,onPressed:(Boolean)->Unit,square:Boolean=false,active:Boolean=false){
    val press by rememberUpdatedState(onPressed)
    val buzz by rememberUpdatedState(feedback)
    var pressed by remember{mutableStateOf(false)}
    val interaction=if(editing)Modifier else Modifier.pointerInput(Unit){
        awaitEachGesture{
            awaitFirstDown(requireUnconsumed=false)
            pressed=true;buzz();press(true)
            try{waitForUpOrCancellation()}finally{pressed=false;press(false)}
        }
    }
    val shape=RoundedCornerShape(if(square)10.dp else 18.dp)
    val fill=when{pressed&&active->TouchPaper;pressed->TouchGold;active->TouchGold;else->TouchInk}
    val ink=if(pressed||active)TouchInk else TouchPaper
    Box(modifier.background(fill.copy(alpha=opacity),shape)
        .border(1.dp,(if(pressed)TouchPaper else TouchPaper.copy(alpha=.6f)),shape).then(interaction),
        contentAlignment=Alignment.Center){
        Text(label,color=ink,fontSize=if(label=="A"||label=="B")22.sp else 12.sp,fontWeight=FontWeight.Bold)
    }
}
@Composable
private fun TouchDirectionPad(modifier:Modifier,settings:GameSettings,input:GameInput,editing:Boolean,feedback:()->Unit){
    val buzz by rememberUpdatedState(feedback)
    var pressedMask by remember{mutableIntStateOf(0)}
    DisposableEffect(input,editing){onDispose{input.set("dpad",0)}}
    val interaction=if(editing)Modifier else Modifier.pointerInput(Unit){
        var previous=0
        try{
            awaitPointerEventScope{
                while(true){
                    val event=awaitPointerEvent()
                    val mask=event.changes.filter{it.pressed}.fold(0){pressed,change->
                        pressed or dpadDirectionAt(change.position,size.width.toFloat(),size.height.toFloat())
                    }
                    if(mask!=0&&mask!=previous)buzz()
                    previous=mask;pressedMask=mask;input.set("dpad",mask)
                    event.changes.forEach{it.consume()}
                }
            }
        }finally{pressedMask=0;input.set("dpad",0)}
    }
    Box(modifier.then(interaction)){
        Canvas(Modifier.fillMaxSize()){
            val side=minOf(size.width,size.height)
            val origin=Offset((size.width-side)/2f,(size.height-side)/2f)
            val pad=dpadPetalPath(side)
            val arrow=dpadArrowPath(side)
            for(index in 0..3){
                val active=when(index){0->pressedMask and 64!=0;1->pressedMask and 16!=0;2->pressedMask and 128!=0;else->pressedMask and 32!=0}
                translate(origin.x,origin.y){
                    rotate(index*90f,Offset(side/2f,side/2f)){
                        drawPath(pad,if(active)TouchGold else TouchInk,alpha=settings.opacity)
                        drawPath(pad,if(active)TouchPaper.copy(alpha=settings.opacity) else TouchPaper.copy(alpha=.7f*settings.opacity),style=androidx.compose.ui.graphics.drawscope.Stroke(width=1.dp.toPx()))
                        drawPath(arrow,Color.White.copy(alpha=settings.opacity))
                    }
                }
            }
        }
    }
}
