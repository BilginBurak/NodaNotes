document.addEventListener('DOMContentLoaded', () => {
  const setupScreen = document.getElementById('setup-screen');
  const clipScreen = document.getElementById('clip-screen');
  
  const tokenInput = document.getElementById('token-input');
  const saveTokenBtn = document.getElementById('save-token-btn');
  const setupStatus = document.getElementById('setup-status');

  const clipTitle = document.getElementById('clip-title');
  const clipTags = document.getElementById('clip-tags');
  const clipBody = document.getElementById('clip-body');
  const submitClipBtn = document.getElementById('submit-clip-btn');
  const clipDropdownContainer = document.getElementById('clip-dropdown-container');
  const dropdownToggleBtn = document.getElementById('dropdown-toggle-btn');
  const dropdownMenu = document.getElementById('dropdown-menu');
  const createNewNoteItem = document.getElementById('create-new-note-item');
  const screenshotToggle = document.getElementById('screenshot-toggle');
  const clipStatus = document.getElementById('clip-status');
  const configLink = document.getElementById('config-link');
  const charCounter = document.getElementById('char-counter');

  let currentTabUrl = '';
  let activeToken = '';
  let extractedAuthor = 'Unknown';
  let extractedPublishedDate = 'Unknown';
  let noteExists = false;
  let checkTimeout;

  // Check storage for token
  chrome.storage.local.get(['noda_clipper_token'], (result) => {
    if (result.noda_clipper_token) {
      activeToken = result.noda_clipper_token;
      showClipScreen();
    } else {
      showSetupScreen();
    }
  });

  // Action listeners
  saveTokenBtn.addEventListener('click', () => {
    const token = tokenInput.value.trim();
    if (!token) {
      showStatus(setupStatus, 'Token cannot be empty', 'error');
      return;
    }
    
    saveTokenBtn.disabled = true;
    saveTokenBtn.innerHTML = '<span class="spinner"></span> <span>Saving...</span>';
    setupStatus.style.display = 'none';

    chrome.runtime.sendMessage({
      action: 'VALIDATE_TOKEN',
      payload: { token }
    }, (response) => {
      saveTokenBtn.disabled = false;
      saveTokenBtn.innerHTML = 'Save Configuration';
      
      if (response && response.success) {
        chrome.storage.local.set({ noda_clipper_token: token }, () => {
          activeToken = token;
          showStatus(setupStatus, 'Configured successfully!', 'success');
          setTimeout(() => {
            setupStatus.style.display = 'none';
            showClipScreen();
          }, 1000);
        });
      } else {
        const errMsg = (response && response.error) ? response.error : 'Could not connect to Noda Notes server. Ensure the app is open.';
        showStatus(setupStatus, errMsg, 'error');
      }
    });
  });

  configLink.addEventListener('click', () => {
    showSetupScreen();
  });

  // Dropdown toggle
  dropdownToggleBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    dropdownMenu.classList.toggle('show');
  });

  document.addEventListener('click', () => {
    dropdownMenu.classList.remove('show');
  });

  // Action Buttons Toggling
  function updateActionButtons(exists) {
    noteExists = exists;
    const submitText = submitClipBtn.querySelector('span');
    if (exists) {
      submitText.textContent = 'Append to Existing Note';
      clipDropdownContainer.style.display = 'flex';
    } else {
      submitText.textContent = 'Clip to NodaNotes';
      clipDropdownContainer.style.display = 'none';
      dropdownMenu.classList.remove('show');
    }
  }

  function checkNoteExists() {
    clearTimeout(checkTimeout);
    const title = clipTitle.value.trim();
    if (!title || !activeToken) {
      updateActionButtons(false);
      return;
    }
    checkTimeout = setTimeout(() => {
      const checkUrl = `http://127.0.0.1:4040/api/clipper/check?title=${encodeURIComponent(title)}`;
      fetch(checkUrl, {
        headers: {
          'Authorization': `Bearer ${activeToken}`
        }
      })
      .then(res => {
        if (!res.ok) throw new Error('Network check failed');
        return res.json();
      })
      .then(data => {
        updateActionButtons(data.exists);
      })
      .catch(err => {
        console.warn('Check note failed:', err);
        updateActionButtons(false);
      });
    }, 250);
  }

  clipTitle.addEventListener('input', checkNoteExists);

  submitClipBtn.addEventListener('click', () => {
    submitClipAction(noteExists);
  });

  createNewNoteItem.addEventListener('click', () => {
    submitClipAction(false);
  });

  function dataURItoUint8Array(dataURI) {
    const byteString = atob(dataURI.split(',')[1]);
    const ab = new ArrayBuffer(byteString.length);
    const ia = new Uint8Array(ab);
    for (let i = 0; i < byteString.length; i++) {
      ia[i] = byteString.charCodeAt(i);
    }
    return ia;
  }

  function submitClipAction(shouldAppend) {
    const title = clipTitle.value.trim();
    const tagsStr = clipTags.value.trim();
    let contentMarkdown = clipBody.value;

    if (!title) {
      showStatus(clipStatus, 'Title is required', 'error');
      return;
    }

    const tags = tagsStr ? tagsStr.split(',').map(t => t.trim()).filter(Boolean) : [];

    setLoading(true);

    const performSubmission = (markdownToSubmit) => {
      chrome.runtime.sendMessage({
        action: 'SUBMIT_CLIP',
        payload: {
          title,
          url: currentTabUrl,
          contentMarkdown: markdownToSubmit,
          tags,
          token: activeToken,
          append: shouldAppend,
          author: extractedAuthor,
          publishedDate: extractedPublishedDate
        }
      }, (response) => {
        setLoading(false);
        if (response && response.success) {
          showStatus(clipStatus, shouldAppend ? 'Appended to Noda Notes!' : 'Clipped to Noda Notes!', 'success');
          setTimeout(() => {
            window.close();
          }, 1500);
        } else {
          const errMsg = (response && response.error) ? response.error : 'Could not connect to Noda Notes server. Ensure the app is open.';
          showStatus(clipStatus, errMsg, 'error');
        }
      });
    };

    // Viewport Capture Snipping Engine
    if (screenshotToggle.checked) {
      chrome.tabs.captureVisibleTab(null, { format: 'png' }, (dataUrl) => {
        if (chrome.runtime.lastError || !dataUrl) {
          console.warn('Screenshot capture failed, submitting without screenshot:', chrome.runtime.lastError);
          performSubmission(contentMarkdown);
          return;
        }

        const base64Data = dataUrl.split(',')[1];
        
        // Stream upload via background.js to bypass CORS
        chrome.runtime.sendMessage({
          action: 'UPLOAD_ATTACHMENT',
          payload: {
            token: activeToken,
            filename: 'screenshot.png',
            base64Data: base64Data
          }
        }, (response) => {
          if (response && response.success && response.data && response.data.url) {
            contentMarkdown += `\n\n![Viewport Screenshot](${response.data.url})\n`;
          } else {
            console.warn('Screenshot upload failed, submitting without screenshot:', response ? response.error : 'Unknown error');
          }
          performSubmission(contentMarkdown);
        });
      });
    } else {
      performSubmission(contentMarkdown);
    }
  }

  function showSetupScreen() {
    clipScreen.classList.remove('active');
    setupScreen.classList.add('active');
    tokenInput.value = activeToken;
    tokenInput.focus();
  }

  function showClipScreen() {
    setupScreen.classList.remove('active');
    clipScreen.classList.add('active');
    
    // Request selection or content from the page
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs && tabs[0]) {
        const tab = tabs[0];
        currentTabUrl = tab.url || '';
        
        // Inject content.js dynamically using scripting API
        chrome.scripting.executeScript({
          target: { tabId: tab.id },
          files: ['content.js']
        }, () => {
          if (chrome.runtime.lastError) {
            console.warn('Could not inject content script:', chrome.runtime.lastError);
            clipTitle.value = tab.title || '';
            clipBody.value = `No content script available on this page.\nURL: ${currentTabUrl}`;
            charCounter.textContent = '0 chars';
            checkNoteExists();
            return;
          }

          // Query content script
          chrome.tabs.sendMessage(tab.id, { action: 'GET_CLIP_DATA' }, (response) => {
            if (chrome.runtime.lastError || !response) {
              // Content script not loaded (e.g. browser settings page or chrome web store)
              clipTitle.value = tab.title || '';
              clipBody.value = `No content script available on this page.\nURL: ${currentTabUrl}`;
              charCounter.textContent = '0 chars';
              extractedAuthor = 'Unknown';
              extractedPublishedDate = 'Unknown';
              checkNoteExists();
              return;
            }

            clipTitle.value = response.title || tab.title || '';
            clipBody.value = response.contentMarkdown || '';
            charCounter.textContent = `${response.contentMarkdown.length} chars`;
            extractedAuthor = response.author || 'Unknown';
            extractedPublishedDate = response.publishedDate || 'Unknown';
            checkNoteExists();
          });
        });
      }
    });
  }

  function showStatus(element, message, type) {
    element.textContent = message;
    element.className = `status-msg ${type}`;
    element.style.display = 'block';
  }

  function setLoading(loading) {
    const submitText = submitClipBtn.querySelector('span');
    if (loading) {
      submitClipBtn.disabled = true;
      submitText.textContent = 'Clipping...';
      clipStatus.style.display = 'none';
    } else {
      submitClipBtn.disabled = false;
      submitText.textContent = noteExists ? 'Append to Existing Note' : 'Clip to NodaNotes';
    }
  }
});
