// Theme toggle functionality for Lona Documentation

function initTheme() {
    const saved = localStorage.getItem('lona-theme');
    if (saved) {
        document.documentElement.setAttribute('data-theme', saved);
        updateToggleButton(saved);
    }
}

function toggleTheme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
    localStorage.setItem('lona-theme', next);
    updateToggleButton(next);
}

function updateToggleButton(theme) {
    const icon = document.getElementById('theme-icon');
    const label = document.getElementById('theme-label');
    if (icon && label) {
        if (theme === 'dark') {
            icon.textContent = '\u2600'; // Sun symbol
            label.textContent = 'Light';
        } else {
            icon.textContent = '\u263e'; // Moon symbol
            label.textContent = 'Dark';
        }
    }
}

document.addEventListener('DOMContentLoaded', initTheme);
