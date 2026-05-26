// compose.js — Compose window as floating panel

const Compose = (() => {
    let composeMode = 'new'; // new | reply | forward
    let currentAccountId = null;

    function init() {
        document.getElementById('compose-close').addEventListener('click', close);
        document.getElementById('send-btn').addEventListener('click', send);

        // CC toggle
        document.getElementById('toggle-cc').addEventListener('click', () => {
            const row = document.querySelector('.field-row.collapsible');
            row.classList.toggle('expanded');
        });

        // Formatting toolbar buttons
        document.querySelectorAll('.format-btn[data-command]').forEach(btn => {
            btn.addEventListener('click', () => {
                document.execCommand(btn.dataset.command, false, null);
                document.getElementById('compose-body').focus();
            });
        });

        // Gemini button
        document.getElementById('gemini-draft-btn').addEventListener('click', () => {
            window.app.toggleGeminiOverlay();
        });

        // Keyboard shortcut to close
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && !document.getElementById('compose-panel').classList.contains('hidden')) {
                close();
            }
        });
    }

    function open(options = {}) {
        const panel = document.getElementById('compose-panel');
        panel.classList.remove('hidden');

        composeMode = options.mode || 'new';
        currentAccountId = options.accountId || window.app?.getFirstAccountId() || null;

        // Reset fields
        document.getElementById('compose-to').value = options.to ? options.to.join(', ') : '';
        document.getElementById('compose-cc').value = '';
        document.getElementById('compose-subject').value = options.subject || '';
        document.getElementById('compose-body').innerHTML = '';

        // Pre-fill for replies/forwards
        if (options.mode === 'reply' && options.originalEmail) {
            const quoteBlock = `<br><br><blockquote>${escapeHtml(options.originalEmail.body_text).replace(/\n/g, '<br>')}</blockquote>`;
            document.getElementById('compose-body').innerHTML = '<br><br>' + quoteBlock;
            moveCursorToStart(document.getElementById('compose-body'));
        } else if (options.mode === 'forward' && options.body) {
            document.getElementById('compose-body').innerHTML = escapeHtml(options.body).replace(/\n/g, '<br>');
        }

        // Update title
        const titles = { new: 'New Message', reply: 'Reply', forward: 'Forward' };
        document.getElementById('compose-title').textContent = titles[composeMode] || 'New Message';

        // Store context for Gemini
        Compose._originalEmail = options.originalEmail || null;
        Compose._inReplyTo = options.inReplyTo || null;
        Compose._references = options.references || null;

        // Focus To field or body
        if (document.getElementById('compose-to').value) {
            document.getElementById('compose-body').focus();
        } else {
            document.getElementById('compose-to').focus();
        }
    }

    function close() {
        document.getElementById('compose-panel').classList.add('hidden');
        document.getElementById('gemini-overlay').classList.add('hidden');
        Compose._originalEmail = null;
        Compose._inReplyTo = null;
        Compose._references = null;
    }

    async function send() {
        const to = parseAddresses(document.getElementById('compose-to').value);
        const cc = parseAddresses(document.getElementById('compose-cc').value);
        const subject = document.getElementById('compose-subject').value.trim();
        const body = document.getElementById('compose-body').innerText.trim();

        if (!currentAccountId) {
            alert('No account selected. Please connect a Gmail account first.');
            return;
        }
        if (to.length === 0) {
            alert('Please add at least one recipient.');
            return;
        }
        if (!subject) {
            alert('Please enter a subject.');
            return;
        }

        const sendBtn = document.getElementById('send-btn');
        sendBtn.disabled = true;
        sendBtn.textContent = 'Sending...';

        try {
            await window.__TAURI__.core.invoke('send_email', {
                accountId: currentAccountId,
                to,
                cc,
                subject,
                body,
                inReplyTo: Compose._inReplyTo,
                references: Compose._references,
            });

            close();
            window.app.refreshEmails();
        } catch (e) {
            console.error('Send failed:', e);
            alert('Failed to send email: ' + e);
        } finally {
            sendBtn.disabled = false;
            sendBtn.innerHTML = '<svg class="icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/></svg> Send';
        }
    }

    function parseAddresses(str) {
        return str.split(',')
            .map(a => a.trim())
            .filter(a => a.length > 0 && a.includes('@'));
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    function moveCursorToStart(el) {
        const range = document.createRange();
        const sel = window.getSelection();
        range.setStart(el, 0);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
    }

    // Public API
    return { init, open, close, get _originalEmail() { return Compose._originalEmail; } };
})();
