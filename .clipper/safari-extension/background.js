// Background service worker for Manifest V3 extension to handle fetch requests without CORS blocking

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'VALIDATE_TOKEN') {
    const { token } = request.payload;
    const urls = ['http://127.0.0.1:4040/api/validate', 'http://localhost:4040/api/validate'];
    
    const tryFetch = (index) => {
      if (index >= urls.length) {
        sendResponse({ success: false, error: 'Could not connect to Noda Notes server. Ensure the app is open.' });
        return;
      }
      
      fetch(urls[index], {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${token}`
        }
      })
      .then(response => {
        if (response.ok) {
          sendResponse({ success: true });
        } else {
          sendResponse({ success: false, error: 'Invalid auth token' });
        }
      })
      .catch(error => {
        console.warn(`Fetch to ${urls[index]} failed, trying next...`, error);
        tryFetch(index + 1);
      });
    };

    tryFetch(0);
    return true; // Keep message channel open for async response
  }

  if (request.action === 'SUBMIT_CLIP') {
    const { title, url, contentMarkdown, tags, token, append, author, publishedDate } = request.payload;
    const urls = ['http://127.0.0.1:4040/api/clipper', 'http://localhost:4040/api/clipper'];

    const tryFetch = (index) => {
      if (index >= urls.length) {
        sendResponse({ success: false, error: 'Could not connect to Noda Notes server. Ensure the app is open.' });
        return;
      }

      fetch(urls[index], {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify({
          title,
          url,
          content_markdown: contentMarkdown,
          tags,
          append,
          author,
          published_date: publishedDate
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
        if (error instanceof TypeError && (error.message.includes('Load failed') || error.message.includes('Failed to fetch'))) {
          console.warn(`Fetch to ${urls[index]} failed, trying next...`, error);
          tryFetch(index + 1);
        } else {
          console.error('Clipper submission error:', error);
          sendResponse({ success: false, error: error.message });
        }
      });
    };

    tryFetch(0);
    return true; // Keep message channel open for async response
  }

  if (request.action === 'UPLOAD_ATTACHMENT') {
    const { token, filename, base64Data } = request.payload;
    const urls = ['http://127.0.0.1:4040/api/attachments/upload', 'http://localhost:4040/api/attachments/upload'];

    // Convert base64 data to Uint8Array in the service worker context
    const byteString = atob(base64Data);
    const ab = new ArrayBuffer(byteString.length);
    const ia = new Uint8Array(ab);
    for (let i = 0; i < byteString.length; i++) {
      ia[i] = byteString.charCodeAt(i);
    }

    const tryFetch = (index) => {
      if (index >= urls.length) {
        sendResponse({ success: false, error: 'Could not upload attachment. Ensure the app is open.' });
        return;
      }

      fetch(`${urls[index]}?filename=${encodeURIComponent(filename)}`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/octet-stream'
        },
        body: ab
      })
      .then(async response => {
        if (!response.ok) {
          const text = await response.text();
          throw new Error(text || `Server returned status ${response.status}`);
        }
        return response.json();
      })
      .then(data => {
        sendResponse({ success: true, data });
      })
      .catch(error => {
        console.warn(`Upload to ${urls[index]} failed, trying next...`, error);
        tryFetch(index + 1);
      });
    };

    tryFetch(0);
    return true; // Keep message channel open for async response
  }

  if (request.action === 'CAPTURE_TAB') {
    chrome.tabs.captureVisibleTab(null, { format: 'png' }, (dataUrl) => {
      if (chrome.runtime.lastError || !dataUrl) {
        sendResponse({ success: false, error: chrome.runtime.lastError ? chrome.runtime.lastError.message : 'Capture failed' });
      } else {
        sendResponse({ success: true, dataUrl: dataUrl });
      }
    });
    return true;
  }
});
