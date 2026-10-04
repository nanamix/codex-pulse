import { stripTypeScriptTypes } from 'node:module';
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
await mkdir('dist/src', { recursive: true });
for (const file of ['main', 'format', 'types']) {
  const source = await readFile(`src/${file}.ts`, 'utf8');
  const output = stripTypeScriptTypes(source, { mode: 'strip' }).replace(/(from\s+['"][^'"]+)\.ts(['"])/g, '$1.js$2');
  await writeFile(`dist/src/${file}.js`, output);
}
await copyFile('src/style.css', 'dist/src/style.css');
await writeFile('dist/index.html', (await readFile('index.html', 'utf8')).replace('/src/main.ts', '/src/main.js'));
console.log('프런트엔드 정적 파일 생성 완료: dist/');

await copyFile('src-tauri/icons/128x128.png', 'dist/brand.png');
