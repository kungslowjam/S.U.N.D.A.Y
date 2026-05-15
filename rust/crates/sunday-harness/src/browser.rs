//! Browser E2E test support using chromiumoxide.

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams;
use futures::stream::StreamExt;
use std::path::PathBuf;

/// Browser automation helper for harness E2E tests.
pub struct HarnessBrowser {
    browser: Browser,
}

impl HarnessBrowser {
    /// Launch a headless browser instance.
    pub async fn launch() -> Result<Self, Box<dyn std::error::Error>> {
        let config = BrowserConfig::builder()
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-gpu")
            .build()?;

        let (browser, mut handler) = Browser::launch(config).await?;

        // Spawn handler task
        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Self { browser })
    }

    /// Navigate to a URL and return the page content.
    pub async fn navigate(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let page = self.browser.new_page(url).await?;
        let content = page.content().await?;
        Ok(content)
    }

    /// Click an element by selector.
    pub async fn click(&self, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.browser.pages().await?.into_iter().next()
            .ok_or("No active page")?;
        page.find_element(selector).await?.click().await?;
        Ok(())
    }

    /// Type text into an element.
    pub async fn type_text(
        &self,
        selector: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.browser.pages().await?.into_iter().next()
            .ok_or("No active page")?;
        let element = page.find_element(selector).await?;
        element.click().await?;
        element.type_str(text).await?;
        Ok(())
    }

    /// Capture a screenshot and save to the given path.
    pub async fn screenshot(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.browser.pages().await?.into_iter().next()
            .ok_or("No active page")?;
        let params = CaptureScreenshotParams::default();
        let data = page.screenshot(params).await?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    /// Evaluate JavaScript on the page.
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let page = self.browser.pages().await?.into_iter().next()
            .ok_or("No active page")?;
        let result = page.evaluate(script).await?;
        Ok(result.into_value()?)
    }

    /// Wait for an element to appear.
    pub async fn wait_for(&self, selector: &str, timeout_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        let page = self.browser.pages().await?.into_iter().next()
            .ok_or("No active page")?;
        page.wait_for_navigation().await?;
        // Simple polling approach
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if page.find_element(selector).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err("Timeout waiting for element".into())
    }

    /// Close the browser.
    pub async fn close(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.browser.close().await?;
        Ok(())
    }
}
