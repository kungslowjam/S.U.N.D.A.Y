use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotParams, CaptureScreenshotFormat};
use chromiumoxide::layout::Point;
use futures::StreamExt;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use rand::Rng;
use std::time::Duration;
use crate::browser_native_js::{POPUP_KILLER_JS, AX_TREE_EXTRACTOR_JS};

/// High-performance Native Browser Controller using chromiumoxide.
pub struct NativeBrowser {
    pub browser: Browser,
    pub page: Arc<Page>,
    pub handle: tokio::task::JoinHandle<()>,
}

impl NativeBrowser {
    pub async fn new(headless: bool) -> Result<Self> {
        let mut builder = BrowserConfig::builder()
            .chrome_executable(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-gpu")
            .arg("--disable-web-security");
        if !headless {
            builder = builder.with_head();
        }
        
        let config = builder.build().map_err(|e| anyhow!(e))?;
        let (browser, mut handler) = Browser::launch(config).await?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(e) = h {
                    // Log the error but NEVER break the browser event loop!
                    tracing::warn!("Native browser handler error: {:?}", e);
                }
            }
        });

        let page = Arc::new(browser.new_page("about:blank").await?);
        page.evaluate_on_new_document(POPUP_KILLER_JS).await?;
        
        Ok(Self { browser, page, handle })
    }
}

/// Thread-safe Session Manager for NativeBrowser.
pub struct NativeBrowserSession {
    instance: Mutex<Option<Arc<NativeBrowser>>>,
}

impl NativeBrowserSession {
    pub fn new() -> Self {
        Self {
            instance: Mutex::new(None),
        }
    }

    pub async fn ensure_page(&self, headless: bool) -> Result<Arc<Page>> {
        let mut lock = self.instance.lock();
        if lock.is_none() {
            let browser = NativeBrowser::new(headless).await?;
            *lock = Some(Arc::new(browser));
        }
        Ok(lock.as_ref().unwrap().page.clone())
    }

    pub async fn human_click(&self, selector: &str) -> Result<()> {
        let page = self.ensure_page(false).await?;
        
        // Get coordinates via JS since box_model is private
        let js = format!(r#"
            (() => {{
                const el = document.querySelector("{}");
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.left + r.width/2, y: r.top + r.height/2 }};
            }})()
        "#, selector);
        
        let result = page.evaluate(js).await?;
        if let Some(coords) = result.value() {
            let x = coords["x"].as_f64().unwrap_or(0.0);
            let y = coords["y"].as_f64().unwrap_or(0.0);
            
            page.move_mouse(Point { x, y }).await?;
            tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(50..150))).await;
            page.find_element(selector).await?.click().await?;
        }
        
        Ok(())
    }

    pub async fn human_type(&self, selector: &str, text: &str) -> Result<()> {
        let page = self.ensure_page(false).await?;
        let element = page.find_element(selector).await?;
        element.click().await?;
        
        let mut rng = rand::thread_rng();
        for c in text.chars() {
            element.type_str(c.to_string()).await?;
            tokio::time::sleep(Duration::from_millis(rng.gen_range(30..120))).await;
        }
        Ok(())
    }

    pub async fn get_ax_tree(&self) -> Result<serde_json::Value> {
        let page = self.ensure_page(false).await?;
        let result = page.evaluate(AX_TREE_EXTRACTOR_JS).await?;
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Array(vec![])))
    }

    pub async fn get_content(&self) -> Result<String> {
        let page = self.ensure_page(false).await?;
        let content = page.content().await?;
        Ok(content)
    }

    pub async fn press_key(&self, key: &str) -> Result<()> {
        let page = self.ensure_page(false).await?;
        page.evaluate(format!("document.dispatchEvent(new KeyboardEvent('keydown', {{key: '{}'}}))", key)).await?;
        Ok(())
    }

    pub async fn scroll(&self, x: i32, y: i32) -> Result<()> {
        let page = self.ensure_page(false).await?;
        page.evaluate(format!("window.scrollBy({}, {})", x, y)).await?;
        Ok(())
    }

    pub async fn capture_screenshot_base64(&self) -> Result<String> {
        use base64::Engine;
        let page = self.ensure_page(false).await?;
        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(80)
            .build();
        let data = page.screenshot(params).await?;
        Ok(base64::engine::general_purpose::STANDARD.encode(data))
    }

    pub async fn capture_screenshot_file(&self) -> Result<String> {
        let page = self.ensure_page(false).await?;
        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(80)
            .build();
        let data = page.screenshot(params).await?;

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        let tmp_dir = std::path::PathBuf::from(home).join(".sunday").join("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir_all(&tmp_dir)?;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let filename = format!("screenshot_{}.jpg", timestamp);
        let file_path = tmp_dir.join(&filename);
        std::fs::write(&file_path, data)?;

        Ok(file_path.to_string_lossy().into_owned())
    }

    pub async fn capture_screenshot_shared(&self, _name: &str) -> Result<usize> {
        let page = self.ensure_page(false).await?;
        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(80)
            .build();
        let data = page.screenshot(params).await?;
        Ok(data.len())
    }
}
