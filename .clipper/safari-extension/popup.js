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
  const clipStatus = document.getElementById('clip-status');
  const configLink = document.getElementById('config-link');
  const charCounter = document.getElementById('char-counter');

  let currentTabUrl = '';
  let activeToken = '';

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

  submitClipBtn.addEventListener('click', () => {
    const title = clipTitle.value.trim();
    const tagsStr = clipTags.value.trim();
    const contentMarkdown = clipBody.value;

    if (!title) {
      showStatus(clipStatus, 'Title is required', 'error');
      return;
    }

    const tags = tagsStr ? tagsStr.split(',').map(t => t.trim()).filter(Boolean) : [];

    setLoading(true);
    chrome.runtime.sendMessage({
      action: 'SUBMIT_CLIP',
      payload: {
        title,
        url: currentTabUrl,
        contentMarkdown,
        tags,
        token: activeToken
      }
    }, (response) => {
      setLoading(false);
      if (response && response.success) {
        showStatus(clipStatus, 'Clipped to Noda Notes!', 'success');
        setTimeout(() => {
          window.close();
        }, 1500);
      } else {
        const errMsg = (response && response.error) ? response.error : 'Could not connect to Noda Notes server. Ensure the app is open.';
        showStatus(clipStatus, errMsg, 'error');
      }
    });
  });

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
            return;
          }

          // Query content script
          chrome.tabs.sendMessage(tab.id, { action: 'GET_CLIP_DATA' }, (response) => {
            if (chrome.runtime.lastError || !response) {
              // Content script not loaded (e.g. browser settings page or chrome web store)
              clipTitle.value = tab.title || '';
              clipBody.value = `No content script available on this page.\nURL: ${currentTabUrl}`;
              charCounter.textContent = '0 chars';
              return;
            }

            clipTitle.value = response.title || tab.title || '';
            clipBody.value = response.contentMarkdown || '';
            charCounter.textContent = `${response.contentMarkdown.length} chars`;
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
    if (loading) {
      submitClipBtn.disabled = true;
      submitClipBtn.innerHTML = '<span class="spinner"></span> <span>Clipping...</span>';
      clipStatus.style.display = 'none';
    } else {
      submitClipBtn.disabled = false;
      submitClipBtn.innerHTML = '<span>Clip to Noda Notes</span>';
    }
  }
});
