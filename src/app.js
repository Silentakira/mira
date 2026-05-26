// app.js — Main application controller

const App = (() => {
    let accounts = [];
    let currentFolder = 'unified';

    async function init() {
        console.log('Mira starting...');

        // Initialize all components
        Panes.init();
        Sidebar.init();
        EmailList.init();
        EmailReader.init();
        Compose.init();
        Gemini.init();
        Settings.init();
        Keyboard.init();

        // Load saved settings
        await loadSettings();

        // Load connected accounts
        try {
            accounts = await window.__TAURI__.core.invoke('get_accounts');
            Sidebar.refresh(accounts);

            if (accounts.length > 0) {
                loadEmails('unified');
            } else {
                showOnboarding();
            }
        } catch (e) {
            console.warn('Failed to initialize:', e);
            showOnboarding();
        }

        // Listen for notification events from backend
        if (window.__TAURI__.event) {
            window.__TAURI__.event.listen('new-email-notification', (event) => {
                const { title, body } = event.payload;
                showNotification(title, body);
            });
        }
    }

    async function connectAccount() {
        try {
            // Show a dialog telling user what's happening
            const confirmed = confirm(
                'Mira will open your browser for Google authentication.\n\n' +
                'After you sign in and grant permission, you\'ll be redirected back to Mira automatically.\n\n' +
                'If the automatic redirect doesn\'t work, Google will show you a code — paste it into the dialog that appears.'
            );
            if (!confirmed) return;

            const account = await window.__TAURI__.core.invoke('connect_account');
            accounts.push(account);
            Sidebar.refresh(accounts);
            Settings.renderAccounts?.();
            loadEmails(currentFolder);
        } catch (e) {
            console.error('Failed to connect account (auto-callback):', e);

            // Fallback: ask user to paste the authorization code manually
            const code = prompt(
                'Automatic callback didn\'t fire.\n\n' +
                'Go back to your browser — after authorizing, Google should have shown you a code.\n' +
                'Paste that authorization code here:'
            );

            if (code && code.trim()) {
                try {
                    const account = await window.__TAURI__.core.invoke('complete_auth_with_code', { code: code.trim() });
                    accounts.push(account);
                    Sidebar.refresh(accounts);
                    Settings.renderAccounts?.();
                    loadEmails(currentFolder);
                } catch (e2) {
                    console.error('Failed with manual code:', e2);
                    alert('Failed to connect: ' + e2);
                }
            }
        }
    }

    async function loadEmails(folder) {
        currentFolder = folder;
        EmailList.setLoading(true);

        try {
            let emails;
            if (folder === 'unified') {
                emails = await window.__TAURI__.core.invoke('fetch_unified_inbox', { page: 0, pageSize: 50 });
            } else {
                // Use first connected account for non-unified folders
                const accountId = accounts.find(a => a.is_connected)?.id;
                if (!accountId) {
                    EmailList.setLoading(false);
                    return;
                }
                emails = await window.__TAURI__.core.invoke('fetch_emails', {
                    accountId,
                    folder,
                    page: 0,
                    pageSize: 50,
                });
            }

            EmailList.render(emails);

            // Update unread count for unified inbox
            if (folder === 'unified') {
                const unreadCount = emails.filter(e => !e.is_read).length;
                Sidebar.updateUnreadBadge(unreadCount);
            }
        } catch (e) {
            console.error('Failed to load emails:', e);
            EmailList.setLoading(false);
        }
    }

    async function refreshEmails() {
        await loadEmails(currentFolder);
    }

    async function openEmail(email) {
        EmailReader.show(email);
    }

    function openCompose(options = {}) {
        // Set default account ID
        if (!options.accountId && accounts.length > 0) {
            options.accountId = accounts[0].id;
        }
        Compose.open(options);
    }

    function openSettings() {
        Settings.open();
    }

    function toggleGeminiOverlay() {
        const overlay = document.getElementById('gemini-overlay');
        if (overlay.classList.contains('hidden')) {
            Gemini.openOverlay();
        } else {
            Gemini.closeOverlay();
        }
    }

    function showOnboarding() {
        const emptyState = document.getElementById('empty-state');
        emptyState.innerHTML = `
            <p>Welcome to Mira</p>
            <p class="subtext">Connect your Gmail account to get started</p>
            <button id="onboarding-connect" class="btn btn-primary" style="margin-top: 16px;">
                Connect Gmail Account
            </button>`;
        document.getElementById('onboarding-connect')?.addEventListener('click', () => {
            connectAccount();
        });
    }

    async function loadSettings() {
        try {
            // Theme
            const theme = await window.__TAURI__.core.invoke('load_setting', { key: 'theme' });
            if (theme) {
                document.documentElement.setAttribute('data-theme', theme);
            }

            // Font size
            const fontSize = await window.__TAURI__.core.invoke('load_setting', { key: 'font_size' });
            if (fontSize) {
                document.documentElement.setAttribute('data-font-size', fontSize);
            }
        } catch (e) {
            console.warn('Failed to load settings:', e);
        }
    }

    function showNotification(title, body) {
        // Request permission and show native notification
        if ('Notification' in window && Notification.permission === 'granted') {
            new Notification(title, { body, icon: '/icons/32x32.png' });
        }
    }

    function getFirstAccountId() {
        return accounts.find(a => a.is_connected)?.id || null;
    }

    // Expose public API on window for cross-module access
    window.app = {
        init,
        connectAccount,
        loadEmails,
        refreshEmails,
        openEmail,
        openCompose,
        openSettings,
        toggleGeminiOverlay,
        getFirstAccountId,
    };

    // Auto-init when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => App.init());
    } else {
        App.init();
    }

    return window.app;
})();
