// email-reader.js — Full email view with actions

const EmailReader = (() => {
    let currentEmail = null;

    function init() {
        document.getElementById('reply-btn').addEventListener('click', () => reply());
        document.getElementById('forward-btn').addEventListener('click', () => forward());
        document.getElementById('archive-btn').addEventListener('click', () => archive());
        document.getElementById('delete-btn').addEventListener('click', () => deleteEmail());
    }

    function show(email) {
        currentEmail = email;

        document.getElementById('reader-empty').classList.add('hidden');
        document.getElementById('reader-content').classList.remove('hidden');

        // Header
        document.getElementById('reader-subject').textContent = email.subject;
        document.getElementById('reader-from-name').textContent = email.from_name || '';
        document.getElementById('reader-from-address').textContent = `<${email.from_address}>`;
        document.getElementById('reader-date').textContent = formatFullDate(email.timestamp);
        document.getElementById('reader-to').textContent = email.to.join(', ');

        // CC line
        const ccLine = document.getElementById('reader-cc-line');
        if (email.cc && email.cc.length > 0) {
            ccLine.classList.remove('hidden');
            document.getElementById('reader-cc').textContent = email.cc.join(', ');
        } else {
            ccLine.classList.add('hidden');
        }

        // Body
        const bodyEl = document.getElementById('reader-body');
        if (email.body_html) {
            bodyEl.innerHTML = sanitizeHtml(email.body_html);
        } else {
            bodyEl.textContent = email.body_text;
            bodyEl.style.whiteSpace = 'pre-wrap';
        }
    }

    function clear() {
        currentEmail = null;
        document.getElementById('reader-empty').classList.remove('hidden');
        document.getElementById('reader-content').classList.add('hidden');
    }

    function reply() {
        if (!currentEmail) return;
        window.app.openCompose({
            mode: 'reply',
            to: [currentEmail.from_address],
            subject: `Re: ${currentEmail.subject}`,
            originalEmail: currentEmail,
            inReplyTo: currentEmail.message_id,
            references: buildReferences(currentEmail),
        });
    }

    function forward() {
        if (!currentEmail) return;
        const fwdSubject = currentEmail.subject.startsWith('Fwd:')
            ? currentEmail.subject
            : `Fwd: ${currentEmail.subject}`;

        const quotedBody = `\n\n---------- Forwarded message ----------\nFrom: ${currentEmail.from_name} <${currentEmail.from_address}>\nDate: ${formatFullDate(currentEmail.timestamp)}\nSubject: ${currentEmail.subject}\n\n${currentEmail.body_text}`;

        window.app.openCompose({
            mode: 'forward',
            subject: fwdSubject,
            body: quotedBody,
            originalEmail: currentEmail,
        });
    }

    async function archive() {
        if (!currentEmail) return;
        try {
            await window.__TAURI__.core.invoke('delete_email', {
                accountId: currentEmail.account_id,
                messageId: currentEmail.message_id
            });
            window.app.refreshEmails();
            clear();
        } catch (e) {
            console.error('Archive failed:', e);
        }
    }

    async function deleteEmail() {
        if (!currentEmail) return;
        try {
            await window.__TAURI__.core.invoke('delete_email', {
                accountId: currentEmail.account_id,
                messageId: currentEmail.message_id
            });
            window.app.refreshEmails();
            clear();
        } catch (e) {
            console.error('Delete failed:', e);
        }
    }

    function buildReferences(email) {
        if (email.references) return `${email.references} ${email.message_id}`;
        return email.message_id;
    }

    function formatFullDate(timestamp) {
        return new Date(timestamp).toLocaleDateString('en-US', {
            weekday: 'short',
            month: 'short',
            day: 'numeric',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        });
    }

    function sanitizeHtml(html) {
        // Basic sanitization — strip scripts, styles, dangerous attributes
        const div = document.createElement('div');
        div.innerHTML = html;
        div.querySelectorAll('script, style, link, meta').forEach(el => el.remove());
        div.querySelectorAll('[onclick], [onerror], [onload]').forEach(el => {
            el.removeAttribute('onclick');
            el.removeAttribute('onerror');
            el.removeAttribute('onload');
        });
        return div.innerHTML;
    }

    // Public API
    return { init, show, clear };
})();
