package com.fftarecomp.port

import org.junit.Assert.*
import org.junit.Test

class TouchSelectionTest {
    @Test fun letterboxBarsAreNotGameClicks(){
        val view=ViewConfig(0,0)
        assertNull(TouchSelection.map(10f,10f,720,720,view))
        val middle=TouchSelection.map(360f,360f,720,720,view)!!
        assertEquals(.5f,middle.first,.001f)
        assertEquals(.5f,middle.second,.001f)
    }
    @Test fun zoomAndFullscreenUseTheDisplayedViewport(){
        for(zoom in 0..6){
            val view=ViewConfig.create(AspectMode.DEVICE,zoom,720,720)
            val middle=TouchSelection.map(360f,360f,720,720,view)!!
            assertEquals(.5f,middle.first,.002f)
            assertEquals(.5f,middle.second,.002f)
        }
        assertNull(TouchSelection.map(Float.NaN,0f,720,720,ViewConfig(0,0)))
    }
    @Test fun menuBackgroundFillsDisplayAndKeepsOriginalForegroundScale(){
        for((w,h)in listOf(720 to 720,2280 to 1080,1280 to 800)){
            val view=ViewConfig.menu(w,h)
            assertTrue(view.fillScreen)
            val oldScale=minOf(w/240f,h/160f)
            val newScale=maxOf(w.toFloat()/view.width,h.toFloat()/view.height)
            assertEquals(oldScale,newScale,.001f)
            assertNotNull(TouchSelection.map(1f,1f,w,h,view))
            assertNotNull(TouchSelection.map(w-1f,h-1f,w,h,view))
        }
        val square=ViewConfig.menu(720,720)
        assertEquals(240,square.width);assertEquals(240,square.height)
    }
}
