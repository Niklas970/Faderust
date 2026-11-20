//! Exports the real window as one HTML page: the picture is a screenshot of
//! the running desk (so it matches exactly), on top of it areas can be drawn
//! and described. "Liste kopieren" and "Bild speichern" give something to send
//! back.

/// What the page needs.
pub struct Page {
    pub version: String,
    /// The screenshot as PNG.
    pub png: Vec<u8>,
    /// Size of the window in logical pixels.
    pub width: u32,
    pub height: u32,
    /// Options that are switched on, for the list.
    pub opts: Vec<(&'static str, bool)>,
    pub design_json: String,
}

/// Standard base64 (RFC 4648) with padding.
pub fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The complete page.
pub fn render(p: &Page) -> String {
    let opts: String = p
        .opts
        .iter()
        .map(|(k, v)| format!("{k}={}", if *v { "an" } else { "aus" }))
        .collect::<Vec<_>>()
        .join(", ");
    PAGE.replace("%VERSION%", &esc(&p.version))
        .replace("%W%", &p.width.to_string())
        .replace("%H%", &p.height.to_string())
        .replace("%OPTS%", &esc(&opts))
        .replace("%JSON%", &p.design_json.replace("</", "<\\/"))
        .replace("%PNG%", &base64(&p.png))
}

const PAGE: &str = r##"<!doctype html><html lang="de"><meta charset="utf-8">
<title>Faderust Design-Export</title>
<style>
*{box-sizing:border-box;margin:0}
body{background:#14161a;color:#d7dae0;font:13px "Segoe UI","DejaVu Sans",sans-serif;display:flex;min-height:100vh}
#stage{padding:16px;flex:none}
#wrap{position:relative;width:%W%px;height:%H%px;border:1px solid #2c3038;cursor:crosshair;user-select:none;touch-action:none}
#wrap img{display:block;width:100%;height:100%;pointer-events:none}
.box{position:absolute;border:2px solid #ff3b81;background:rgba(255,59,129,.12)}
.box.sel{background:rgba(255,59,129,.28);box-shadow:0 0 0 2px #fff}
.box i{position:absolute;left:-2px;top:-20px;background:#ff3b81;color:#fff;font:700 11px monospace;font-style:normal;min-width:18px;height:18px;border-radius:9px;display:grid;place-items:center;padding:0 5px}
.box.dot{border-radius:50%}
aside{width:340px;flex:none;padding:16px 16px 16px 0;position:sticky;top:0;height:100vh;overflow:auto}
aside h2{font-size:15px;margin-bottom:6px}
aside p{color:#9aa0ab;line-height:1.45;margin-bottom:10px}
.btns{display:flex;gap:8px;flex-wrap:wrap;margin-bottom:12px}
button{background:#ff8a1f;border:0;color:#000;font-weight:700;padding:8px 12px;border-radius:6px;cursor:pointer}
button.alt{background:#2c3038;color:#e6e8ec}
.item{background:#1d2026;border:1px solid #2c3038;border-radius:6px;padding:8px;margin-bottom:8px}
.item.sel{border-color:#ff3b81}
.item b{color:#ff3b81}.item small{color:#9aa0ab}
.item input{width:100%;margin-top:6px;background:#0f1114;border:1px solid #2c3038;color:#e6e8ec;padding:6px;border-radius:4px}
.item .x{float:right;background:none;color:#9aa0ab;padding:0 4px}
textarea{width:100%;min-height:64px;background:#0f1114;border:1px solid #2c3038;color:#e6e8ec;padding:6px;border-radius:4px}
.meta{margin-top:10px;font-size:11px;color:#7d8490}
</style>
<div id="stage"><div id="wrap"><img id="img" alt="Faderust" src="data:image/png;base64,%PNG%"></div></div>
<aside>
<h2>Design markieren</h2>
<p>Das Bild ist ein Screenshot deiner echten Oberfläche. Ziehe ein Rechteck über das, was du meinst, oder klicke einfach auf eine Stelle. Schreibe rechts dazu, was anders sein soll.</p>
<div class="btns"><button id="copy">Liste kopieren</button><button class="alt" id="save">Bild mit Markierungen</button><button class="alt" id="undo">Letzte weg</button></div>
<div id="list"></div>
<textarea id="gen" placeholder="Allgemeine Anmerkung zum Design"></textarea>
<div class="meta">Faderust v%VERSION% · Fenster %W%×%H% px<br>Optionen: %OPTS%</div>
</aside>
<script type="application/json" id="faderust-design">%JSON%</script>
<script>
const W=%W%,H=%H%,wrap=document.getElementById('wrap'),list=document.getElementById('list');
const marks=[];let sel=null,drag=null;
const pos=e=>{const r=wrap.getBoundingClientRect();return{x:Math.max(0,Math.min(W,e.clientX-r.left)),y:Math.max(0,Math.min(H,e.clientY-r.top))}};
function draw(){wrap.querySelectorAll('.box').forEach(b=>b.remove());marks.forEach((m,i)=>{const b=document.createElement('div');b.className='box'+(m===sel?' sel':'')+(m.dot?' dot':'');
b.style.cssText='left:'+m.x+'px;top:'+m.y+'px;width:'+m.w+'px;height:'+m.h+'px';b.innerHTML='<i>'+(i+1)+'</i>';wrap.appendChild(b)});
list.innerHTML='';marks.forEach((m,i)=>{const d=document.createElement('div');d.className='item'+(m===sel?' sel':'');
d.innerHTML='<button class="x" title="Entfernen">✕</button><b>'+(i+1)+'.</b> <small>'+Math.round(m.x)+','+Math.round(m.y)+' · '+Math.round(m.w)+'×'+Math.round(m.h)+' px</small>';
d.querySelector('.x').onclick=()=>{marks.splice(i,1);sel=null;draw()};
const inp=document.createElement('input');inp.placeholder='Was soll anders sein?';inp.value=m.note;inp.oninput=()=>m.note=inp.value;inp.onfocus=()=>{sel=m;wrap.querySelectorAll('.box').forEach((b,j)=>b.classList.toggle('sel',marks[j]===m))};d.appendChild(inp);list.appendChild(d)})}
wrap.addEventListener('pointerdown',e=>{wrap.setPointerCapture(e.pointerId);const p=pos(e);drag={x0:p.x,y0:p.y,m:{x:p.x,y:p.y,w:0,h:0,note:''}};marks.push(drag.m);sel=drag.m;draw()});
wrap.addEventListener('pointermove',e=>{if(!drag)return;const p=pos(e),m=drag.m;m.x=Math.min(drag.x0,p.x);m.y=Math.min(drag.y0,p.y);m.w=Math.abs(p.x-drag.x0);m.h=Math.abs(p.y-drag.y0);
const b=wrap.querySelectorAll('.box')[marks.indexOf(m)];if(b){b.style.cssText='left:'+m.x+'px;top:'+m.y+'px;width:'+m.w+'px;height:'+m.h+'px'}});
wrap.addEventListener('pointerup',()=>{if(!drag)return;const m=drag.m;if(m.w<6&&m.h<6){m.dot=true;m.x=Math.max(0,drag.x0-14);m.y=Math.max(0,drag.y0-14);m.w=28;m.h=28}drag=null;draw();const inp=list.querySelectorAll('input');if(inp.length)inp[inp.length-1].focus()});
document.getElementById('undo').onclick=()=>{marks.pop();sel=null;draw()};
function text(){return 'Faderust Design-Markierungen (v%VERSION%, Fenster %W%×%H% px)\n'+marks.map((m,i)=>(i+1)+'. Bereich x='+Math.round(m.x)+' y='+Math.round(m.y)+' b='+Math.round(m.w)+' h='+Math.round(m.h)+(m.note?': '+m.note:'')).join('\n')+(document.getElementById('gen').value?'\n\nAllgemein: '+document.getElementById('gen').value:'')}
document.getElementById('copy').onclick=()=>{(navigator.clipboard?navigator.clipboard.writeText(text()):Promise.reject()).catch(()=>{const g=document.getElementById('gen');g.value=text();g.select()})};
document.getElementById('save').onclick=()=>{const img=document.getElementById('img'),c=document.createElement('canvas'),k=img.naturalWidth/W;c.width=img.naturalWidth;c.height=img.naturalHeight;const x=c.getContext('2d');x.drawImage(img,0,0);
marks.forEach((m,i)=>{x.lineWidth=2*k;x.strokeStyle='#ff3b81';x.fillStyle='rgba(255,59,129,.15)';x.fillRect(m.x*k,m.y*k,m.w*k,m.h*k);x.strokeRect(m.x*k,m.y*k,m.w*k,m.h*k);x.fillStyle='#ff3b81';x.beginPath();x.arc(m.x*k,m.y*k,10*k,0,7);x.fill();x.fillStyle='#fff';x.font='bold '+(12*k)+'px monospace';x.textAlign='center';x.textBaseline='middle';x.fillText(i+1,m.x*k,m.y*k)});
const a=document.createElement('a');a.href=c.toDataURL('image/png');a.download='faderust-design-markiert.png';a.click()};
</script></html>"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn page_holds_picture_size_and_safe_json() {
        let h = render(&Page {
            version: "0.8.7".into(),
            png: vec![1, 2, 3],
            width: 1300,
            height: 800,
            opts: vec![("hide_readout", true)],
            design_json: "{\"a\":\"</script>\"}".into(),
        });
        assert!(h.contains("data:image/png;base64,AQID"));
        assert!(h.contains("width:1300px") || h.contains("W=1300"));
        assert!(h.contains("hide_readout=an"));
        // The JSON block cannot end the script early.
        assert_eq!(h.matches("</script>").count(), 2);
        for key in ["%VERSION%", "%W%", "%H%", "%OPTS%", "%JSON%", "%PNG%"] {
            assert!(!h.contains(key), "{key} not replaced");
        }
    }
}
