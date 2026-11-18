# Writes the key icons (icons/*-key.svg, mute-*, *-active) as SVG; then run node scripts/icons.cjs.
import pathlib
G="#5ccf6a"; OR="#e0703a"
defs='''<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1e2226"/><stop offset="1" stop-color="#050607"/></linearGradient><linearGradient id="cap" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffc49f"/><stop offset="1" stop-color="#c65f27"/></linearGradient><radialGradient id="glow" cx="0.5" cy="0.4" r="0.7"><stop offset="0" stop-color="COLOR" stop-opacity="0.28"/><stop offset="1" stop-color="COLOR" stop-opacity="0"/></radialGradient></defs>'''
def tile(body, glow=None, border=None):
    d=defs.replace("COLOR", glow or G)
    s=f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 72" width="144" height="144">{d}<rect width="72" height="72" fill="#000"/><rect x="2" y="2" width="68" height="68" rx="10" fill="url(#bg)"/>'
    if glow: s+='<rect x="2" y="2" width="68" height="68" rx="10" fill="url(#glow)"/>'
    s+='<rect x="2.5" y="2.5" width="67" height="67" rx="9.5" fill="none" stroke="#ffffff" stroke-opacity="0.08"/>'
    s+=body
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
out["master-key"]=tile('<rect x="12" y="6" width="48" height="52" rx="7" fill="none" stroke="#3a434c" stroke-width="2"/>'+fader(36,13,51,(24,G),cw=22,ch=11,w=4.4))
out["volume-key"]=three((34,22,40),G,"url(#cap)")
out["preset-key"]=three((28,38,24),"#3a434c","#7a858f")
out["preset-active"]=three((28,38,24),G,"url(#cap)",G,G)
out["mute-on"]=tile(speaker(G)+f'<g fill="none" stroke="{G}" stroke-width="3.4" stroke-linecap="round"><path d="M41 27a9 9 0 0 1 0 14"/><path d="M46 21a17 17 0 0 1 0 26"/></g>')
out["mute-off"]=tile(f'<rect x="2" y="2" width="68" height="68" rx="10" fill="#5a1f12" opacity="0.5"/>'+speaker(OR)+f'<g stroke="{OR}" stroke-width="3.8" stroke-linecap="round"><line x1="43" y1="26" x2="57" y2="42"/><line x1="57" y1="26" x2="43" y2="42"/></g>',OR,OR)
hp=lambda c:f'<path d="M18 40v-6a18 18 0 0 1 36 0v6" fill="none" stroke="{c}" stroke-width="4" stroke-linecap="round"/><rect x="13" y="36" width="11" height="18" rx="4" fill="{c}"/><rect x="48" y="36" width="11" height="18" rx="4" fill="{c}"/>'
out["device-key"]=tile(hp("#8a959f"))
out["device-active"]=tile(hp(G)+f'<rect x="22" y="58" width="28" height="3" rx="1.5" fill="{G}"/>',G,G)
for k,v in out.items(): pathlib.Path(f"icons/{k}.svg").write_text(v+"\n")
