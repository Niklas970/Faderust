//! Exports the current look of the desk as one HTML file: real colours,
//! real strips, the chosen options. Opened in a browser, parts can be clicked
//! and marked with a note; "Kopieren" gives a list to send back.

/// One strip (input or output) as drawn on the desk.
pub struct StripDoc {
    pub name: String,
    pub sub: String,
    pub db: f32,
    pub level: f32,
    pub muted: bool,
    /// Output buttons A1.. and whether the strip is routed to each.
    pub routes: Vec<bool>,
    pub tint: String,
}

/// Everything the page shows.
pub struct Doc {
    pub version: String,
    pub colors: Colors,
    pub outputs: Vec<StripDoc>,
    pub devices: Vec<StripDoc>,
    pub programs: Vec<StripDoc>,
    pub opts: Vec<(&'static str, bool)>,
    pub design_json: String,
    pub width: u32,
    pub height: u32,
}

/// Colours as "#rrggbb".
pub struct Colors {
    pub accent: String,
    pub signal: String,
    pub bg: String,
    pub panel: String,
    pub panel_2: String,
    pub well: String,
    pub border: String,
    pub text: String,
    pub muted: String,
}

impl Doc {
    fn on(&self, key: &str) -> bool {
        self.opts.iter().any(|(k, v)| *k == key && *v)
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fader position in percent from the top (0 dB at 36 %, as on the desk).
fn top_pct(db: f32) -> f32 {
    let t = if db >= 0.0 {
        36.0 - db / 12.0 * 18.0
    } else {
        36.0 + (-db / 48.0).min(1.0) * 64.0
    };
    t.clamp(0.0, 100.0)
}

fn db_text(db: f32) -> String {
    if db <= -90.0 {
        "-∞".into()
    } else if db > 0.0 {
        format!("+{db:.1}")
    } else {
        format!("{db:.1}")
    }
}

const TICKS: [(&str, f32); 9] = [
    ("+12", 18.0),
    ("+6", 27.0),
    ("0", 36.0),
    ("-6", 44.0),
    ("-12", 56.0),
    ("-24", 74.0),
    ("-48", 90.0),
    ("-∞", 100.0),
    ("+24", 0.0),
];

fn strip(d: &Doc, s: &StripDoc, program: bool, slim: bool) -> String {
    let abtn: String = s
        .routes
        .iter()
        .enumerate()
        .map(|(i, on)| {
            format!(
                "<i data-p=\"A-Knopf\" class=\"{}\">A{}</i>",
                if *on { "on" } else { "" },
                i + 1
            )
        })
        .collect();
    let ticks: String = if d.on("scale_off") {
        String::new()
    } else {
        TICKS
            .iter()
            .map(|(t, p)| format!("<span style=\"top:{p}%\">{t}</span>"))
            .collect()
    };
    let top = top_pct(s.db);
    let lvl = (s.level.clamp(0.0, 1.0) * 100.0).round();
    let readout = if d.on("hide_readout") {
        String::new()
    } else {
        format!(
            "<div class=\"lcd{}\" data-p=\"dB-Anzeige\">{} dB</div>",
            if d.on("readout_display") { " disp" } else { "" },
            db_text(s.db)
        )
    };
    let mute = if s.muted { " on" } else { "" };
    let bottom = if d.on("button_row") {
        let mut b =
            format!("<div class=\"row\"><b data-p=\"Mute-Knopf\" class=\"btn{mute}\">M</b>");
        if program || !s.routes.is_empty() {
            b.push_str("<b data-p=\"FX-Knopf\" class=\"btn\">FX</b>");
        }
        if d.on("echo_button") {
            b.push_str("<b data-p=\"Echo-Knopf\" class=\"btn\">E</b>");
        }
        if program {
            b.push_str("<b data-p=\"Behalten-Knopf\" class=\"btn\">○</b>");
        }
        b.push_str("</div>");
        b
    } else {
        let mut b = format!("<b data-p=\"Mute-Knopf\" class=\"btn wide{mute}\">MUTE</b>");
        if program {
            b.push_str("<div class=\"row\"><b data-p=\"FX-Knopf\" class=\"btn\">FX</b><b data-p=\"Behalten-Knopf\" class=\"btn\">○</b></div>");
        } else {
            b.push_str("<b data-p=\"FX-Knopf\" class=\"btn sm wide\">Effekte</b>");
        }
        if d.on("echo_button") {
            b.push_str("<b data-p=\"Echo-Knopf\" class=\"btn sm wide\">Echo</b>");
        }
        b
    };
    let narrow = program || slim;
    format!(
        "<div class=\"strip{}\" data-s=\"{name}\" style=\"--tint:{tint}\">\
<div class=\"bar\" data-p=\"Farbleiste\"></div>\
<div class=\"ab\">{abtn}</div>\
<div class=\"nm\" data-p=\"Name\">{name}</div>\
<div class=\"sub\" data-p=\"Gerätename\">{sub}</div>\
<div class=\"fw\"><div class=\"sc\" data-p=\"Skala\">{ticks}</div>\
<div class=\"fd\" data-p=\"Fader-Bahn\"><i style=\"height:{h}%\"></i>\
<s data-p=\"Fader-Kappe\" style=\"top:{top}%\">{val}</s></div>\
<div class=\"mt\" data-p=\"Pegelanzeige\"><i style=\"height:{lvl}%\"></i></div></div>\
{readout}{bottom}</div>",
        if narrow { " narrow" } else { "" },
        name = esc(&s.name),
        sub = esc(&s.sub),
        tint = s.tint,
        h = 100.0 - top,
        val = if d.on("cap_value_off") {
            String::new()
        } else {
            db_text(s.db)
        },
    )
}

fn group(
    d: &Doc,
    title: &str,
    items: &[StripDoc],
    program: bool,
    slim: bool,
    extra: &str,
) -> String {
    let strips: String = items.iter().map(|s| strip(d, s, program, slim)).collect();
    format!(
        "<section data-s=\"Gruppe {t}\"><h3 data-p=\"Gruppenkopf\">{t}{extra}</h3><div class=\"strips\">{strips}</div></section>",
        t = esc(title)
    )
}

/// The complete page.
pub fn render(d: &Doc) -> String {
    let c = &d.colors;
    let slim = d.on("slim_strips");
    let scroll = if d.on("slim_scroll") {
        ""
    } else {
        "<div class=\"hs\" data-p=\"Scrollleiste\"><i></i></div>"
    };
    let thin = if d.on("thin_track") { "5px" } else { "16px" };
    let cap_r = if d.on("cap_square") { "4px" } else { "8px" };
    let body = format!(
        "{}{}{}",
        group(d, "AUSGÄNGE", &d.outputs, false, slim, ""),
        group(d, "GERÄTE", &d.devices, false, slim, ""),
        group(d, "PROGRAMME", &d.programs, true, false, ""),
    );
    let opts: String = d
        .opts
        .iter()
        .map(|(k, v)| format!("{k}={}", if *v { "an" } else { "aus" }))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r##"<!doctype html><html lang="de"><meta charset="utf-8">
<title>Faderust Design-Export</title>
<style>
:root{{--accent:{accent};--signal:{signal};--bg:{bg};--panel:{panel};--panel2:{panel2};--well:{well};--border:{border};--text:{text};--muted:{muted};--track:{thin};--cap-r:{cap_r}}}
*{{box-sizing:border-box;margin:0}}
body{{background:#14161a;color:var(--text);font:12px "Segoe UI","DejaVu Sans",sans-serif;display:flex;min-height:100vh}}
#win{{background:var(--bg);border:1px solid var(--border);margin:16px;width:{w}px;min-height:{h}px;display:flex;flex-direction:column;flex:none;position:relative}}
.top{{height:44px;border-bottom:1px solid var(--border);display:flex;align-items:center;padding:0 12px;gap:10px;font-weight:700}}
.top small{{color:var(--muted);font-weight:400}}
.main{{display:flex;padding:8px 6px;flex:1}}
section{{padding:0 10px;border-right:2px solid var(--accent)}}
section:last-child{{border-right:0;flex:1}}
h3{{font-size:10px;letter-spacing:.08em;color:var(--muted);height:26px;display:flex;align-items:center}}
.strips{{display:flex;gap:6px;height:620px}}
.strip{{width:136px;border:1px solid var(--border);border-radius:6px;background:var(--panel);padding:8px 6px;display:flex;flex-direction:column;align-items:center;gap:6px;position:relative;overflow:hidden}}
.strip.narrow{{width:104px}}
.bar{{position:absolute;left:0;right:0;top:0;height:3px;background:var(--tint)}}
.ab{{display:flex;gap:3px;margin-top:2px}}
.ab i{{font:700 9px monospace;font-style:normal;border-radius:10px;padding:2px 6px;color:var(--muted);background:var(--panel2)}}
.ab i.on{{background:var(--accent);color:var(--bg)}}
.nm{{width:100%;text-align:center;background:var(--panel2);padding:3px;font-weight:700;font-size:11px;letter-spacing:.04em}}
.sub{{font-size:9px;color:var(--signal);max-width:100%;white-space:nowrap;overflow:hidden}}
.fw{{flex:1;width:100%;display:flex;justify-content:center;gap:12px;min-height:0;margin:2px 0}}
.sc{{width:28px;position:relative;font:8px monospace;color:var(--muted)}}
.sc span{{position:absolute;right:0;transform:translateY(-50%)}}
.fd{{width:var(--track);background:var(--well);border:1px solid var(--border);border-radius:8px;position:relative;margin:8px 0}}
.fd i{{position:absolute;bottom:0;left:2px;right:2px;background:var(--accent);border-radius:6px}}
.fd s{{position:absolute;left:-12px;right:-12px;height:30px;background:linear-gradient(#41454f,#272a31);border:2px solid var(--accent);border-radius:var(--cap-r);transform:translateY(-50%);font:10px monospace;color:#fff;display:grid;place-items:center;text-decoration:none}}
.mt{{width:10px;background:var(--well);border:1px solid var(--border);margin:8px 0;position:relative}}
.mt i{{position:absolute;bottom:0;left:0;right:0;background:var(--signal)}}
.lcd{{font:700 11px monospace;color:var(--accent);text-align:center}}
.lcd.disp{{background:var(--well);border:1px solid var(--border);padding:3px 0;width:86%}}
.btn{{font:700 11px monospace;border:1.5px solid var(--border);border-radius:5px;padding:5px 0;background:var(--panel2);color:var(--text);text-align:center;letter-spacing:.05em;flex:1}}
.btn.wide{{width:100%;flex:none}}.btn.sm{{font-weight:400;font-size:10px;padding:3px 0}}
.btn.on{{background:var(--accent);color:var(--bg);border-color:var(--accent)}}
.row{{display:flex;gap:3px;width:100%}}
.hs{{height:8px;background:var(--panel2);border-radius:4px;margin-top:6px;position:relative}}
.hs i{{position:absolute;left:0;width:45%;height:100%;background:var(--muted);border-radius:4px;opacity:.6}}
[data-p]{{cursor:crosshair}}
[data-p]:hover{{outline:2px dashed var(--signal);outline-offset:1px}}
.mark{{outline:2px solid #ff3b81!important;outline-offset:1px}}
.pin{{position:absolute;z-index:9;background:#ff3b81;color:#fff;font:700 10px monospace;border-radius:9px;min-width:18px;height:18px;display:grid;place-items:center;pointer-events:none;transform:translate(-50%,-50%)}}
aside{{width:330px;flex:none;padding:16px 16px 16px 0;position:sticky;top:0;height:100vh;overflow:auto;color:#d7dae0}}
aside h2{{font-size:14px;margin-bottom:6px}}
aside p{{color:#9aa0ab;margin-bottom:10px;line-height:1.4}}
.item{{background:#1d2026;border:1px solid #2c3038;border-radius:6px;padding:8px;margin-bottom:8px}}
.item b{{color:#ff3b81}}
.item input{{width:100%;margin-top:6px;background:#0f1114;border:1px solid #2c3038;color:#e6e8ec;padding:5px;border-radius:4px}}
.item button{{float:right;background:none;border:0;color:#9aa0ab;cursor:pointer}}
#copy{{background:#ff8a1f;border:0;color:#000;font-weight:700;padding:8px 14px;border-radius:6px;cursor:pointer;margin-bottom:12px}}
#gen{{width:100%;margin-top:6px;background:#0f1114;border:1px solid #2c3038;color:#e6e8ec;padding:5px;border-radius:4px;min-height:60px}}
</style>
<div id="win"><div class="top"><span data-p="Titelleiste">FADERUST</span><small>v{version}</small></div><div class="main">{body}</div>{scroll}</div>
<aside><h2>Design markieren</h2>
<p>Klicke Teile der Oberfläche an, um sie zu markieren, und schreibe dazu, was du dir wünschst. Mit „Kopieren“ bekommst du eine Liste, die du Claude schickst, am besten zusammen mit dieser Datei.</p>
<button id="copy">Liste kopieren</button>
<div id="list"></div>
<textarea id="gen" placeholder="Allgemeine Anmerkung zum Design"></textarea>
<p style="margin-top:10px;font-size:11px">Optionen: {opts}</p></aside>
<script type="application/json" id="faderust-design">{json}</script>
<script>
const marks=[];const win=document.getElementById('win');const list=document.getElementById('list');
function pinAt(m){{const r=m.el.getBoundingClientRect(),w=win.getBoundingClientRect();m.pin.style.left=(r.left-w.left+r.width/2)+'px';m.pin.style.top=(r.top-w.top+r.height/2)+'px'}}
function redraw(){{list.innerHTML='';marks.forEach((m,i)=>{{m.pin.textContent=i+1;const d=document.createElement('div');d.className='item';
d.innerHTML='<button title="Entfernen">✕</button><b>'+(i+1)+'.</b> '+m.part+' <span style="color:#9aa0ab">('+m.strip+')</span>';
d.querySelector('button').onclick=()=>remove(m);const inp=document.createElement('input');inp.placeholder='Was soll anders sein?';inp.value=m.note;inp.oninput=()=>m.note=inp.value;d.appendChild(inp);list.appendChild(d)}})}}
function remove(m){{m.el.classList.remove('mark');m.pin.remove();marks.splice(marks.indexOf(m),1);redraw()}}
document.addEventListener('click',e=>{{const el=e.target.closest('[data-p]');if(!el||!win.contains(el))return;
const ex=marks.find(m=>m.el===el);if(ex){{remove(ex);return}}
const s=el.closest('[data-s]');const pin=document.createElement('div');pin.className='pin';win.appendChild(pin);
const m={{el,part:el.dataset.p,strip:s?s.dataset.s:'Fenster',note:'',pin}};marks.push(m);el.classList.add('mark');pinAt(m);redraw()}});
document.getElementById('copy').onclick=()=>{{const t='Faderust Design-Markierungen (v{version})\n'+marks.map((m,i)=>(i+1)+'. '+m.part+' ['+m.strip+']'+(m.note?': '+m.note:'')).join('\n')+(document.getElementById('gen').value?'\n\nAllgemein: '+document.getElementById('gen').value:'');
(navigator.clipboard?navigator.clipboard.writeText(t):Promise.reject()).catch(()=>{{const g=document.getElementById('gen');g.value=t;g.select()}})}};
</script></html>"##,
        accent = c.accent,
        signal = c.signal,
        bg = c.bg,
        panel = c.panel,
        panel2 = c.panel_2,
        well = c.well,
        border = c.border,
        text = c.text,
        muted = c.muted,
        w = d.width,
        h = d.height,
        version = esc(&d.version),
        json = d.design_json.replace("</", "<\\/"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(opts: Vec<(&'static str, bool)>) -> Doc {
        let s = |n: &str| StripDoc {
            name: n.into(),
            sub: "Gerät".into(),
            db: -6.0,
            level: 0.5,
            muted: false,
            routes: vec![true, false],
            tint: "#9b6df0".into(),
        };
        Doc {
            version: "0.8.7".into(),
            colors: Colors {
                accent: "#ff8a1f".into(),
                signal: "#5ccf6a".into(),
                bg: "#000000".into(),
                panel: "#0c0d10".into(),
                panel_2: "#16181d".into(),
                well: "#050608".into(),
                border: "#3a404c".into(),
                text: "#ecedf1".into(),
                muted: "#8b93a1".into(),
            },
            outputs: vec![s("LAUTSPRECHER")],
            devices: vec![s("MIKROFON")],
            programs: vec![s("<BRAVE>")],
            opts,
            design_json: "{\"a\":\"</script>\"}".into(),
            width: 1300,
            height: 800,
        }
    }

    #[test]
    fn export_contains_strips_and_escapes() {
        let h = render(&doc(vec![]));
        assert!(h.contains("LAUTSPRECHER") && h.contains("MIKROFON"));
        assert!(h.contains("&lt;BRAVE&gt;"));
        assert!(!h.contains("<BRAVE>"));
        // The JSON block cannot end the script early.
        assert_eq!(h.matches("</script>").count(), 2);
    }

    #[test]
    fn export_follows_the_space_options() {
        let plain = render(&doc(vec![]));
        assert!(plain.contains("class=\"lcd\"") && plain.contains("MUTE"));
        let tight = render(&doc(vec![("hide_readout", true), ("button_row", true)]));
        assert!(!tight.contains("class=\"lcd") && !tight.contains(">MUTE<"));
        assert!(tight.contains(">M</b>"));
    }

    #[test]
    fn fader_position_matches_the_scale() {
        assert_eq!(top_pct(0.0), 36.0);
        assert_eq!(top_pct(12.0), 18.0);
        assert_eq!(top_pct(-96.0), 100.0);
    }
}
