// gemini.js — Gemini AI email drafting integration

const Gemini = (() => {
    let isGenerating = false;

    function init() {
        document.getElementById('gemini-close').addEventListener('click', closeOverlay);
        document.getElementById('gemini-generate').addEventListener('click', generate);
        document.getElementById('gemini-insert').addEventListener('click', insertDraft);
        document.getElementById('gemini-regenerate').addEventListener('click', generate);

        // Manage custom styles button
        document.getElementById('gemini-manage-styles').addEventListener('click', () => {
            window.app.openSettings();
            // Switch to custom styles tab
            setTimeout(() => {
                const tab = document.querySelector('[data-settings-tab="custom-styles"]');
                if (tab) tab.click();
            }, 100);
        });
    }

    function openOverlay() {
        const overlay = document.getElementById('gemini-overlay');
        overlay.classList.remove('hidden');
        document.getElementById('gemini-result').classList.add('hidden');

        // Load saved API key
        loadApiKey();

        // Focus prompt
        setTimeout(() => {
            document.getElementById('gemini-prompt').focus();
        }, 150);
    }

    function closeOverlay() {
        document.getElementById('gemini-overlay').classList.add('hidden');
        isGenerating = false;
    }

    async function generate() {
        if (isGenerating) return;

        const prompt = document.getElementById('gemini-prompt').value.trim();
        if (!prompt) {
            alert('Please describe what you want to write.');
            return;
        }

        // Get or check for API key
        let apiKey = await getApiKey();
        if (!apiKey) {
            alert('Please add your Gemini API key in Settings > Gemini.');
            closeOverlay();
            return;
        }

        const style = document.getElementById('gemini-style-select').value;
        const originalEmail = Compose._originalEmail
            ? JSON.stringify({
                subject: Compose._originalEmail.subject,
                from: Compose._originalEmail.from_name,
                body: Compose._originalEmail.body_text,
              })
            : null;

        isGenerating = true;
        const genBtn = document.getElementById('gemini-generate');
        genBtn.disabled = true;
        genBtn.textContent = 'Generating...';

        try {
            const result = await window.__TAURI__.core.invoke('draft_with_gemini', {
                apiKey,
                prompt,
                style,
                originalEmail,
            });

            showDraft(result.draft);
        } catch (e) {
            console.error('Gemini generation failed:', e);
            alert('Failed to generate draft: ' + e);
        } finally {
            isGenerating = false;
            genBtn.disabled = false;
            genBtn.textContent = 'Generate';
        }
    }

    function showDraft(draftText) {
        const resultDiv = document.getElementById('gemini-result');
        const previewEl = document.getElementById('gemini-draft-preview');
        previewEl.textContent = draftText;
        resultDiv.classList.remove('hidden');
    }

    function insertDraft() {
        const draftText = document.getElementById('gemini-draft-preview').textContent;
        const bodyEl = document.getElementById('compose-body');

        // Insert at cursor position or append
        bodyEl.focus();
        if (window.getSelection().rangeCount > 0) {
            const range = window.getSelection().getRangeAt(0);
            range.deleteContents();
            range.insertNode(document.createTextNode(draftText));
        } else {
            bodyEl.innerText += '\n\n' + draftText;
        }

        closeOverlay();
    }

    async function getApiKey() {
        try {
            return await window.__TAURI__.core.invoke('load_setting', { key: 'gemini_api_key' });
        } catch (e) {
            console.warn('Failed to load API key:', e);
            return null;
        }
    }

    async function loadApiKey() {
        const key = await getApiKey();
        if (key) {
            document.getElementById('gemini-api-key-input').value = '••••••••';
        }
    }

    function toggleVisibility(visible) {
        const btn = document.getElementById('gemini-draft-btn');
        btn.style.display = visible ? '' : 'none';
    }

    // Public API
    return { init, openOverlay, closeOverlay, toggleVisibility };
})();
