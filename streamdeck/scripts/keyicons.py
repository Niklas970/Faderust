# Writes the key icons (icons/*-key.svg, mute-*, *-active) as SVG; then run node scripts/icons.cjs.
import pathlib,re
# The Faderust logo as a small badge in the top-left corner of every key.
_logo=pathlib.Path(__file__).resolve().parent.parent.parent/"assets"/"logo.svg"
_inner=re.sub(r'<svg[^>]*>|</svg>','',_logo.read_text())
for _i in ("bg","cap","signal"): _inner=_inner.replace(f'id="{_i}"',f'id="lg-{_i}"').replace(f'url(#{_i})',f'url(#lg-{_i})')
LOGO=f'<svg x="5" y="5" width="15" height="15" viewBox="0 0 64 64">{_inner}</svg>'
G="#5ccf6a"; OR="#e0703a"
defs='''<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1e2226"/><stop offset="1" stop-color="#050607"/></linearGradient><linearGradient id="cap" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffc49f"/><stop offset="1" stop-color="#c65f27"/></linearGradient><radialGradient id="glow" cx="0.5" cy="0.4" r="0.7"><stop offset="0" stop-color="COLOR" stop-opacity="0.28"/><stop offset="1" stop-color="COLOR" stop-opacity="0"/></radialGradient></defs>'''
LOGO_ON=True
def tile(body, glow=None, border=None):
    d=defs.replace("COLOR", glow or G)
    s=f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 72" width="144" height="144">{d}<rect width="72" height="72" fill="#000"/><rect x="2" y="2" width="68" height="68" rx="10" fill="url(#bg)"/>'
    if glow: s+='<rect x="2" y="2" width="68" height="68" rx="10" fill="url(#glow)"/>'
    s+='<rect x="2.5" y="2.5" width="67" height="67" rx="9.5" fill="none" stroke="#ffffff" stroke-opacity="0.08"/>'
    s+=body+(LOGO if LOGO_ON else '')
    if border: s+=f'<rect x="2" y="2" width="68" height="68" rx="10" fill="none" stroke="{border}" stroke-width="2.5"/>'
    return s+'</svg>'
def fader(x,top,bot,lit,capc="url(#cap)",w=3.6,cw=15,ch=9,track="#2c333a"):
    return (f'<line x1="{x}" y1="{top}" x2="{x}" y2="{bot}" stroke="{track}" stroke-width="{w}" stroke-linecap="round"/>'
            f'<line x1="{x}" y1="{lit[0]}" x2="{x}" y2="{bot}" stroke="{lit[1]}" stroke-width="{w}" stroke-linecap="round"/>'
            f'<rect x="{x-cw/2}" y="{lit[0]-ch/2}" width="{cw}" height="{ch}" rx="3" fill="{capc}"/>'
            f'<rect x="{x-cw/2+2}" y="{lit[0]-0.5}" width="{cw-4}" height="1.2" rx="0.6" fill="#fff" fill-opacity="0.55"/>')
def three(levels, litc, cap, glow=None, border=None):
    b=''.join(fader(x,12,50,(y,litc),cap,w=3.4,cw=14) for x,y in zip((20,36,52),levels))
    if border: b+=f'<rect x="22" y="58" width="28" height="3" rx="1.5" fill="{border}"/>'
    return tile(b,glow,border)
speaker=lambda c:f'<path d="M14 28h9l12-10v32L23 40h-9z" fill="{c}" stroke="{c}" stroke-width="2" stroke-linejoin="round"/>'
out={}
out["strip-key"]=tile(fader(28,10,56,(26,G),cw=20,ch=11,w=4.4)+f'<g fill="{G}"><circle cx="54" cy="16" r="3"/><circle cx="54" cy="30" r="3"/><circle cx="54" cy="44" r="3" opacity="0.3"/></g>')
out["master-key"]=tile('<rect x="19" y="8" width="40" height="46" rx="7" fill="none" stroke="#3a434c" stroke-width="2"/>'+fader(36,13,51,(24,G),cw=22,ch=11,w=4.4))
out["volume-key"]=three((34,22,40),G,"url(#cap)")
out["preset-key"]=three((28,38,24),"#3a434c","#7a858f")
out["preset-active"]=three((28,38,24),G,"url(#cap)",G,G)
out["mute-on"]=tile(speaker(G)+f'<g fill="none" stroke="{G}" stroke-width="3.4" stroke-linecap="round"><path d="M41 27a9 9 0 0 1 0 14"/><path d="M46 21a17 17 0 0 1 0 26"/></g>')
out["mute-off"]=tile(f'<rect x="2" y="2" width="68" height="68" rx="10" fill="#5a1f12" opacity="0.5"/>'+speaker(OR)+f'<g stroke="{OR}" stroke-width="3.8" stroke-linecap="round"><line x1="43" y1="26" x2="57" y2="42"/><line x1="57" y1="26" x2="43" y2="42"/></g>',OR,OR)
hp=lambda c:f'<path d="M18 40v-6a18 18 0 0 1 36 0v6" fill="none" stroke="{c}" stroke-width="4" stroke-linecap="round"/><rect x="13" y="36" width="11" height="18" rx="4" fill="{c}"/><rect x="48" y="36" width="11" height="18" rx="4" fill="{c}"/>'
out["device-key"]=tile(hp("#8a959f"))
out["device-active"]=tile(hp(G)+f'<rect x="22" y="58" width="28" height="3" rx="1.5" fill="{G}"/>',G,G)

# --- selection, step, page keys (glyphs stay above the title line) ---
GREY="#8a959f"
def pick(master,on):
    c=G if on else GREY
    frame=f'<rect x="19" y="9" width="40" height="38" rx="7" fill="none" stroke="{"#3a434c" if not on else G}" stroke-width="2"/>' if master else ""
    b=frame+fader(36,12,44,(24,c if on else "#3a434c"),"url(#cap)" if on else "#7a858f",cw=20 if master else 18,ch=10,w=4)
    if not master: b+=f'<circle cx="56" cy="14" r="4" fill="{G if on else "none"}" stroke="{c}" stroke-width="2"/>'
    if on: b+=f'<rect x="22" y="58" width="28" height="3" rx="1.5" fill="{G}"/>'
    return tile(b,G if on else None,G if on else None)
out["select-key"]=pick(False,False); out["select-on"]=pick(False,True)
out["select-master"]=pick(True,False); out["select-master-on"]=pick(True,True)
out["select-clear"]=tile(f'<g fill="none" stroke="{GREY}" stroke-width="4" stroke-linecap="round"><circle cx="36" cy="26" r="16" stroke-dasharray="4 5" stroke-width="3"/><line x1="29" y1="19" x2="43" y2="33"/><line x1="43" y1="19" x2="29" y2="33"/></g>')
def step(sign,c):
    bars=''.join(f'<line x1="{x}" y1="10" x2="{x}" y2="46" stroke="#232a30" stroke-width="3.4" stroke-linecap="round"/>' for x in (14,58))
    h=f'<line x1="26" y1="28" x2="46" y2="28" stroke="{c}" stroke-width="6" stroke-linecap="round"/>'
    if sign>0: h+=f'<line x1="36" y1="18" x2="36" y2="38" stroke="{c}" stroke-width="6" stroke-linecap="round"/>'
    return tile(bars+h)
out["group-up"]=step(1,G); out["group-down"]=step(-1,OR)
out["level-key"]=tile(f'<path d="M16 42a22 22 0 1 1 40 0" fill="none" stroke="#2c333a" stroke-width="5" stroke-linecap="round"/><path d="M16 42a22 22 0 0 1 24-24" fill="none" stroke="{G}" stroke-width="5" stroke-linecap="round"/><line x1="36" y1="36" x2="44" y2="22" stroke="url(#cap)" stroke-width="4.5" stroke-linecap="round"/><circle cx="36" cy="36" r="4" fill="#ffc49f"/>')
out["page-key"]=tile(''.join(f'<rect x="{x}" y="{y}" width="12" height="11" rx="3" fill="{G}" opacity="{o}"/>' for (x,y,o) in [(15,8,1),(30,8,1),(45,8,0.45),(15,22,1),(30,22,0.45),(45,22,1),(15,36,0.45),(30,36,1),(45,36,1)]))
out["back-key"]=tile(f'<path d="M44 14L24 28l20 14" fill="none" stroke="{GREY}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>')

def selmute(on):
    c=OR if on else G
    sp=speaker(c).replace('M14 28h9l12-10v32L23 40h-9z','M18 30h8l10-9v30l-10-9h-8z')
    x=(f'<g stroke="{c}" stroke-width="3.6" stroke-linecap="round"><line x1="42" y1="28" x2="54" y2="42"/><line x1="54" y1="28" x2="42" y2="42"/></g>' if on else
       f'<g fill="none" stroke="{c}" stroke-width="3.2" stroke-linecap="round"><path d="M42 29a8 8 0 0 1 0 12"/><path d="M47 24a15 15 0 0 1 0 22"/></g>')
    mark=f'<circle cx="58" cy="14" r="4" fill="{c}"/>'
    bg=f'<rect x="2" y="2" width="68" height="68" rx="10" fill="#5a1f12" opacity="0.5"/>' if on else ''
    return tile(bg+sp+x+mark,c if on else None,OR if on else None)
out["selmute-on"]=selmute(True); out["selmute-off"]=selmute(False)

# Plain variants without the logo: the plugin draws a program's own icon there.
def plain(make):
    global LOGO_ON
    LOGO_ON=False
    try: return make()
    finally: LOGO_ON=True
out["strip-plain"]=plain(lambda: tile(fader(28,10,56,(26,G),cw=20,ch=11,w=4.4)+f'<g fill="{G}"><circle cx="54" cy="16" r="3"/><circle cx="54" cy="30" r="3"/><circle cx="54" cy="44" r="3" opacity="0.3"/></g>'))
out["volume-plain"]=plain(lambda: three((34,22,40),G,"url(#cap)"))
out["select-plain"]=plain(lambda: pick(False,False))
out["select-on-plain"]=plain(lambda: pick(False,True))
for k,v in out.items(): pathlib.Path(f"icons/{k}.svg").write_text(v+"\n")
