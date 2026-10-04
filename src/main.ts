import type { Bucket, DesktopState, MonitorState, Settings } from './types.ts';
import { duration, percent, resetTime, selectedBucket, resetCount, resetExpiryRows } from './format.ts';

const root = document.querySelector<HTMLDivElement>('#app')!;
root.innerHTML = `
<header class="header"><div class="brand"><img class="brand-icon" src="/brand.png" alt="" width="48" height="48"><div><h1>Codex <span>Pulse</span></h1><p>작업의 흐름을 이어가는 사용량 모니터</p></div></div><button id="refresh" class="button secondary" type="button">새로고침</button></header>
<nav class="tabs" aria-label="화면 선택"><button class="tab active" data-tab="overview" aria-pressed="true">사용량</button><button class="tab" data-tab="settings" aria-pressed="false">표시 설정</button><button id="console-tab" class="tab" data-tab="console" aria-pressed="false" hidden>콘솔</button></nav>
<main>
<div id="notice" class="notice" role="status" hidden></div>
<section id="overview" class="panel">
<div class="overview-top"><div><span class="eyebrow">LIVE USAGE</span><h2>다음 작업을 위한 여유</h2></div><span id="connection" class="status" aria-live="polite">연결 중</span></div>
<p id="updated" class="muted">첫 사용량 정보를 기다리고 있습니다.</p>
<div id="pulse-summary" class="pulse-summary" hidden><span class="eyebrow">REMAINING CAPACITY</span><div class="pulse-value"><strong id="pulse-remaining">—</strong><span>잔여 한도</span></div><p id="pulse-reset"></p><p id="pulse-freshness" class="muted"></p></div>
<div id="buckets" class="bucket-list"></div>
<div id="reset-summary" class="reset-summary" hidden><div><strong>남은 초기화</strong><p class="muted small">사용 가능한 한도 초기화 횟수</p><div id="reset-expiry" class="muted small"></div></div><strong id="reset-count"></strong></div>
<div id="tokens" class="token-summary" hidden></div>
<div class="footnote"><span class="tiny-dot" aria-hidden="true"></span><p>서버가 제공한 사용 한도입니다. 변경 알림과 주기적 조회로 갱신됩니다.</p></div>
</section>
<section id="settings" class="panel" hidden>
<span class="eyebrow">YOUR WORKSPACE</span><h2>원하는 곳에서 확인하세요</h2><p class="muted">표시 방식은 여러 개를 함께 사용할 수 있습니다.</p>
<form id="settings-form">
<div class="settings-grid">
<label class="setting-card"><span><strong>상태창</strong><small>모든 한도와 초기화 시각</small></span><input name="showWindow" type="checkbox" role="switch"></label>
<label class="setting-card"><span><strong>시스템 트레이</strong><small>선택한 한도의 잔여율</small></span><input name="showTray" type="checkbox" role="switch"></label>
<label id="touchbar-setting" class="setting-card"><span><strong>Touch Bar</strong><small>지원 MacBook Pro · 앱 활성화 시</small></span><input name="showTouchbar" type="checkbox" role="switch"></label>
<label class="setting-card"><span><strong>앱 내부 콘솔</strong><small>연결 및 갱신 상태 기록</small></span><input name="showConsole" type="checkbox" role="switch"></label>
</div>
<div class="settings-detail"><label><span id="representative-label">대표 한도</span><select name="selectedLimitId"><option value="">첫 번째 사용 가능한 한도</option></select></label><label>조회 주기 (초)<input name="intervalSecs" type="number" min="5" max="3600" required></label><label class="wide">Codex 실행 파일 경로<input name="codexPath" type="text" required spellcheck="false" placeholder="Codex 실행 파일의 전체 경로"></label><label class="inline wide"><input name="alwaysOnTop" type="checkbox">상태창을 항상 위에 표시</label></div>
<p class="muted small">터미널에서도 <code>codex-usage --watch</code>로 확인할 수 있습니다. 시스템 트레이를 켜면 창을 닫아도 조회가 계속됩니다.</p>
<div class="form-footer"><span id="save-status" role="status"></span><button id="save" class="button primary" type="submit">설정 저장</button></div>
</form></section>
<section id="console" class="panel" hidden><div class="overview-top"><div><span class="eyebrow">ACTIVITY</span><h2>연결 · 갱신 기록</h2></div><button id="clear-console" class="button secondary">기록 지우기</button></div><p class="muted small">현재 실행의 최근 200개 상태만 보관합니다.</p><pre id="console-output" aria-live="polite"></pre></section>
</main><footer class="footer"><span>Codex Pulse <span class="footer-author">by nanamix</span></span><span><span id="platform-name">macOS · Linux · Windows</span></span></footer>`;

function el<T extends HTMLElement = HTMLElement>(id: string): T { return document.getElementById(id) as T; }
const form = el<HTMLFormElement>('settings-form');
const api = window.__TAURI__;
let state: DesktopState | undefined;
let dirty = false;
let saving = false;
let logs: string[] = [];
let lastLog = '';
const statusLabels: Record<MonitorState['status'], string> = { connecting: '연결 중', ready: '연결됨', loginRequired: '로그인 필요', unsupportedAuth: '지원하지 않는 인증', error: '연결 오류', stopped: '종료됨' };
function showNotice(text: string | null) { el('notice').hidden = !text; el('notice').textContent = text; }
function selectTab(name: string) {
  if (name === 'console' && !state?.settings.showConsole) name = 'overview';
  for (const panel of ['overview', 'settings', 'console']) el(panel).hidden = panel !== name;
  for (const tab of document.querySelectorAll<HTMLButtonElement>('[data-tab]')) {
    tab.classList.toggle('active', tab.dataset.tab === name);
    tab.setAttribute('aria-pressed', String(tab.dataset.tab === name));
  }
}
for (const tab of document.querySelectorAll<HTMLButtonElement>('[data-tab]')) tab.addEventListener('click', () => selectTab(tab.dataset.tab!));
function textNode(tag: string, text: string, className?: string): HTMLElement {
  const node = document.createElement(tag); node.textContent = text; if (className) node.className = className; return node;
}
function renderBucket(bucket: Bucket, representative: boolean): HTMLElement {
  const card = document.createElement('article'); card.className = 'bucket-card';
  const header = document.createElement('div'); header.className = 'bucket-header';
  const name = document.createElement('div'); name.append(textNode('h3', bucket.name || bucket.id));
  if (bucket.planType) name.append(textNode('span', bucket.planType.toUpperCase(), 'plan'));
  header.append(name); if (representative) header.append(textNode('span', '대표 한도', 'badge')); card.append(header);
  let displayed = false;
  for (const [label, window] of [['기본 한도', bucket.primary], ['추가 한도', bucket.secondary]] as const) {
    if (!window) continue;
    displayed = true;
    const section = document.createElement('div'); section.className = 'quota';
    const row = document.createElement('div'); row.className = 'quota-top';
    row.append(textNode('span', `${label} · ${duration(window.windowDurationMins)}`, 'muted'));
    const remaining = Math.max(0, Math.min(100, 100 - window.usedPercent));
    row.append(textNode('strong', `잔여 ${percent(remaining)}`, remaining <= 10 ? 'danger' : 'remaining'));
    const progress = document.createElement('progress'); progress.max = 100; progress.value = window.usedPercent;
    progress.setAttribute('aria-label', `${label} 사용률 ${percent(window.usedPercent)}`);
    if (remaining <= 10) progress.classList.add('critical');
    section.append(row, progress, textNode('p', `사용 ${percent(window.usedPercent)} · ${resetTime(window.resetsAt)} 초기화`, 'quota-meta'));
    card.append(section);
  }
  if (!displayed) card.append(textNode('p', '이 한도의 사용률 정보가 제공되지 않았습니다.', 'muted'));
  if (bucket.credits) card.append(textNode('p', bucket.credits.unlimited ? '크레딧: 무제한' : `크레딧 잔액: ${bucket.credits.balance ?? '정보 없음'}`, 'quota-meta'));
  return card;
}
function renderUsage() {
  if (!state) return;
  const usage = state.usage;
  const platform = state.platform;
  el('platform-name').textContent = ({ macos: 'macOS', linux: 'Linux', windows: 'Windows' } as Record<string, string>)[platform] ?? platform;
  el('touchbar-setting').hidden = platform !== 'macos';
  el('representative-label').textContent = platform === 'macos' ? '시스템 트레이 · Touch Bar 대표 한도' : '시스템 트레이 대표 한도';
  field('showTouchbar').disabled = platform !== 'macos';
  const representative = usage.snapshot && selectedBucket(usage.snapshot.buckets, state.settings.selectedLimitId);
  const window = representative?.primary ?? representative?.secondary;
  el('pulse-summary').hidden = !window;
  el('pulse-remaining').textContent = window ? percent(Math.max(0, Math.min(100, 100 - window.usedPercent))) : '—';
  el('pulse-reset').textContent = window ? `${duration(window.windowDurationMins)} 한도 · ${resetTime(window.resetsAt)} 초기화` : '';
  el('pulse-freshness').textContent = usage.stale ? '이전 조회 값입니다. 연결을 확인해 주세요.' : '선택한 대표 한도의 서버 조회 값';
  el('pulse-summary').classList.toggle('stale', usage.stale);
  el('connection').textContent = statusLabels[usage.status];
  el('connection').className = `status ${usage.status === 'ready' ? 'connected' : 'disconnected'}`;
  showNotice(usage.message || state.startupWarning || null);
  el('updated').textContent = usage.snapshot ? `${usage.stale ? '이전 데이터 · ' : ''}마지막 성공 조회 ${new Date(usage.snapshot.fetchedAt * 1000).toLocaleString('ko-KR')}` : '사용량 정보가 아직 없습니다.';
  const buckets = el('buckets'); buckets.replaceChildren();
  if (usage.snapshot) {
    const selected = selectedBucket(usage.snapshot.buckets, state.settings.selectedLimitId);
    for (const bucket of usage.snapshot.buckets) buckets.append(renderBucket(bucket, bucket.id === selected?.id));
    if (usage.snapshot.ordinaryUsageAllowed === false) buckets.prepend(textNode('p', '서버에서 일반 사용을 허용하지 않는 상태입니다.', 'notice'));
  } else {
    const empty = document.createElement('div'); empty.className = 'empty';
    empty.append(textNode('h3', '사용량을 확인할 준비 중입니다'), textNode('p', 'Codex CLI 설치와 로그인 상태를 확인해 주세요.\n로그인이 필요하면 터미널에서 codex login을 실행하세요.', 'muted'));
    buckets.append(empty);
  }
  const summary = usage.snapshot?.tokenSummary;
  el('reset-summary').hidden = !usage.snapshot;
  el('reset-count').textContent = resetCount(usage.snapshot?.remainingResetCount);
  el('reset-expiry').replaceChildren(...resetExpiryRows(usage.snapshot?.remainingResetCount, usage.snapshot?.resetCreditExpirations).map(row => textNode('p', row)));
  el('tokens').hidden = !summary;
  el('tokens').replaceChildren();
  if (summary) for (const [label, value] of [['누적 토큰', summary.lifetimeTokens], ['최대 일일 토큰', summary.peakDailyTokens], ['연속 사용일', summary.currentStreakDays]] as const) {
    const item = document.createElement('div'); item.append(textNode('small', label), textNode('strong', value == null ? '정보 없음' : value.toLocaleString('ko-KR'))); el('tokens').append(item);
  }
  el('console-tab').hidden = !state.settings.showConsole;
  if (!state.settings.showConsole && !el('console').hidden) selectTab('overview');
}
function field(name: string): HTMLInputElement { return form.elements.namedItem(name) as HTMLInputElement; }
function populateSettings(settings: Settings) {
  for (const name of ['showWindow', 'showTray', 'showConsole', 'showTouchbar', 'alwaysOnTop'] as const) field(name).checked = settings[name];
  field('codexPath').value = settings.codexPath; field('intervalSecs').value = String(settings.intervalSecs);
  const select = form.elements.namedItem('selectedLimitId') as HTMLSelectElement;
  select.replaceChildren(new Option('첫 번째 사용 가능한 한도', ''));
  const buckets = state?.usage.snapshot?.buckets ?? [];
  for (const b of buckets) select.add(new Option(b.name || b.id, b.id));
  if (settings.selectedLimitId && !buckets.some(b => b.id === settings.selectedLimitId)) select.add(new Option(`${settings.selectedLimitId} (현재 없음 · 첫 한도로 대체)`, settings.selectedLimitId));
  select.value = settings.selectedLimitId || '';
}
function receive(next: DesktopState) {
  state = next; renderUsage(); if (!dirty && !saving) populateSettings(next.settings);
  const key = `${next.usage.status}:${next.usage.snapshot?.fetchedAt ?? ''}:${next.usage.message ?? ''}`;
  if (key !== lastLog) {
    lastLog = key;
    logs.push(`${new Date().toLocaleTimeString('ko-KR')}  ${statusLabels[next.usage.status]}${next.usage.stale ? ' · 오래된 데이터' : ''}${next.usage.message ? ` · ${next.usage.message}` : ''}`);
    logs = logs.slice(-200); el('console-output').textContent = logs.join('\n');
  }
}
form.addEventListener('input', () => { dirty = true; el('save-status').textContent = '저장하지 않은 변경이 있습니다.'; });
form.addEventListener('submit', async event => {
  event.preventDefault(); if (!api || saving) return;
  saving = true; el<HTMLButtonElement>('save').disabled = true;
  const settings: Settings = {
    showWindow: field('showWindow').checked, showTray: field('showTray').checked,
    showConsole: field('showConsole').checked, showTouchbar: !field('showTouchbar').disabled && field('showTouchbar').checked,
    alwaysOnTop: field('alwaysOnTop').checked, selectedLimitId: field('selectedLimitId').value || null,
    intervalSecs: Number(field('intervalSecs').value), codexPath: field('codexPath').value.trim(),
  };
  try {
    const saved = await api.core.invoke<Settings>('save_settings', { settings });
    dirty = false; if (state) { state.settings = saved; renderUsage(); populateSettings(saved); }
    el('save-status').textContent = '설정을 저장했습니다.';
  } catch (error) { el('save-status').textContent = String(error); }
  finally { saving = false; el<HTMLButtonElement>('save').disabled = false; }
});
el('refresh').addEventListener('click', async () => {
  if (!api) return;
  const button = el<HTMLButtonElement>('refresh'); button.disabled = true;
  try { await api.core.invoke('refresh_usage'); }
  catch (error) { showNotice(String(error)); }
  finally { setTimeout(() => { button.disabled = false; }, 800); }
});
el('clear-console').addEventListener('click', () => { logs = []; lastLog = ''; el('console-output').textContent = ''; });
async function start() {
  if (!api) {
    el('touchbar-setting').hidden = true;
    showNotice('브라우저 미리보기입니다. 실제 Codex 사용량은 Tauri 앱에서 연결됩니다.');
    el<HTMLButtonElement>('refresh').disabled = true; el<HTMLButtonElement>('save').disabled = true;
    el('buckets').append(textNode('div', '실제 계정 데이터가 아직 연결되지 않았습니다.', 'empty'));
    el('connection').textContent = '미리보기'; return;
  }
  try {
    // Subscribe before reading, then read the latest state again if an event races initialization.
    let eventVersion = 0;
    const unlisten = await api.event.listen<DesktopState>('usage-state', event => { eventVersion++; receive(event.payload); });
    const version = eventVersion;
    const initial = await api.core.invoke<DesktopState>('get_state');
    if (version === eventVersion) receive(initial);
    window.addEventListener('beforeunload', unlisten, { once: true });
  } catch (error) { showNotice(`앱 상태를 읽지 못했습니다: ${String(error)}`); }
}
void start();
