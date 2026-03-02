#!/usr/bin/env node
// install.js -- beu-skills installer
// Called via: npx @shuozeli/beu-skills [--root <path>] [--force]
//
// Writes skill rule files into agent rule directories:
//   <root>/.claude/rules/beu.md
//   <root>/.gemini/rules/beu.md
//   <root>/.agent/rules/beu.md
//
// Output: one line per file written, to stdout.
// Exit code 0 on success, 1 on error.

'use strict';

const fs = require('fs');
const path = require('path');

const AGENT_DIRS = ['.claude/rules', '.gemini/rules', '.agent/rules'];
const RULE_FILE = 'beu.md';

function parseArgs(argv) {
    let root = process.cwd();
    let force = false;

    for (let i = 0; i < argv.length; i++) {
        if (argv[i] === '--root' && argv[i + 1]) {
            root = path.resolve(argv[++i]);
        } else if (argv[i] === '--force') {
            force = true;
        }
    }

    return { root, force };
}

function main() {
    const { root, force } = parseArgs(process.argv.slice(2));

    const skillPath = path.join(__dirname, '..', 'skills', RULE_FILE);

    if (!fs.existsSync(skillPath)) {
        process.stderr.write(`error: skill file not found: ${skillPath}\n`);
        process.exit(1);
    }

    const content = fs.readFileSync(skillPath, 'utf8');
    const written = [];

    for (const dir of AGENT_DIRS) {
        const rulesDir = path.join(root, dir);
        const rulePath = path.join(rulesDir, RULE_FILE);

        if (fs.existsSync(rulePath) && !force) {
            continue;
        }

        fs.mkdirSync(rulesDir, { recursive: true });
        fs.writeFileSync(rulePath, content, 'utf8');
        written.push(`${dir}/${RULE_FILE}`);
    }

    for (const p of written) {
        process.stdout.write(p + '\n');
    }
}

main();
