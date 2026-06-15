// Background service worker for Manifest V3 extension to handle fetch requests without CORS blocking

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'SUBMIT_CLIP') {
    const { title, url, contentMarkdown, tags, token } = request.payload;

    fetch('http://127.0.0.1:4040/api/clipper', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
      },
      body: JSON.stringify({
        title,
        url,
        content_markdown: contentMarkdown,
        tags
      })
    })
    .then(async response => {
      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `Server returned status ${response.status}`);
      }
      return response.text();
    })
    .then(data => {
      sendResponse({ success: true, message: data });
    })
    .catch(error => {
      console.error('Clipper submission error:', error);
      sendResponse({ success: false, error: error.message });
    });

    return true; // Keep message channel open for async response
  }
});
