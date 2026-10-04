import http from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { stripTypeScriptTypes } from 'node:module';
const root = process.cwd();
const mime = { '.html': 'text/html', '.css': 'text/css', '.ts': 'text/javascript', '.js': 'text/javascript', '.png': 'image/png' };
const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost');
    const relative = url.pathname === '/' ? '/index.html' : decodeURIComponent(url.pathname);
    // Serve only frontend files. Never expose settings, operations, or arbitrary source directories.
    if (relative !== '/index.html' && relative !== '/brand.png' && !/^\/src\/(main|format|types)\.ts$|^\/src\/style\.css$/.test(relative)) { res.writeHead(404); res.end('Not found'); return; }
    const file = relative === '/brand.png' ? path.join(root, 'src-tauri/icons/128x128.png') : path.join(root, relative);
    let data = await readFile(file);
    const ext = path.extname(file);
    if (ext === '.ts') data = Buffer.from(stripTypeScriptTypes(data.toString(), { mode: 'strip' }));
    res.writeHead(200, { 'Content-Type': `${mime[ext] ?? 'application/octet-stream'}; charset=utf-8`, 'Cache-Control': 'no-store' }); res.end(data);
  } catch { res.writeHead(404); res.end('Not found'); }
});
server.on('error', error => { console.error(error.message); process.exitCode = 1; });
server.listen(1420, '127.0.0.1', () => console.log('개발 화면: http://127.0.0.1:1420 (새로고침으로 변경 반영)'));
