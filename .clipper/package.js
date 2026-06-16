#!/usr/bin/env bun
/**
 * Noda Clipper Packaging Script
 * Supports selective bundling for Chromium (.zip) and Firefox (.xpi)
 * Resolves symbolic links into physical files during the build process.
 
 1. Etkileşimli Menüden Seçerek (Rakamla):
  Sadece aşağıdaki komutu çalıştırın, gelen menüde terminale 1, 2 veya 3 yazıp Enter'a basın:
  ```bash
  bun .clipper/package.js
  ```
 2. Doğrudan Komut Satırından Rakam Girerek (Tek Satır):
  Parametre olarak kelime yazmak yerine doğrudan rakam yazarak derleyebilirsiniz:

 - Chromium (Brave) zip dosyası için:
  ```bash
  bun .clipper/package.js 1
  ```
 - Firefox (Zen) xpi dosyası için:
  ```bash
  bun .clipper/package.js 2
  ```
 - Her ikisi birden derlensin diye:
  ```bash
  bun .clipper/package.js 3
  ```
 */

import { execSync } from 'child_process';
import { existsSync, rmSync, mkdirSync } from 'fs';
import { resolve } from 'path';
import { createInterface } from 'readline';

const CLIPPER_DIR = resolve(__dirname);
const EXTENSIONS_DIR = resolve(CLIPPER_DIR, 'extensions');

function cleanBuildDir(buildPath) {
  if (existsSync(buildPath)) {
    rmSync(buildPath, { recursive: true, force: true });
  }
}

function packageTarget(target, extension) {
  console.log(`\n📦 Packaging ${target} Extension...`);
  const targetDir = resolve(EXTENSIONS_DIR, target);
  const buildDir = resolve(CLIPPER_DIR, `temp-build-${target}`);
  const buildOutputDir = resolve(CLIPPER_DIR, 'build');
  if (!existsSync(buildOutputDir)) {
    mkdirSync(buildOutputDir, { recursive: true });
  }
  const destFile = resolve(buildOutputDir, `noda-clipper-${target}.${extension}`);

  try {
    // 1. Clean previous builds
    cleanBuildDir(buildDir);
    if (existsSync(destFile)) {
      rmSync(destFile, { force: true });
    }

    // 2. Create clean build directory
    mkdirSync(buildDir, { recursive: true });

    // 3. Copy files dereferencing symlinks (-L)
    console.log(`   - Copying source files (dereferencing symlinks)...`);
    execSync(`cp -rL "${targetDir}"/* "${buildDir}/"`);

    // 4. Zip the contents of build directory
    console.log(`   - Creating archive: ${destFile}`);
    execSync(`cd "${buildDir}" && zip -rq "${destFile}" .`);

    console.log(`   - Cleaning up temporary build folder...`);
    cleanBuildDir(buildDir);

    console.log(`✅ ${target.toUpperCase()} extension packaged successfully at:\n   ${destFile}`);
  } catch (error) {
    console.error(`❌ Failed to package ${target} extension:`, error.message);
  }
}

function run(action) {
  const arg = action || process.argv[2];

  if (arg === 'chromium' || arg === '1') {
    packageTarget('chromium', 'zip');
  } else if (arg === 'firefox' || arg === '2') {
    packageTarget('firefox', 'xpi');
  } else if (arg === 'all' || arg === '3') {
    packageTarget('chromium', 'zip');
    packageTarget('firefox', 'xpi');
  } else if (arg === '4') {
    console.log('Packaging cancelled.');
  } else {
    // Interactive menu
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout
    });

    console.log('\n======================================');
    console.log('   Noda Clipper Extension Packager   ');
    console.log('======================================');
    console.log('1. Package Chromium Extension (.zip)');
    console.log('2. Package Firefox Extension (.xpi)');
    console.log('3. Package Both Extensions (All)');
    console.log('4. Cancel');

    rl.question('\nSelect an option (1-4): ', (answer) => {
      rl.close();
      const choice = answer.trim();
      if (choice === '1') {
        packageTarget('chromium', 'zip');
      } else if (choice === '2') {
        packageTarget('firefox', 'xpi');
      } else if (choice === '3') {
        packageTarget('chromium', 'zip');
        packageTarget('firefox', 'xpi');
      } else if (choice === '4') {
        console.log('Packaging cancelled.');
      } else {
        console.log('Invalid option.');
      }
    });
  }
}

run();
