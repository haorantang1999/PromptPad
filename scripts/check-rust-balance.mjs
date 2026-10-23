// Rust 源码括号平衡静态检查。
//
// 本机没有 Rust 工具链时，用它替代 `cargo check` 的第一道闸：
// 逐字符扫描，跳过行注释、块注释、字符串（含 raw string）与字符字面量，
// 只统计真正参与语法结构的括号。发现不平衡时报告行号。
//
// 用法：node scripts/check-rust-balance.mjs [目录或文件...]

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, extname, relative } from 'node:path';

const targets = process.argv.slice(2);
if (targets.length === 0) {
  targets.push('src-tauri/src');
}

function collect(target, out = []) {
  const stat = statSync(target);
  if (stat.isFile()) {
    if (extname(target) === '.rs') out.push(target);
    return out;
  }
  for (const name of readdirSync(target)) {
    collect(join(target, name), out);
  }
  return out;
}

const PAIRS = { '(': ')', '[': ']', '{': '}' };
const CLOSERS = new Set([')', ']', '}']);

function check(file) {
  const text = readFileSync(file, 'utf8');
  const stack = [];
  const errors = [];
  let i = 0;
  let line = 1;
  const n = text.length;

  while (i < n) {
    const ch = text[i];
    if (ch === '\n') {
      line += 1;
      i += 1;
      continue;
    }
    // 行注释
    if (ch === '/' && text[i + 1] === '/') {
      while (i < n && text[i] !== '\n') i += 1;
      continue;
    }
    // 块注释（可嵌套）
    if (ch === '/' && text[i + 1] === '*') {
      let depth = 1;
      i += 2;
      while (i < n && depth > 0) {
        if (text[i] === '\n') line += 1;
        if (text[i] === '/' && text[i + 1] === '*') {
          depth += 1;
          i += 2;
          continue;
        }
        if (text[i] === '*' && text[i + 1] === '/') {
          depth -= 1;
          i += 2;
          continue;
        }
        i += 1;
      }
      continue;
    }
    // raw string: r"..." / r#"..."# / br##"..."##
    const rawMatch = /^(b?r)(#*)"/.exec(text.slice(i, i + 20));
    if (rawMatch) {
      const hashes = rawMatch[2].length;
      const terminator = `"${'#'.repeat(hashes)}`;
      const end = text.indexOf(terminator, i + rawMatch[0].length);
      if (end === -1) {
        errors.push(`第 ${line} 行：raw string 未闭合`);
        break;
      }
      for (let k = i; k < end; k += 1) if (text[k] === '\n') line += 1;
      i = end + terminator.length;
      continue;
    }
    // 普通字符串
    if (ch === '"') {
      i += 1;
      while (i < n) {
        if (text[i] === '\\') {
          i += 2;
          continue;
        }
        if (text[i] === '\n') line += 1;
        if (text[i] === '"') {
          i += 1;
          break;
        }
        i += 1;
      }
      continue;
    }
    // 字符字面量
    if (ch === "'") {
      const rest = text.slice(i, i + 8);
      const charMatch = /^'(\\.|.)'/.exec(rest);
      if (charMatch) {
        i += charMatch[0].length;
        continue;
      }
      // 生命周期标注（'a），不当成字面量
      i += 1;
      continue;
    }
    if (PAIRS[ch]) {
      stack.push({ ch, line });
      i += 1;
      continue;
    }
    if (CLOSERS.has(ch)) {
      const top = stack.pop();
      if (!top) {
        errors.push(`第 ${line} 行：多余的 ${ch}`);
      } else if (PAIRS[top.ch] !== ch) {
        errors.push(`第 ${line} 行：${ch} 与第 ${top.line} 行的 ${top.ch} 不匹配`);
      }
      i += 1;
      continue;
    }
    i += 1;
  }

  for (const open of stack) {
    errors.push(`第 ${open.line} 行的 ${open.ch} 没有闭合`);
  }
  return errors;
}

const files = targets.flatMap((t) => collect(t));
let failed = 0;
for (const file of files) {
  const errors = check(file);
  if (errors.length > 0) {
    failed += 1;
    console.log(`✗ ${relative(process.cwd(), file)}`);
    for (const e of errors) console.log(`    ${e}`);
  }
}
console.log(`\n共检查 ${files.length} 个 Rust 文件，${failed} 个存在括号问题。`);
process.exit(failed === 0 ? 0 : 1);
