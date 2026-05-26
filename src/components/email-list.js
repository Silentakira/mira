// email-list.js — Email list rendering and selection

const EmailList = (() => {
    let emails = [];
    let selectedIndex = -1;
    let isLoading = false;

    function init() {
        // Refresh button
        document.getElementById('refresh-btn').addEventListener('click', () => {
            window.app.refreshEmails();
        });
    }

    function setLoading(loading) {
        isLoading = loading;
        const emptyState = document.getElementById('empty-state');
        if (loading && emails.length === 0) {
            emptyState.innerHTML = '<p><span class="loading-spinner"></span> Loading...</p>';
        }
    }

    function render(emailList) {
        emails = emailList;
        selectedIndex = -1;
        const container = document.getElementById('email-items');

        if (emails.length === 0) {
            container.innerHTML = `
                <div id="empty-state" class="empty-state">
                    <p>No emails</p>
                    <p class="subtext">This folder is empty</p>
                </div>`;
            return;
        }

        container.innerHTML = emails.map((email, i) => createEmailRow(email, i)).join('');

        // Attach click handlers
        container.querySelectorAll('.email-item').forEach((item, i) => {
            item.addEventListener('click', () => selectEmail(i));
        });
    }

    function createEmailRow(email, index) {
        const dateStr = formatRelativeTime(email.timestamp);
        const unreadClass = email.is_read ? '' : 'unread';
        const starredClass = email.is_starred ? 'starred' : '';

        return `
            <div class="email-item ${unreadClass}" data-index="${index}">
                <div class="email-row-top">
                    <span class="email-from">${escapeHtml(email.from_name || email.from_address)}</span>
                    <span class="email-date">${dateStr}</span>
                </div>
                <div class="email-subject">${escapeHtml(email.subject)}</div>
                <div class="email-preview">${escapeHtml(email.preview)}</div>
            </div>`;
    }

    function selectEmail(index) {
        if (index < 0 || index >= emails.length) return;

        // Deselect previous
        const prevSelected = document.querySelector('.email-item.selected');
        if (prevSelected) prevSelected.classList.remove('selected');

        selectedIndex = index;
        const item = document.querySelector(`.email-item[data-index="${index}"]`);
        if (item) item.classList.add('selected');

        const email = emails[index];

        // Mark as read in UI
        item.classList.remove('unread');

        // Show in reader
        window.app.openEmail(email);

        // Mark as read on server (fire and forget)
        if (!email.is_read) {
            window.__TAURI__.core.invoke('mark_as_read', {
                accountId: email.account_id,
                messageId: email.message_id
            }).catch(console.warn);
        }
    }

    function navigate(direction) {
        if (emails.length === 0) return;

        let newIndex = selectedIndex + direction;
        if (newIndex < 0) newIndex = 0;
        if (newIndex >= emails.length) newIndex = emails.length - 1;

        selectEmail(newIndex);

        // Scroll into view
        const item = document.querySelector(`.email-item[data-index="${newIndex}"]`);
        if (item) item.scrollIntoView({ block: 'nearest' });
    }

    function getSelectedEmail() {
        if (selectedIndex >= 0 && selectedIndex < emails.length) {
            return emails[selectedIndex];
        }
        return null;
    }

    // --- Helpers ---

    function formatRelativeTime(timestamp) {
        const diff = Date.now() - timestamp;
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(diff / 3600000);
        const days = Math.floor(diff / 86400000);

        if (minutes < 1) return 'now';
        if (minutes < 60) return `${minutes}m ago`;
        if (hours < 24) return `${hours}h ago`;
        if (days < 7) return `${days}d ago`;

        return new Date(timestamp).toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric'
        });
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    // Public API
    return {
        init,
        render,
        setLoading,
        navigate,
        getSelectedEmail,
        get emails() { return emails; },
        get selectedIndex() { return selectedIndex; },
    };
})();
