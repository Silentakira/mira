// keyboard.js — Keyboard navigation and shortcuts

const Keyboard = (() => {
    let activePane = 'email-list'; // sidebar | email-list | reader

    function init() {
        document.addEventListener('keydown', handleGlobalKeydown);

        // Help overlay
        document.getElementById('help-close').addEventListener('click', closeHelp);
        document.querySelector('.overlay-backdrop')?.addEventListener('click', closeHelp);
    }

    function handleGlobalKeydown(e) {
        // Don't capture when typing in inputs
        const tag = e.target.tagName;
        const isInput = tag === 'INPUT' || tag === 'TEXTAREA' || e.target.isContentEditable;
        if (isInput && !['Escape'].includes(e.key)) return;

        switch (e.key.toLowerCase()) {
            case 'j':
                e.preventDefault();
                EmailList.navigate(1);
                break;
            case 'k':
                e.preventDefault();
                EmailList.navigate(-1);
                break;
            case 'enter':
                if (!isInput) {
                    e.preventDefault();
                    const selected = EmailList.getSelectedEmail();
                    if (selected) window.app.openEmail(selected);
                }
                break;
            case 'c':
                if (!isInput) {
                    e.preventDefault();
                    window.app.openCompose({ mode: 'new' });
                }
                break;
            case 'r':
                if (!isInput) {
                    e.preventDefault();
                    EmailReader.reply();
                }
                break;
            case 'f':
                if (!isInput) {
                    e.preventDefault();
                    EmailReader.forward();
                }
                break;
            case 'e':
                if (!isInput) {
                    e.preventDefault();
                    EmailReader.archive();
                }
                break;
            case '#':
                if (!isInput) {
                    e.preventDefault();
                    EmailReader.deleteEmail();
                }
                break;
            case 'escape':
                // Close compose, settings, help — handled by individual modules
                break;
            case 'tab':
                e.preventDefault();
                cyclePane();
                break;
            case '?':
                if (!isInput) {
                    e.preventDefault();
                    toggleHelp();
                }
                break;
        }
    }

    function cyclePane() {
        const panes = ['sidebar', 'email-list', 'reader'];
        const currentIndex = panes.indexOf(activePane);
        activePane = panes[(currentIndex + 1) % panes.length];

        // Visual focus indicator
        document.querySelectorAll('.pane').forEach(p => p.classList.remove('pane-focused'));
        const paneEl = document.getElementById(activePane);
        if (paneEl) paneEl.classList.add('pane-focused');
    }

    function toggleHelp() {
        const overlay = document.getElementById('help-overlay');
        overlay.classList.toggle('hidden');
    }

    function closeHelp() {
        document.getElementById('help-overlay').classList.add('hidden');
    }

    // Public API
    return { init };
})();
