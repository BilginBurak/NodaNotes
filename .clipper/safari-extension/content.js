// Zero-dependency HTML to Markdown Parser and Content Extractor

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'GET_CLIP_DATA') {
    const selectionHtml = getSelectionHtml();
    let contentMarkdown = '';
    
    if (selectionHtml) {
      contentMarkdown = htmlToMarkdown(selectionHtml);
    } else {
      // Fallback: search for article or main elements, else body
      const mainElement = document.querySelector('article') || document.querySelector('main') || document.body;
      contentMarkdown = htmlToMarkdown(mainElement.innerHTML);
    }

    sendResponse({
      title: document.title || 'Clipped Note',
      url: window.location.href,
      contentMarkdown: contentMarkdown.trim(),
      author: extractAuthor(),
      publishedDate: extractPublishedDate(),
    });
  } else if (request.action === 'START_AREA_SELECTION') {
    startAreaSelection();
    sendResponse({ success: true });
  }
  return true; // Keep message channel open for async response
});

function getMetaValue(selectors) {
  for (const selector of selectors) {
    const el = document.querySelector(selector);
    if (el) {
      const content = el.getAttribute('content') || el.getAttribute('datetime') || el.innerText;
      if (content && content.trim()) {
        return content.trim();
      }
    }
  }
  return '';
}

function extractAuthor() {
  return getMetaValue([
    'meta[name="author"]',
    'meta[property="article:author"]',
    'meta[name="twitter:creator"]',
    '[itemprop="author"]',
    '.author',
    '.byline'
  ]) || 'Unknown';
}

function extractPublishedDate() {
  return getMetaValue([
    'meta[property="article:published_time"]',
    'meta[name="publish-date"]',
    'meta[name="pubdate"]',
    'meta[name="dc.date"]',
    'time[datetime]'
  ]) || 'Unknown';
}

function getSelectionHtml() {
  let html = "";
  if (typeof window.getSelection !== "undefined") {
    const sel = window.getSelection();
    if (sel.rangeCount > 0) {
      const container = document.createElement("div");
      for (let i = 0, len = sel.rangeCount; i < len; ++i) {
        container.appendChild(sel.getRangeAt(i).cloneContents());
      }
      html = container.innerHTML;
    }
  }
  return html;
}

function htmlToMarkdown(htmlString) {
  const parser = new DOMParser();
  const doc = parser.parseFromString(htmlString, 'text/html');
  
  // Clean up unwanted tags (scripts, styles, inputs, nav, footer, etc.)
  const tagsToRemove = ['script', 'style', 'noscript', 'iframe', 'nav', 'footer', 'header', 'input', 'button', 'select', 'textarea'];
  tagsToRemove.forEach(tag => {
    const elements = doc.querySelectorAll(tag);
    elements.forEach(el => el.remove());
  });

  return nodeToMarkdown(doc.body);
}

function nodeToMarkdown(node) {
  if (node.nodeType === Node.TEXT_NODE) {
    return node.textContent;
  }
  
  if (node.nodeType !== Node.ELEMENT_NODE) {
    return '';
  }

  let childrenMarkdown = '';
  for (const child of node.childNodes) {
    childrenMarkdown += nodeToMarkdown(child);
  }

  const tagName = node.tagName.toLowerCase();
  switch (tagName) {
    case 'h1':
      return `\n\n# ${childrenMarkdown.trim()}\n\n`;
    case 'h2':
      return `\n\n## ${childrenMarkdown.trim()}\n\n`;
    case 'h3':
      return `\n\n### ${childrenMarkdown.trim()}\n\n`;
    case 'h4':
      return `\n\n#### ${childrenMarkdown.trim()}\n\n`;
    case 'h5':
      return `\n\n##### ${childrenMarkdown.trim()}\n\n`;
    case 'h6':
      return `\n\n###### ${childrenMarkdown.trim()}\n\n`;
    case 'p':
      return `\n\n${childrenMarkdown.trim()}\n\n`;
    case 'strong':
    case 'b':
      return `**${childrenMarkdown}**`;
    case 'em':
    case 'i':
      return `*${childrenMarkdown}*`;
    case 'code':
      // Check if parent is pre
      if (node.parentNode && node.parentNode.tagName.toLowerCase() === 'pre') {
        return childrenMarkdown;
      }
      return `\`${childrenMarkdown}\``;
    case 'pre':
      return `\n\`\`\`\n${node.textContent.trim()}\n\`\`\`\n`;
    case 'a':
      const href = node.getAttribute('href') || '';
      // Exclude anchor links or empty hrefs
      if (!href || href.startsWith('#')) {
        return childrenMarkdown;
      }
      // Absolute URL conversion
      let absoluteUrl = href;
      try {
        absoluteUrl = new URL(href, window.location.href).href;
      } catch (e) {}
      return `[${childrenMarkdown || href}](${absoluteUrl})`;
    case 'img':
      const src = node.getAttribute('src') || '';
      const alt = node.getAttribute('alt') || 'image';
      if (!src) return '';
      let absoluteImgUrl = src;
      try {
        absoluteImgUrl = new URL(src, window.location.href).href;
      } catch (e) {}
      return `![${alt}](${absoluteImgUrl})`;
    case 'ul':
    case 'ol':
      return `\n${childrenMarkdown}\n`;
    case 'li':
      const parentTag = node.parentNode ? node.parentNode.tagName.toLowerCase() : 'ul';
      if (parentTag === 'ol') {
        const index = Array.from(node.parentNode.children).indexOf(node) + 1;
        return `${index}. ${childrenMarkdown.trim()}\n`;
      }
      return `- ${childrenMarkdown.trim()}\n`;
    case 'br':
      return '\n';
    case 'blockquote':
      return `\n> ${childrenMarkdown.trim().split('\n').join('\n> ')}\n\n`;
    case 'table':
      return `\n\n${childrenMarkdown}\n\n`;
    case 'tr':
      return `| ${childrenMarkdown} |\n`;
    case 'th':
    case 'td':
      return `${childrenMarkdown.trim()} |`;
    default:
      return childrenMarkdown;
  }
}

let activeToast = null;

function showToast(message, type = 'info', keepAlive = false) {
  if (activeToast) {
    activeToast.remove();
    activeToast = null;
  }
  
  const toast = document.createElement('div');
  toast.id = 'noda-clipper-toast';
  
  // Style toast dynamically (Japandi aesthetic)
  let bg = '#181816'; // Sıcak Gece (dark mode surface)
  let fg = '#E2E2DF'; // Warm light text
  let border = '1px solid #2E302C';
  
  if (type === 'success') {
    bg = '#FAF9F5'; // Mat Keten (light mode surface)
    fg = '#2A2A28';
    border = '1px solid #7D8F82'; // Sage Green
  } else if (type === 'error') {
    bg = '#fef2f2';
    fg = '#991b1b';
    border = '1px solid #f87171';
  }
  
  toast.style.position = 'fixed';
  toast.style.bottom = '24px';
  toast.style.right = '24px';
  toast.style.backgroundColor = bg;
  toast.style.color = fg;
  toast.style.border = border;
  toast.style.borderRadius = '8px';
  toast.style.padding = '12px 20px';
  toast.style.fontFamily = '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif';
  toast.style.fontSize = '13px';
  toast.style.fontWeight = '500';
  toast.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.08)';
  toast.style.zIndex = '2147483647';
  toast.style.display = 'flex';
  toast.style.alignItems = 'center';
  toast.style.gap = '8px';
  toast.style.transition = 'opacity 0.2s ease, transform 0.2s ease';
  toast.style.opacity = '0';
  toast.style.transform = 'translateY(8px)';
  
  // Icon
  const dot = document.createElement('span');
  dot.style.display = 'inline-block';
  dot.style.width = '6px';
  dot.style.height = '6px';
  dot.style.borderRadius = '50%';
  if (type === 'success') {
    dot.style.backgroundColor = '#7D8F82';
  } else if (type === 'error') {
    dot.style.backgroundColor = '#ef4444';
  } else {
    dot.style.backgroundColor = '#5E6F65';
  }
  toast.appendChild(dot);
  
  const text = document.createElement('span');
  text.textContent = message;
  toast.appendChild(text);
  
  document.documentElement.appendChild(toast);
  activeToast = toast;
  
  // Trigger transition
  requestAnimationFrame(() => {
    toast.style.opacity = '1';
    toast.style.transform = 'translateY(0)';
  });
  
  if (!keepAlive) {
    setTimeout(() => {
      if (activeToast === toast) {
        toast.style.opacity = '0';
        toast.style.transform = 'translateY(8px)';
        setTimeout(() => {
          if (toast.parentNode) {
            toast.parentNode.removeChild(toast);
          }
          if (activeToast === toast) {
            activeToast = null;
          }
        }, 200);
      }
    }, 4000);
  }
}

function startAreaSelection() {
  const existingCanvas = document.getElementById('noda-clipper-crop-canvas');
  if (existingCanvas) {
    existingCanvas.remove();
  }
  
  const canvas = document.createElement('canvas');
  canvas.id = 'noda-clipper-crop-canvas';
  
  canvas.style.position = 'fixed';
  canvas.style.top = '0';
  canvas.style.left = '0';
  canvas.style.width = '100vw';
  canvas.style.height = '100vh';
  canvas.style.zIndex = '2147483647';
  canvas.style.cursor = 'crosshair';
  
  const dpr = window.devicePixelRatio || 1;
  canvas.width = window.innerWidth * dpr;
  canvas.height = window.innerHeight * dpr;
  
  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);
  
  const originalOverflow = document.body.style.overflow;
  document.body.style.overflow = 'hidden';
  
  document.documentElement.appendChild(canvas);
  
  let startX = 0;
  let startY = 0;
  let isDrawing = false;
  
  const drawOverlay = (currentX, currentY) => {
    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
    
    // Semi-transparent overlay
    ctx.fillStyle = 'rgba(0, 0, 0, 0.4)';
    ctx.fillRect(0, 0, window.innerWidth, window.innerHeight);
    
    if (isDrawing) {
      const x = Math.min(startX, currentX);
      const y = Math.min(startY, currentY);
      const w = Math.abs(startX - currentX);
      const h = Math.abs(startY - currentY);
      
      ctx.clearRect(x, y, w, h);
      
      // Border color matches Japandi Accent (Moss/Sage)
      ctx.strokeStyle = '#5E6F65';
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);
      
      // Pixel dimensions badge
      if (w > 20 && h > 20) {
        ctx.fillStyle = '#5E6F65';
        ctx.font = '11px sans-serif';
        const label = `${Math.round(w)}px × ${Math.round(h)}px`;
        const labelWidth = ctx.measureText(label).width;
        ctx.fillRect(x, y - 20 >= 0 ? y - 20 : y + h + 5, labelWidth + 10, 16);
        ctx.fillStyle = '#FAF9F5';
        ctx.fillText(label, x + 5, y - 20 >= 0 ? y - 8 : y + h + 17);
      }
    }
  };
  
  drawOverlay(0, 0);
  
  const onPointerDown = (e) => {
    if (e.button !== 0) return;
    startX = e.clientX;
    startY = e.clientY;
    isDrawing = true;
    drawOverlay(startX, startY);
  };
  
  const onPointerMove = (e) => {
    if (!isDrawing) return;
    drawOverlay(e.clientX, e.clientY);
  };
  
  const onPointerUp = (e) => {
    if (!isDrawing) return;
    isDrawing = false;
    
    const endX = e.clientX;
    const endY = e.clientY;
    
    const x = Math.min(startX, endX);
    const y = Math.min(startY, endY);
    const w = Math.abs(startX - endX);
    const h = Math.abs(startY - endY);
    
    cleanup();
    
    if (w > 5 && h > 5) {
      captureAndCrop(x, y, w, h);
    }
  };
  
  const onKeyDown = (e) => {
    if (e.key === 'Escape') {
      cleanup();
    }
  };
  
  const cleanup = () => {
    if (canvas.parentNode) {
      canvas.parentNode.removeChild(canvas);
    }
    document.body.style.overflow = originalOverflow;
    window.removeEventListener('pointerdown', onPointerDown);
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('keydown', onKeyDown);
  };
  
  window.addEventListener('pointerdown', onPointerDown);
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', onPointerUp);
  window.addEventListener('keydown', onKeyDown);
}

function captureAndCrop(x, y, w, h) {
  showToast("Capturing viewport...", "info", true);

  chrome.runtime.sendMessage({ action: 'CAPTURE_TAB' }, (response) => {
    if (!response || !response.success || !response.dataUrl) {
      const errorMsg = (response && response.error) ? response.error : 'Unknown error';
      showToast("Capture failed: " + errorMsg, "error");
      return;
    }
    
    const img = new Image();
    img.onload = () => {
      try {
        const scaleX = img.width / window.innerWidth;
        const scaleY = img.height / window.innerHeight;
        
        const cropX = x * scaleX;
        const cropY = y * scaleY;
        const cropW = w * scaleX;
        const cropH = h * scaleY;
        
        const cropCanvas = document.createElement('canvas');
        cropCanvas.width = cropW;
        cropCanvas.height = cropH;
        const cropCtx = cropCanvas.getContext('2d');
        
        cropCtx.drawImage(img, cropX, cropY, cropW, cropH, 0, 0, cropW, cropH);
        
        const croppedDataUrl = cropCanvas.toDataURL('image/png');
        const base64Data = croppedDataUrl.split(',')[1];
        
        uploadCroppedScreenshot(base64Data);
      } catch (err) {
        console.error("Cropping error:", err);
        showToast("Cropping failed", "error");
      }
    };
    img.onerror = () => {
      showToast("Failed to process viewport image", "error");
    };
    img.src = response.dataUrl;
  });
}

function uploadCroppedScreenshot(base64Data) {
  chrome.storage.local.get(['noda_clipper_token', 'temp_clip_state'], (result) => {
    const token = result.noda_clipper_token;
    const state = result.temp_clip_state || {};
    
    if (!token) {
      showToast("Error: Noda auth token not configured.", "error");
      return;
    }
    
    showToast("Uploading selected area...", "info", true);
    
    chrome.runtime.sendMessage({
      action: 'UPLOAD_ATTACHMENT',
      payload: {
        token: token,
        filename: 'area_selection.png',
        base64Data: base64Data
      }
    }, (uploadResponse) => {
      if (uploadResponse && uploadResponse.success && uploadResponse.data && uploadResponse.data.url) {
        const url = uploadResponse.data.url;
        state.screenshotUrl = url;
        chrome.storage.local.set({ temp_clip_state: state }, () => {
          showToast("Area screenshot attached! Reopen Noda Clipper to submit.", "success");
        });
      } else {
        const errorMsg = (uploadResponse && uploadResponse.error) ? uploadResponse.error : 'Upload failed';
        showToast("Failed to upload screenshot: " + errorMsg, "error");
      }
    });
  });
}
