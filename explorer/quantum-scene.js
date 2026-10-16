// Projection is visual only. Probabilities always originate in the Rust worker.
const canvas=document.getElementById('quantum-scene'), ctx=canvas.getContext('2d');
const wave=document.getElementById('phase-scene'), wc=wave.getContext('2d');
let states=[], yaw=.35, pitch=.3, dragging=null, clock=0, previous=0, dirty=true;
const tau=Math.PI*2, reduced=matchMedia('(prefers-reduced-motion: reduce)');
export function updateConstellation(probabilities,n){
  states=probabilities.map((p,i)=>({p,label:i.toString(2).padStart(n,'0'),i})).filter(s=>s.p>1e-12);
  document.getElementById('spatial-status').textContent=`${n} qubits · ${states.length} populated basis states · exact WASM probabilities`;
  dirty=true;
}
function fit(c){const r=c.getBoundingClientRect(),d=Math.min(devicePixelRatio||1,2);if(c.width!==Math.round(r.width*d)||c.height!==Math.round(r.height*d)){c.width=Math.round(r.width*d);c.height=Math.round(r.height*d);}const context=c.getContext('2d');context?.setTransform(d,0,0,d,0,0);return [r.width,r.height];}
function project(x,y,z,w,h){const a=x*Math.cos(yaw)-z*Math.sin(yaw),b=x*Math.sin(yaw)+z*Math.cos(yaw),v=y*Math.cos(pitch)-b*Math.sin(pitch),depth=y*Math.sin(pitch)+b*Math.cos(pitch),scale=1/(1+depth*.28);return {x:w/2+a*Math.min(w*.32,h*.34)*scale,y:h/2+v*Math.min(w*.32,h*.34)*scale,depth,scale};}
function scene(){if(!ctx)return;const [w,h]=fit(canvas);ctx.clearRect(0,0,w,h);if(!w||!h)return;
  for(let ring=0;ring<3;ring++){ctx.beginPath();for(let j=0;j<=100;j++){const a=j/100*tau;const p=project(Math.cos(a),ring===0?0:Math.sin(a),ring===0?Math.sin(a):ring===1?0:Math.cos(a)*.5,w,h);j?ctx.lineTo(p.x,p.y):ctx.moveTo(p.x,p.y);}ctx.strokeStyle='#63dce030';ctx.lineWidth=1;ctx.stroke();}
  const nodes=states.map((s,k)=>{const a=k/Math.max(states.length,1)*tau;const y=states.length>2?1-2*(k+.5)/states.length:0;const r=Math.sqrt(1-y*y);return {...s,...project(Math.cos(a)*r,y,Math.sin(a)*r,w,h)};}).sort((a,b)=>b.depth-a.depth);
  for(const node of nodes){const radius= Math.max(3,Math.sqrt(node.p)*Math.min(w*.12,48))*node.scale;ctx.beginPath();ctx.moveTo(w/2,h/2);ctx.lineTo(node.x,node.y);ctx.strokeStyle='#71cce332';ctx.stroke();const g=ctx.createRadialGradient(node.x,node.y,0,node.x,node.y,radius*2.4);g.addColorStop(0,'#75f3ed80');g.addColorStop(1,'#75f3ed00');ctx.fillStyle=g;ctx.beginPath();ctx.arc(node.x,node.y,radius*2.4,0,tau);ctx.fill();ctx.beginPath();ctx.arc(node.x,node.y,radius,0,tau);ctx.fillStyle='#74e8e5';ctx.fill();ctx.strokeStyle='#deffff';ctx.stroke();if(states.length<=16){ctx.fillStyle='#e4f8ff';ctx.font='12px monospace';ctx.textAlign='center';ctx.fillText(`|${node.label}⟩ ${(node.p*100).toFixed(1)}%`,node.x,node.y+radius+20);}}
  if(!states.length){ctx.textAlign='center';ctx.fillStyle='#9db6cd';ctx.font='15px sans-serif';ctx.fillText('Run a circuit to reveal its state',w/2,h/2);}
}
const slider=document.getElementById('phase-angle');
function waves(){if(!wc)return;const [w,h]=fit(wave);wc.clearRect(0,0,w,h);const phi=Number(slider.value)/100*Math.PI;for(let k=0;k<3;k++){wc.beginPath();for(let x=0;x<=w;x+=2){const a=x/Math.max(w,1)*tau*2-clock;const value=k===0?Math.sin(a):k===1?Math.sin(a+phi):(Math.sin(a)+Math.sin(a+phi))/2;const y=(k+1)*h/4+value*19;x?wc.lineTo(x,y):wc.moveTo(x,y);}wc.strokeStyle=['#6ee8ed','#ae97ff','#f1ca79'][k];wc.lineWidth=2;wc.stroke();} }
slider.addEventListener('input',()=>{const a=Number(slider.value)/100;document.getElementById('phase-value').textContent=`${a.toFixed(2)}π`;const p=Math.cos(a*Math.PI/2)**2;document.getElementById('phase-explanation').textContent=p>.999?'The waves line up. The result is always 0.':p<.001?'The waves cancel for 0. The result is always 1.':`Both answers are possible: about ${Math.round(p*100)} in 100 results would be 0 over many fresh runs.`;document.getElementById('phase-probability').textContent=`P(0) ${(p*100).toFixed(1)}% · P(1) ${((1-p)*100).toFixed(1)}%`;dirty=true;});
canvas.addEventListener('pointerdown',e=>{dragging={x:e.clientX,y:e.clientY};canvas.setPointerCapture(e.pointerId);});
canvas.addEventListener('pointermove',e=>{if(!dragging)return;yaw+=(e.clientX-dragging.x)*.008;pitch=Math.max(-1.2,Math.min(1.2,pitch+(e.clientY-dragging.y)*.008));dragging={x:e.clientX,y:e.clientY};dirty=true;});
for(const name of ['pointerup','pointercancel','lostpointercapture'])canvas.addEventListener(name,()=>dragging=null);
canvas.addEventListener('keydown',e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;e.preventDefault();yaw+=e.key==='ArrowLeft'?-.12:e.key==='ArrowRight'?.12:0;pitch=Math.max(-1.2,Math.min(1.2,pitch+(e.key==='ArrowUp'?-.12:e.key==='ArrowDown'?.12:0)));dirty=true;});
document.getElementById('reset-camera').onclick=()=>{yaw=.35;pitch=.3;dirty=true;};
new ResizeObserver(()=>dirty=true).observe(canvas);
new ResizeObserver(()=>dirty=true).observe(wave);
function frame(t){requestAnimationFrame(frame);if(document.hidden||canvas.closest('.view').hidden){previous=t;return;}const paused=document.body.classList.contains('motion-paused');const dt=Math.min((t-previous)/1000,.05);if(t-previous<33&&!dirty)return;previous=t;if(!paused){clock+=dt*.9;if(!dragging)yaw+=dt*.07;}if(dirty||!paused){scene();waves();dirty=false;}}
requestAnimationFrame(frame);
