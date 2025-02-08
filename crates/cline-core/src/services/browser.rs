use anyhow::Result;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use html2md::parse_html;
use std::fmt;
use std::sync::Arc;

pub struct BrowserSession {
    browser: Option<Browser>,
    tab: Option<Arc<Tab>>,
}

impl fmt::Debug for BrowserSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserSession")
            .field("browser", &self.browser.is_some())
            .field("tab", &self.tab.is_some())
            .finish()
    }
}

impl BrowserSession {
    pub fn new() -> Self {
        Self {
            browser: None,
            tab: None,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.browser.is_some() && self.tab.is_some()
    }

    pub async fn launch_browser(&mut self) -> Result<()> {
        let options = LaunchOptionsBuilder::default()
            .headless(true)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build browser options: {}", e))?;

        let browser = Browser::new(options)
            .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;

        let tab = browser
            .wait_for_initial_tab()
            .map_err(|e| anyhow::anyhow!("Failed to get initial tab: {}", e))?;

        self.browser = Some(browser);
        self.tab = Some(tab);
        Ok(())
    }

    pub async fn close_browser(&mut self) -> Result<()> {
        self.browser = None;
        self.tab = None;
        Ok(())
    }

    pub async fn url_to_markdown(&self, url: &str) -> Result<String> {
        let tab = self
            .tab
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Browser not initialized"))?;

        // ページに移動して読み込み完了を待つ
        tab.navigate_to(url)
            .map_err(|e| anyhow::anyhow!("Failed to navigate to URL: {}", e))?;
        tab.wait_until_navigated()
            .map_err(|e| anyhow::anyhow!("Failed to wait for navigation: {}", e))?;

        // DOMが完全に読み込まれるまで待機
        tab.wait_for_element("body")
            .map_err(|e| anyhow::anyhow!("Failed to wait for body element: {}", e))?;

        // HTMLコンテンツを取得
        let content = tab
            .get_content()
            .map_err(|e| anyhow::anyhow!("Failed to get page content: {}", e))?;

        // HTMLをMarkdownに変換
        let markdown = parse_html(&content);
        Ok(markdown)
    }
}
