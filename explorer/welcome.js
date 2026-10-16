// Native dialog provides focus containment, Escape dismissal and inert background.
const dialog=document.getElementById('welcome');
const title=document.getElementById('welcome-title'),copy=document.getElementById('welcome-copy');
const next=document.getElementById('welcome-next'),back=document.getElementById('welcome-back');
const steps=[
  ['A tiny lab for quantum ideas.','Build a few instructions, run them, and see the chances of each answer. No physics background needed. Everything runs in your browser.'],
  ['One experiment. Two possible answers.','The Bell example links two qubits. When measured, they both give 0 or both give 1, each with a 50% chance. The illustration shows possible outcomes, not particles travelling.'],
  ['Change a step. See what changes.','Start with the Bell pair. Press Run circuit to calculate its chances. Then try the interference example to see how combining amplitudes can change the answer.']
];
let step=0,returnFocus;
function render(){title.textContent=steps[step][0];copy.textContent=steps[step][1];dialog.dataset.step=step;document.getElementById('welcome-progress').textContent=`${step+1} of 3`;document.querySelectorAll('[data-welcome-art]').forEach(e=>e.toggleAttribute('hidden',Number(e.dataset.welcomeArt)!==step));back.disabled=step===0;next.textContent=step===2?'Try Bell pair ↗':'Next →';}
function remember(){try{localStorage.setItem('ruqu-welcome-v1','seen');}catch{/* Tour remains available without storage. */}}
function open(){if(dialog.open)return;returnFocus=document.activeElement;step=0;render();dialog.showModal();document.body.classList.add('welcome-open');document.getElementById('welcome-close').focus();}
function close(){remember();dialog.close();}
next.onclick=()=>{if(step<2){step++;render();return;}close();document.dispatchEvent(new Event('ruqu-welcome-bell'));};
back.onclick=()=>{if(step>0){step--;render();}};
document.getElementById('welcome-close').onclick=close;
document.getElementById('welcome-replay').onclick=open;
document.getElementById('welcome-motion').onclick=()=>{const paused=dialog.classList.toggle('tour-paused');document.getElementById('welcome-motion').textContent=paused?'Play animation':'Pause animation';document.getElementById('welcome-motion').setAttribute('aria-pressed',String(paused));};
dialog.addEventListener('cancel',remember);
dialog.addEventListener('close',()=>{document.body.classList.remove('welcome-open');returnFocus?.focus();});
let seen=false;try{seen=localStorage.getItem('ruqu-welcome-v1')==='seen';}catch{}
if(!seen)open();
