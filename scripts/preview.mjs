import { readFile, writeFile } from 'node:fs/promises';
const html = await readFile('dist/index.html', 'utf8');
const brand = (await readFile('dist/brand.png')).toString('base64');
const css = await readFile('dist/src/style.css', 'utf8');
const format = await readFile('dist/src/format.js', 'utf8');
const main = (await readFile('dist/src/main.js', 'utf8')).replace('/brand.png', `data:image/png;base64,${brand}`);
// Browser-only preview has no Tauri bridge and never simulates account usage.
const script = format.replace(/export /g, '') + '\n' + main.replace(/^import .*?;\s*/gm, '');
await writeFile('dist/preview.html', html.replace('<link rel="stylesheet" href="/src/style.css">', `<style>${css}</style>`).replace('<script type="module" src="/src/main.js"></script>', `<script type="module">${script}</script>`));
console.log('브라우저 단일 파일 미리보기: dist/preview.html');
