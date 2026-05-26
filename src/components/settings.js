// settings.js — Settings panel with tabs

const Settings = (() => {
    let isOpen = false;

    function init() {
        document.getElementById('settings-close').addEventListener('click', close);
        document.getElementById('settings-add-account').addEventListener('click', () => {
            window.app.connectAccount();
        });

        // Tab switching
        document.querySelectorAll('.settings-tab').forEach(tab => {
            tab.addEventListener('click', () => switchTab(tab.dataset.settingsTab));
        });

        // Gemini settings
        document.getElementById('save-gemini-key').addEventListener('click', saveGeminiKey);
        document.getElementById('show-gemini-toggle').addEventListener('change', (e) => {
            saveSetting('show_gemini_button', e.target.checked ? 'true' : 'false');
            Gemini.toggleVisibility(e.target.checked);
        });

        // Notification settings
        document.getElementById('notif-enabled-toggle').addEventListener('change', (e) => {
            saveSetting('notifications_enabled', e.target.checked ? 'true' : 'false');
        });
        document.querySelectorAll('input[name="notif-level"]').forEach(radio => {
            radio.addEventListener('change', (e) => {
                saveSetting('notification_level', e.target.value);
            });
        });
        document.getElementById('notif-sound-toggle').addEventListener('change', (e) => {
            saveSetting('notification_sound', e.target.checked ? 'true' : 'false');
        });

        // Appearance settings
        document.querySelectorAll('.theme-option').forEach(btn => {
            btn.addEventListener('click', () => setTheme(btn.dataset.theme));
        });
        document.getElementById('font-size-select').addEventListener('change', (e) => {
            setFontSize(e.target.value);
        });

        // Custom styles
        document.getElementById('add-custom-style').addEventListener('click', () => {
            document.getElementById('new-style-form').classList.toggle('hidden');
        });
        document.getElementById('save-new-style').addEventListener('click', saveCustomStyle);
        document.getElementById('cancel-new-style').addEventListener('click', () => {
            document.getElementById('new-style-form').classList.add('hidden');
        });

        // Close on Escape
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && isOpen) close();
        });
    }

    function open() {
        isOpen = true;
        document.getElementById('settings-panel').classList.remove('hidden');
        loadAllSettings();
        renderAccounts();
        renderCustomStyles();
    }

    function close() {
        isOpen = false;
        document.getElementById('settings-panel').classList.add('hidden');
    }

    function switchTab(tabId) {
        document.querySelectorAll('.settings-tab').forEach(t => t.classList.remove('active'));
        document.querySelectorAll('.settings-tab-content').forEach(c => c.classList.remove('active'));

        document.querySelector(`[data-settings-tab="${tabId}"]`).classList.add('active');
        document.getElementById(`tab-${tabId}`).classList.add('active');
    }

    async function loadAllSettings() {
        // Gemini
        const geminiKey = await loadSetting('gemini_api_key');
        if (geminiKey) {
            document.getElementById('gemini-api-key-input').value = '••••••••';
        }
        const defaultStyle = await loadSetting('default_style') || 'professional';
        document.getElementById('default-style-select').value = defaultStyle;
        const showGemini = await loadSetting('show_gemini_button') !== 'false';
        document.getElementById('show-gemini-toggle').checked = showGemini;

        // Notifications
        const notifEnabled = await loadSetting('notifications_enabled') !== 'false';
        document.getElementById('notif-enabled-toggle').checked = notifEnabled;
        const notifLevel = await loadSetting('notification_level') || 'all';
        const notifRadio = document.querySelector(`input[name="notif-level"][value="${notifLevel}"]`);
        if (notifRadio) notifRadio.checked = true;
        const notifSound = await loadSetting('notification_sound') !== 'false';
        document.getElementById('notif-sound-toggle').checked = notifSound;

        // Appearance
        const theme = await loadSetting('theme') || 'dark';
        setTheme(theme);
        const fontSize = await loadSetting('font_size') || 'medium';
        setFontSize(fontSize);
    }

    async function renderAccounts() {
        try {
            const accounts = await window.__TAURI__.core.invoke('get_accounts');
            const container = document.getElementById('settings-accounts-list');
            container.innerHTML = '';

            accounts.forEach(account => {
                const entry = document.createElement('div');
                entry.className = 'settings-account-entry';
                entry.innerHTML = `
                    <div class="settings-account-info">
                        <div class="settings-account-email">${escapeHtml(account.email)}</div>
                        <div class="settings-account-name">${escapeHtml(account.name)}</div>
                    </div>
                    <div class="settings-account-actions">
                        <button class="btn btn-secondary btn-sm reauth-btn" data-id="${account.id}">Re-auth</button>
                        <button class="btn btn-danger btn-sm remove-btn" data-id="${account.id}">Remove</button>
                    </div>`;
                container.appendChild(entry);
            });

            // Attach event listeners
            container.querySelectorAll('.remove-btn').forEach(btn => {
                btn.addEventListener('click', async () => {
                    if (confirm('Remove this account?')) {
                        await window.__TAURI__.core.invoke('remove_account', { accountId: btn.dataset.id });
                        Sidebar.refresh(await window.__TAURI__.core.invoke('get_accounts'));
                        renderAccounts();
                    }
                });
            });
        } catch (e) {
            console.warn('Failed to load settings accounts:', e);
        }
    }

    async function renderCustomStyles() {
        const stylesJson = await loadSetting('custom_styles') || '[]';
        let styles;
        try { styles = JSON.parse(stylesJson); } catch { styles = []; }

        const container = document.getElementById('custom-styles-list');
        container.innerHTML = '';

        styles.forEach((style, i) => {
            const entry = document.createElement('div');
            entry.className = 'custom-style-entry';
            entry.innerHTML = `
                <span>${escapeHtml(style.name)} — ${escapeHtml(style.description.substring(0, 50))}</span>
                <button class="btn-icon btn-xs delete-style-btn" data-index="${i}">
                    <svg class="icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                </button>`;
            container.appendChild(entry);
        });

        container.querySelectorAll('.delete-style-btn').forEach(btn => {
            btn.addEventListener('click', () => deleteCustomStyle(parseInt(btn.dataset.index)));
        });
    }

    async function saveGeminiKey() {
        const input = document.getElementById('gemini-api-key-input');
        const key = input.value.trim();
        if (!key || key === '••••••••') return;

        await saveSetting('gemini_api_key', key);
        input.value = '••••••••';
    }

    async function saveCustomStyle() {
        const name = document.getElementById('new-style-name').value.trim();
        const desc = document.getElementById('new-style-desc').value.trim();

        if (!name || !desc) {
            alert('Please fill in both name and description.');
            return;
        }

        const stylesJson = await loadSetting('custom_styles') || '[]';
        let styles;
        try { styles = JSON.parse(stylesJson); } catch { styles = []; }

        styles.push({ name, description: desc });
        await saveSetting('custom_styles', JSON.stringify(styles));

        document.getElementById('new-style-name').value = '';
        document.getElementById('new-style-desc').value = '';
        document.getElementById('new-style-form').classList.add('hidden');

        renderCustomStyles();
        updateStyleDropdown(styles);
    }

    async function deleteCustomStyle(index) {
        const stylesJson = await loadSetting('custom_styles') || '[]';
        let styles;
        try { styles = JSON.parse(stylesJson); } catch { styles = []; }

        styles.splice(index, 1);
        await saveSetting('custom_styles', JSON.stringify(styles));

        renderCustomStyles();
        updateStyleDropdown(styles);
    }

    function updateStyleDropdown(styles) {
        const select = document.getElementById('gemini-style-select');
        // Keep preset options, remove existing custom ones
        while (select.options.length > 5) {
            select.remove(5);
        }
        styles.forEach(style => {
            const opt = document.createElement('option');
            opt.value = style.description; // Use description as the style instruction
            opt.textContent = style.name;
            select.appendChild(opt);
        });
    }

    function setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        saveSetting('theme', theme);

        document.querySelectorAll('.theme-option').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.theme === theme);
        });
    }

    function setFontSize(size) {
        document.documentElement.setAttribute('data-font-size', size);
        saveSetting('font_size', size);
        document.getElementById('font-size-select').value = size;
    }

    // --- Helpers ---

    async function saveSetting(key, value) {
        try {
            await window.__TAURI__.core.invoke('save_setting', { key, value });
        } catch (e) {
            console.warn('Failed to save setting:', key, e);
        }
    }

    async function loadSetting(key) {
        try {
            return await window.__TAURI__.core.invoke('load_setting', { key });
        } catch (e) {
            return null;
        }
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    // Public API
    return { init, open, close, renderAccounts };
})();
