use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::page::Page;
use chromiumoxide::layout::Point;
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};
use rand::Rng;
use std::time::Duration;
use serde_json;
#[cfg(feature = "vision")]
use sunday_vision::VisionEngine;
use sunday_core::SharedMemorySegment;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::browser_native_js::*;

pub struct BrowserController {
    browser: Browser,
    handle: tokio::task::JoinHandle<()>,
}

impl BrowserController {
    pub async fn launch(headless: bool) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = BrowserConfig::builder()
            .viewport(Viewport {
                width: 1280,
                height: 720,
                ..Default::default()
            })
            .args(vec![
                "--disable-blink-features=AutomationControlled",
                "--no-sandbox",
                "--disable-infobars",
                "--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            ]);
        
        let config = if headless {
            config.build()?
        } else {
            config.with_head().build()?
        };

        let (browser, mut handler) = Browser::launch(config).await?;

        // Spawn handler in background
        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(e) = h {
                    error!("Browser handler error: {}", e);
                    break;
                }
            }
        });

        Ok(Self { browser, handle })
    }

    pub async fn new_page(&self) -> Result<Page, Box<dyn std::error::Error + Send + Sync>> {
        let page = self.browser.new_page("about:blank").await?;
        
        // Inject JARVIS and Visuals (CDP way)
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(JARVIS_SCRIPT.to_string())).await?;
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(VISUAL_CURSOR_SCRIPT.to_string())).await?;
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(AX_TREE_SCRIPT.to_string())).await?;
        
        // Spoof navigator.webdriver
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new("Object.defineProperty(navigator, 'webdriver', {get: () => undefined})".to_string())).await?;

        Ok(page)
    }

    pub async fn close(mut self) {
        let _ = self.browser.close().await;
        self.handle.abort();
    }
}

pub struct NativeBrowserSession {
    controller: Arc<Mutex<Option<BrowserController>>>,
    current_page: Arc<Mutex<Option<Page>>>,
}

impl NativeBrowserSession {
    pub fn new() -> Self {
        Self {
            controller: Arc::new(Mutex::new(None)),
            current_page: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn ensure_page(&self, headless: bool) -> Result<Page, Box<dyn std::error::Error + Send + Sync>> {
        let mut ctrl_lock = self.controller.lock().await;
        if ctrl_lock.is_none() {
            info!("Launching native Rust browser...");
            *ctrl_lock = Some(BrowserController::launch(headless).await?);
        }

        let mut page_lock = self.current_page.lock().await;
        if page_lock.is_none() {
            if let Some(ctrl) = ctrl_lock.as_ref() {
                *page_lock = Some(ctrl.new_page().await?);
            }
        }

        Ok(page_lock.as_ref().unwrap().clone())
    }

    /// Move mouse in a human-like way with randomized jitter and curves
    pub async fn move_mouse_human(&self, target_x: f64, target_y: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        
        let steps = 10;
        let mut rng = rand::thread_rng();
        
        for i in 1..=steps {
            let progress = i as f64 / steps as f64;
            let jitter_x = rng.gen_range(-2.0..2.0);
            let jitter_y = rng.gen_range(-2.0..2.0);
            
            let cur_x = target_x * progress;
            let cur_y = target_y * progress;

            // We also update the visual cursor
            let _ = page.evaluate(format!("window.__move_cursor({}, {})", cur_x, cur_y)).await;
            
            page.move_mouse(Point { x: cur_x + jitter_x, y: cur_y + jitter_y }).await?;
            tokio::time::sleep(Duration::from_millis(rng.gen_range(5..15))).await;
        }
        
        // Final precise move
        page.move_mouse(Point { x: target_x, y: target_y }).await?;
        let _ = page.evaluate(format!("window.__move_cursor({}, {})", target_x, target_y)).await;
        
        Ok(())
    }

    pub async fn get_ax_tree(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        let tree = page.evaluate("window.__get_ax_tree()").await?.into_value()?;
        Ok(tree)
    }

    pub async fn capture_screenshot(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        let screenshot = page.screenshot(chromiumoxide::page::ScreenshotParams::builder().full_page(false).build()).await?;
        Ok(screenshot)
    }

    pub async fn capture_screenshot_base64(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let screenshot = self.capture_screenshot().await?;
        Ok(BASE64.encode(screenshot))
    }

    pub async fn get_content(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        let content = page.content().await?;
        Ok(content)
    }

    pub async fn press_key(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        // Use JS evaluation for keyboard events to bypass library version issues
        page.evaluate(format!(
            "window.dispatchEvent(new KeyboardEvent('keydown', {{'key': '{}', 'bubbles': true}})); \
             window.dispatchEvent(new KeyboardEvent('keyup', {{'key': '{}', 'bubbles': true}}));",
            key, key
        )).await?;
        Ok(())
    }

    pub async fn scroll(&self, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page = self.current_page.lock().await.as_ref().ok_or("No active page")?.clone();
        page.evaluate(format!("window.scrollBy({}, {})", x, y)).await?;
        Ok(())
    }

    /// Perform vision analysis on the current screen
    #[cfg(feature = "vision")]
    pub async fn vision_analyze(&self, model_path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let screenshot = self.capture_screenshot().await?;
        let mut engine = VisionEngine::new(model_path)?;
        let results = engine.detect_objects(&screenshot)?;
        Ok(results)
    }

    /// Capture screenshot and write it to shared memory for Zero-latency bridge
    pub async fn capture_screenshot_shared(&self, buffer_name: &str) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let screenshot = self.capture_screenshot().await?;
        let size = screenshot.len();
        
        // Ensure shared memory segment exists
        let buffer = SharedMemorySegment::new(buffer_name);
        buffer.write(&screenshot)?;
        
        Ok(size)
    }
}
