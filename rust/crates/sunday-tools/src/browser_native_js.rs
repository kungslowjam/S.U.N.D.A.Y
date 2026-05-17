pub const POPUP_KILLER_JS: &str = r#"
    setInterval(() => {
        const closeSelectors = [
            '[aria-label="Close"]', '[aria-label="close"]', '.close-button', 
            '.modal-close', '.popup-close', '.btn-close'
        ];
        closeSelectors.forEach(s => {
            document.querySelectorAll(s).forEach(el => {
                if (el && el.offsetHeight > 0 && !el.hasAttribute('data-sunday-handled')) {
                    el.click();
                    el.setAttribute('data-sunday-handled', 'true');
                }
            });
        });
    }, 2000);
"#;

pub const AX_TREE_EXTRACTOR_JS: &str = r#"
    (() => {
        const interactiveRoles = ['button', 'link', 'checkbox', 'menuitem', 'option', 'tab', 'textbox', 'combobox', 'searchbox'];
        const tree = [];
        let idCounter = 1;
        
        const processNode = (node) => {
            const elements = node.querySelectorAll('button, a, input, select, textarea, [role], [onclick]');
            elements.forEach(el => {
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                if (rect.width > 2 && rect.height > 2 && style.display !== 'none' && style.visibility !== 'hidden') {
                    const role = el.getAttribute('role') || el.tagName.toLowerCase();
                    const text = (el.innerText || el.value || el.placeholder || '').trim();
                    if (text.length > 0 || interactiveRoles.includes(role)) {
                        const id = idCounter++;
                        el.setAttribute('data-sunday-id', id);
                        tree.push({
                            id, role, text: text.substring(0, 50),
                            x: Math.round(rect.left + rect.width/2),
                            y: Math.round(rect.top + rect.height/2)
                        });
                    }
                }
            });
        };
        processNode(document);
        return tree.slice(0, 50);
    })()
"#;
