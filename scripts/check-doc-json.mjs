// 校验 docs/ 下所有 Markdown 里的 ```json 代码块都能被 JSON.parse 解析。
//
// 契约文档是「代码向文档看齐」的唯一真源，文档里的示例 JSON 一旦写坏，
// 前端与后端就会照着坏样例实现，因此把它纳入可自动执行的检查。
//
// 用法：node scripts/check-doc-json.mjs [目录...]

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, extname, relative } from 'node:path';

const roots = process.argv.slice(2);
if (roots.length === 0) roots.push('docs');

function collect(target, out = []) {
  const stat = statSync(target);
  if (stat.isFile()) {
    if (extname(target) === '.md') out.push(target);
    return out;
  }
  for (const name of readdirSync(target)) collect(join(target, name), out);
  return out;
}

const FENCE = '```';
let total = 0;
let bad = 0;

for (const file of roots.flatMap((r) => collect(r))) {
  const md = readFileSync(file, 'utf8');
  const lines = md.split('\n');
  let inBlock = false;
  let buffer = [];
  let startLine = 0;
  let isJson = false;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (!inBlock) {
      if (line.trim() === `${FENCE}json`) {
        inBlock = true;
        isJson = true;
        buffer = [];
        startLine = i + 1;
      } else if (line.trim().startsWith(FENCE)) {
        inBlock = true;
        isJson = false;
        buffer = [];
        startLine = i + 1;
      }
      continue;
    }
    if (line.trim() === FENCE) {
      if (isJson) {
        total += 1;
        try {
          JSON.parse(buffer.join('\n'));
        } catch (err) {
          bad += 1;
          console.log(`✗ ${relative(process.cwd(), file)} 第 ${startLine} 行起：${err.message}`);
        }
      }
      inBlock = false;
      isJson = false;
      continue;
    }
    buffer.push(line);
  }
}

console.log(`\n共检查 ${total} 个 JSON 代码块，${bad} 个无法解析。`);
process.exit(bad === 0 ? 0 : 1);
