pub const JARVIS_SCRIPT: &str = r#"
setInterval(() => {
    const closeSelectors = [
        '[aria-label="Close"]', '[aria-label="close"]', '[aria-label="ปิด"]',
        '.close-button', '.modal-close', '.popup-close', '.btn-close', 
        '.fancybox-close', '.insider-opt-in-notification-close-button',
        'img[src*="close"]', 'i.fa-times', 'i.fa-close', 'svg[data-icon="xmark"]',
        'div[id*="close"]', 'div[class*="close"]', '.ab-close-button',
        '[data-action-type="DISMISS"]', '.a-button-close', '#nav-main .nav-sprite',
        'input[data-action-type="SELECT_LOCATION"]',
        '.bui-modal__close', '[aria-label="Dismiss sign-in info."]',
        '.modal-mask-close', '.sgn-x', '.shopee-popup__close-btn'
    ];
    
    document.querySelectorAll('[role="dialog"], [aria-modal="true"], .modal, .popup-container').forEach(el => {
        if (!el.hasAttribute('data-sunday-handled-modal')) {
            const possibleButtons = el.querySelectorAll('button, [role="button"], i, svg');
            possibleButtons.forEach(btn => {
                const rect = btn.getBoundingClientRect();
                if (rect.width < 50 && rect.height < 50) {
                    btn.click();
                }
            });
            el.setAttribute('data-sunday-handled-modal', 'true');
        }
    });

    document.querySelectorAll(closeSelectors.join(',')).forEach(el => {
        const style = window.getComputedStyle(el);
        if(style.display !== 'none' && style.visibility !== 'hidden' && el.getBoundingClientRect().width > 0) {
            try { 
                if (!el.hasAttribute('data-sunday-handled')) {
                    el.click(); 
                    el.setAttribute('data-sunday-handled', 'true');
                }
            } catch(e){}
        }
    });

    document.querySelectorAll('div').forEach(el => {
        const rect = el.getBoundingClientRect();
        const style = window.getComputedStyle(el);
        if (style.position === 'fixed' && 
            rect.width > window.innerWidth * 0.5 && 
            rect.height > window.innerHeight * 0.5 &&
            style.zIndex > 1000) {
            const startTime = parseInt(el.getAttribute('data-sunday-start') || Date.now());
            el.setAttribute('data-sunday-start', startTime);
            if (Date.now() - startTime > 5000) {
                el.style.display = 'none';
                el.style.pointerEvents = 'none';
            }
        }
    });
}, 1000);
"#;

pub const AX_TREE_SCRIPT: &str = r#"
window.__get_ax_tree = () => {
    const interactiveRoles = ['button', 'link', 'checkbox', 'menuitem', 'option', 'tab', 'textbox', 'combobox', 'searchbox'];
    const tree = [];
    let idCounter = 1;
    
    const processNode = (node) => {
        if (!node || !node.querySelectorAll) return;
        const elements = node.querySelectorAll('button, a, input, select, textarea, [role], [onclick]');
        elements.forEach(el => {
            if (el.hasAttribute('data-sunday-id')) return;

            const rect = el.getBoundingClientRect();
            const style = window.getComputedStyle(el);
            
            if (rect.width > 2 && rect.height > 2 && style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0') {
                const role = el.getAttribute('role') || el.tagName.toLowerCase();
                const text = (el.innerText || el.value || el.placeholder || el.getAttribute('aria-label') || '').trim();
                
                const isClickable = style.cursor === 'pointer' || el.onclick || el.hasAttribute('onclick') || interactiveRoles.includes(role);
                
                if (text.length > 0 || isClickable) {
                    const id = idCounter++;
                    el.setAttribute('data-sunday-id', id);
                    tree.push({
                        id,
                        role,
                        text: text.substring(0, 60).replace(/\n/g, ' '),
                        x: Math.round(rect.left + rect.width / 2),
                        y: Math.round(rect.top + rect.height / 2)
                    });
                }
            }
            if (el.shadowRoot) processNode(el.shadowRoot);
        });
    };
    
    processNode(document);
    return tree.slice(0, 80);
};
"#;

pub const VISUAL_CURSOR_SCRIPT: &str = r#"
const injectStyles = () => {
    if (document.getElementById('__sunday_visuals_root')) return;
    
    const root = document.createElement('div');
    root.id = '__sunday_visuals_root';
    const shadow = root.attachShadow({mode: 'open'});
    
    const border = document.createElement('div');
    Object.assign(border.style, {
        position: 'fixed', top: '0', left: '0', width: '100vw', height: '100vh',
        boxSizing: 'border-box', pointerEvents: 'none', zIndex: '2147483647',
        border: '8px solid rgba(0, 123, 255, 0.6)',
        animation: 'sunday-pulse 2s infinite ease-in-out'
    });
    
    const style = document.createElement('style');
    style.textContent = `
        @keyframes sunday-pulse {
            0% { box-shadow: inset 0 0 40px 10px rgba(0, 242, 254, 0.4); }
            50% { box-shadow: inset 0 0 80px 30px rgba(0, 242, 254, 0.7); }
            100% { box-shadow: inset 0 0 40px 10px rgba(0, 242, 254, 0.4); }
        }
    `;
    shadow.appendChild(style);
    shadow.appendChild(border);
    
    if (!window.__playwright_cursor) {
        window.__playwright_cursor = document.createElement('div');
        Object.assign(window.__playwright_cursor.style, {
            position: 'fixed', width: '16px', height: '16px',
            background: 'radial-gradient(circle, #00f2fe 0%, #4facfe 100%)',
            border: '2px solid #fff', borderRadius: '50%', pointerEvents: 'none',
            zIndex: '2147483647', boxShadow: '0 0 15px #00f2fe', display: 'none'
        });
        shadow.appendChild(window.__playwright_cursor);
    }
    (document.body || document.documentElement).appendChild(root);
};
setInterval(injectStyles, 500);
injectStyles();

window.__move_cursor = (x, y) => {
    if (!window.__playwright_cursor) return;
    window.__playwright_cursor.style.display = 'block';
    window.__playwright_cursor.style.left = x + 'px';
    window.__playwright_cursor.style.top = y + 'px';
};
"#;
