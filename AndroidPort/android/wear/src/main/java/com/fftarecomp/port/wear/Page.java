package com.fftarecomp.port.wear;

import java.util.HashMap;
import java.util.Map;

/**
 * Upload page served by {@link RomInbox} to the phone/PC on the same Wi-Fi as the watch.
 * Game name, accepted files and accent color come from {@link RomInboxConfig}.
 * Shared across every Wear OS port; keep the copies identical except for the package.
 */
final class Page {

    private Page() {}

    private static final Map<String, Map<String, String>> T = new HashMap<>();

    private static void lang(String code, String... kv) {
        Map<String, String> m = new HashMap<>();
        for (int i = 0; i + 1 < kv.length; i += 2) m.put(kv[i], kv[i + 1]);
        T.put(code, m);
    }

    static {
        lang("pt",
                "title", "Enviar para o relógio",
                "pick", "Escolher arquivo",
                "drop", "ou arraste o arquivo para cá",
                "privacy", "O arquivo vai direto para o relógio pela sua rede Wi-Fi. Nada passa pela internet.",
                "sending", "Enviando",
                "keep", "Mantenha esta página aberta até terminar.",
                "busy", "O relógio já está recebendo outro arquivo.",
                "notRom", "Esse arquivo não é aceito por este jogo.",
                "okGame", "Pronto! O jogo está abrindo no relógio.",
                "cut", "O envio foi interrompido. Tente de novo.",
                "noSpace", "Não há espaço livre suficiente no relógio.",
                "noWifi", "Sem Wi-Fi. Ligue o Wi-Fi do relógio.",
                "noPort", "Não foi possível abrir o servidor.",
                "lost", "Conexão com o relógio perdida. Confira se ele continua na tela do QR e no mesmo Wi-Fi.",
                "again", "Enviar outro arquivo");
        lang("en",
                "title", "Send to your watch",
                "pick", "Choose file",
                "drop", "or drop the file here",
                "privacy", "The file goes straight to the watch over your Wi-Fi. Nothing goes through the internet.",
                "sending", "Sending",
                "keep", "Keep this page open until it finishes.",
                "busy", "The watch is already receiving another file.",
                "notRom", "This game does not accept that file.",
                "okGame", "Done! The game is opening on the watch.",
                "cut", "The upload was interrupted. Try again.",
                "noSpace", "Not enough free space on the watch.",
                "noWifi", "No Wi-Fi. Turn on the watch's Wi-Fi.",
                "noPort", "Could not start the server.",
                "lost", "Lost the connection to the watch. Check it is still on the QR screen and on the same Wi-Fi.",
                "again", "Send another file");
    }

    static String t(String lang, String key) {
        Map<String, String> m = T.get(lang);
        String v = m != null ? m.get(key) : null;
        if (v == null) {
            Map<String, String> en = T.get("en");
            v = en != null ? en.get(key) : null;
        }
        return v != null ? v : key;
    }

    private static String esc(String s) {
        return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;");
    }

    private static String js(String s) {
        return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"").replace("</", "<\\/") + "\"";
    }

    private static String hex(int argb) {
        return String.format("#%06x", argb & 0xFFFFFF);
    }

    static String html(String lang) {
        String l = T.containsKey(lang) ? lang : "en";
        String accent = hex(RomInboxConfig.ACCENT);
        return "<!doctype html><html lang=\"" + l + "\"><head><meta charset=\"utf-8\">"
                + "<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">"
                + "<meta name=\"color-scheme\" content=\"dark light\">"
                + "<title>" + esc(RomInboxConfig.GAME) + "</title><style>"
                + ":root{--bg:#0e0e10;--fg:#f2f2f3;--muted:#a3a3ab;--line:#2a2a30;--card:#18181c;"
                + "--accent:" + accent + ";--ok:#3ccf6e;--err:#ff6b5e;--eyebrow:var(--accent);--r:14px}"
                + "@media (prefers-color-scheme:light){:root{--bg:#f6f6f7;--fg:#141416;--muted:#5c5c66;"
                + "--line:#dcdce0;--card:#fff;--ok:#16853f;--err:#c42b1c;--eyebrow:#5c5c66}}"
                + "*{box-sizing:border-box}"
                + "body{margin:0;min-height:100vh;background:var(--bg);color:var(--fg);"
                + "font:16px/1.5 system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;display:flex;justify-content:center}"
                + "main{width:100%;max-width:460px;padding:32px 16px}"
                + ".eyebrow{margin:0;font-size:13px;font-weight:600;letter-spacing:.08em;text-transform:uppercase;color:var(--eyebrow)}"
                + "h1{margin:4px 0 8px;font-size:26px;line-height:1.2}"
                + "p{margin:0;color:var(--muted)}"
                + ".drop{margin:24px 0 16px;padding:24px 16px;border:2px dashed var(--line);border-radius:var(--r);"
                + "background:var(--card);text-align:center;transition:border-color .15s,background .15s}"
                + ".drop.over{border-color:var(--accent)}"
                + ".btn{display:inline-flex;align-items:center;justify-content:center;gap:8px;min-height:48px;width:100%;"
                + "padding:12px 20px;border:0;border-radius:12px;background:var(--accent);color:#111;font:inherit;"
                + "font-weight:700;cursor:pointer;transition:transform .1s,filter .15s}"
                + ".btn:hover{filter:brightness(1.08)}.btn:active{transform:scale(.98)}"
                + ".btn:focus-visible{outline:3px solid var(--fg);outline-offset:3px}"
                + ".btn[disabled]{opacity:.45;cursor:not-allowed}"
                + ".btn svg{width:20px;height:20px}"
                + ".sub{margin-top:10px;font-size:14px}"
                + "input[type=file]{position:absolute;width:1px;height:1px;opacity:0}"
                + ".file{display:none;margin-top:4px;font-size:14px;word-break:break-all}"
                + ".bar{height:8px;margin-top:14px;border-radius:4px;background:var(--line);overflow:hidden;display:none}"
                + ".bar div{height:100%;width:0;background:var(--accent);transition:width .2s}"
                + "#msg{min-height:1.5em;margin-top:12px;font-weight:600}"
                + ".ok{color:var(--ok)}.err{color:var(--err)}"
                + ".note{margin-top:28px;padding-top:16px;border-top:1px solid var(--line);font-size:14px}"
                + "@media (prefers-reduced-motion:reduce){*{transition:none!important}}"
                + "</style></head><body><main>"
                + "<p class=\"eyebrow\">" + esc(RomInboxConfig.GAME) + "</p>"
                + "<h1>" + esc(t(l, "title")) + "</h1>"
                + "<p>" + esc(RomInboxConfig.hint(l)) + "</p>"
                + "<div class=\"drop\" id=\"d\">"
                + "<label class=\"btn\" id=\"pick\" tabindex=\"0\" role=\"button\">"
                + "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" "
                + "stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">"
                + "<path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><polyline points=\"17 8 12 3 7 8\"/>"
                + "<line x1=\"12\" x2=\"12\" y1=\"3\" y2=\"15\"/></svg>"
                + "<span>" + esc(t(l, "pick")) + "</span>"
                + "<input id=\"f\" type=\"file\" accept=\"" + esc(RomInboxConfig.ACCEPT) + "\"></label>"
                + "<p class=\"sub\">" + esc(t(l, "drop")) + "</p>"
                + "<p class=\"file\" id=\"fn\"></p>"
                + "<div class=\"bar\" id=\"b\" role=\"progressbar\" aria-valuemin=\"0\" aria-valuemax=\"100\"><div id=\"p\"></div></div>"
                + "<div id=\"msg\" role=\"status\" aria-live=\"polite\"></div></div>"
                + "<p class=\"note\">" + esc(t(l, "privacy")) + "</p>"
                + "<script>(function(){"
                + "var S=" + js(t(l, "sending")) + ",K=" + js(t(l, "keep")) + ",C=" + js(t(l, "cut"))
                + ",L=" + js(t(l, "lost")) + ",A=" + js(t(l, "again")) + ",P=" + js(t(l, "pick")) + ";"
                + "var f=document.getElementById('f'),d=document.getElementById('d'),pk=document.getElementById('pick'),"
                + "m=document.getElementById('msg'),b=document.getElementById('b'),p=document.getElementById('p'),"
                + "fn=document.getElementById('fn'),lbl=pk.querySelector('span'),busy=false;"
                + "function size(n){return n>1048576?(n/1048576).toFixed(1)+' MB':Math.max(1,Math.round(n/1024))+' KB';}"
                + "function set(k){p.style.width=k+'%';b.setAttribute('aria-valuenow',k);}"
                + "function send(file){if(!file||busy)return;busy=true;pk.setAttribute('aria-disabled','true');pk.style.opacity='.45';"
                + "fn.style.display='block';fn.textContent=file.name+' \\u00b7 '+size(file.size);"
                + "m.className='';m.textContent=S+'... '+K;b.style.display='block';set(0);"
                + "var x=new XMLHttpRequest();x.open('POST','/upload?name='+encodeURIComponent(file.name));"
                + "x.upload.onprogress=function(e){if(e.lengthComputable){var k=Math.round(e.loaded*100/e.total);"
                + "set(k);m.textContent=S+' '+k+'%. '+K;}};"
                + "function done(ok,t){busy=false;pk.removeAttribute('aria-disabled');pk.style.opacity='';"
                + "m.className=ok?'ok':'err';m.textContent=t;set(ok?100:0);lbl.textContent=ok?A:P;}"
                + "x.onload=function(){var r;try{r=JSON.parse(x.responseText)}catch(e){r={ok:false,msg:C}}done(!!r.ok,r.msg||C);};"
                + "x.onerror=function(){done(false,L);};x.send(file);}"
                + "f.onchange=function(){send(this.files[0]);this.value='';};"
                + "pk.onkeydown=function(e){if(e.key==='Enter'||e.key===' '){e.preventDefault();if(!busy)f.click();}};"
                + "pk.onclick=function(e){if(busy)e.preventDefault();};"
                + "['dragenter','dragover'].forEach(function(t){d.addEventListener(t,function(e){e.preventDefault();d.classList.add('over');});});"
                + "['dragleave','drop'].forEach(function(t){d.addEventListener(t,function(e){e.preventDefault();d.classList.remove('over');});});"
                + "d.addEventListener('drop',function(e){if(e.dataTransfer.files.length)send(e.dataTransfer.files[0]);});"
                + "})();</script></main></body></html>";
    }
}
