// sidebar.js — Sidebar navigation, accounts, folders

const Sidebar = (() => {
    let currentFolder = 'unified';
    let selectedAccountId = null;

    function init() {
        // Folder navigation clicks
        document.querySelectorAll('.nav-item[data-folder]').forEach(item => {
            item.addEventListener('click', () => selectFolder(item));
        });

        // Add account button
        document.getElementById('add-account-btn').addEventListener('click', () => {
            window.app.connectAccount();
        });

        // Settings button
        document.getElementById('settings-btn').addEventListener('click', () => {
            window.app.openSettings();
        });

        loadAccounts();
    }

    function selectFolder(item) {
        // Update active state
        document.querySelectorAll('.nav-item[data-folder]').forEach(i => i.classList.remove('active'));
        item.classList.add('active');

        currentFolder = item.dataset.folder;
        selectedAccountId = null; // Reset to unified when switching folders

        // Update header title
        const titles = {
            unified: 'Unified Inbox',
            INBOX: 'Inbox',
            Sent: 'Sent',
            Drafts: 'Drafts',
            '[Gmail]/Trash': 'Trash',
            '[Gmail]/Spam': 'Spam'
        };
        document.getElementById('list-title').textContent = titles[currentFolder] || currentFolder;

        // Load emails for this folder
        window.app.loadEmails(currentFolder);
    }

    async function loadAccounts() {
        try {
            const accounts = await window.__TAURI__.core.invoke('get_accounts');
            renderAccounts(accounts);
        } catch (e) {
            console.warn('Failed to load accounts:', e);
        }
    }

    function renderAccounts(accounts) {
        const container = document.getElementById('account-list');
        container.innerHTML = '';

        accounts.forEach(account => {
            const entry = document.createElement('div');
            entry.className = 'account-entry';
            entry.dataset.accountId = account.id;
            entry.innerHTML = `
                <div class="account-avatar">${account.name.charAt(0).toUpperCase()}</div>
                <span class="account-email">${account.email}</span>
                <div class="account-status ${account.is_connected ? '' : 'disconnected'}"></div>
            `;
            entry.addEventListener('click', () => {
                selectedAccountId = account.id;
                // Could filter emails by account here
            });
            container.appendChild(entry);
        });
    }

    function refresh(accounts) {
        renderAccounts(accounts);
    }

    function updateUnreadBadge(count) {
        const badge = document.getElementById('unread-badge');
        if (count > 0) {
            badge.textContent = count > 99 ? '99+' : count;
            badge.classList.remove('hidden');
        } else {
            badge.classList.add('hidden');
        }
    }

    // Public API
    return {
        init,
        refresh,
        updateUnreadBadge,
        get currentFolder() { return currentFolder; },
        get selectedAccountId() { return selectedAccountId; },
    };
})();
