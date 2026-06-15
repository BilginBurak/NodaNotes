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
    });
  }
  return true; // Keep message channel open for async response
});

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
