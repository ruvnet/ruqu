#!/usr/bin/env node
'use strict';
// Browser acceptance tests against the committed Rust WASM, served at a project
// subpath like GitHub Pages. Requires Playwright and its Chromium installation.
// Optional: PLAYWRIGHT_MODULE=/absolute/path/to/playwright
// Optional: CHROMIUM_EXECUTABLE_PATH=/absolute/path/to/chrome-headless-shell
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const os = require('node:os');
const modulePath = process.env.PLAYWRIGHT_MODULE || 'playwright';
let chromium;
try { ({chromium} = require(modulePath)); }
catch { ({chromium} = require('/opt/codex/runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')); }
const root = path.resolve(__dirname, '../explorer');
const screenshotDirectory = process.env.EXPLORER_SCREENSHOTS || path.join(os.tmpdir(), 'ruqu-explorer-qa');
const mime = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.wasm':'application/wasm'};
async function main() {
  const server = http.createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      if (!url.pathname.startsWith('/ruqu/')) { response.writeHead(404).end(); return; }
      const relative = decodeURIComponent(url.pathname.slice('/ruqu/'.length)) || 'index.html';
      const file = path.resolve(root, relative);
      if (!file.startsWith(root + path.sep)) { response.writeHead(403).end(); return; }
      const bytes = await fs.readFile(file);
      response.writeHead(200, {'Content-Type':mime[path.extname(file)] || 'application/octet-stream'}).end(bytes);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  const errors = [];
  const failures = [];
  try {
    browser = await chromium.launch({headless:true, ...(process.env.CHROMIUM_EXECUTABLE_PATH ? {executablePath:process.env.CHROMIUM_EXECUTABLE_PATH} : {})});
    const page = await browser.newPage({viewport:{width:1440,height:1100}, reducedMotion:'reduce'});
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    page.on('response', response => { if(response.status() >= 400) failures.push(`${response.status()} ${response.url()}`); });
    const url = `http://127.0.0.1:${server.address().port}/ruqu/`;
    await page.goto(url);
    await page.locator('#welcome').waitFor({state:'visible'});
    assert.match(await page.locator('#welcome-title').innerText(), /tiny lab/);
    await page.locator('#welcome-next').click();
    assert.match(await page.locator('#welcome-copy').innerText(), /50%/);
    assert.equal(await page.locator('[data-welcome-art="1"]').isVisible(),true);
    assert.equal(await page.locator('[data-welcome-art="0"]').isVisible(),false);
    await page.locator('#welcome-next').click();
    await page.locator('#welcome-back').click();
    await page.locator('#welcome-next').click();
    await page.locator('#welcome-next').click();
    assert.equal(await page.locator('#welcome').isVisible(), false);
    await page.locator('#welcome-replay').click();
    await page.keyboard.press('Escape');
    assert.equal(await page.locator('#welcome').isVisible(), false);
    await page.reload();
    assert.equal(await page.locator('#welcome').isVisible(), false);
    await page.locator('#run-circuit').waitFor({state:'visible'});
    await page.waitForFunction(() => !document.querySelector('#run-circuit').disabled, null, {timeout:30000});
    const circuitLocked = await page.locator('#run-circuit').evaluate(button => {
      button.click();
      return ['qubits','gate','add-gate','reset-circuit'].every(id=>document.getElementById(id).disabled);
    });
    assert.equal(circuitLocked,true);
    await page.waitForFunction(() => !document.querySelector('#run-circuit').disabled && !document.querySelector('#probabilities .empty-state'));
    const bellText = await page.locator('#probabilities').innerText();
    assert.match(bellText,/00/);
    assert.match(bellText,/11/);
    assert.match(bellText,/50(?:\.0*)?\s*%/);
    assert.match(await page.locator('#circuit-meta').innerText(),/2/);
    assert.match(await page.locator('#spatial-status').innerText(), /2 qubits · 2 populated/);
    await page.locator('#phase-angle').fill('100');
    assert.match(await page.locator('#phase-probability').innerText(), /P\(0\) 0.0% · P\(1\) 100.0%/);
    await page.locator('#phase-angle').fill('50');
    assert.match(await page.locator('#phase-probability').innerText(), /P\(0\) 50.0% · P\(1\) 50.0%/);
    await page.locator('#quantum-scene').focus();
    await page.keyboard.press('ArrowRight');
    await page.locator('#reset-camera').click();
    await fs.mkdir(screenshotDirectory,{recursive:true});
    await page.screenshot({path:path.join(screenshotDirectory,'circuit-desktop.png'),fullPage:true});
    for (const [preset, expected] of [['ghz',['|000⟩','|111⟩']],['interference',['|01⟩']]]) {
      await page.locator(`[data-preset="${preset}"]`).click();
      assert.match(await page.locator('#circuit-result-caption').innerText(),/previous run/i);
      await page.locator('#run-circuit').click();
      await page.waitForFunction(() => !document.querySelector('#run-circuit').disabled && !document.querySelector('#circuit-result-caption').textContent.includes('previous run'));
      assert.deepEqual(await page.locator('#probabilities .basis').allTextContents(),expected);
    }
    const gateCount = await page.locator('#gate-list button').count();
    await page.locator('#gate-list button').first().click();
    assert.equal(await page.locator('#gate-list button').count(),gateCount-1);
    await page.locator('#add-gate').click();
    assert.equal(await page.locator('#gate-list button').count(),gateCount);
    await page.locator('#gate').selectOption('cnot');
    await page.locator('#gate-target').selectOption('0');
    await page.locator('#add-gate').click();
    assert.match(await page.locator('#toast').innerText(),/must differ/i);
    assert.equal(await page.locator('#gate-list button').count(),gateCount);
    await page.locator('#reset-circuit').click();
    assert.equal(await page.locator('#gate-list button').count(),0);
    await page.locator('[data-preset="bell"]').click();
    async function navigate(view) {
      const button = page.locator(`[data-view="${view}"]`);
      if (!(await button.isVisible())) await page.locator('#nav-toggle').click();
      await button.click();
      await page.locator(`#view-${view}`).waitFor({state:'visible'});
    }
    async function downloadEvidence(filename) {
      await navigate('evidence');
      assert.equal(await page.locator('#download').isEnabled(),true);
      const promise = page.waitForEvent('download');
      await page.locator('#download').click();
      const download = await promise;
      const file = path.join(screenshotDirectory, filename);
      await download.saveAs(file);
      return JSON.parse(await fs.readFile(file,'utf8'));
    }
    await navigate('factory');
    await page.locator('#factory-shots').selectOption('1000');
    await page.locator('#factory-form button[type=submit]').click();
    await page.waitForFunction(() => !document.querySelector('#factory-results .empty-state') && !document.querySelector('#factory-form button[type=submit]').disabled);
    const factoryText = await page.locator('#factory-results').innerText();
    assert.match(factoryText,/accept/i);
    assert.match(factoryText,/reject/i);
    await page.screenshot({path:path.join(screenshotDirectory,'factory-desktop.png'),fullPage:true});
    const factoryEvidence = await downloadEvidence('factory-evidence.json');
    const factory = factoryEvidence.result;
    assert.equal(factoryEvidence.engine,'ruqu Rust WebAssembly');
    assert.equal(factory.sample.attempts,1000);
    assert.equal(factory.sample.accepted + factory.sample.rejected,1000);
    const t = 1 - 2 * 0.01;
    assert.ok(Math.abs(factory.exact.acceptance_probability - (1 + 15*t**8)/16) < 1e-12);
    const expectedError = (1 - 15*t**7 + 15*t**8 - t**15)/(2*(1 + 15*t**8));
    assert.ok(Math.abs(factory.exact.conditional_output_error - expectedError) < 1e-12);
    await navigate('factory');
    // Native browser validation must prevent out of range input.
    await page.locator('#factory-error').fill('0.9');
    assert.equal(await page.locator('#factory-error').evaluate(element => element.checkValidity()),false);
    await page.locator('#factory-error').fill('0.01');
    await navigate('cultivation');
    await page.locator('#cultivation-attempts').selectOption('1');
    for(const field of ['#cultivation-gate','#cultivation-prep','#cultivation-measure']) await page.locator(field).fill('0');
    const cultivationLocked = await page.locator('#cultivation-form').evaluate(form => {
      form.requestSubmit();
      return ['qubits','gate','add-gate','reset-circuit','cultivation-attempts','factory-error'].every(id=>document.getElementById(id).disabled);
    });
    assert.equal(cultivationLocked,true);
    await page.waitForFunction(() => !document.querySelector('#cultivation-results .empty-state') && !document.querySelector('#cultivation-form button[type=submit]').disabled, null, {timeout:120000});
    assert.equal(await page.locator('#qubits').isEnabled(),true);
    const cultivationText = await page.locator('#cultivation-results').innerText();
    assert.match(cultivationText,/accept/i);
    assert.match(cultivationText,/fidelity/i);
    await page.screenshot({path:path.join(screenshotDirectory,'cultivation-desktop.png'),fullPage:true});
    const cultivationEvidence = await downloadEvidence('cultivation-evidence.json');
    const cultivation = cultivationEvidence.result;
    assert.equal(cultivation.accepted + cultivation.rejected,1);
    assert.equal(cultivation.accepted,1);
    assert.ok(cultivation.mean_accepted_fidelity > 0.999999);
    // The worker independently refuses malformed requests if native form
    // validation is bypassed. No numerical result may accompany this error.
    const invalidReply = await page.evaluate(() => new Promise((resolve,reject) => {
      const worker = new Worker('./worker.js',{type:'module'});
      const timeout = setTimeout(() => {worker.terminate();reject(Error('validation worker timeout'));},30000);
      worker.onmessage = ({data}) => {
        if(data.type === 'ready') worker.postMessage({id:1,type:'research',request:{op:'magic-factory',input_error:0.9,shots:1000,rounds:1,seed:'42'}});
        else {clearTimeout(timeout);worker.terminate();resolve(data);}
      };
      worker.onerror = error => {clearTimeout(timeout);worker.terminate();reject(Error(error.message));};
    }));
    assert.equal(invalidReply.type,'error');
    assert.equal(invalidReply.result,undefined);
    await navigate('cultivation');
    await page.locator('#cultivation-attempts').selectOption('32');
    const cancellationUnlocked = await page.locator('#cultivation-form').evaluate(form => {
      form.requestSubmit();
      const locked = document.getElementById('qubits').disabled;
      document.getElementById('cancel').click();
      return locked && !document.getElementById('qubits').disabled;
    });
    assert.equal(cancellationUnlocked,true);
    await page.waitForFunction(() => !document.querySelector('#run-circuit').disabled);
    assert.equal(await page.locator('#cultivation-attempts').isEnabled(),true);
    await page.setViewportSize({width:320,height:800});
    for(const view of ['circuit','factory','cultivation','evidence']) {
      await navigate(view);
      const bounds = await page.evaluate(() => ({viewport:innerWidth,width:document.documentElement.scrollWidth}));
      assert.ok(bounds.width <= bounds.viewport + 1, `${view} overflows mobile width: ${JSON.stringify(bounds)}`);
    }
    await page.screenshot({path:path.join(screenshotDirectory,'mobile-320.png'),fullPage:true});
    await page.locator('#welcome-replay').click();
    assert.ok(await page.locator('#welcome').evaluate(e=>e.scrollWidth<=e.clientWidth+1));
    await page.locator('#welcome-next').click();
    await page.screenshot({path:path.join(screenshotDirectory,'welcome-mobile.png')});
    await page.locator('#welcome-close').click();
    assert.equal(await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches),true);
    assert.equal(await page.locator('#motion').getAttribute('aria-pressed'),'true');
    assert.deepEqual(errors,[],`Browser errors: ${errors.join('\n')}`);
    assert.deepEqual(failures,[],`HTTP failures: ${failures.join('\n')}`);
    console.log(JSON.stringify({pass:true,base_path:'/ruqu/',checks:['welcome navigation, replay, Escape and persistence','welcome 320px layout','Bell circuit','spatial WASM state','phase interference endpoints','keyboard camera','GHZ and interference','gate add remove reset','CNOT validation','factory','native validation','cultivation','evidence download','busy control locks','cancel worker restart','320px all views','reduced motion','console errors','HTTP errors'],screenshots:screenshotDirectory,factory_text:factoryText,cultivation_text:cultivationText},null,2));
  } finally {
    if(browser) await browser.close();
    await new Promise(resolve=>server.close(resolve));
  }
}
main().catch(error=>{ console.error(error); process.exitCode=1; });
