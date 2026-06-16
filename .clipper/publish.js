#!/usr/bin/env bun
/**
 * Noda Clipper WebDAV Publisher
 * Packages the extension, manages updates.json, and uploads updates metadata to InfiniCloud WebDAV.
 */

import { execSync } from 'child_process';
import { writeFileSync, existsSync, readFileSync, mkdirSync } from 'fs';
import { resolve } from 'path';
import { createInterface } from 'readline';

const CLIPPER_DIR = resolve(__dirname);
const BUILD_DIR = resolve(CLIPPER_DIR, 'build');
const WEBDAV_URL = 'https://rausu.infini-cloud.net/dav/softwareDevelopment/NodaNotes/clipper';
const WEBDAV_USER = 'kerimaydinn168';
const WEBDAV_PASS = 'jgRFjaSZL3rP2f4q';

const rl = createInterface({
  input: process.stdin,
  output: process.stdout
});

function askQuestion(query) {
  return new Promise((resolve) => rl.question(query, resolve));
}

// Helper to run curl command safely
function uploadToWebDAV(localPath, remoteName) {
  console.log(`📤 Uploading ${remoteName} to WebDAV...`);
  const remoteUrl = `${WEBDAV_URL}/${remoteName}`;
  const curlCmd = `curl -s -o /dev/null -w "%{http_code}" -u "${WEBDAV_USER}:${WEBDAV_PASS}" -T "${localPath}" "${remoteUrl}"`;
  
  try {
    const statusCode = execSync(curlCmd).toString().trim();
    if (statusCode === '200' || statusCode === '201' || statusCode === '204') {
      console.log(`✅ Upload successful: ${remoteUrl}`);
      return true;
    } else {
      console.error(`❌ Upload failed with status code: ${statusCode}`);
      return false;
    }
  } catch (error) {
    console.error(`❌ WebDAV upload failed:`, error.message);
    return false;
  }
}

// Helper to create remote directory structure if it doesn't exist
function ensureRemoteDirectory() {
  console.log(`📁 Checking/Creating WebDAV target directory...`);
  const curlCmd = `curl -s -o /dev/null -w "%{http_code}" -u "${WEBDAV_USER}:${WEBDAV_PASS}" -X MKCOL "${WEBDAV_URL}/"`;
  try {
    execSync(curlCmd);
  } catch (e) {
    // Ignore error if directory already exists
  }
}

// Helper to dynamically sign JWT
function generateJwt(key, secret) {
  const crypto = require('crypto');
  const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url');
  const issuedAt = Math.floor(Date.now() / 1000);
  const payload = Buffer.from(JSON.stringify({
    iss: key,
    jti: Math.random().toString(),
    iat: issuedAt,
    exp: issuedAt + 300
  })).toString('base64url');
  const signature = crypto.createHmac('sha256', secret).update(header + '.' + payload).digest('base64url');
  return header + '.' + payload + '.' + signature;
}

async function main() {
  console.log('\n================================================');
  console.log('   Noda Clipper WebDAV Automated Publisher      ');
  console.log('================================================\n');

  // 1. Read manifest to extract current version
  const manifestPath = resolve(CLIPPER_DIR, 'extensions/firefox/manifest.json');
  if (!existsSync(manifestPath)) {
    console.error(`❌ Manifest not found at: ${manifestPath}`);
    rl.close();
    process.exit(1);
  }

  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const version = manifest.version;
  const addonId = manifest.browser_specific_settings?.gecko?.id || 'clipper@nodanotes.com';
  console.log(`ℹ️ Detected Extension Version: ${version}`);

  const JWT_USER = 'user:19988212:128';
  const JWT_SECRET = '3e0534bdc4626d54fa89dfbc8e56e65cecfb1556feefc46581f4a39908ee4eed';

  console.log('\nSelect execution mode:');
  console.log('1. Full Pack, Sign & Upload (Sign new code & update manifest)');
  console.log('2. Only Update Manifest (Just query AMO API & upload updates.json to WebDAV)');
  
  const modeChoice = await askQuestion('\nSelect an option (1-2): ');

  if (modeChoice.trim() === '1') {
    // 2. Package and Sign the extension using Mozilla web-ext API
    console.log('\n📦 Step 1: Packaging & Signing extension via Mozilla API...');
    
    const firefoxExtSourceDir = resolve(CLIPPER_DIR, 'extensions/firefox');
    const tempBuildDir = resolve(CLIPPER_DIR, `temp-build-firefox`);
    
    if (existsSync(tempBuildDir)) {
      execSync(`rm -rf "${tempBuildDir}"`);
    }
    mkdirSync(tempBuildDir, { recursive: true });

    try {
      console.log('   - Preparing source folder (resolving symlinks)...');
      execSync(`cp -rL "${firefoxExtSourceDir}"/* "${tempBuildDir}/"`);

      console.log('   - Submitting extension to Mozilla AMO for signing...');
      const signCmd = `bun x web-ext sign --source-dir "${tempBuildDir}" --artifacts-dir "${BUILD_DIR}" --api-key "${JWT_USER}" --api-secret "${JWT_SECRET}" --channel unlisted`;
      execSync(signCmd, { stdio: 'inherit' });

      execSync(`rm -rf "${tempBuildDir}"`);

      const files = require('fs').readdirSync(BUILD_DIR);
      const signedFile = files.find(f => f.endsWith(`-${version}.xpi`));

      if (!signedFile) {
        throw new Error(`Could not find signed .xpi file ending with '-${version}.xpi' in ${BUILD_DIR}`);
      }

      const signedFilePath = resolve(BUILD_DIR, signedFile);
      const targetXpiPath = resolve(BUILD_DIR, 'noda-clipper-firefox.xpi');

      execSync(`mv "${signedFilePath}" "${targetXpiPath}"`);
      console.log(`✅ Extension signed and saved locally: ${targetXpiPath}`);

    } catch (e) {
      console.error('❌ Signing failed:', e.message);
      if (existsSync(tempBuildDir)) {
        execSync(`rm -rf "${tempBuildDir}"`);
      }
      rl.close();
      process.exit(1);
    }
  } else {
    console.log('\n⏩ Skipping packaging and signing. Proceeding directly to manifest generation...');
  }

  // 3. Ensure WebDAV target folder exists
  ensureRemoteDirectory();

  // 4. Query Mozilla AMO API dynamically to get the signed file's official direct download link
  console.log('\n🔍 Fetching signed download link from Mozilla AMO API...');
  
  let fileUrl = '';
  try {
    const jwtToken = generateJwt(JWT_USER, JWT_SECRET);
    const amoUrl = `https://addons.mozilla.org/api/v5/addons/addon/${addonId}/`;
    
    const curlCmd = `curl -s -H "Authorization: JWT ${jwtToken}" "${amoUrl}"`;
    const responseText = execSync(curlCmd).toString();
    const responseJson = JSON.parse(responseText);
    
    // Read direct link from latest unlisted version
    const latestVersion = responseJson.latest_unlisted_version;
    if (latestVersion && latestVersion.version === version && latestVersion.file && latestVersion.file.url) {
      fileUrl = latestVersion.file.url;
      console.log(`✅ Found direct signed download link: ${fileUrl}`);
    } else {
      console.warn('⚠️ Could not match latest unlisted version in API response, generating fallback URL...');
      if (latestVersion && latestVersion.file && latestVersion.file.id) {
        fileUrl = `https://addons.mozilla.org/firefox/downloads/file/${latestVersion.file.id}/${responseJson.slug || 'c950ae93f33741c28626'}-${version}.xpi`;
        console.log(`✅ Generated fallback download link: ${fileUrl}`);
      } else {
        throw new Error('No unlisted version details available in AMO API response.');
      }
    }
  } catch (error) {
    console.error('❌ Failed to fetch signed download link from AMO API:', error.message);
    rl.close();
    process.exit(1);
  }

  // 5. Generate / Update updates.json
  console.log('\n⚙️ Step 2: Updating updates.json file...');
  const localUpdatesPath = resolve(BUILD_DIR, 'updates.json');

  const updatesData = {
    addons: {
      [addonId]: {
        updates: [
          {
            version: version,
            update_link: fileUrl
          }
        ]
      }
    }
  };

  writeFileSync(localUpdatesPath, JSON.stringify(updatesData, null, 2), 'utf8');
  console.log(`✅ Created local updates.json at: ${localUpdatesPath}`);

  // 6. Upload updates.json to WebDAV
  const jsonSuccess = uploadToWebDAV(localUpdatesPath, 'updates.json');
  
  if (jsonSuccess) {
    console.log('\n🚀 PUBLISH COMPLETED SUCCESSFULLY!');
    console.log('================================================');
    console.log(`🔗 Firefox Public Update URL: https://rausu.infini-cloud.net/v2/api/share/public/13214bb444e0a2b8/download`);
    console.log(`📦 Firefox Public XPI Package (Mozilla CDN): ${fileUrl}`);
    console.log('================================================\n');
  } else {
    console.error('❌ Failed to upload updates.json to WebDAV.');
  }

  rl.close();
}

main().catch((err) => {
  console.error('An error occurred during publication:', err);
  rl.close();
});
