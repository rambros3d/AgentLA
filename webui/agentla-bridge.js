/**
 * AgentLA OS Bridge Script
 * Injected or included into custom LittleFS web apps under /apps/{app-name}/
 * Provides:
 * 1. Universal Clean OS Top Bar (Matching Workbench & Launcher)
 * 2. Floating macOS-Style Glass Dock with live Pinned Apps, Telemetry HUD & Storage Info
 * 3. Universal Real-Time Theme Synchronization Engine
 * 4. Target Power Rail & Emergency Safe State Controls with SSE Telemetry
 * 5. Hardware Profile Switcher & Global Device Specs
 */
(function() {
    'use strict';

    if (window.__AGENTLA_BRIDGE_LOADED__) return;
    window.__AGENTLA_BRIDGE_LOADED__ = true;

    var isDemo = window.location.protocol === 'file:' ||
                 window.location.protocol === 'https:' || 
                 window.location.hostname.includes('github.io') ||
                 window.location.search.includes('demo=1');

    var homeUrl = isDemo ? (window.location.pathname.includes('/apps/') || window.location.pathname.includes('/app/') ? '../../index.html' : 'index.html') : '/home';
    var labUrl = isDemo ? (window.location.pathname.includes('/apps/') || window.location.pathname.includes('/app/') ? '../../lab.html' : 'lab.html') : '/scope';

    // -------------------------------------------------------------------------
    // Universal Theme Synchronization Engine
    // -------------------------------------------------------------------------
    var themeState = {
        mode: localStorage.getItem('agentla-theme-mode') || 'system',
        defaultLightTheme: 'emerald',
        defaultDarkTheme: 'forest'
    };

    function syncTheme(theme, scheme) {
        try {
            if (!theme) {
                theme = localStorage.getItem('agentla-theme');
                var mode = localStorage.getItem('agentla-theme-mode') || 'system';
                themeState.mode = mode;
                if (!theme) {
                    var prefersDark = !window.matchMedia || window.matchMedia('(prefers-color-scheme: dark)').matches;
                    theme = (mode === 'light' || (mode === 'system' && !prefersDark)) ? 'emerald' : 'forest';
                }
            }
            if (!scheme) {
                var isLight = (theme === 'emerald' || theme === 'caramellatte' || theme === 'light');
                scheme = isLight ? 'light' : 'dark';
            }
            document.documentElement.setAttribute('data-theme', theme);
            document.documentElement.setAttribute('data-theme-scheme', scheme);
            document.documentElement.style.colorScheme = scheme;

            var toggleBtn = document.getElementById('btn-theme-toggle');
            if (toggleBtn) {
                toggleBtn.setAttribute('data-mode', themeState.mode);
                toggleBtn.className = 'theme-toggle-btn mode-' + themeState.mode;
            }

            window.dispatchEvent(new CustomEvent('agentla-theme-changed', { detail: { theme: theme, scheme: scheme } }));
        } catch (_) {}
    }

    // Apply saved theme immediately
    syncTheme();

    window.addEventListener('storage', function(e) {
        if (e.key === 'agentla-theme' || e.key === 'agentla-theme-mode') {
            syncTheme();
        }
    });

    if (typeof BroadcastChannel !== 'undefined') {
        try {
            var themeChan = new BroadcastChannel('agentla-theme-channel');
            themeChan.onmessage = function(event) {
                if (event.data && event.data.type === 'THEME_CHANGE') {
                    syncTheme(event.data.theme, event.data.scheme);
                }
            };
        } catch (_) {}
    }

    window.addEventListener('message', function(event) {
        if (event.data && (event.data.type === 'THEME_CHANGE' || event.data.type === 'AGENTLA_THEME_CHANGE')) {
            syncTheme(event.data.theme, event.data.scheme);
        }
    });

    function cycleThemeMode() {
        if (themeState.mode === 'system') themeState.mode = 'light';
        else if (themeState.mode === 'light') themeState.mode = 'dark';
        else themeState.mode = 'system';

        var prefersDark = !window.matchMedia || window.matchMedia('(prefers-color-scheme: dark)').matches;
        var effectiveTheme = (themeState.mode === 'light') ? 'emerald' :
                             (themeState.mode === 'dark') ? 'forest' :
                             (prefersDark ? 'forest' : 'emerald');

        try {
            localStorage.setItem('agentla-theme-mode', themeState.mode);
            localStorage.setItem('agentla-theme', effectiveTheme);
        } catch (_) {}

        syncTheme(effectiveTheme, (effectiveTheme === 'emerald') ? 'light' : 'dark');

        if (typeof BroadcastChannel !== 'undefined') {
            try {
                var ch = new BroadcastChannel('agentla-theme-channel');
                ch.postMessage({ type: 'THEME_CHANGE', theme: effectiveTheme, mode: themeState.mode, scheme: (effectiveTheme === 'emerald') ? 'light' : 'dark' });
            } catch (_) {}
        }
    }

    // -------------------------------------------------------------------------
    // Hardware Profiles Support
    // -------------------------------------------------------------------------
    var cachedProfileSlots = [
        { id: 0, name: "Safe State", is_boot: true, active: true },
        { id: 1, name: "Logic Analyzer", is_boot: false, active: false },
        { id: 2, name: "CAN-FD Sniffer", is_boot: false, active: false },
        { id: 3, name: "I2C / SPI Master", is_boot: false, active: false }
    ];
    var activeProfileSlot = 0;

    function toggleProfileMenu(forceState) {
        var menu = document.getElementById('profile-dropdown-menu');
        if (!menu) return;
        var shouldShow = forceState !== undefined ? forceState : (menu.style.display === 'none');
        menu.style.display = shouldShow ? 'block' : 'none';
    }

    function renderProfileMenu(activeName) {
        var nameEl = document.getElementById('active-profile-name');
        if (nameEl && activeName) nameEl.textContent = activeName;
        var slotsContainer = document.getElementById('profile-menu-slots');
        if (!slotsContainer) return;
        slotsContainer.innerHTML = '';

        cachedProfileSlots.forEach(function(slot) {
            var item = document.createElement('div');
            item.className = 'profile-slot-item' + (slot.id === activeProfileSlot ? ' active-slot' : '');
            item.innerHTML = '<span class="profile-slot-title">' + (slot.name || 'Profile ' + slot.id) + '</span>' +
                (slot.is_boot ? '<span class="profile-boot-badge">BOOT</span>' : '');
            item.addEventListener('click', function() {
                applyHardwareProfile(slot.id);
            });
            slotsContainer.appendChild(item);
        });
    }

    async function fetchProfiles() {
        try {
            var res = await fetch('/api/profiles');
            if (res.ok) {
                var data = await res.json();
                var list = Array.isArray(data) ? data : (data.slots || []);
                if (list.length > 0) cachedProfileSlots = list;
                if (data && data.active_slot !== undefined) activeProfileSlot = data.active_slot;
            }
        } catch (_) {}
        renderProfileMenu();
    }

    async function applyHardwareProfile(slotId) {
        try {
            await fetch('/api/profiles/apply', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ slot: slotId })
            });
        } catch (_) {}
        activeProfileSlot = slotId;
        var target = cachedProfileSlots.find(function(s) { return s.id === slotId; });
        renderProfileMenu(target ? target.name : 'Profile ' + slotId);
        toggleProfileMenu(false);
    }

    // -------------------------------------------------------------------------
    // Unified Top Bar Wiring & Mounting
    // -------------------------------------------------------------------------
    function initTopBar() {
        var topBar = document.getElementById('top-bar');
        if (!topBar) {
            // Mount top bar dynamically if not already present
            topBar = document.createElement('header');
            topBar.id = 'top-bar';
            topBar.className = 'app-header navbar bg-base-100 border-b border-base-300';
            topBar.innerHTML = `
                <div class="bar-left navbar-start">
                    <a href="${homeUrl}" class="brand-item" id="brand-logo" title="AgentLA App Launcher (Return to Apps)" style="text-decoration:none; color:inherit; cursor:pointer; display:inline-flex; align-items:center; gap:8px;">
                        <span class="start-menu-icon" aria-hidden="true"></span>
                        <span class="brand-title">AgentLA</span>
                        <span class="status status-success status-xs" title="Hardware Engine Online" aria-label="Online"></span>
                    </a>
                </div>
                <div class="bar-right navbar-end">
                    <button class="preset-pill target-power-pill active-on" id="btn-target-power-pill" title="Target Power Rail (Click to toggle ON/OFF)">
                        <svg class="solar-icon power-bolt-icon" width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
                        </svg>
                        <span class="power-pill-label" id="top-bar-power-text"><span id="top-bar-voltage-val">3.31V</span> · <span id="top-bar-current-val">14mA</span></span>
                    </button>
                    <button class="preset-pill emergency-safe-btn" id="btn-emergency-safe" title="Emergency Safe State: High-Z all pins & cut power (Hotkey: Shift+Esc)">
                        <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.2" style="vertical-align: middle; margin-right: 4px;">
                            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                            <line x1="12" y1="8" x2="12" y2="12"/>
                            <line x1="12" y1="16" x2="12.01" y2="16"/>
                        </svg>
                        SAFE STATE
                    </button>
                    <div class="profile-dropdown-wrapper" id="profile-dropdown-wrapper">
                        <button class="preset-pill profile-pill" id="btn-profile-toggle" title="Hardware Profiles stored in flash">
                            <span class="profile-name-text" id="active-profile-name">Safe State</span>
                            <span class="dropdown-arrow">▾</span>
                        </button>
                        <div class="profile-dropdown-menu" id="profile-dropdown-menu" style="display: none;">
                            <div class="profile-menu-header">HARDWARE PROFILES (FLASH)</div>
                            <div class="profile-menu-slots" id="profile-menu-slots"></div>
                        </div>
                    </div>
                    <button class="theme-toggle-btn mode-${themeState.mode}" id="btn-theme-toggle" data-mode="${themeState.mode}" title="Theme Mode" aria-label="Toggle Theme Mode">
                        <svg class="solar-icon icon-sun" width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path d="M18 12C18 15.3137 15.3137 18 12 18C8.68629 18 6 15.3137 6 12C6 8.68629 8.68629 6 12 6C15.3137 6 18 8.68629 18 12Z" fill="currentColor"/>
                            <path fill-rule="evenodd" clip-rule="evenodd" d="M12 1.25C12.4142 1.25 12.75 1.58579 12.75 2V3C12.75 3.41421 12.4142 3.75 12 3.75C11.5858 3.75 11.25 3.41421 11.25 3V2C11.25 1.58579 11.5858 1.25 12 1.25ZM1.25 12C1.25 11.5858 1.58579 11.25 2 11.25H3C3.41421 11.25 3.75 11.5858 3.75 12C3.75 12.4142 3.41421 12.75 3 12.75H2C1.58579 12.75 1.25 12.4142 1.25 12ZM20.25 12C20.25 11.5858 20.5858 11.25 21 11.25H22C22.4142 11.25 22.75 11.5858 22.75 12C22.75 12.4142 22.4142 12.75 22 12.75H21C20.5858 12.75 20.25 12.4142 20.25 12ZM12 20.25C12.4142 20.25 12.75 20.5858 12.75 21V22C12.75 22.4142 12.4142 22.75 12 22.75C11.5858 22.75 11.25 22.4142 11.25 22V21C11.25 20.5858 11.5858 20.25 12 20.25Z" fill="currentColor"/>
                        </svg>
                    </button>
                    <button class="action-btn icon-btn header-btn" id="btn-header-settings" title="System Settings">
                        <svg class="solar-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" fill-rule="evenodd" clip-rule="evenodd" d="M14.2788 2.15224C13.9085 2 13.439 2 12.5 2C11.561 2 11.0915 2 10.7212 2.15224C10.2274 2.35523 9.83509 2.74458 9.63056 3.23463C9.53719 3.45834 9.50065 3.7185 9.48635 4.09799C9.46534 4.65568 9.17716 5.17189 8.69017 5.45093C8.20318 5.72996 7.60864 5.71954 7.11149 5.45876C6.77318 5.2813 6.52789 5.18262 6.28599 5.15102C5.75609 5.08178 5.22018 5.22429 4.79616 5.5472C4.47814 5.78938 4.24339 6.1929 3.7739 6.99993C3.30441 7.80697 3.06967 8.21048 3.01735 8.60491C2.94758 9.1308 3.09118 9.66266 3.41655 10.0835C3.56506 10.2756 3.77377 10.437 4.0977 10.639C4.57391 10.936 4.88032 11.4419 4.88029 12C4.88026 12.5581 4.57386 13.0639 4.0977 13.3608C3.77372 13.5629 3.56497 13.7244 3.41645 13.9165C3.09108 14.3373 2.94749 14.8691 3.01725 15.395C3.06957 15.7894 3.30432 16.193 3.7738 17C4.24329 17.807 4.47804 18.2106 4.79606 18.4527C5.22008 18.7756 5.75599 18.9181 6.28589 18.8489C6.52778 18.8173 6.77305 18.7186 7.11133 18.5412C7.60852 18.2804 8.2031 18.27 8.69012 18.549C9.17714 18.8281 9.46533 19.3443 9.48635 19.9021C9.50065 20.2815 9.53719 20.5417 9.63056 20.7654C9.83509 21.2554 10.2274 21.6448 10.7212 21.8478C11.0915 22 11.561 22 12.5 22C13.439 22 13.9085 22 14.2788 21.8478C14.7726 21.6448 15.1649 21.2554 15.3694 20.7654C15.4628 20.5417 15.4994 20.2815 15.5137 19.902C15.5347 19.3443 15.8228 18.8281 16.3098 18.549C16.7968 18.2699 17.3914 18.2804 17.8886 18.5412C18.2269 18.7186 18.4721 18.8172 18.714 18.8488C19.2439 18.9181 19.7798 18.7756 20.2038 18.4527C20.5219 18.2105 20.7566 17.807 21.2261 16.9999C21.6956 16.1929 21.9303 15.7894 21.9827 15.395C22.0524 14.8691 21.9088 14.3372 21.5835 13.9164C21.4349 13.7243 21.2262 13.5628 20.9022 13.3608C20.4261 13.0639 20.1197 12.558 20.1197 11.9999C20.1197 11.4418 20.4261 10.9361 20.9022 10.6392C21.2263 10.4371 21.435 10.2757 21.5836 10.0835C21.9089 9.66273 22.0525 9.13087 21.9828 8.60497C21.9304 8.21055 21.6957 7.80703 21.2262 7C20.7567 6.19297 20.522 5.78945 20.2039 5.54727C19.7799 5.22436 19.244 5.08185 18.7141 5.15109C18.4722 5.18269 18.2269 5.28136 17.8887 5.4588C17.3915 5.71959 16.7969 5.73002 16.3099 5.45096C15.8229 5.17191 15.5347 4.65566 15.5136 4.09794C15.4993 3.71848 15.4628 3.45833 15.3694 3.23463C15.1649 2.74458 14.7726 2.35523 14.2788 2.15224Z" fill="currentColor"/>
                            <path d="M15.5227 12C15.5227 13.6569 14.1694 15 12.4999 15C10.8304 15 9.47705 13.6569 9.47705 12C9.47705 10.3431 10.8304 9 12.4999 9C14.1694 9 15.5227 10.3431 15.5227 12Z" fill="currentColor"/>
                        </svg>
                    </button>
                </div>
            `;
            document.body.prepend(topBar);
        } else {
            // Ensure bar-right contains profile selector and settings button
            var barRight = topBar.querySelector('.bar-right');
            if (barRight) {
                if (!document.getElementById('profile-dropdown-wrapper')) {
                    var profileWrapper = document.createElement('div');
                    profileWrapper.className = 'profile-dropdown-wrapper';
                    profileWrapper.id = 'profile-dropdown-wrapper';
                    profileWrapper.innerHTML = `
                        <button class="preset-pill profile-pill" id="btn-profile-toggle" title="Hardware Profiles stored in flash">
                            <span class="profile-name-text" id="active-profile-name">Safe State</span>
                            <span class="dropdown-arrow">▾</span>
                        </button>
                        <div class="profile-dropdown-menu" id="profile-dropdown-menu" style="display: none;">
                            <div class="profile-menu-header">HARDWARE PROFILES (FLASH)</div>
                            <div class="profile-menu-slots" id="profile-menu-slots"></div>
                        </div>
                    `;
                    var themeBtnEl = document.getElementById('btn-theme-toggle');
                    if (themeBtnEl) {
                        barRight.insertBefore(profileWrapper, themeBtnEl);
                    } else {
                        barRight.appendChild(profileWrapper);
                    }
                }

                if (!document.getElementById('btn-header-settings')) {
                    var settingsBtn = document.createElement('button');
                    settingsBtn.className = 'action-btn icon-btn header-btn';
                    settingsBtn.id = 'btn-header-settings';
                    settingsBtn.title = 'System Settings';
                    settingsBtn.innerHTML = `
                        <svg class="solar-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" fill-rule="evenodd" clip-rule="evenodd" d="M14.2788 2.15224C13.9085 2 13.439 2 12.5 2C11.561 2 11.0915 2 10.7212 2.15224C10.2274 2.35523 9.83509 2.74458 9.63056 3.23463C9.53719 3.45834 9.50065 3.7185 9.48635 4.09799C9.46534 4.65568 9.17716 5.17189 8.69017 5.45093C8.20318 5.72996 7.60864 5.71954 7.11149 5.45876C6.77318 5.2813 6.52789 5.18262 6.28599 5.15102C5.75609 5.08178 5.22018 5.22429 4.79616 5.5472C4.47814 5.78938 4.24339 6.1929 3.7739 6.99993C3.30441 7.80697 3.06967 8.21048 3.01735 8.60491C2.94758 9.1308 3.09118 9.66266 3.41655 10.0835C3.56506 10.2756 3.77377 10.437 4.0977 10.639C4.57391 10.936 4.88032 11.4419 4.88029 12C4.88026 12.5581 4.57386 13.0639 4.0977 13.3608C3.77372 13.5629 3.56497 13.7244 3.41645 13.9165C3.09108 14.3373 2.94749 14.8691 3.01725 15.395C3.06957 15.7894 3.30432 16.193 3.7738 17C4.24329 17.807 4.47804 18.2106 4.79606 18.4527C5.22008 18.7756 5.75599 18.9181 6.28589 18.8489C6.52778 18.8173 6.77305 18.7186 7.11133 18.5412C7.60852 18.2804 8.2031 18.27 8.69012 18.549C9.17714 18.8281 9.46533 19.3443 9.48635 19.9021C9.50065 20.2815 9.53719 20.5417 9.63056 20.7654C9.83509 21.2554 10.2274 21.6448 10.7212 21.8478C11.0915 22 11.561 22 12.5 22C13.439 22 13.9085 22 14.2788 21.8478C14.7726 21.6448 15.1649 21.2554 15.3694 20.7654C15.4628 20.5417 15.4994 20.2815 15.5137 19.902C15.5347 19.3443 15.8228 18.8281 16.3098 18.549C16.7968 18.2699 17.3914 18.2804 17.8886 18.5412C18.2269 18.7186 18.4721 18.8172 18.714 18.8488C19.2439 18.9181 19.7798 18.7756 20.2038 18.4527C20.5219 18.2105 20.7566 17.807 21.2261 16.9999C21.6956 16.1929 21.9303 15.7894 21.9827 15.395C22.0524 14.8691 21.9088 14.3372 21.5835 13.9164C21.4349 13.7243 21.2262 13.5628 20.9022 13.3608C20.4261 13.0639 20.1197 12.558 20.1197 11.9999C20.1197 11.4418 20.4261 10.9361 20.9022 10.6392C21.2263 10.4371 21.435 10.2757 21.5836 10.0835C21.9089 9.66273 22.0525 9.13087 21.9828 8.60497C21.9304 8.21055 21.6957 7.80703 21.2262 7C20.7567 6.19297 20.522 5.78945 20.2039 5.54727C19.7799 5.22436 19.244 5.08185 18.7141 5.15109C18.4722 5.18269 18.2269 5.28136 17.8887 5.4588C17.3915 5.71959 16.7969 5.73002 16.3099 5.45096C15.8229 5.17191 15.5347 4.65566 15.5136 4.09794C15.4993 3.71848 15.4628 3.45833 15.3694 3.23463C15.1649 2.74458 14.7726 2.35523 14.2788 2.15224Z" fill="currentColor"/>
                            <path d="M15.5227 12C15.5227 13.6569 14.1694 15 12.4999 15C10.8304 15 9.47705 13.6569 9.47705 12C9.47705 10.3431 10.8304 9 12.4999 9C14.1694 9 15.5227 10.3431 15.5227 12Z" fill="currentColor"/>
                        </svg>
                    `;
                    barRight.appendChild(settingsBtn);
                }
            }
        }

        // Brand logo
        var brand = document.getElementById('brand-logo');
        if (brand && !brand.getAttribute('href')) {
            brand.setAttribute('href', homeUrl);
        }

        // Theme toggle
        var themeBtn = document.getElementById('btn-theme-toggle');
        if (themeBtn && !themeBtn._wired) {
            themeBtn._wired = true;
            themeBtn.addEventListener('click', cycleThemeMode);
        }

        // Target Power Pill
        var powerPill = document.getElementById('btn-target-power-pill');
        if (powerPill && !powerPill._wired) {
            powerPill._wired = true;
            var isPowerOn = true;
            powerPill.addEventListener('click', async function() {
                isPowerOn = !isPowerOn;
                powerPill.classList.toggle('active-on', isPowerOn);
                try {
                    await fetch('/api/v1/pins/power', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ power_on: isPowerOn })
                    });
                } catch (_) {}
            });
        }

        // Emergency Safe Button
        var safeBtn = document.getElementById('btn-emergency-safe');
        if (safeBtn && !safeBtn._wired) {
            safeBtn._wired = true;
            safeBtn.addEventListener('click', async function() {
                if (!confirm('Activate Emergency Safe State? All GPIO pins will be quiesced to High-Z.')) return;
                try {
                    await fetch('/api/v1/pins/safe', { method: 'POST' });
                } catch (_) {}
                alert('Safe state activated: All pins quiesced.');
            });
        }

        // Hardware Profile Toggle Button
        var profileToggle = document.getElementById('btn-profile-toggle');
        if (profileToggle && !profileToggle._wired) {
            profileToggle._wired = true;
            profileToggle.addEventListener('click', function(e) {
                e.stopPropagation();
                toggleProfileMenu();
            });
        }
        fetchProfiles();

        // Settings Button
        var settingsBtnEl = document.getElementById('btn-header-settings');
        if (settingsBtnEl && !settingsBtnEl._wired) {
            settingsBtnEl._wired = true;
            settingsBtnEl.addEventListener('click', function() {
                if (typeof window.openSettingsModal === 'function') {
                    window.openSettingsModal();
                } else {
                    window.location.href = homeUrl + '#settings';
                }
            });
        }

        // Telemetry Stream SSE for power pill & telemetry popover
        try {
            var sse = new EventSource('/api/v1/system/telemetry/stream');
            sse.addEventListener('telemetry', function(e) {
                try {
                    var data = JSON.parse(e.data);
                    var vEl = document.getElementById('top-bar-voltage-val');
                    var iEl = document.getElementById('top-bar-current-val');
                    if (vEl && data.v_target_v !== undefined) vEl.textContent = data.v_target_v.toFixed(2) + 'V';
                    if (iEl && data.i_target_ma !== undefined) iEl.textContent = Math.round(data.i_target_ma) + 'mA';

                    var hudV = document.getElementById('hud-vtarget');
                    var hudI = document.getElementById('hud-itarget');
                    var hudP = document.getElementById('hud-ptarget');
                    var hudT = document.getElementById('hud-temp');
                    if (hudV && data.v_target_v !== undefined) hudV.textContent = data.v_target_v.toFixed(2) + ' V';
                    if (hudI && data.i_target_ma !== undefined) hudI.textContent = Math.round(data.i_target_ma) + ' mA';
                    if (hudP && data.v_target_v !== undefined && data.i_target_ma !== undefined) {
                        hudP.textContent = Math.round(data.v_target_v * data.i_target_ma) + ' mW';
                    }
                    if (hudT && data.core_temp_c !== undefined) hudT.textContent = Math.round(data.core_temp_c) + ' °C';

                    if (data.pin_ownership && Array.isArray(data.pin_ownership)) {
                        renderPinOwnershipMatrix(data.pin_ownership);
                    }
                } catch (_) {}
            });
        } catch (_) {}
    }

    function renderPinOwnershipMatrix(ownership) {
        var matrix = document.getElementById('hud-pin-matrix');
        if (!matrix) return;
        var html = '';
        for (var i = 0; i < 30; i++) {
            var owner = ownership[i] || 'FREE';
            var isClaimed = owner !== 'FREE' && owner !== 'safe' && owner !== 'Safe';
            var cls = isClaimed ? 'hud-pin-claimed' : 'hud-pin-free';
            html += '<div class="hud-pin-dot ' + cls + '" title="GP' + i + ': ' + owner + '">GP' + i + '</div>';
        }
        matrix.innerHTML = html;
    }

    // -------------------------------------------------------------------------
    // Device Info & Flash Storage Fetcher
    // -------------------------------------------------------------------------
    var deviceStorage = { total_bytes: 8388608, used_bytes: 1468006, free_bytes: 6920602 };
    var deviceSystem = { chip: "RP2350B", clock_mhz: 150, board_name: "BakuProbe / Pico 2", flash_bytes: 8388608, firmware_version: "v2.0" };

    function formatBytes(bytes) {
        if (!bytes || bytes === 0) return '0 B';
        var k = 1024;
        var sizes = ['B', 'KB', 'MB', 'GB'];
        var i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    async function fetchSystemSpecs() {
        try {
            var res = await fetch('/api/v1/system/info');
            if (res.ok) {
                var data = await res.json();
                deviceSystem = Object.assign(deviceSystem, data);
            }
        } catch (_) {}

        var elChip = document.getElementById('popover-info-chip');
        var elClock = document.getElementById('popover-info-clock');
        var elBoard = document.getElementById('popover-info-board');
        var elFw = document.getElementById('popover-info-fw');
        if (elChip) elChip.innerText = deviceSystem.chip || "RP2350B Cortex-M33";
        if (elClock) elClock.innerText = (deviceSystem.clock_mhz ? deviceSystem.clock_mhz + "MHz" : "150MHz") + " · 520KB";
        if (elBoard) elBoard.innerText = deviceSystem.board_name || "BakuProbe / Pico 2";
        if (elFw) elFw.innerText = deviceSystem.firmware_version || "v2.0";
    }

    async function fetchStorage() {
        try {
            var res = await fetch('/api/v1/apps/storage');
            if (res.ok) {
                deviceStorage = await res.json();
            }
        } catch (_) {}
        updateStorageDisplay();
    }

    function updateStorageDisplay() {
        var romBytes = 1638400; // ~1.56 MB System ROM
        var totalFlash = deviceSystem.flash_bytes || (8 * 1024 * 1024);
        var lfsTotal = deviceStorage.total_bytes || (totalFlash - romBytes);
        var lfsUsed = deviceStorage.used_bytes || 1468006;
        var lfsFree = (lfsTotal > lfsUsed) ? (lfsTotal - lfsUsed) : (deviceStorage.free_bytes || 6920602);

        var romPct = ((romBytes / totalFlash) * 100).toFixed(1);
        var appsPct = ((lfsUsed / totalFlash) * 100).toFixed(1);
        var freePct = (100 - parseFloat(romPct) - parseFloat(appsPct)).toFixed(1);

        var segRom = document.getElementById('seg-rom');
        var segApps = document.getElementById('seg-apps');
        var segFree = document.getElementById('seg-free');
        if (segRom) segRom.style.width = romPct + '%';
        if (segApps) segApps.style.width = appsPct + '%';
        if (segFree) segFree.style.width = freePct + '%';

        var lblRom = document.getElementById('label-rom');
        var lblApps = document.getElementById('label-apps');
        var lblFree = document.getElementById('label-free');
        if (lblRom) lblRom.innerText = formatBytes(romBytes);
        if (lblApps) lblApps.innerText = formatBytes(lfsUsed) + ' (' + appsPct + '%)';
        if (lblFree) lblFree.innerText = formatBytes(lfsFree) + ' (' + freePct + '%)';

        var totFlashEl = document.getElementById('popover-total-flash');
        if (totFlashEl) totFlashEl.innerText = formatBytes(totalFlash);
    }

    function toggleStoragePopover(e, forceState) {
        if (e && e.stopPropagation) e.stopPropagation();
        var popover = document.getElementById('storage-glass-popover');
        var hud = document.getElementById('telemetry-hud');
        var btn = document.getElementById('dock-btn-storage');
        var telemBtn = document.getElementById('dock-btn-telemetry');
        if (!popover) return;
        if (hud) {
            hud.style.display = 'none';
            if (telemBtn) telemBtn.classList.remove('active');
        }
        var isVisible = popover.style.display !== 'none';
        var show = (typeof forceState === 'boolean') ? forceState : !isVisible;
        popover.style.display = show ? 'flex' : 'none';
        if (btn) {
            if (show) btn.classList.add('active');
            else btn.classList.remove('active');
        }
        if (show) {
            fetchSystemSpecs();
            fetchStorage();
        }
    }

    function toggleTelemetryHUD(e, forceState) {
        if (e && e.stopPropagation) e.stopPropagation();
        var hud = document.getElementById('telemetry-hud');
        var popover = document.getElementById('storage-glass-popover');
        var btn = document.getElementById('dock-btn-telemetry');
        var storageBtn = document.getElementById('dock-btn-storage');
        if (!hud) return;
        if (popover) {
            popover.style.display = 'none';
            if (storageBtn) storageBtn.classList.remove('active');
        }
        var isVisible = hud.style.display !== 'none';
        var show = (typeof forceState === 'boolean') ? forceState : !isVisible;
        hud.style.display = show ? 'flex' : 'none';
        if (btn) {
            if (show) btn.classList.add('active');
            else btn.classList.remove('active');
        }
    }

    // Close popovers and menus when clicking anywhere outside
    document.addEventListener('click', function(e) {
        var popover = document.getElementById('storage-glass-popover');
        var storageBtn = document.getElementById('dock-btn-storage');
        if (popover && popover.style.display !== 'none') {
            if (!storageBtn || (!storageBtn.contains(e.target) && !popover.contains(e.target))) {
                popover.style.display = 'none';
                if (storageBtn) storageBtn.classList.remove('active');
            }
        }

        var hud = document.getElementById('telemetry-hud');
        var telemetryBtn = document.getElementById('dock-btn-telemetry');
        if (hud && hud.style.display !== 'none') {
            if (!telemetryBtn || (!telemetryBtn.contains(e.target) && !hud.contains(e.target))) {
                hud.style.display = 'none';
                if (telemetryBtn) telemetryBtn.classList.remove('active');
            }
        }

        var wrapper = document.getElementById('profile-dropdown-wrapper');
        if (wrapper && !wrapper.contains(e.target)) {
            toggleProfileMenu(false);
        }
    });

    // -------------------------------------------------------------------------
    // Floating macOS Glass Dock Mounting & Synchronization
    // -------------------------------------------------------------------------
    function getPinnedApps() {
        try {
            var stored = localStorage.getItem('agentla_pinned_dock_apps');
            if (stored) {
                var list = JSON.parse(stored);
                if (Array.isArray(list)) {
                    return list;
                }
            }
        } catch (_) {}
        return ['scope', 'adapters', 'target', 'pins', 'console', 'terminal'];
    }

    function renderDockPinnedApps(container) {
        if (!container) container = document.getElementById('dock-pinned-apps');
        if (!container) return;

        var pinned = getPinnedApps();
        ['scope', 'adapters', 'target', 'pins', 'console', 'terminal'].forEach(function(sysId) {
            var el = document.getElementById('dock-app-' + sysId);
            if (el) {
                el.style.display = pinned.indexOf(sysId) !== -1 ? 'inline-flex' : 'none';
            }
        });
        var html = '';

        pinned.forEach(function(appId) {
            if (['scope', 'adapters', 'target', 'pins', 'console', 'terminal'].indexOf(appId) !== -1) return;

            var isCurrent = window.location.pathname.includes('/' + appId + '/');
            var activeClass = isCurrent ? ' active' : '';

            if (appId === 'servo-master') {
                var servoHref = isDemo ? (window.location.pathname.includes('/apps/') ? '../servo-master/index.html' : 'apps/servo-master/index.html') : '/app/servo-master/';
                html += `
                    <a href="${servoHref}" class="dock-icon-btn${activeClass}" data-dock-tooltip="Servo Master">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2Z" fill="currentColor"/>
                            <path d="M12 8C9.79086 8 8 9.79086 8 12C8 14.2091 9.79086 16 12 16C14.2091 16 16 14.2091 16 12C16 9.79086 14.2091 8 12 8Z" fill="currentColor"/>
                            <path d="M12 4V7M12 17V20M4 12H7M17 12H20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        </svg>
                    </a>`;
            } else {
                var appHref = isDemo ? (window.location.pathname.includes('/apps/') ? `../${encodeURIComponent(appId)}/index.html` : `apps/${encodeURIComponent(appId)}/index.html`) : `/app/${encodeURIComponent(appId)}/`;
                html += `
                    <a href="${appHref}" class="dock-icon-btn${activeClass}" data-dock-tooltip="${appId}">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2Z" fill="currentColor"/>
                            <path d="M12 8v8M8 12h8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        </svg>
                    </a>`;
            }
        });

        container.innerHTML = html;
    }

    function mountDock() {
        var dock = document.getElementById('floating-glass-dock');
        if (!dock) {
            var pinned = getPinnedApps();
            function sysStyle(id) {
                return pinned.indexOf(id) === -1 ? ' style="display: none;"' : '';
            }

            dock = document.createElement('div');
            dock.id = 'floating-glass-dock';
            dock.className = 'floating-glass-dock';
            dock.innerHTML = `
                <!-- Launcher -->
                <a href="${homeUrl}" class="dock-icon-btn" data-dock-tooltip="Apps Launcher">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <g fill="currentColor" opacity="0.5">
                            <path d="M17.5 13C19.6213 13 20.6818 13.0002 21.3408 13.6592C21.9998 14.3182 22 15.3787 22 17.5C22 19.6213 21.9998 20.6818 21.3408 21.3408C20.6818 21.9998 19.6213 22 17.5 22C15.3787 22 14.3182 21.9998 13.6592 21.3408C13.0002 20.6818 13 19.6213 13 17.5C13 15.3787 13.0002 14.3182 13.6592 13.6592C14.3182 13.0002 15.3787 13 17.5 13Z"/>
                            <path d="M6.5 2C8.62132 2 9.68181 2.00017 10.3408 2.65918C10.9998 3.31819 11 4.37868 11 6.5C11 8.62132 10.9998 9.68181 10.3408 10.3408C9.68181 10.9998 8.62132 11 6.5 11C4.37868 11 3.31819 10.9998 2.65918 10.3408C2.00017 9.68181 2 8.62132 2 6.5C2 4.37868 2.00017 3.31819 2.65918 2.65918C3.31819 2.00017 4.37868 2 6.5 2Z"/>
                        </g>
                        <path fill="currentColor" d="M6.5 13C8.62132 13 9.68181 13.0002 10.3408 13.6592C10.9998 14.3182 11 15.3787 11 17.5C11 19.6213 10.9998 20.6818 10.3408 21.3408C20.6818 21.9998 8.62132 22 6.5 22C4.37868 22 3.31819 21.9998 2.65918 21.3408C2.00017 20.6818 2 19.6213 2 17.5C2 15.3787 2.00017 14.3182 2.65918 13.6592C3.31819 13.0002 4.37868 13 6.5 13Z"/>
                        <path fill="currentColor" d="M17.5 2C19.6213 2 20.6818 2.00017 21.3408 2.65918C21.9998 3.31819 22 4.37868 22 6.5C22 8.62132 21.9998 9.68181 21.3408 10.3408C20.6818 10.9998 19.6213 11 17.5 11C15.3787 11 14.3182 10.9998 13.6592 10.3408C13.0002 9.68181 13 8.62132 13 6.5C13 4.37868 13.0002 3.31819 13.6592 2.65918C14.3182 2.00017 15.3787 2 17.5 2Z"/>
                    </svg>
                </a>

                <!-- Scope Waveform Quick Launch -->
                <a href="${isDemo ? labUrl + '#scope' : '/scope/'}" class="dock-icon-btn" id="dock-app-scope"${sysStyle('scope')} data-dock-tooltip="Scope (100 MS/s)">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M17.5762 10.4802C17.8414 10.1619 17.7984 9.68903 17.4802 9.42385C17.1619 9.15868 16.689 9.20167 16.4239 9.51988L14.627 11.6761C14.2562 12.1211 14.0285 12.3915 13.8409 12.5609C13.7539 12.6395 13.7023 12.6708 13.6776 12.6827C13.6725 12.6852 13.6689 12.6866 13.6667 12.6875C13.6667 12.6875 13.6624 12.6858 13.659 12.6843L13.6558 12.6827C13.6311 12.6708 13.5795 12.6395 13.4925 12.5609C13.3049 12.3915 13.0772 12.1211 12.7064 11.6761L12.414 11.3253C12.0855 10.931 11.7894 10.5756 11.5128 10.3259C11.2119 10.0541 10.8328 9.81207 10.3334 9.81207C9.83386 9.81207 9.4548 10.0541 9.15386 10.3259C8.87727 10.5756 8.58115 10.931 8.25269 11.3253L6.42385 13.5199C6.15868 13.8381 6.20167 14.311 6.51988 14.4239L6.75428 14.2285C7.41285 13.6797 7.84348 13.3185 8.11968 13.0126C8.38196 12.7222 8.40694 12.586 8.40694 12.5C8.40694 12.414 8.38196 12.2779 8.11968 11.9874C7.84348 11.6815 7.41285 11.3203 6.75429 10.7715L6.51988 10.5762C6.20167 10.311 6.15868 9.83809 6.42385 9.51988Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Adapters -->
                <a href="${isDemo ? labUrl + '#adapters' : '/adapter/'}" class="dock-icon-btn" id="dock-app-adapters"${sysStyle('adapters')} data-dock-tooltip="Protocol Adapters">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M13.5303 6.46967L12.5303 5.46967C12.2374 5.17678 11.7626 5.17678 11.4697 5.46967L10.4697 6.46967C10.1768 6.76256 10.1768 7.23744 10.4697 7.53033C10.6807 7.74135 10.9862 7.80034 11.25 7.70729V12.7415C11.1787 12.7115 11.1055 12.6847 11.0306 12.6613L9.62716 12.2227C9.10531 12.0596 8.75 11.5763 8.75 11.0296V10.5816C9.34124 10.3007 9.75 9.6981 9.75 9C9.75 8.0335 8.9665 7.25 8 7.25C7.0335 7.25 6.25 8.0335 6.25 9C6.25 9.6981 6.65876 10.3007 7.25 10.5816V11.0296C7.25 12.2324 8.03169 13.2957 9.17974 13.6544L10.5832 14.093C10.9799 14.217 11.25 14.5844 11.25 15V15.4184C10.6588 15.6993 10.25 16.3019 10.25 17C10.25 17.9665 11.0335 18.75 12 18.75C12.9665 18.75 13.75 17.9665 13.75 17C13.75 16.3605 13.4069 15.8011 12.8948 15.4958C13.0124 15.3081 13.1948 15.1624 13.4168 15.093L14.8203 14.6544C15.9683 14.2957 16.75 13.2324 16.75 12.0296V11.7084C16.9502 11.6695 17.1831 11.5847 17.3839 11.3839C17.6197 11.1481 17.6955 10.8679 17.725 10.6486C17.7502 10.4614 17.7501 10.2396 17.75 10.0345V9.96555C17.7501 9.76045 17.7502 9.53864 17.725 9.35143C17.6955 9.1321 17.6197 8.85192 17.3839 8.61612C17.1481 8.38032 16.8679 8.30448 16.6486 8.27499C16.4614 8.24982 16.2396 8.24991 16.0345 8.24999H15.9656C15.7604 8.24991 15.5386 8.24982 15.3514 8.27499C15.1321 8.30448 14.8519 8.38032 14.6161 8.61612C14.3803 8.85192 14.3045 9.1321 14.275 9.35143C14.2498 9.53864 14.2499 9.76042 14.25 9.96552V9.96553V10.0344V10.0344C14.2499 10.2396 14.2498 10.4614 14.275 10.6486C14.3045 10.8679 14.3803 11.1481 14.6161 11.3839C14.8169 11.5847 15.0498 11.6695 15.25 11.7084V12.0296C15.25 12.5763 14.8947 13.0596 14.3728 13.2227L12.9694 13.6613C12.8945 13.6847 12.8213 13.7115 12.75 13.7415V7.70729C13.0138 7.80034 13.3193 7.74135 13.5303 7.53033C13.8232 7.23744 13.8232 6.76256 13.5303 6.46967Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- SWD Debugger -->
                <a href="${isDemo ? labUrl + '#target' : '/debug/'}" class="dock-icon-btn" id="dock-app-target"${sysStyle('target')} data-dock-tooltip="SWD Debugger">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M11.1566 20.3133C16.2137 20.3133 20.3133 16.2137 20.3133 11.1566C20.3133 6.09956 16.2137 2 11.1566 2C6.09956 2 2 6.09956 2 11.1566C2 16.2137 6.09956 20.3133 11.1566 20.3133Z" fill="currentColor"/>
                        <path d="M21.7881 20.7656C22.0704 21.0479 22.0704 21.5058 21.7881 21.7881C21.5058 22.0704 21.0479 22.0704 20.7656 21.7881L17.0996 18.1221C17.4666 17.8086 17.8086 17.4666 18.1221 17.0996L21.7881 20.7656Z" fill="currentColor"/>
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M11 6.5C12.1408 6.5 13.1567 7.03207 13.8105 7.86035L14.4902 7.52246C14.843 7.34709 15.2717 7.48911 15.4482 7.83984C15.6246 8.19079 15.4817 8.61744 15.1289 8.79297L14.4502 9.13086C14.5293 9.42476 14.5713 9.734 14.5713 10.0527V10.2891H15.2861C15.6804 10.2893 16 10.6077 16 11C16 11.3923 15.6804 11.7107 15.2861 11.7109H14.5713V11.9473C14.5713 12.266 14.5293 12.5752 14.4502 12.8691L15.1289 13.207C15.4817 13.3826 15.6246 13.8092 15.4482 14.1602C15.2717 14.5109 14.843 14.6529 14.4902 14.4775L13.8105 14.1396C13.1567 14.9679 12.1408 15.5 11 15.5C9.85919 15.5 8.8433 14.9679 8.18945 14.1396L7.50977 14.4775C7.15704 14.6529 6.72826 14.5109 6.55176 14.1602C6.37536 13.8092 6.51835 13.3826 6.87109 13.207L7.5498 12.8691C7.47066 12.5752 7.42871 12.266 7.42871 11.9473V11.7109H6.71387C6.31957 11.7107 6 11.3923 6 11C6 10.6077 6.31957 10.2893 6.71387 10.2891H7.42871V10.0527C7.42871 9.734 7.47066 9.42476 7.5498 9.13086L6.87109 8.79297C6.51835 8.61744 6.37536 8.19079 6.55176 7.83984C6.72826 7.48911 7.15704 7.34709 7.50977 7.52246L8.18945 7.86035C8.8433 7.03207 9.85919 6.5 11 6.5ZM8.85742 10.7627V11.9473C8.85744 12.8753 9.45388 13.6644 10.2861 13.957V10.7627H8.85742ZM11.7148 10.7627V13.957C12.5469 13.6643 13.1435 12.8752 13.1436 11.9473V10.7627H11.7148ZM11 7.9209C10.0672 7.92102 9.27368 8.51407 8.97949 9.3418H13.0215C12.7273 8.51392 11.933 7.9209 11 7.9209Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Pins -->
                <a href="${isDemo ? labUrl + '#pins' : '/pin/'}" class="dock-icon-btn" id="dock-app-pins"${sysStyle('pins')} data-dock-tooltip="Pin Routing Crossbar">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M12 3C12.3853 3 12.6977 3.31236 12.6977 3.69767V6.48884C13.2084 6.48991 13.6717 6.49342 14.0932 6.50495L14.093 6.48837V3.69767C14.093 3.31236 14.4054 3 14.7907 3C15.176 3 15.4884 3.31236 15.4884 3.69767V6.48837C15.4884 6.52487 15.4856 6.56072 15.4802 6.5957C16.4162 6.71067 17.0648 6.94879 17.558 7.44198C18.0512 7.93517 18.2893 8.58381 18.4043 9.51984C18.4393 9.51443 18.4751 9.51163 18.5116 9.51163H21.3023C21.6876 9.51163 22 9.82399 22 10.2093C22 10.5946 21.6876 10.907 21.3023 10.907H18.5116L18.495 10.9068C18.5066 11.3283 18.5106 11.7916 18.5116 12.3023H21.3023C21.6876 12.3023 22 12.6147 22 13C22 13.3853 21.6876 13.6977 21.3023 13.6977L18.5112 13.6977C18.5101 14.2084 18.5066 14.6717 18.495 15.0932L18.5116 15.093H21.3023C21.6876 15.093 22 15.4054 22 15.7907C22 16.176 21.6876 16.4884 21.3023 16.4884H18.5116C18.4751 16.4884 18.4393 16.4856 18.4043 16.4802C18.2893 17.4162 18.0512 18.0648 17.558 18.558C17.0648 19.0512 16.4162 19.2893 15.4802 19.4043C15.4856 19.4393 15.4884 19.4751 15.4884 19.5116V22.3023C15.4884 22.6876 15.176 23 14.7907 23C14.4054 23 14.093 22.6876 14.093 22.3023V19.5116L14.0932 19.495C13.6717 19.5066 13.2084 19.5106 12.6977 19.5116V22.3023C12.6977 22.6876 12.3853 23 12 23C11.6147 23 11.3023 22.6876 11.3023 22.3023L11.3023 19.5112C10.7916 19.5101 10.3283 19.5066 9.90678 19.495L9.90698 19.5116V22.3023C9.90698 22.6876 9.59462 23 9.2093 23C8.82399 23 8.51163 22.6876 8.51163 22.3023V19.5116C8.51163 19.4751 8.51443 19.4393 8.51984 19.4043C7.58381 19.2893 6.93517 19.0512 6.44198 18.558C5.94879 18.0648 5.71067 17.4162 5.5957 16.4802C5.56071 16.4856 5.52487 16.4884 5.48837 16.4884H2.69767C2.31236 16.4884 2 16.176 2 15.7907C2 15.4054 2.31236 15.093 2.69767 15.093H5.48837L5.50495 15.0932C5.49342 14.6717 5.48944 14.2084 5.48837 13.6977H2.69767C2.31236 13.6977 2 13.3853 2 13C2 12.6147 2.31236 12.3023 2.69767 12.3023L5.48884 12.3023C5.48991 11.7916 5.49342 11.3283 5.50495 10.9068L5.48837 10.907H2.69767C2.31236 10.907 2 10.2093 2 10.2093C2 9.82399 2.31236 9.51163 2.69767 9.51163H5.48837C5.52487 9.51163 5.56071 9.51443 5.5957 9.51984C5.71067 8.58381 5.94879 7.93517 6.44198 7.44198C6.93517 6.94879 7.58381 6.71067 8.51984 6.5957C8.51443 6.56072 8.51163 6.52487 8.51163 6.48837V3.69767C8.51163 3.31236 8.82399 3 9.2093 3C9.59462 3 9.90698 3.31236 9.90698 3.69767V6.48837L9.90678 6.50495C10.3283 6.49342 10.7916 6.48944 11.3023 6.48837V3.69767C11.3023 3.31236 11.6147 3 12 3ZM11.0238 8.5814C10.4054 8.58136 9.87247 8.58133 9.44573 8.63871C8.98839 8.70019 8.55001 8.83885 8.19443 9.19443C7.83885 9.55001 7.70019 9.98839 7.63871 10.4457C7.58133 10.8725 7.58136 11.4054 7.5814 12.0238V13.9762C7.58136 14.5946 7.58133 15.1275 7.63871 15.5543C7.70019 16.0116 7.83885 16.45 8.19443 16.8056C8.55001 17.1612 8.98839 17.2998 9.44573 17.3613C9.87247 17.4187 10.4054 17.4186 11.0238 17.4186H12.9762C13.5946 17.4186 14.1275 17.4187 14.5543 17.3613C15.0116 17.2998 15.45 17.1612 15.8056 16.8056C16.1612 16.45 16.2998 16.0116 16.3613 15.5543C16.4187 15.1275 16.4186 14.5946 16.4186 13.9762V12.0238C16.4186 11.4054 16.4187 10.8725 16.3613 10.4457C16.2998 9.98839 16.1612 9.55001 15.8056 9.19443C15.45 8.83885 15.0116 8.70019 14.5543 8.63871C14.1275 8.58133 13.5947 8.58136 12.9762 8.5814H11.0238Z" fill="currentColor"/>
                        <path opacity="0.5" d="M9.18091 10.1809C9.23402 10.1278 9.32886 10.0621 9.63147 10.0214C9.95415 9.97804 10.3921 9.97656 11.0696 9.97656H12.9301C13.6075 9.97656 14.0455 9.97804 14.3682 10.0214C14.6708 10.0621 14.7656 10.1278 14.8187 10.1809C14.8718 10.234 14.9375 10.3289 14.9782 10.6315C15.0216 10.9542 15.0231 11.3921 15.0231 12.0696V13.9301C15.0231 14.6075 15.0216 15.0455 14.9782 15.3682C14.9375 15.6708 14.8718 15.7656 14.8187 15.8187C14.7656 15.8718 14.6708 15.9375 14.3682 15.9782C14.0455 16.0216 13.6075 16.0231 12.9301 16.0231H11.0696C10.3921 16.0231 9.95415 16.0216 9.63147 15.9782C9.32886 15.9375 9.23402 15.8718 9.18091 15.8187C9.1278 15.7656 9.06211 15.6708 9.02143 15.3682C8.97804 15.0455 8.97656 14.6075 8.97656 13.9301V12.0696C8.97656 11.3921 8.97804 10.9542 9.02143 10.6315C9.06211 10.3289 9.1278 10.234 9.18091 10.1809Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Terminal -->
                <a href="${isDemo ? 'terminal.html' : '/terminal/'}" class="dock-icon-btn" id="dock-app-terminal"${sysStyle('terminal')} data-dock-tooltip="Serial Terminal">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M6.42385 9.51988C6.68903 9.20167 7.16195 9.15868 7.48016 9.42385L7.75658 9.6542L7.75661 9.65423C8.36154 10.1583 8.87654 10.5874 9.23295 10.9821C9.61151 11.4013 9.90694 11.8834 9.90694 12.5C9.90694 13.1166 9.61151 13.5987 9.23295 14.0179C8.87653 14.4126 8.36153 14.8418 7.75658 15.3458L7.48016 15.5762C7.16195 15.8414 6.68903 15.7984 6.42385 15.4802C6.15868 15.1619 6.20167 14.689 6.51988 14.4239L6.75428 14.2285C7.41285 13.6797 7.84348 13.3185 8.11968 13.0126C8.38196 12.7222 8.40694 12.586 8.40694 12.5C8.40694 12.414 8.38196 12.2779 8.11968 11.9874C7.84348 11.6815 7.41285 11.3203 6.75429 10.7715L6.51988 10.5762C6.20167 10.311 6.15868 9.83809 6.42385 9.51988Z" fill="currentColor"/>
                        <path d="M17.75 15C17.75 15.4142 17.4142 15.75 17 15.75H12C11.5858 15.75 11.25 15.4142 11.25 15C11.25 14.5858 11.5858 14.25 12 14.25H17C17.4142 14.25 17.75 14.5858 17.75 15Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Console -->
                <a href="${isDemo ? labUrl + '#console' : '/agent/'}" class="dock-icon-btn" id="dock-app-console"${sysStyle('console')} data-dock-tooltip="MCP Console">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2.73223 12.9391C3.06096 12.7418 3.47962 12.6331 4.05448 12.5732C4.14064 11.3357 4.36306 10.5351 4.93726 9.9519C5.87452 9 7.38301 9 10.4 9H13.6C16.617 9 18.1255 9 19.0627 9.9519C19.6369 10.5351 19.8596 11.3357 19.9458 12.5732C20.5207 12.6331 20.9393 12.7418 21.268 12.9391C22.0003 13.3784 22.0003 14.0855 22.0003 15.4997C22.0003 16.9139 22.0003 17.621 21.268 18.0604C20.9393 18.2576 20.5207 18.3663 19.9458 18.4262C19.8596 19.6637 19.6369 20.4649 19.0627 21.0481C18.1255 22 16.617 22 13.6 22H13.6H10.4C7.38301 22 5.87452 22 4.93726 21.0481C4.36306 20.4649 4.14064 19.6637 4.05448 18.4262C3.47962 18.3663 3.06096 18.2576 2.73223 18.0604C2 17.621 2 16.9139 2 15.4997C2 14.0855 2 13.3784 2.73223 12.9391Z" fill="currentColor"/>
                        <path d="M14.5537 17.3975C14.8865 17.151 15.3559 17.221 15.6025 17.5537C15.849 17.8865 15.779 18.3559 15.4463 18.6025C14.4739 19.3232 13.2849 19.75 12 19.75C10.7151 19.75 9.52608 19.3232 8.55371 18.6025C8.22102 18.3559 8.15099 17.8865 8.39746 17.5537C8.64407 17.221 9.11354 17.151 9.44629 17.3975C10.1746 17.9373 11.0541 18.25 12 18.25C12.9459 18.25 13.8254 17.9373 14.5537 17.3975Z" fill="currentColor"/>
                        <path d="M9 12C9.42949 12 9.68372 12.3352 9.80078 12.5693C9.93278 12.8334 10 13.1631 10 13.5C10 13.8369 9.93278 14.1666 9.80078 14.4307C9.68372 14.6648 9.42949 15 9 15C8.57051 15 8.31628 14.6648 8.19922 14.4307C8.06722 14.1666 8 13.8369 8 13.5C8 13.1631 8.06722 12.8334 8.19922 12.5693C8.31628 12.3352 8.57051 12 9 12Z" fill="currentColor"/>
                        <path d="M15 12C15.4295 12 15.6837 12.3352 15.8008 12.5693C15.9328 12.8334 16 13.1631 16 13.5C16 13.8369 15.9328 14.1666 15.8008 14.4307C15.6837 14.6648 15.4295 15 15 15C14.5705 15 14.3163 14.6648 14.1992 14.4307C14.0672 14.1666 14 13.8369 14 13.5C14 13.1631 14.0672 12.8334 14.1992 12.5693C14.3163 12.3352 14.5705 12 15 12Z" fill="currentColor"/>
                        <path d="M12 2C12.8284 2 13.5 2.67157 13.5 3.5C13.5 4.05495 13.198 4.5384 12.75 4.79785V9H11.25V4.79785C10.802 4.5384 10.5 4.05495 10.5 3.5C10.5 2.67157 11.1716 2 12 2Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Terminal -->
                <a href="${isDemo ? 'terminal.html' : '/terminal/'}" class="dock-icon-btn" id="dock-app-terminal" data-dock-tooltip="Serial Terminal">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M6.42385 9.51988C6.68903 9.20167 7.16195 9.15868 7.48016 9.42385L7.75658 9.6542L7.75661 9.65423C8.36154 10.1583 8.87654 10.5874 9.23295 10.9821C9.61151 11.4013 9.90694 11.8834 9.90694 12.5C9.90694 13.1166 9.61151 13.5987 9.23295 14.0179C8.87653 14.4126 8.36153 14.8418 7.75658 15.3458L7.48016 15.5762C7.16195 15.8414 6.68903 15.7984 6.42385 15.4802C6.15868 15.1619 6.20167 14.689 6.51988 14.4239L6.75428 14.2285C7.41285 13.6797 7.84348 13.3185 8.11968 13.0126C8.38196 12.7222 8.40694 12.586 8.40694 12.5C8.40694 12.414 8.38196 12.2779 8.11968 11.9874C7.84348 11.6815 7.41285 11.3203 6.75429 10.7715L6.51988 10.5762C6.20167 10.311 6.15868 9.83809 6.42385 9.51988Z" fill="currentColor"/>
                        <path d="M17.75 15C17.75 15.4142 17.4142 15.75 17 15.75H12C11.5858 15.75 11.25 15.4142 11.25 15C11.25 14.5858 11.5858 14.25 12 14.25H17C17.4142 14.25 17.75 14.5858 17.75 15Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Console -->
                <a href="${isDemo ? labUrl + '#console' : '/agent/'}" class="dock-icon-btn" id="dock-app-console" data-dock-tooltip="MCP Console">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2.73223 12.9391C3.06096 12.7418 3.47962 12.6331 4.05448 12.5732C4.14064 11.3357 4.36306 10.5351 4.93726 9.9519C5.87452 9 7.38301 9 10.4 9H13.6C16.617 9 18.1255 9 19.0627 9.9519C19.6369 10.5351 19.8596 11.3357 19.9458 12.5732C20.5207 12.6331 20.9393 12.7418 21.268 12.9391C22.0003 13.3784 22.0003 14.0855 22.0003 15.4997C22.0003 16.9139 22.0003 17.621 21.268 18.0604C20.9393 18.2576 20.5207 18.3663 19.9458 18.4262C19.8596 19.6637 19.6369 20.4649 19.0627 21.0481C18.1255 22 16.617 22 13.6 22H13.6H10.4C7.38301 22 5.87452 22 4.93726 21.0481C4.36306 20.4649 4.14064 19.6637 4.05448 18.4262C3.47962 18.3663 3.06096 18.2576 2.73223 18.0604C2 17.621 2 16.9139 2 15.4997C2 14.0855 2 13.3784 2.73223 12.9391Z" fill="currentColor"/>
                        <path d="M14.5537 17.3975C14.8865 17.151 15.3559 17.221 15.6025 17.5537C15.849 17.8865 15.779 18.3559 15.4463 18.6025C14.4739 19.3232 13.2849 19.75 12 19.75C10.7151 19.75 9.52608 19.3232 8.55371 18.6025C8.22102 18.3559 8.15099 17.8865 8.39746 17.5537C8.64407 17.221 9.11354 17.151 9.44629 17.3975C10.1746 17.9373 11.0541 18.25 12 18.25C12.9459 18.25 13.8254 17.9373 14.5537 17.3975Z" fill="currentColor"/>
                        <path d="M9 12C9.42949 12 9.68372 12.3352 9.80078 12.5693C9.93278 12.8334 10 13.1631 10 13.5C10 13.8369 9.93278 14.1666 9.80078 14.4307C9.68372 14.6648 9.42949 15 9 15C8.57051 15 8.31628 14.6648 8.19922 14.4307C8.06722 14.1666 8 13.8369 8 13.5C8 13.1631 8.06722 12.8334 8.19922 12.5693C8.31628 12.3352 8.57051 12 9 12Z" fill="currentColor"/>
                        <path d="M15 12C15.4295 12 15.6837 12.3352 15.8008 12.5693C15.9328 12.8334 16 13.1631 16 13.5C16 13.8369 15.9328 14.1666 15.8008 14.4307C15.6837 14.6648 15.4295 15 15 15C14.5705 15 14.3163 14.6648 14.1992 14.4307C14.0672 14.1666 14 13.8369 14 13.5C14 13.1631 14.0672 12.8334 14.1992 12.5693C14.3163 12.3352 14.5705 12 15 12Z" fill="currentColor"/>
                        <path d="M12 2C12.8284 2 13.5 2.67157 13.5 3.5C13.5 4.05495 13.198 4.5384 12.75 4.79785V9H11.25V4.79785C10.802 4.5384 10.5 4.05495 10.5 3.5C10.5 2.67157 11.1716 2 12 2Z" fill="currentColor"/>
                    </svg>
                </a>

                
                <!-- Pinned LittleFS Applications in Dock -->
                <div id="dock-pinned-apps" style="display: contents;"></div>

                <div class="dock-separator"></div>

                <!-- Dedicated Hardware Telemetry HUD -->
                <div class="dock-item-wrapper">
                    <button class="dock-icon-btn" id="dock-btn-telemetry" data-dock-tooltip="Hardware Telemetry">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" fill="currentColor" d="M3.46447 3.46447C2 4.92893 2 7.28595 2 12C2 12.8185 2 13.5659 2.00767 14.25H5L5.10773 14.2499C5.63376 14.2491 6.10075 14.2483 6.52918 14.4319C6.9576 14.6155 7.27913 14.9542 7.64131 15.3357L7.71552 15.4138L8.85712 16.6125C8.9656 16.7264 9.14832 16.7141 9.24061 16.5867L12.8944 11.5435C13.0432 11.3379 13.1985 11.1233 13.3518 10.9653C13.5272 10.7844 13.7992 10.5708 14.1937 10.5527C14.5882 10.5346 14.8787 10.7223 15.0699 10.8863C15.2371 11.0297 15.4114 11.229 15.5785 11.4201L17.3823 13.4816C17.868 14.0368 17.9718 14.1316 18.085 14.183C18.1982 14.2343 18.3379 14.25 19.0756 14.25H21.9923C22 13.5659 22 12.8185 22 12C22 7.28595 22 4.92893 20.5355 3.46447C19.0711 2 16.714 2 12 2C7.28595 2 4.92893 2 3.46447 3.46447Z"/>
                            <path fill="currentColor" d="M2.00781 14.25C2.0443 17.5057 2.25442 19.3254 3.46461 20.5356C4.92908 22 7.2861 22 12.0001 22C16.7142 22 19.0712 22 20.5357 20.5356C21.7459 19.3254 21.956 17.5057 21.9925 14.25H19.0757C18.3381 14.25 18.1984 14.2344 18.0852 14.183C17.972 14.1316 17.8682 14.0368 17.3824 13.4817L15.5786 11.4202C15.4115 11.229 15.2372 11.0297 15.07 10.8863C14.8788 10.7223 14.5884 10.5346 14.1939 10.5527C13.7994 10.5709 13.5274 10.7844 13.352 10.9653C13.1987 11.1234 13.0434 11.3379 12.8945 11.5435L9.24075 16.5868C9.14847 16.7141 8.96574 16.7264 8.85727 16.6125L7.71566 15.4138L7.64145 15.3357C7.27927 14.9542 6.95775 14.6156 6.52932 14.4319C6.1009 14.2483 5.63391 14.2491 5.10787 14.2499L5.00015 14.25H2.00781Z"/>
                        </svg>
                    </button>
                    <!-- Floating Glass Popover -->
                    <div class="storage-glass-popover telemetry-glass-popover" id="telemetry-hud" style="display: none;" onclick="event.stopPropagation()">
                        <div class="popover-header">
                            <div style="display: flex; align-items: center; gap: 6px;">
                                <svg class="solar-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                                    <path opacity="0.5" fill="currentColor" d="M3.46447 3.46447C2 4.92893 2 7.28595 2 12C2 12.8185 2 13.5659 2.00767 14.25H5L5.10773 14.2499C5.63376 14.2491 6.10075 14.2483 6.52918 14.4319C6.9576 14.6155 7.27913 14.9542 7.64131 15.3357L7.71552 15.4138L8.85712 16.6125C8.9656 16.7264 9.14832 16.7141 9.24061 16.5867L12.8944 11.5435C13.0432 11.3379 13.1985 11.1233 13.3518 10.9653C13.5272 10.7844 13.7992 10.5708 14.1937 10.5527C14.5882 10.5346 14.8787 10.7223 15.0699 10.8863C15.2371 11.0297 15.4114 11.229 15.5785 11.4201L17.3823 13.4816C17.868 14.0368 17.9718 14.1316 18.085 14.183C18.1982 14.2343 18.3379 14.25 19.0756 14.25H21.9923C22 13.5659 22 12.8185 22 12C22 7.28595 22 4.92893 20.5355 3.46447C19.0711 2 16.714 2 12 2C7.28595 2 4.92893 2 3.46447 3.46447Z"/>
                                    <path fill="currentColor" d="M2.00781 14.25C2.0443 17.5057 2.25442 19.3254 3.46461 20.5356C4.92908 22 7.2861 22 12.0001 22C16.7142 22 19.0712 22 20.5357 20.5356C21.7459 19.3254 21.956 17.5057 21.9925 14.25H19.0757C18.3381 14.25 18.1984 14.2344 18.0852 14.183C17.972 14.1316 17.8682 14.0368 17.3824 13.4817L15.5786 11.4202C15.4115 11.229 15.2372 11.0297 15.07 10.8863C14.8788 10.7223 14.5884 10.5346 14.1939 10.5527C13.7994 10.5709 13.5274 10.7844 13.352 10.9653C13.1987 11.1234 13.0434 11.3379 12.8945 11.5435L9.24075 16.5868C9.14847 16.7141 8.96574 16.7264 8.85727 16.6125L7.71566 15.4138L7.64145 15.3357C7.27927 14.9542 6.95775 14.6156 6.52932 14.4319C6.1009 14.2483 5.63391 14.2491 5.10787 14.2499L5.00015 14.25H2.00781Z"/>
                                </svg>
                                <span class="popover-title">Hardware Telemetry</span>
                            </div>
                            <span id="hud-alarm-badge" class="badge-alarm" style="display: none;">OVERCURRENT</span>
                        </div>
                        <div class="popover-info-grid">
                            <div class="info-pill"><span class="info-label">V_TARGET</span><span class="info-val" id="hud-vtarget">-- V</span></div>
                            <div class="info-pill"><span class="info-label">I_TARGET</span><span class="info-val" id="hud-itarget">-- mA</span></div>
                            <div class="info-pill"><span class="info-label">POWER</span><span class="info-val" id="hud-ptarget">-- mW</span></div>
                            <div class="info-pill"><span class="info-label">CORE TEMP</span><span class="info-val" id="hud-temp">-- °C</span></div>
                        </div>
                        <div class="popover-storage-section">
                            <div style="font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">
                                GPIO Pin Claims (GP0–GP29)
                            </div>
                            <div class="hud-pin-matrix" id="hud-pin-matrix"></div>
                        </div>
                        <div id="hud-overcurrent-banner" class="hud-alert-banner" style="display: none; margin-top: 6px;">
                            <div>Overcurrent trip! Pins in High-Z.</div>
                            <button style="background: #ef4444; color: white; border: none; border-radius: 4px; padding: 2px 6px; font-size: 0.6875rem; cursor: pointer;" onclick="fetch('/api/v1/system/telemetry')">Reset</button>
                        </div>
                    </div>
                </div>

                <!-- Device & Storage Info Icon -->
                <div class="dock-item-wrapper">
                    <button class="dock-icon-btn" id="dock-btn-storage" data-dock-tooltip="Device info">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" d="M12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22Z" fill="currentColor"/>
                            <path d="M12 17.75C12.4142 17.75 12.75 17.4142 12.75 17V11C12.75 10.5858 12.4142 10.25 12 10.25C11.5858 10.25 11.25 10.5858 11.25 11V17C11.25 17.4142 11.5858 17.75 12 17.75Z" fill="currentColor"/>
                            <path d="M12 7C12.5523 7 13 7.44772 13 8C13 8.55228 12.5523 9 12 9C11.4477 9 11 8.55228 11 8C11 7.44772 11.4477 7 12 7Z" fill="currentColor"/>
                        </svg>
                    </button>
                    <!-- Floating Glass Popover -->
                    <div class="storage-glass-popover" id="storage-glass-popover" style="display: none;" onclick="event.stopPropagation()">
                        <div class="popover-header">
                            <div style="display: flex; align-items: center; gap: 6px;">
                                <svg class="solar-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                                    <path opacity="0.5" d="M12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22Z" fill="currentColor"/>
                                    <path d="M12 17.75C12.4142 17.75 12.75 17.4142 12.75 17V11C12.75 10.5858 12.4142 10.25 12 10.25C11.5858 10.25 11.25 10.5858 11.25 11V17C11.25 17.4142 11.5858 17.75 12 17.75Z" fill="currentColor"/>
                                    <path d="M12 7C12.5523 7 13 7.44772 13 8C13 8.55228 12.5523 9 12 9C11.4477 9 11 8.55228 11 8C11 7.44772 11.4477 7 12 7Z" fill="currentColor"/>
                                </svg>
                                <span class="popover-title">Device info</span>
                            </div>
                        </div>
                        <div class="popover-info-grid">
                            <div class="info-pill"><span class="info-label">PROCESSOR</span><span class="info-val" id="popover-info-chip">RP2350B Cortex-M33</span></div>
                            <div class="info-pill"><span class="info-label">CORES / CLOCK</span><span class="info-val" id="popover-info-clock">150MHz · 520KB</span></div>
                            <div class="info-pill"><span class="info-label">SRAM MEMORY</span><span class="info-val" id="popover-info-board">BakuProbe / Pico 2</span></div>
                            <div class="info-pill"><span class="info-label">FIRMWARE</span><span class="info-val" id="popover-info-fw">v2.0</span></div>
                        </div>
                        <div class="popover-storage-section">
                            <div style="font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">
                                Flash Partitioning (<span id="popover-total-flash">8.0 MB</span> Total)
                            </div>
                            <div class="storage-segmented-bar" id="storage-bar">
                                <div class="segment-rom" id="seg-rom" style="width: 19.5%;" title="System ROM Firmware"></div>
                                <div class="segment-apps" id="seg-apps" style="width: 17.5%;" title="Installed Apps (LittleFS)"></div>
                                <div class="segment-free" id="seg-free" style="width: 63.0%;" title="Free LittleFS Flash Space"></div>
                            </div>
                            <div class="storage-dock-legend" style="margin-top: 6px;">
                                <span class="legend-item"><span class="legend-dot seg-dot-rom"></span> ROM: <span id="label-rom">1.56 MB</span></span>
                                <span class="legend-item"><span class="legend-dot seg-dot-apps"></span> Apps: <span id="label-apps">1.4 MB</span></span>
                                <span class="legend-item"><span class="legend-dot seg-dot-free"></span> Free: <span id="label-free">5.0 MB</span></span>
                            </div>
                        </div>
                    </div>
                </div>
            `;
            document.body.appendChild(dock);
        }

        // Wire popover triggers
        var teleBtn = document.getElementById('dock-btn-telemetry');
        if (teleBtn && !teleBtn._wired) {
            teleBtn._wired = true;
            teleBtn.addEventListener('click', toggleTelemetryHUD);
        }
        var storBtn = document.getElementById('dock-btn-storage');
        if (storBtn && !storBtn._wired) {
            storBtn._wired = true;
            storBtn.addEventListener('click', toggleStoragePopover);
        }

        renderDockPinnedApps();
    }

    if (typeof BroadcastChannel !== 'undefined') {
        try {
            var dockChan = new BroadcastChannel('agentla-dock-channel');
            dockChan.onmessage = function(e) {
                if (e.data && e.data.type === 'DOCK_PIN_CHANGE') {
                    renderDockPinnedApps();
                }
            };
        } catch (_) {}
    }
    window.addEventListener('storage', function(e) {
        if (e.key === 'agentla_pinned_dock_apps') {
            renderDockPinnedApps();
        }
    });

    // =========================================================================
    // Universal AgentLA JavaScript SDK
    // =========================================================================
    var activeSessionClaimedPins = new Set();

    var AgentLA = {
        version: "1.0.0",
        isDemo: isDemo,

        // 1. System & Device Info
        system: {
            specs: async function() {
                if (isDemo) return { mcu: "RP2350A", clock_mhz: 150, flash: "4MB", sram: "520KB" };
                const res = await fetch('/api/v1/system');
                return res.json();
            },
            firmwareStatus: async function() {
                if (isDemo) return { current_slot: "A", active_slot: "A", state: "Confirmed" };
                const res = await fetch('/api/v1/system/firmware/status');
                return res.json();
            }
        },

        // 2. Telemetry & Power Rail
        telemetry: {
            get: async function() {
                if (isDemo) return { vtarget_mv: 3310, itarget_ma: 14, power_mw: 46, temp_c: 29, overcurrent: false };
                const res = await fetch('/api/v1/system/telemetry');
                return res.json();
            },
            emergencySafeState: async function() {
                activeSessionClaimedPins.clear();
                if (isDemo) return { status: "safe" };
                const res = await fetch('/api/v1/profiles/apply', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ slot: 0 })
                });
                return res.json();
            }
        },

        // 3. Pin Crossbar & Arbiter
        pins: {
            status: async function() {
                if (isDemo) {
                    return { pins: Array.from({length: 30}, (_, i) => ({ pin: i, claimed: false, driver: "none" })) };
                }
                const res = await fetch('/api/v1/pins');
                return res.json();
            },
            claim: async function(pin, driver) {
                if (isDemo) {
                    activeSessionClaimedPins.add(pin);
                    return { status: "ok", pin: pin, driver: driver };
                }
                const res = await fetch('/api/v1/pins/claim', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ pin: pin, driver: driver || "adapter" })
                });
                const data = await res.json();
                if (res.ok && data.status !== 'error') {
                    activeSessionClaimedPins.add(pin);
                }
                return data;
            },
            release: async function(pin) {
                activeSessionClaimedPins.delete(pin);
                if (isDemo) return { status: "ok", pin: pin, released: true };
                const res = await fetch('/api/v1/pins/release', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ pin: pin })
                });
                return res.json();
            }
        },

        // 4. CAN Bus Subsystem
        can: {
            status: async function() {
                if (isDemo) return { active: true, bitrate: 500000, termination: true, tx_frames: 42, rx_frames: 180 };
                const res = await fetch('/api/v1/bridge/can');
                return res.json();
            },
            setTermination: async function(enable) {
                if (isDemo) return { status: "ok", termination: !!enable };
                const res = await fetch('/api/v1/bridge/can/termination', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ enabled: !!enable })
                });
                return res.json();
            },
            send: async function(frame) {
                if (isDemo) {
                    console.log("[Virtual CAN TX]", frame);
                    return { status: "ok", transmitted: true, id: frame.id };
                }
                if (window.__agentla_can_ws && window.__agentla_can_ws.readyState === WebSocket.OPEN) {
                    const payload = {
                        id: frame.id,
                        ext: !!frame.extended,
                        rtr: !!frame.rtr,
                        dlc: frame.dlc !== undefined ? frame.dlc : (frame.data ? frame.data.length / 2 : 0),
                        data: typeof frame.data === 'string' ? frame.data : (Array.isArray(frame.data) ? frame.data.map(b => b.toString(16).padStart(2, '0')).join('') : '')
                    };
                    window.__agentla_can_ws.send(JSON.stringify(payload));
                    return { status: "ok", transmitted: true, id: frame.id };
                }
                return AgentLA.mcp.call("can_send", {
                    id: frame.id,
                    data_hex: typeof frame.data === 'string' ? frame.data : (Array.isArray(frame.data) ? frame.data.map(b => b.toString(16).padStart(2, '0')).join('') : ''),
                    extended: !!frame.extended,
                    rtr: !!frame.rtr
                });
            },
            openStream: function(callbacks) {
                callbacks = callbacks || {};
                var onFrame = callbacks.onFrame || function() {};
                var onStatus = callbacks.onStatus || function() {};
                var onError = callbacks.onError || function() {};

                if (isDemo) {
                    onStatus('connected');
                    var demoTimer = setInterval(function() {
                        var randId = [0x7E8, 0x18DAF110, 0x0C9, 0x320][Math.floor(Math.random() * 4)];
                        var randData = Array.from({length: 8}, function() { return Math.floor(Math.random() * 256).toString(16).padStart(2, '0'); }).join('').toUpperCase();
                        onFrame({
                            id: randId,
                            ext: randId > 0x7FF,
                            rtr: false,
                            dlc: 8,
                            data: randData,
                            ts: Date.now() * 1000
                        });
                    }, 400);
                    return {
                        close: function() { clearInterval(demoTimer); onStatus('disconnected'); }
                    };
                }

                var proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                var wsUrl = `${proto}//${window.location.host}/api/v1/can/ws`;
                var ws = null;
                var shouldReconnect = true;
                var retryCount = 0;

                function connect() {
                    onStatus('connecting');
                    try {
                        ws = new WebSocket(wsUrl);
                        window.__agentla_can_ws = ws;

                        ws.onopen = function() {
                            retryCount = 0;
                            onStatus('connected');
                        };
                        ws.onmessage = function(e) {
                            try {
                                var frame = JSON.parse(e.data);
                                onFrame(frame);
                            } catch (_) {}
                        };
                        ws.onerror = function(err) {
                            onError(err);
                        };
                        ws.onclose = function() {
                            onStatus('disconnected');
                            if (shouldReconnect) {
                                var delay = Math.min(1000 * Math.pow(1.5, retryCount), 5000);
                                retryCount++;
                                setTimeout(connect, delay);
                            }
                        };
                    } catch (e) {
                        onError(e);
                    }
                }
                connect();

                return {
                    send: function(frame) { return AgentLA.can.send(frame); },
                    close: function() {
                        shouldReconnect = false;
                        if (ws) ws.close();
                    }
                };
            }
        },

        // 5. Serial / UART Terminal
        uart: {
            openTerminal: function(callbacks) {
                callbacks = callbacks || {};
                var onData = callbacks.onData || function() {};
                var onStatus = callbacks.onStatus || function() {};
                var onError = callbacks.onError || function() {};

                if (isDemo) {
                    onStatus('connected');
                    setTimeout(function() {
                        onData("\r\n\x1b[32mAgentLA Virtual Shell (Demo Mode)\x1b[0m\r\nType 'help' for commands.\r\n> ");
                    }, 200);
                    return {
                        write: function(text) {
                            if (text.includes("help")) {
                                onData("\r\nAvailable commands: status, pins, can, reboot\r\n> ");
                            } else {
                                onData("\r\nUnknown command: " + text.trim() + "\r\n> ");
                            }
                        },
                        close: function() { onStatus('disconnected'); }
                    };
                }

                var proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                var baud = callbacks.baud || 115200;
                var tx = callbacks.txPin !== undefined ? callbacks.txPin : 0;
                var rx = callbacks.rxPin !== undefined ? callbacks.rxPin : 1;
                var wsUrl = `${proto}//${window.location.host}/api/v1/terminal/ws?baud=${baud}&tx=${tx}&rx=${rx}`;
                var ws = null;
                var shouldReconnect = true;

                function connect() {
                    onStatus('connecting');
                    try {
                        ws = new WebSocket(wsUrl);
                        ws.binaryType = 'arraybuffer';
                        ws.onopen = function() { onStatus('connected'); };
                        ws.onmessage = function(e) {
                            if (typeof e.data === 'string') {
                                onData(e.data);
                            } else if (e.data instanceof ArrayBuffer) {
                                var text = new TextDecoder('utf-8', { fatal: false }).decode(e.data);
                                onData(text);
                            }
                        };
                        ws.onerror = function(err) { onError(err); };
                        ws.onclose = function() {
                            onStatus('disconnected');
                            if (shouldReconnect) setTimeout(connect, 1500);
                        };
                    } catch (e) {
                        onError(e);
                    }
                }
                connect();

                return {
                    write: function(text) {
                        if (ws && ws.readyState === WebSocket.OPEN) {
                            ws.send(text);
                        }
                    },
                    close: function() {
                        shouldReconnect = false;
                        if (ws) ws.close();
                    }
                };
            }
        },

        // 6. I2C & SPI
        i2c: {
            scan: async function(sda, scl) {
                if (isDemo) return [0x3C, 0x68];
                const res = await AgentLA.mcp.call("i2c_scan", { sda_pin: sda || 16, scl_pin: scl || 17 });
                try {
                    const parsed = JSON.parse(res.content[0].text);
                    return parsed.devices || [];
                } catch (_) {
                    return [];
                }
            },
            read: async function(addr, len) {
                if (isDemo) return new Uint8Array(len || 1);
                const res = await fetch(`/api/v1/bridge/i2c/read?addr=${addr}&len=${len || 1}`);
                return res.json();
            },
            write: async function(addr, bytes) {
                if (isDemo) return { status: "ok" };
                const hex = Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
                const res = await fetch('/api/v1/bridge/i2c/write', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ addr: addr, data_hex: hex })
                });
                return res.json();
            }
        },
        spi: {
            transfer: async function(txBytes, speedKhz) {
                if (isDemo) return new Uint8Array(txBytes ? txBytes.length : 1);
                const hex = Array.from(txBytes).map(b => b.toString(16).padStart(2, '0')).join('');
                const res = await AgentLA.mcp.call("spi_transfer", { mosi_hex: hex, speed_khz: speedKhz || 1000 });
                try {
                    const parsed = JSON.parse(res.content[0].text);
                    const miso = parsed.miso_hex || "";
                    const out = new Uint8Array(miso.length / 2);
                    for (let i = 0; i < miso.length; i += 2) {
                        out[i / 2] = parseInt(miso.substr(i, 2), 16);
                    }
                    return out;
                } catch (_) {
                    return new Uint8Array(0);
                }
            }
        },

        // 7. Servos & PWM
        servo: {
            attach: async function(pin, opts) {
                opts = opts || {};
                activeSessionClaimedPins.add(pin);
                if (isDemo) return { status: "ok", pin: pin };
                const url = `/api/v1/servo/attach?pin=${pin}&min_us=${opts.minUs || 500}&max_us=${opts.maxUs || 2500}&continuous=${opts.continuous ? 1 : 0}`;
                const res = await fetch(url, { method: 'POST' });
                return res.json();
            },
            writeAngle: async function(pin, angle) {
                if (isDemo) return { status: "ok", pin: pin, angle: angle };
                const url = `/api/v1/servo/write?pin=${pin}&angle=${Math.round(angle)}`;
                const res = await fetch(url, { method: 'POST' });
                return res.json();
            },
            writeMicroseconds: async function(pin, pulseUs) {
                if (isDemo) return { status: "ok", pin: pin, pulse_us: pulseUs };
                const url = `/api/v1/servo/write_us?pin=${pin}&pulse_us=${Math.round(pulseUs)}`;
                const res = await fetch(url, { method: 'POST' });
                return res.json();
            },
            detach: async function(pin) {
                activeSessionClaimedPins.delete(pin);
                if (isDemo) return { status: "ok", pin: pin };
                const url = `/api/v1/servo/detach?pin=${pin}`;
                const res = await fetch(url, { method: 'POST' });
                return res.json();
            },
            list: async function() {
                if (isDemo) return [{ pin: 16, angle: 90, pulse_us: 1500 }];
                const res = await fetch('/api/v1/servo');
                return res.json();
            }
        },

        // 8. SWD / Target Programmer
        programmer: {
            detectTarget: async function(port) {
                if (isDemo) return { detected: true, name: "STM32F103 Medium Density", flash_size: 131072 };
                const res = await fetch(port ? `/api/v1/programmer/target?port=${port}` : '/api/v1/programmer/target');
                return res.json();
            },
            readMemory: async function(addr, length) {
                if (isDemo) return { status: "ok", bytes: [0x00, 0x50, 0x00, 0x20] };
                const res = await AgentLA.mcp.call("programmer_read", { address: addr, length: length || 16 });
                try {
                    return JSON.parse(res.content[0].text);
                } catch (_) {
                    return { status: "error" };
                }
            },
            flashMemory: async function(addr, hexData) {
                if (isDemo) return { status: "ok", bytes_written: hexData.length / 2 };
                const res = await AgentLA.mcp.call("programmer_flash", { address: addr, hex_data: hexData });
                try {
                    return JSON.parse(res.content[0].text);
                } catch (_) {
                    return { status: "error" };
                }
            },
            resetTarget: async function() {
                if (isDemo) return { status: "ok" };
                const res = await fetch('/api/v1/programmer/reset', { method: 'POST' });
                return res.json();
            },
            unbrick: async function(target) {
                if (isDemo) return { status: "ok", target: target, mass_erased: true };
                const res = await AgentLA.mcp.call("programmer_unbrick", { target: target || "stm32" });
                try {
                    return JSON.parse(res.content[0].text);
                } catch (_) {
                    return { status: "error" };
                }
            }
        },

        // 9. In-Browser MCP JSON-RPC 2.0 Dispatch
        mcp: {
            call: async function(toolName, args) {
                if (isDemo) {
                    return { content: [{ type: "text", text: JSON.stringify({ status: "simulated", tool: toolName }) }], isError: false };
                }
                const res = await fetch('/mcp', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        jsonrpc: '2.0',
                        method: 'tools/call',
                        params: { name: toolName, arguments: args || {} },
                        id: Date.now()
                    })
                });
                const rpc = await res.json();
                return rpc.result || { isError: true, content: [{ type: "text", text: rpc.error ? rpc.error.message : "RPC Error" }] };
            },
            listTools: async function() {
                if (isDemo) return { tools: [{ name: "can_send" }, { name: "target_diagnose" }] };
                const res = await fetch('/mcp', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ jsonrpc: '2.0', method: 'tools/list', id: Date.now() })
                });
                const rpc = await res.json();
                return rpc.result || { tools: [] };
            }
        },

        // 10. UI Helpers
        ui: {
            toast: function(message, type) {
                type = type || 'info';
                var container = document.getElementById('agentla-toast-container');
                if (!container) {
                    container = document.createElement('div');
                    container.id = 'agentla-toast-container';
                    container.className = 'toast toast-top toast-end z-50';
                    document.body.appendChild(container);
                }
                var alert = document.createElement('div');
                var alertClass = type === 'error' ? 'alert alert-error' : (type === 'success' ? 'alert alert-success' : 'alert alert-info');
                alert.className = alertClass + ' py-2 px-3 text-xs shadow-lg animate-fade-in flex items-center gap-2';
                alert.innerHTML = `<span>${message}</span>`;
                container.appendChild(alert);
                setTimeout(function() {
                    alert.style.opacity = '0';
                    alert.style.transition = 'opacity 0.3s ease';
                    setTimeout(function() { alert.remove(); }, 300);
                }, 3000);
            },
            setTheme: function(theme) { syncTheme(theme); },
            getTheme: function() { return { theme: document.documentElement.getAttribute('data-theme'), mode: themeState.mode }; }
        }
    };

    // Automatic Safe-State on Unload Hook
    function teardownSessionPins() {
        if (activeSessionClaimedPins.size > 0) {
            try {
                if (navigator.sendBeacon) {
                    navigator.sendBeacon('/api/v1/profiles/apply', JSON.stringify({ slot: 0 }));
                } else {
                    fetch('/api/v1/profiles/apply', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ slot: 0 }),
                        keepalive: true
                    });
                }
            } catch (_) {}
            activeSessionClaimedPins.clear();
        }
    }
    window.addEventListener('pagehide', teardownSessionPins);
    window.addEventListener('beforeunload', teardownSessionPins);

    window.AgentLA = AgentLA;

    function initBridge() {
        try {
            var params = new URLSearchParams(window.location.search);
            if (params.get('no_launcher') === '1' || window.AgentLANoLauncher === true) {
                return;
            }
        } catch (_) {}
        initTopBar();
        mountDock();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initBridge);
    } else {
        initBridge();
    }
})();
