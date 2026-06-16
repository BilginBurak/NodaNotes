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
  const selectAreaBtn = document.getElementById('select-area-btn');
  const areaCapturedIndicator = document.getElementById('area-captured-indicator');
  const clipStatus = document.getElementById('clip-status');
  const configLink = document.getElementById('config-link');
  const charCounter = document.getElementById('char-counter');

  let currentTabUrl = '';
  let activeToken = '';
  let extractedAuthor = 'Unknown';
  let extractedPublishedDate = 'Unknown';
  let noteExists = false;
  let areaScreenshotUrl = '';
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

  // Clear area selection when viewport toggle is clicked on
  screenshotToggle.addEventListener('change', () => {
    if (screenshotToggle.checked) {
      areaScreenshotUrl = '';
      areaCapturedIndicator.style.display = 'none';
      chrome.storage.local.get(['temp_clip_state'], (result) => {
        if (result.temp_clip_state) {
          const state = result.temp_clip_state;
          delete state.screenshotUrl;
          chrome.storage.local.set({ temp_clip_state: state });
        }
      });
    }
  });

  // "Select Area" Action
  selectAreaBtn.addEventListener('click', () => {
    const title = clipTitle.value.trim();
    const tagsStr = clipTags.value.trim();
    const contentMarkdown = clipBody.value;

    const tempState = {
      title,
      tags: tagsStr,
      contentMarkdown,
      author: extractedAuthor,
      publishedDate: extractedPublishedDate,
      url: currentTabUrl,
      screenshotUrl: areaScreenshotUrl
    };

    chrome.storage.local.set({ temp_clip_state: tempState }, () => {
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs && tabs[0]) {
          chrome.tabs.sendMessage(tabs[0].id, { action: 'START_AREA_SELECTION' }, () => {
            window.close(); // Popup closes instantly
          });
        }
      });
    });
  });

  submitClipBtn.addEventListener('click', () => {
    submitClipAction(noteExists);
  });

  createNewNoteItem.addEventListener('click', () => {
    submitClipAction(false);
  });

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
          // Clear temp clip state from storage upon success
          chrome.storage.local.remove(['temp_clip_state'], () => {
            showStatus(clipStatus, shouldAppend ? 'Appended to Noda Notes!' : 'Clipped to Noda Notes!', 'success');
            setTimeout(() => {
              window.close();
            }, 1500);
          });
        } else {
          const errMsg = (response && response.error) ? response.error : 'Could not connect to Noda Notes server. Ensure the app is open.';
          showStatus(clipStatus, errMsg, 'error');
        }
      });
    };

    // If an area selection screenshot is attached
    if (areaScreenshotUrl) {
      contentMarkdown += `\n\n![Viewport Screenshot](${areaScreenshotUrl})\n`;
      performSubmission(contentMarkdown);
    } else if (screenshotToggle.checked) {
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
        
        // Restore from storage if exists
        chrome.storage.local.get(['temp_clip_state'], (result) => {
          if (result.temp_clip_state) {
            const state = result.temp_clip_state;
            clipTitle.value = state.title || '';
            clipTags.value = state.tags || '';
            clipBody.value = state.contentMarkdown || '';
            extractedAuthor = state.author || 'Unknown';
            extractedPublishedDate = state.publishedDate || 'Unknown';
            currentTabUrl = state.url || currentTabUrl;
            
            if (state.screenshotUrl) {
              areaScreenshotUrl = state.screenshotUrl;
              areaCapturedIndicator.style.display = 'flex';
              screenshotToggle.checked = false;
            }
            charCounter.textContent = `${clipBody.value.length} chars`;
            checkNoteExists();
          } else {
            // Load fresh content
            injectAndLoadContent(tab);
          }
        });
      }
    });
  }

  function injectAndLoadContent(tab) {
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

      chrome.tabs.sendMessage(tab.id, { action: 'GET_CLIP_DATA' }, (response) => {
        if (chrome.runtime.lastError || !response) {
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
