// Lifeboat's window. All the work happens in the Rust side (lifeboat-core); this only shows it.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;
const $ = (id) => document.getElementById(id);

let rows = [];
let lastSummary = null;

function base(p) { return String(p).split(/[\\/]/).pop(); }
function showDrop() { $('drop').hidden = false; $('work').hidden = true; }
function showWork() { $('drop').hidden = true; $('work').hidden = false; }

async function convert(paths) {
  if (!paths || !paths.length) return;
  await invoke('convert', { paths, format: $('format').value });
}

function reset(total) {
  rows = [];
  $('files').innerHTML = '';
  $('summary').hidden = true;
  $('bar').max = Math.max(total, 1);
  $('bar').value = 0;
  $('statusText').textContent = total ? 'Converting…' : 'No FreeHand files found.';
  $('count').textContent = total ? `0 of ${total}` : '';
  showWork();
}

function row(index, input) {
  let li = rows[index];
  if (!li) {
    li = document.createElement('li');
    li.innerHTML = '<div><span class="tag"></span><span class="name"></span></div><div class="why"></div>';
    $('files').appendChild(li);
    rows[index] = li;
  }
  li.querySelector('.name').textContent = base(input);
  li.title = input;
  return li;
}

function setRow(li, kind, label, why) {
  li.className = kind;
  li.querySelector('.tag').textContent = label;
  li.querySelector('.why').textContent = why || '';
}

function onEvent(e) {
  switch (e.event) {
    case 'start': reset(e.total); break;
    case 'begin': { const li = row(e.index, e.input); setRow(li, 'working', 'Working', ''); li.scrollIntoView({ block: 'nearest' }); break; }
    case 'done': {
      const o = e.outcome, li = row(e.index, o.input);
      if (o.status === 'converted') setRow(li, 'converted', 'Done', 'Saved as ' + o.outputs.map(base).join(', ') + (o.notes.length ? ' · ' + o.notes.join(' ') : ''));
      else if (o.status === 'skipped') setRow(li, 'skipped', 'Skipped', o.reason);
      else setRow(li, 'failed', 'Failed', o.reason);
      $('bar').value = e.index + 1;
      $('count').textContent = `${e.index + 1} of ${e.total}`;
      break;
    }
  }
}

function plural(n, one, many) { return n === 1 ? `1 ${one}` : `${n} ${many}`; }

function onSummary(s) {
  lastSummary = s;
  // Problems found before the batch started (a missing folder) get rows too.
  const listed = new Set(rows.filter(Boolean).map((li) => li.title));
  for (const o of [...s.skipped, ...s.failed]) {
    if (!listed.has(o.input)) { const li = row(rows.length, o.input); setRow(li, o.status, o.status === 'failed' ? 'Failed' : 'Skipped', o.reason); }
  }
  const c = s.converted.length, k = s.skipped.length, f = s.failed.length;
  $('statusText').textContent = 'Finished';
  $('summaryTitle').textContent = `${plural(c, 'file', 'files')} converted` + (k ? `, ${k} skipped` : '') + (f ? `, ${f} failed` : '');
  $('summaryNote').textContent = (c ? 'Each new file is next to its original. ' : '') +
    (s.ignored_in_folders ? `${plural(s.ignored_in_folders, 'other file', 'other files')} in the folders weren't FreeHand drawings and were left alone.` : '');
  $('openFolder').hidden = !s.output_folder;
  $('summary').hidden = false;
}

$('choose').addEventListener('click', async () => {
  const picked = await dialog.open({ multiple: true, directory: false, title: 'Choose FreeHand files' });
  if (picked) convert(Array.isArray(picked) ? picked : [picked]);
});
$('chooseFolder').addEventListener('click', async () => {
  const picked = await dialog.open({ multiple: true, directory: true, title: 'Choose folders with FreeHand files' });
  if (picked) convert(Array.isArray(picked) ? picked : [picked]);
});
$('again').addEventListener('click', showDrop);
$('openFolder').addEventListener('click', () => lastSummary && lastSummary.output_folder && invoke('open_folder', { path: lastSummary.output_folder }));
$('copyReport').addEventListener('click', async () => {
  const text = await invoke('report');
  try { await navigator.clipboard.writeText(text); $('copyReport').textContent = 'Copied'; setTimeout(() => ($('copyReport').textContent = 'Copy report'), 1500); } catch (_) {}
});
$('format').addEventListener('change', () => invoke('set_format', { format: $('format').value }));

window.__TAURI__.webview.getCurrentWebview().onDragDropEvent((ev) => {
  const t = ev.payload.type;
  $('drop').classList.toggle('hover', t === 'enter' || t === 'over');
  // The drop itself is handled in Rust, so it works the same when the window is busy.
});

listen('lifeboat://event', (e) => onEvent(e.payload));
listen('lifeboat://summary', (e) => onSummary(e.payload));

// Files dropped on the app icon before this page loaded: replay what already happened.
invoke('status').then((st) => {
  if (st.format) $('format').value = st.format;
  for (const e of st.events) onEvent(e);
  if (st.summary) onSummary(st.summary);
});
