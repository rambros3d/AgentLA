/**
 * AgentLA OS Bridge Script
 * Injected or included into custom LittleFS web apps under /apps/{app-name}/
 * Provides:
 * 1. Unified Clean OS Top Bar (Matching Workbench & Launcher)
 * 2. Floating macOS-Style Glass Dock with live Pinned Apps
 * 3. Universal Real-Time Theme Synchronization Engine
 * 4. Target Power Rail & Emergency Safe State Controls with SSE Telemetry
 */
(function() {
    'use strict';

    if (window.__AGENTLA_BRIDGE_LOADED__) return;
    window.__AGENTLA_BRIDGE_LOADED__ = true;

    var isDemo = window.location.protocol === 'https:' || 
                 window.location.hostname.includes('github.io') ||
                 window.location.search.includes('demo=1') ||
                 window.location.pathname.endsWith('.html');

    var homeUrl = isDemo ? (window.location.pathname.includes('/apps/') ? '../../apps.html' : 'apps.html') : '/apps';
    var labUrl = isDemo ? (window.location.pathname.includes('/apps/') ? '../../index.html' : 'index.html') : '/';

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
                    <button class="theme-toggle-btn mode-${themeState.mode}" id="btn-theme-toggle" data-mode="${themeState.mode}" title="Theme Mode" aria-label="Toggle Theme Mode">
                        <svg class="solar-icon icon-sun" width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path d="M18 12C18 15.3137 15.3137 18 12 18C8.68629 18 6 15.3137 6 12C6 8.68629 8.68629 6 12 6C15.3137 6 18 8.68629 18 12Z" fill="currentColor"/>
                            <path fill-rule="evenodd" clip-rule="evenodd" d="M12 1.25C12.4142 1.25 12.75 1.58579 12.75 2V3C12.75 3.41421 12.4142 3.75 12 3.75C11.5858 3.75 11.25 3.41421 11.25 3V2C11.25 1.58579 11.5858 1.25 12 1.25ZM1.25 12C1.25 11.5858 1.58579 11.25 2 11.25H3C3.41421 11.25 3.75 11.5858 3.75 12C3.75 12.4142 3.41421 12.75 3 12.75H2C1.58579 12.75 1.25 12.4142 1.25 12ZM20.25 12C20.25 11.5858 20.5858 11.25 21 11.25H22C22.4142 11.25 22.75 11.5858 22.75 12C22.75 12.4142 22.4142 12.75 22 12.75H21C20.5858 12.75 20.25 12.4142 20.25 12ZM12 20.25C12.4142 20.25 12.75 20.5858 12.75 21V22C12.75 22.4142 12.4142 22.75 12 22.75C11.5858 22.75 11.25 22.4142 11.25 22V21C11.25 20.5858 11.5858 20.25 12 20.25Z" fill="currentColor"/>
                        </svg>
                    </button>
                </div>
            `;
            document.body.prepend(topBar);
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

        // Telemetry Stream SSE for power pill
        try {
            var sse = new EventSource('/api/v1/system/telemetry/stream');
            sse.addEventListener('telemetry', function(e) {
                try {
                    var data = JSON.parse(e.data);
                    var vEl = document.getElementById('top-bar-voltage-val');
                    var iEl = document.getElementById('top-bar-current-val');
                    if (vEl && data.v_target_v !== undefined) vEl.textContent = data.v_target_v.toFixed(2) + 'V';
                    if (iEl && data.i_target_ma !== undefined) iEl.textContent = Math.round(data.i_target_ma) + 'mA';
                } catch (_) {}
            });
        } catch (_) {}
    }

    // -------------------------------------------------------------------------
    // Floating macOS Glass Dock Mounting & Synchronization
    // -------------------------------------------------------------------------
    function getPinnedApps() {
        try {
            var stored = localStorage.getItem('agentla_pinned_dock_apps');
            if (stored) {
                var list = JSON.parse(stored);
                if (Array.isArray(list)) {
                    var hasSystem = list.some(function(id) { return ['scope', 'adapters', 'target', 'pins', 'console'].indexOf(id) !== -1; });
                    var migrated = localStorage.getItem('agentla_dock_migrated_v2');
                    if (!hasSystem && !migrated) {
                        var combined = ['scope', 'adapters', 'target', 'pins', 'console'].concat(list);
                        localStorage.setItem('agentla_pinned_dock_apps', JSON.stringify(combined));
                        localStorage.setItem('agentla_dock_migrated_v2', '1');
                        return combined;
                    }
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
        ['scope', 'adapters', 'target', 'pins', 'console'].forEach(function(sysId) {
            var el = document.getElementById('dock-app-' + sysId);
            if (el) {
                el.style.display = pinned.indexOf(sysId) !== -1 ? 'inline-flex' : 'none';
            }
        });
        var html = '';

        pinned.forEach(function(appId) {
            var isCurrent = window.location.pathname.includes('/' + appId + '/');
            var activeClass = isCurrent ? ' active' : '';

            if (appId === 'terminal') {
                var termHref = isDemo ? (window.location.pathname.includes('/apps/') ? '../terminal/index.html' : 'apps/terminal/index.html') : '/apps/terminal/';
                html += `
                    <a href="${termHref}" class="dock-icon-btn${activeClass}" data-dock-tooltip="Serial Terminal">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355Z" fill="currentColor"/>
                            <path d="M13.4876 6.44591C13.8877 6.55311 14.1251 6.96437 14.0179 7.36447L11.4297 17.0237C11.3225 17.4238 10.9113 17.6613 10.5112 17.5541C10.1111 17.4468 9.87362 17.0356 9.98082 16.6355L12.569 6.97624C12.6762 6.57614 13.0875 6.3387 13.4876 6.44591Z" fill="currentColor"/>
                            <path d="M14.9692 8.46967C15.2621 8.17678 15.7369 8.17678 16.0298 8.46967L16.2382 8.67801C16.8735 9.3133 17.4033 9.84308 17.7674 10.3202C18.1516 10.8238 18.4211 11.3559 18.4211 12C18.4211 12.6441 18.1516 13.1762 17.7674 13.6798C17.4033 14.1569 16.8735 14.6867 16.2382 15.322L16.0298 15.5303C15.7369 15.8232 15.2621 15.8232 14.9692 15.5303C14.6763 15.2374 14.6763 14.7626 14.9692 14.4697L15.1408 14.2981C15.8224 13.6164 16.2792 13.1574 16.5748 12.7699C16.8573 12.3998 16.9211 12.1843 16.9211 12C16.9211 11.8157 16.8573 11.6002 16.5748 11.2301C16.2792 10.8426 15.8224 10.3836 15.1408 9.70191L14.9692 9.53033C14.6763 9.23744 14.6763 8.76257 14.9692 8.46967Z" fill="currentColor"/>
                            <path d="M7.96937 8.46967C8.26226 8.17678 8.73713 8.17678 9.03003 8.46967C9.32292 8.76257 9.32292 9.23744 9.03003 9.53033L8.85846 9.70191C8.1768 10.3836 7.72003 10.8426 7.42439 11.2301C7.14196 11.6002 7.07812 11.8157 7.07812 12C7.07812 12.1843 7.14196 12.3998 7.42439 12.7699C7.72003 13.1574 8.1768 13.6164 8.85846 14.2981L9.03003 14.4697C9.32292 14.7626 9.32292 15.2374 9.03003 15.5303C8.73714 15.8232 8.26226 15.8232 7.96937 15.5303L7.76102 15.322C7.12568 14.6867 6.59589 14.1569 6.23186 13.6798C5.84762 13.1762 5.57812 12.6441 5.57812 12C5.57812 11.3559 5.84762 10.8238 6.23186 10.3202C6.59589 9.84308 7.12569 9.31331 7.76102 8.67801L7.96937 8.46967Z" fill="currentColor"/>
                        </svg>
                    </a>`;
            } else if (appId === 'servo-master') {
                var servoHref = isDemo ? (window.location.pathname.includes('/apps/') ? '../servo-master/index.html' : 'apps/servo-master/index.html') : '/apps/servo-master/';
                html += `
                    <a href="${servoHref}" class="dock-icon-btn${activeClass}" data-dock-tooltip="Servo Master">
                        <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <path opacity="0.5" d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2Z" fill="currentColor"/>
                            <path d="M12 8C9.79086 8 8 9.79086 8 12C8 14.2091 9.79086 16 12 16C14.2091 16 16 14.2091 16 12C16 9.79086 14.2091 8 12 8Z" fill="currentColor"/>
                            <path d="M12 4V7M12 17V20M4 12H7M17 12H20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        </svg>
                    </a>`;
            } else {
                var appHref = isDemo ? (window.location.pathname.includes('/apps/') ? `../${encodeURIComponent(appId)}/index.html` : `apps/${encodeURIComponent(appId)}/index.html`) : `/apps/${encodeURIComponent(appId)}/`;
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
                <a href="${labUrl}#scope" class="dock-icon-btn" id="dock-app-scope" data-dock-tooltip="Scope (100 MS/s)">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M17.5762 10.4802C17.8414 10.1619 17.7984 9.68903 17.4802 9.42385C17.1619 9.15868 16.689 9.20167 16.4239 9.51988L14.627 11.6761C14.2562 12.1211 14.0285 12.3915 13.8409 12.5609C13.7539 12.6395 13.7023 12.6708 13.6776 12.6827C13.6725 12.6852 13.6689 12.6866 13.6667 12.6875C13.6667 12.6875 13.6624 12.6858 13.659 12.6843L13.6558 12.6827C13.6311 12.6708 13.5795 12.6395 13.4925 12.5609C13.3049 12.3915 13.0772 12.1211 12.7064 11.6761L12.414 11.3253C12.0855 10.931 11.7894 10.5756 11.5128 10.3259C11.2119 10.0541 10.8328 9.81207 10.3334 9.81207C9.83386 9.81207 9.4548 10.0541 9.15386 10.3259C8.87727 10.5756 8.58115 10.931 8.25269 11.3253L6.42385 13.5199C6.15868 13.8381 6.20167 14.311 6.51988 14.5762C6.83809 14.8414 7.31101 14.7984 7.57619 14.4802L9.37308 12.3239C9.74387 11.8789 9.97157 11.6085 10.1592 11.4391C10.2461 11.3606 10.2978 11.3293 10.3225 11.3173L10.3292 11.3142L10.3334 11.3126C10.3356 11.3134 10.3392 11.3149 10.3442 11.3173C10.369 11.3293 10.4206 11.3606 10.5076 11.4391C10.6951 11.6085 10.9228 11.8789 11.2936 12.3239L11.586 12.6748L11.586 12.6748C11.9145 13.069 12.2106 13.4244 12.4872 13.6742C12.7881 13.9459 13.1672 14.188 13.6667 14.188C14.1662 14.188 14.5452 13.9459 14.8462 13.6742C15.1228 13.4244 15.4189 13.069 15.7473 12.6748L17.5762 10.4802Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Adapters -->
                <a href="${labUrl}#adapters" class="dock-icon-btn" id="dock-app-adapters" data-dock-tooltip="Protocol Adapters">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M3.46447 20.5355C4.92893 22 7.28595 22 12 22C16.714 22 19.0711 22 20.5355 20.5355C22 19.0711 22 16.714 22 12C22 7.28595 22 4.92893 20.5355 3.46447C19.0711 2 16.714 2 12 2C7.28595 2 4.92893 2 3.46447 3.46447C2 4.92893 2 7.28595 2 12C2 16.714 2 19.0711 3.46447 20.5355Z" fill="currentColor"/>
                        <path d="M9.5 8.75C7.70507 8.75 6.25 10.2051 6.25 12C6.25 13.7949 7.70507 15.25 9.5 15.25C11.2949 15.25 12.75 13.7949 12.75 12C12.75 11.5858 13.0858 11.25 13.5 11.25C13.9142 11.25 14.25 11.5858 14.25 12C14.25 14.6234 12.1234 16.75 9.5 16.75C6.87665 16.75 4.75 14.6234 4.75 12C4.75 9.37665 6.87665 7.25 9.5 7.25C9.91421 7.25 10.25 7.58579 10.25 8C10.25 8.41421 9.91421 8.75 9.5 8.75Z" fill="currentColor"/>
                        <path d="M17.75 12C17.75 13.7949 16.2949 15.25 14.5 15.25C14.0858 15.25 13.75 15.5858 13.75 16C13.75 16.4142 14.0858 16.75 14.5 16.75C17.1234 16.75 19.25 14.6234 19.25 12C19.25 9.37665 17.1234 7.25 14.5 7.25C11.8766 7.25 9.75 9.37665 9.75 12C9.75 12.4142 10.0858 12.75 10.5 12.75C10.9142 12.75 11.25 12.4142 11.25 12C11.25 10.2051 12.7051 8.75 14.5 8.75C16.2949 8.75 17.75 10.2051 17.75 12Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- SWD Debugger -->
                <a href="${labUrl}#target" class="dock-icon-btn" id="dock-app-target" data-dock-tooltip="SWD Debugger">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M11.1566 20.3133C16.2137 20.3133 20.3133 16.2137 20.3133 11.1566C20.3133 6.09956 16.2137 2 11.1566 2C6.09956 2 2 6.09956 2 11.1566C2 16.2137 6.09956 20.3133 11.1566 20.3133Z" fill="currentColor"/>
                        <path d="M21.7881 20.7656C22.0704 21.0479 22.0704 21.5058 21.7881 21.7881C21.5058 22.0704 21.0479 22.0704 20.7656 21.7881L17.0996 18.1221C17.4666 17.8086 17.8086 17.4666 18.1221 17.0996L21.7881 20.7656Z" fill="currentColor"/>
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M11 6.5C12.1408 6.5 13.1567 7.03207 13.8105 7.86035L14.4902 7.52246C14.843 7.34709 15.2717 7.48911 15.4482 7.83984C15.6246 8.19079 15.4817 8.61744 15.1289 8.79297L14.4502 9.13086C14.5293 9.42476 14.5713 9.734 14.5713 10.0527V10.2891H15.2861C15.6804 10.2893 16 10.6077 16 11C16 11.3923 15.6804 11.7107 15.2861 11.7109H14.5713V11.9473C14.5713 12.266 14.5293 12.5752 14.4502 12.8691L15.1289 13.207C15.4817 13.3826 15.6246 13.8092 15.4482 14.1602C15.2717 14.5109 14.843 14.6529 14.4902 14.4775L13.8105 14.1396C13.1567 14.9679 12.1408 15.5 11 15.5C9.85919 15.5 8.8433 14.9679 8.18945 14.1396L7.50977 14.4775C7.15704 14.6529 6.72826 14.5109 6.55176 14.1602C6.37536 13.8092 6.51835 13.3826 6.87109 13.207L7.5498 12.8691C7.47066 12.5752 7.42871 12.266 7.42871 11.9473V11.7109H6.71387C6.31957 11.7107 6 11.3923 6 11C6 10.6077 6.31957 10.2893 6.71387 10.2891H7.42871V10.0527C7.42871 9.734 7.47066 9.42476 7.5498 9.13086L6.87109 8.79297C6.51835 8.61744 6.37536 8.19079 6.55176 7.83984C6.72826 7.48911 7.15704 7.34709 7.50977 7.52246L8.18945 7.86035C8.8433 7.03207 9.85919 6.5 11 6.5ZM8.85742 10.7627V11.9473C8.85744 12.8753 9.45388 13.6644 10.2861 13.957V10.7627H8.85742ZM11.7148 10.7627V13.957C12.5469 13.6643 13.1435 12.8752 13.1436 11.9473V10.7627H11.7148ZM11 7.9209C10.0672 7.92102 9.27368 8.51407 8.97949 9.3418H13.0215C12.7273 8.51392 11.933 7.9209 11 7.9209Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Pins -->
                <a href="${labUrl}#pins" class="dock-icon-btn" id="dock-app-pins" data-dock-tooltip="Pin Routing Crossbar">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M12 3C12.3853 3 12.6977 3.31236 12.6977 3.69767V6.48884C13.2084 6.48991 13.6717 6.49342 14.0932 6.50495L14.093 6.48837V3.69767C14.093 3.31236 14.4054 3 14.7907 3C15.176 3 15.4884 3.31236 15.4884 3.69767V6.48837C15.4884 6.52487 15.4856 6.56072 15.4802 6.5957C16.4162 6.71067 17.0648 6.94879 17.558 7.44198C18.0512 7.93517 18.2893 8.58381 18.4043 9.51984C18.4393 9.51443 18.4751 9.51163 18.5116 9.51163H21.3023C21.6876 9.51163 22 9.82399 22 10.2093C22 10.5946 21.6876 10.907 21.3023 10.907H18.5116L18.495 10.9068C18.5066 11.3283 18.5106 11.7916 18.5116 12.3023H21.3023C21.6876 12.3023 22 12.6147 22 13C22 13.3853 21.6876 13.6977 21.3023 13.6977L18.5112 13.6977C18.5101 14.2084 18.5066 14.6717 18.495 15.0932L18.5116 15.093H21.3023C21.6876 15.093 22 15.4054 22 15.7907C22 16.176 21.6876 16.4884 21.3023 16.4884H18.5116C18.4751 16.4884 18.4393 16.4856 18.4043 16.4802C18.2893 17.4162 18.0512 18.0648 17.558 18.558C17.0648 19.0512 16.4162 19.2893 15.4802 19.4043C15.4856 19.4393 15.4884 19.4751 15.4884 19.5116V22.3023C15.4884 22.6876 15.176 23 14.7907 23C14.4054 23 14.093 22.6876 14.093 22.3023V19.5116L14.0932 19.495C13.6717 19.5066 13.2084 19.5106 12.6977 19.5116V22.3023C12.6977 22.6876 12.3853 23 12 23C11.6147 23 11.3023 22.6876 11.3023 22.3023L11.3023 19.5112C10.7916 19.5101 10.3283 19.5066 9.90678 19.495L9.90698 19.5116V22.3023C9.90698 22.6876 9.59462 23 9.2093 23C8.82399 23 8.51163 22.6876 8.51163 22.3023V19.5116C8.51163 19.4751 8.51443 19.4393 8.51984 19.4043C7.58381 19.2893 6.93517 19.0512 6.44198 18.558C5.94879 18.0648 5.71067 17.4162 5.5957 16.4802C5.56071 16.4856 5.52487 16.4884 5.48837 16.4884H2.69767C2.31236 16.4884 2 16.176 2 15.7907C2 15.4054 2.31236 15.093 2.69767 15.093H5.48837L5.50495 15.0932C5.49342 14.6717 5.48944 14.2084 5.48837 13.6977H2.69767C2.31236 13.6977 2 13.3853 2 13C2 12.6147 2.31236 12.3023 2.69767 12.3023L5.48884 12.3023C5.48991 11.7916 5.49342 11.3283 5.50495 10.9068L5.48837 10.907H2.69767C2.31236 10.907 2 10.2093 2 10.2093C2 9.82399 2.31236 9.51163 2.69767 9.51163H5.48837C5.52487 9.51163 5.56071 9.51443 5.5957 9.51984C5.71067 8.58381 5.94879 7.93517 6.44198 7.44198C6.93517 6.94879 7.58381 6.71067 8.51984 6.5957C8.51443 6.56072 8.51163 6.52487 8.51163 6.48837V3.69767C8.51163 3.31236 8.82399 3 9.2093 3C9.59462 3 9.90698 3.31236 9.90698 3.69767V6.48837L9.90678 6.50495C10.3283 6.49342 10.7916 6.48944 11.3023 6.48837V3.69767C11.3023 3.31236 11.6147 3 12 3ZM11.0238 8.5814C10.4054 8.58136 9.87247 8.58133 9.44573 8.63871C8.98839 8.70019 8.55001 8.83885 8.19443 9.19443C7.83885 9.55001 7.70019 9.98839 7.63871 10.4457C7.58133 10.8725 7.58136 11.4054 7.5814 12.0238V13.9762C7.58136 14.5946 7.58133 15.1275 7.63871 15.5543C7.70019 16.0116 7.83885 16.45 8.19443 16.8056C8.55001 17.1612 8.98839 17.2998 9.44573 17.3613C9.87247 17.4187 10.4054 17.4186 11.0238 17.4186H12.9762C13.5946 17.4186 14.1275 17.4187 14.5543 17.3613C15.0116 17.2998 15.45 17.1612 15.8056 16.8056C16.1612 16.45 16.2998 16.0116 16.3613 15.5543C16.4187 15.1275 16.4186 14.5946 16.4186 13.9762V12.0238C16.4186 11.4054 16.4187 10.8725 16.3613 10.4457C16.2998 9.98839 16.1612 9.55001 15.8056 9.19443C15.45 8.83885 15.0116 8.70019 14.5543 8.63871C14.1275 8.58133 13.5947 8.58136 12.9762 8.5814H11.0238Z" fill="currentColor"/>
                        <path opacity="0.5" d="M9.18091 10.1809C9.23402 10.1278 9.32886 10.0621 9.63147 10.0214C9.95415 9.97804 10.3921 9.97656 11.0696 9.97656H12.9301C13.6075 9.97656 14.0455 9.97804 14.3682 10.0214C14.6708 10.0621 14.7656 10.1278 14.8187 10.1809C14.8718 10.234 14.9375 10.3289 14.9782 10.6315C15.0216 10.9542 15.0231 11.3921 15.0231 12.0696V13.9301C15.0231 14.6075 15.0216 15.0455 14.9782 15.3682C14.9375 15.6708 14.8718 15.7656 14.8187 15.8187C14.7656 15.8718 14.6708 15.9375 14.3682 15.9782C14.0455 16.0216 13.6075 16.0231 12.9301 16.0231H11.0696C10.3921 16.0231 9.95415 16.0216 9.63147 15.9782C9.32886 15.9375 9.23402 15.8718 9.18091 15.8187C9.1278 15.7656 9.06211 15.6708 9.02143 15.3682C8.97804 15.0455 8.97656 14.6075 8.97656 13.9301V12.0696C8.97656 11.3921 8.97804 10.9542 9.02143 10.6315C9.06211 10.3289 9.1278 10.234 9.18091 10.1809Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Console -->
                <a href="${labUrl}#console" class="dock-icon-btn" id="dock-app-console" data-dock-tooltip="Firmware Console">
                    <svg class="solar-icon" width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12Z" fill="currentColor"/>
                        <path d="M6.42385 9.51988C6.68903 9.20167 7.16195 9.15868 7.48016 9.42385L7.75658 9.6542L7.75661 9.65423C8.36154 10.1583 8.87654 10.5874 9.23295 10.9821C9.61151 11.4013 9.90694 11.8834 9.90694 12.5C9.90694 13.1166 9.61151 13.5987 9.23295 14.0179C8.87653 14.4126 8.36153 14.8418 7.75658 15.3458L7.48016 15.5762C7.16195 15.8414 6.68903 15.7984 6.42385 15.4802C6.15868 15.1619 6.20167 14.689 6.51988 14.4239L6.75428 14.2285C7.41285 13.6797 7.84348 13.3185 8.11968 13.0126C8.38196 12.7222 8.40694 12.586 8.40694 12.5C8.40694 12.414 8.38196 12.2779 8.11968 11.9874C7.84348 11.6815 7.41285 11.3203 6.75429 10.7715L6.51988 10.5762C6.20167 10.311 6.15868 9.83809 6.42385 9.51988Z" fill="currentColor"/>
                        <path d="M17.75 15C17.75 15.4142 17.4142 15.75 17 15.75H12C11.5858 15.75 11.25 15.4142 11.25 15C11.25 14.5858 11.5858 14.25 12 14.25H17C17.4142 14.25 17.75 14.5858 17.75 15Z" fill="currentColor"/>
                    </svg>
                </a>

                <!-- Pinned LittleFS Applications in Dock -->
                <div id="dock-pinned-apps" style="display: contents;"></div>
            `;
            document.body.appendChild(dock);
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

    function initBridge() {
        initTopBar();
        mountDock();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initBridge);
    } else {
        initBridge();
    }
})();
