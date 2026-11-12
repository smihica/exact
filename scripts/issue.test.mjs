import { expect, spyOn, test } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fixIssue, main, renderNewIssue, runFormatCheck, splitStatus, validateIssue } from './issue.mjs';

const openPath = 'issues/20261110-example.md';
const closedPath = 'issues/closed/20261110-example.md';

function issue(status, resolution) {
  return renderNewIssue({ title: 'Example', status, systems: 'Tooling', author: 'Test', date: '2026-11-10', body: '## Reproduction\n\nBody.' })
    .replace('**Systems:**', `${resolution ? `**Resolution:** ${resolution}\n` : ''}**Systems:**`);
}

function fixture(run) {
  const target = resolve(import.meta.dir, '../target');
  mkdirSync(target, { recursive: true });
  const root = mkdtempSync(resolve(target, 'issue-test-'));
  const write = (path, text) => {
    const absolute = resolve(root, path);
    mkdirSync(dirname(absolute), { recursive: true });
    writeFileSync(absolute, text);
    return absolute;
  };
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
    expect(result.status, result.stderr).toBe(0);
  };
  try {
    git('init', '--quiet');
    run({ root, write, git });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function command(root, args) {
  const output = [];
  const errors = [];
  const log = spyOn(console, 'log').mockImplementation(text => output.push(text));
  const error = spyOn(console, 'error').mockImplementation(text => errors.push(text));
  try {
    return { status: main(args, root), output: output.join('\n'), errors: errors.join('\n') };
  } finally {
    log.mockRestore();
    error.mockRestore();
  }
}

for (const tracked of [false, true]) {
  test(`check reads a ${tracked ? 'tracked' : 'new'} issue by relative, dotted and absolute path`, () => fixture(({ root, write, git }) => {
    const absolute = write(openPath, '# No header\n\nBody.\n');
    if (tracked) git('add', openPath);
    for (const path of [openPath, `./${openPath}`, absolute]) {
      const result = runFormatCheck(root, { paths: [path] });
      expect(result.scanned).toBe(1);
      expect(result.problems.map(problem => problem.code)).toEqual(['missing:Systems', 'missing:Author', 'missing:Date', 'missing:Status']);
      expect(command(root, ['check', path]).status).toBe(1);
    }
    expect(runFormatCheck(root, { paths: [openPath, `./${openPath}`, absolute] }).scanned).toBe(1);
  }));
}

test('check reports every invalid path alongside malformed issues and returns a usage failure', () => fixture(({ root, write }) => {
  write(openPath, '# No header\n\nBody.\n');
  write('docs/20261110-example.md', issue('Open'));
  write('issues/README.md', '# Issues\n');
  const outside = write('other/issues/20261110-example.md', issue('Open'));
  mkdirSync(resolve(root, 'issues/20261110-directory.md'));
  const invalid = ['issues/no-such-file.md', 'docs/20261110-example.md', 'issues/README.md', 'issues/20261110-directory.md', outside];
  const result = command(root, ['check', openPath, ...invalid]);
  expect(result.status).toBe(2);
  for (const path of invalid) expect(result.errors).toContain(path);
  expect(result.errors).toContain('Missing required **Systems:**');
  expect(result.output).not.toContain('Issue format passed.');
}));

test('check --fix reads an untracked path once and the unscoped check still selects tracked issues', () => fixture(({ root, write, git }) => {
  const absolute = write(openPath, issue('open'));
  expect(runFormatCheck(root).scanned).toBe(0);
  const result = command(root, ['check', '--fix', `./${openPath}`, absolute]);
  expect(result.status).toBe(0);
  expect(result.output).toContain('1 issue scanned, 1 rewritten');
  expect(readFileSync(absolute, 'utf8')).toBe(issue('Open'));
  git('add', openPath);
  expect(runFormatCheck(root)).toEqual({ problems: [], fixed: [], scanned: 1 });
}));

test('an issue made by new can be checked before staging', () => fixture(({ root }) => {
  expect(command(root, ['new', 'Example', '--systems', 'Tooling', '--author', 'Test', '--date', '2026-11-10']).status).toBe(0);
  const result = runFormatCheck(root, { paths: [openPath] });
  expect(result).toEqual({ problems: [], fixed: [], scanned: 1 });
}));

test.each(['Open', 'Open - --wasm builds only'])('check --fix leaves a quoted Status %s in the body', status => fixture(({ root, write }) => {
  const body = `## Reproduction\n\n\`\`\`\n**Status:** ${status}\n\`\`\`\n`;
  const source = issue('Open').replace('**Status:** Open\n', '').replace('## Reproduction\n\nBody.\n', body);
  write(openPath, source);
  const result = command(root, ['check', '--fix', openPath]);
  expect(result.status).toBe(0);
  const text = readFileSync(resolve(root, openPath), 'utf8');
  expect(text).toBe(source.replace('**Systems:**', '**Status:** Open\n**Systems:**'));
  expect(validateIssue(openPath, source).map(problem => problem.code)).toEqual(['missing:Status']);
}));

test('repair inserts Status under the title when the only metadata is quoted', () => {
  const source = '# Example\n\n## Reproduction\n\n```\n**Status:** Open\n```\n';
  const result = fixIssue(openPath, source);
  expect(result.text).toStartWith('# Example\n\n**Status:** Open\n');
  expect(result.text).toContain(source.slice(source.indexOf('## Reproduction')));
  expect(validateIssue(openPath, result.text).map(problem => problem.code)).toEqual(['missing:Systems', 'missing:Author']);
});

test.each(['Open', 'Open - --wasm builds only'])('close leaves a quoted Status %s in the body', status => fixture(({ root, write, git }) => {
  const body = `## Reproduction\n\n\`\`\`\n**Status:** ${status}\n\`\`\`\n`;
  const source = issue('Open').replace('**Status:** Open\n', '').replace('## Reproduction\n\nBody.\n', body);
  write(openPath, source);
  git('add', openPath);
  expect(command(root, ['close', openPath, '--resolution', 'Fixed on main']).status).toBe(0);
  const text = readFileSync(resolve(root, closedPath), 'utf8');
  expect(text).toBe(source.replace('**Systems:**', '**Status:** Closed\n**Resolution:** Fixed on main\n**Systems:**'));
  expect(validateIssue(closedPath, text)).toEqual([]);
}));

test('Status diagnostics distinguish unrecognized values from closure tokens', () => {
  for (const value of ['Open-ended: needs a decision', 'Closed-ended: needs a decision', 'Resolved-later: needs a decision']) {
    const [problem] = validateIssue(openPath, issue(value));
    expect(problem.code).toBe('status:value');
    expect(problem.fixable).toBe(false);
    expect(problem.hint).toContain('will not rewrite');
    expect(problem.hint).not.toContain('issue.mjs close');
  }
  for (const value of ['Closed', 'Resolved', 'Fixed', 'Done', 'Complete', 'Completed']) {
    const [problem] = validateIssue(openPath, issue(value));
    expect(problem.code).toBe('status:value');
    expect(problem.fixable).toBe(false);
    expect(problem.hint).toContain('issue.mjs close');
  }
});

test('Status diagnostics explain continuation lines', () => {
  const [problem] = validateIssue(openPath, issue('Open\nkept sentence'));
  expect(problem.code).toBe('status:value');
  expect(problem.message).toContain('continuation lines');
  expect(problem.found).toBe('**Status:** Open\nkept sentence');
  expect(problem.fixable).toBe(true);
});

test.each([
  ['Closed: --fix now moves the qualifier', 'Closed', '--fix now moves the qualifier'],
  ['Closed - .gitignore entry added', 'Closed', '.gitignore entry added'],
  ['Closed, -Zbuild-std is pinned', 'Closed', '-Zbuild-std is pinned'],
  ['Closed: -1 is now refused', 'Closed', '-1 is now refused'],
  ['Open: .exact/ is not ignored', 'Open', '.exact/ is not ignored'],
  ['Closed --fix now works', 'Closed', '--fix now works'],
  ['Closed .gitignore entry added', 'Closed', '.gitignore entry added'],
  ['Closed -1 is now refused', 'Closed', '-1 is now refused'],
  ['Closed — fixed', 'Closed', 'fixed'],
  ['Closed. fixed', 'Closed', 'fixed'],
  ['Closed (fixed on main)', 'Closed', 'fixed on main'],
  ['Closed (fixed) and (verified)', 'Closed', '(fixed) and (verified)'],
  ['Resolved: --fix works', 'Closed', 'Resolved: --fix works'],
])('repair preserves the qualifier in %s', (value, expected, remainder) => {
  expect(splitStatus(value, expected)).toEqual({ remainder, recognized: true });
  const path = expected === 'Closed' ? closedPath : openPath;
  const result = fixIssue(path, issue(value));
  expect(result.text).toContain(expected === 'Closed' ? `**Resolution:** ${remainder}\n` : `): ${remainder}\n`);
  expect(validateIssue(path, result.text)).toEqual([]);
  expect(fixIssue(path, result.text).text).toBe(result.text);
});

test.each(['Open-ended: needs a decision', 'Closed-ended: needs a decision', 'Resolved-later: needs a decision'])(
  '%s is author text, not a status token', value => {
    const path = value.startsWith('Open') ? openPath : closedPath;
    expect(splitStatus(value, path === openPath ? 'Open' : 'Closed')).toEqual({ remainder: value, recognized: false });
    const text = issue(value);
    expect(fixIssue(path, text).text).toBe(text);
  },
);

test.each(['\n', '\r\n'])('repair preserves continuation lines and indentation with %j endings', eol => {
  const qualifier = `blocked on:${eol}- the kernel change${eol}  - the web host  `;
  for (const token of ['Open, ', 'Open\n']) {
    const source = issue(`${token}blocked on:\n- the kernel change\n  - the web host  `).replaceAll('\n', eol);
    const result = fixIssue(openPath, source);
    expect(result.text).toContain(qualifier);
    expect(result.text).toContain(`**Status:** Open${eol}`);
    expect(validateIssue(openPath, result.text)).toEqual([]);
    expect(fixIssue(openPath, result.text).text).toBe(result.text);
  }
  for (const resolution of [undefined, 'Fixed on main']) {
    const source = issue('Closed, blocked on:\n- the kernel change\n  - the web host  ', resolution).replaceAll('\n', eol);
    const result = fixIssue(closedPath, source);
    expect(result.text).toContain(`Status note (`);
    expect(result.text).toContain(qualifier);
    expect(result.text).not.toContain('**Resolution:** blocked on:');
    expect(validateIssue(closedPath, result.text).map(problem => problem.code)).toEqual(resolution ? [] : ['missing:Resolution']);
    expect(fixIssue(closedPath, result.text).text).toBe(result.text);
  }
});

test.each(['\n', '\r\n'])('repair preserves parenthesized multiline qualifiers with %j endings', eol => {
  for (const qualifier of ['\n    --wasm builds only\n', '    --wasm builds only\n    .exact/ is retained  ']) {
    const remainder = qualifier.replaceAll('\n', eol);
    for (const [status, resolution] of [['Open', undefined], ['Closed', undefined], ['Closed', 'Fixed on main']]) {
      const value = `${status} (${remainder})`;
      expect(splitStatus(value, status)).toEqual({ remainder, recognized: true });
      const path = status === 'Open' ? openPath : closedPath;
      const result = fixIssue(path, issue(`${status} (${qualifier})`, resolution).replaceAll('\n', eol));
      expect(result.text).toContain(`): ${remainder}${eol}`);
      expect(result.text).toContain(`**Status:** ${status}${eol}`);
      expect(result.text).not.toContain('**Resolution:** --wasm');
      expect(validateIssue(path, result.text).map(problem => problem.code))
        .toEqual(status === 'Closed' && !resolution ? ['missing:Resolution'] : []);
      expect(fixIssue(path, result.text).text).toBe(result.text);
    }
  }
});

test.each(['\n', '\r\n'])('repair preserves a parenthesized qualifier on a continuation with %j endings', eol => {
  const remainder = `${eol}    (--wasm builds only)`;
  for (const [status, resolution] of [['Open', undefined], ['Closed', undefined], ['Closed', 'Fixed on main']]) {
    const value = `${status}${remainder}`;
    expect(splitStatus(value, status)).toEqual({ remainder, recognized: true });
    const path = status === 'Open' ? openPath : closedPath;
    const result = fixIssue(path, issue(`${status}\n    (--wasm builds only)`, resolution).replaceAll('\n', eol));
    expect(result.text).toContain(`): ${remainder}${eol}`);
    expect(result.text).not.toContain('**Resolution:** --wasm');
    expect(validateIssue(path, result.text).map(problem => problem.code))
      .toEqual(status === 'Closed' && !resolution ? ['missing:Resolution'] : []);
    expect(fixIssue(path, result.text).text).toBe(result.text);
  }
});

test.each(['\n', '\r\n'])('close preserves a parenthesized multiline qualifier with %j endings', eol => fixture(({ root, write, git }) => {
  const qualifier = '\n    --wasm builds only\n';
  write(openPath, issue(`Open (${qualifier})`).replaceAll('\n', eol));
  git('add', openPath);
  expect(command(root, ['close', openPath, '--resolution', 'Fixed on main']).status).toBe(0);
  const text = readFileSync(resolve(root, closedPath), 'utf8');
  expect(text).toContain(`): ${qualifier.replaceAll('\n', eol)}${eol}`);
  expect(text).toContain(`**Resolution:** Fixed on main${eol}`);
  expect(validateIssue(closedPath, text)).toEqual([]);
}));

test.each(['Open - --wasm builds only', 'Open-ended: needs a decision', 'Open, blocked on:\n- the kernel change\n  - the web host', 'Open\n    (--wasm builds only)'])(
  'close preserves author text from %s in the body', status => fixture(({ root, write, git }) => {
    write(openPath, issue(status));
    git('add', openPath);
    expect(command(root, ['close', openPath, '--resolution', 'Fixed on main']).status).toBe(0);
    const text = readFileSync(resolve(root, closedPath), 'utf8');
    const expected = status === 'Open-ended: needs a decision' ? status : status.replace(/^Open(?: - |, )?/, '');
    expect(text).toContain(`): ${expected}\n`);
    expect(text).toContain('**Resolution:** Fixed on main\n');
    expect(validateIssue(closedPath, text)).toEqual([]);
  }),
);

test.each([undefined, 'Previous header resolution'])('close carries only a replaced header Resolution (%j)', resolution => fixture(({ root, write, git }) => {
  const body = '## Reproduction\n\n```\n**Resolution:** fix now moves the qualifier\n```\n';
  write(openPath, issue('Open', resolution).replace('## Reproduction\n\nBody.\n', body));
  git('add', openPath);
  expect(command(root, ['close', openPath, '--resolution', 'Fixed on main']).status).toBe(0);
  const text = readFileSync(resolve(root, closedPath), 'utf8');
  expect(text).toContain(body);
  expect(text).toContain('**Resolution:** Fixed on main\n');
  expect(text.match(/Status note \(/g) ?? []).toHaveLength(resolution ? 1 : 0);
  if (resolution) expect(text).toContain(`previous **Resolution:** ${resolution}\n`);
  expect(validateIssue(closedPath, text)).toEqual([]);
}));
