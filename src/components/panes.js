// panes.js — Resizable three-pane layout with drag dividers

const Panes = (() => {
    let isDragging = false;
    let activeDivider = null;
    let startX = 0;
    let startLeftWidth = 0;
    let startRightFlex = 0;

    function init() {
        const dividers = document.querySelectorAll('.divider');
        dividers.forEach(div => {
            div.addEventListener('mousedown', onDividerMouseDown);
            // Touch support
            div.addEventListener('touchstart', onDividerTouchStart, { passive: false });
        });

        // Set initial widths from data attributes
        document.querySelectorAll('.pane[data-default-width]').forEach(pane => {
            const width = parseInt(pane.dataset.defaultWidth) || 240;
            pane.style.width = width + 'px';
        });

        // Collapse handles
        document.querySelectorAll('.pane-collapse-handle').forEach(handle => {
            handle.addEventListener('click', () => {
                const targetId = handle.dataset.target;
                togglePaneCollapse(targetId);
            });
        });

        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mouseup', onMouseUp);
        document.addEventListener('touchmove', onTouchMove, { passive: false });
        document.addEventListener('touchend', onTouchEnd);
    }

    function onDividerMouseDown(e) {
        if (e.button !== 0) return;
        startDrag(e.target, e.clientX);
        e.preventDefault();
    }

    function onDividerTouchStart(e) {
        if (e.touches.length !== 1) return;
        startDrag(e.target, e.touches[0].clientX);
        e.preventDefault();
    }

    function startDrag(divider, clientX) {
        isDragging = true;
        activeDivider = divider;
        divider.classList.add('active');
        startX = clientX;

        const leftPane = document.getElementById(divider.dataset.left);
        const rightPane = document.getElementById(divider.dataset.right);

        if (leftPane && rightPane) {
            startLeftWidth = leftPane.offsetWidth;
            startRightFlex = rightPane.style.flex || '1';
        }
    }

    function onMouseMove(e) {
        if (!isDragging) return;
        resize(e.clientX);
    }

    function onTouchMove(e) {
        if (!isDragging) return;
        e.preventDefault();
        resize(e.touches[0].clientX);
    }

    function resize(clientX) {
        if (!activeDivider) return;

        const leftPane = document.getElementById(activeDivider.dataset.left);
        const rightPane = document.getElementById(activeDivider.dataset.right);
        if (!leftPane || !rightPane) return;

        const delta = clientX - startX;
        const newLeftWidth = Math.max(
            parseInt(leftPane.dataset.minWidth) || 200,
            startLeftWidth + delta
        );
        const rightMin = parseInt(rightPane.dataset.minWidth) || 400;
        const containerWidth = leftPane.parentElement.offsetWidth;
        const maxLeft = containerWidth - rightMin - (activeDivider.offsetWidth);

        leftPane.style.width = Math.min(newLeftWidth, maxLeft) + 'px';
        rightPane.style.flex = '1';
    }

    function onMouseUp() {
        endDrag();
    }

    function onTouchEnd() {
        endDrag();
    }

    function endDrag() {
        isDragging = false;
        if (activeDivider) {
            activeDivider.classList.remove('active');
            activeDivider = null;
        }
    }

    function togglePaneCollapse(paneId) {
        const pane = document.getElementById(paneId);
        if (!pane) return;

        if (pane.classList.contains('collapsed')) {
            pane.classList.remove('collapsed');
            pane.style.width = (pane.dataset.defaultWidth || '240') + 'px';
        } else {
            pane.classList.add('collapsed');
        }
    }

    // Public API
    return { init };
})();

document.addEventListener('DOMContentLoaded', () => Panes.init());
