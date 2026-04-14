"""
Xiaohongshu browser automation client.

Provides capabilities to search notes, publish content (images/videos),
interact with posts (like, collect, comment, follow), and extract data.
"""

from __future__ import annotations

import asyncio
import json
import random
import re
from pathlib import Path
from typing import Optional

from loguru import logger
from playwright.async_api import Browser, BrowserContext, Page, async_playwright

from xiaohongshu.config import XHSConfig
from xiaohongshu.models import (
    CommentResult,
    InteractionResult,
    NoteImage,
    NoteResult,
    NoteVideo,
    PublishResult,
    SearchResult,
    UserProfile,
)

BASE_URL = "https://www.xiaohongshu.com"
EXPLORE_URL = f"{BASE_URL}/explore"
SEARCH_URL = f"{BASE_URL}/search_result"
PUBLISH_URL = "https://creator.xiaohongshu.com/publish/publish"


class XHSClient:
    """
    Xiaohongshu client using Playwright for browser automation.

    Supports:
    - Searching notes by keyword
    - Fetching note details
    - Publishing image/video notes
    - Liking, collecting, commenting on notes
    - Following/unfollowing users
    - Extracting user profiles
    """

    def __init__(self, config: Optional[XHSConfig] = None):
        self.config = config or XHSConfig.from_env()
        self._playwright = None
        self._browser: Optional[Browser] = None
        self._context: Optional[BrowserContext] = None
        self._page: Optional[Page] = None
        self._initialized = False

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    async def start(self) -> None:
        """Launch browser and initialize context with cookies."""
        if self._initialized:
            return

        logger.info("Starting Xiaohongshu client...")
        self._playwright = await async_playwright().start()

        launch_kwargs: dict = {"headless": self.config.browser.headless}
        if self.config.proxy.server:
            launch_kwargs["proxy"] = {
                "server": self.config.proxy.server,
                "username": self.config.proxy.username,
                "password": self.config.proxy.password,
            }

        launcher = getattr(self._playwright, self.config.browser.browser_type)
        self._browser = await launcher.launch(**launch_kwargs)

        self._context = await self._browser.new_context(
            user_agent=self.config.browser.user_agent,
            viewport={"width": 1920, "height": 1080},
        )

        # Try to load saved cookies first, then fall back to cookie string
        cookie_loaded = await self._load_cookies_from_file()
        if not cookie_loaded and self.config.cookie:
            cookies = self._parse_cookies(self.config.cookie)
            await self._context.add_cookies(cookies)

        self._page = await self._context.new_page()
        self._initialized = True
        logger.info("Xiaohongshu client started successfully.")

    async def stop(self) -> None:
        """Close browser and clean up resources."""
        if self._browser:
            await self._browser.close()
        if self._playwright:
            await self._playwright.stop()
        self._initialized = False
        logger.info("Xiaohongshu client stopped.")

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.stop()

    # ------------------------------------------------------------------
    # Login & Cookie Management
    # ------------------------------------------------------------------

    async def login_by_qrcode(self, timeout: int = 120) -> bool:
        """
        Login to Xiaohongshu by scanning QR code.

        Opens the login page in a visible browser window and waits for the user
        to scan the QR code with the Xiaohongshu mobile app.

        Args:
            timeout: Maximum seconds to wait for login completion.

        Returns:
            True if login was successful, False otherwise.
        """
        await self._ensure_started()
        page = self._page
        logger.info("Opening Xiaohongshu login page for QR code scanning...")

        await page.goto(f"{BASE_URL}/explore", wait_until="networkidle")
        await self._random_delay(1, 2)

        # Check if already logged in
        if await self._check_login_status():
            logger.info("Already logged in!")
            await self._save_cookies_to_file()
            return True

        # Click login button to trigger QR code display
        login_btn = await page.query_selector(
            'div.login-btn, '
            'button:has-text("登录"), '
            'a:has-text("登录"), '
            'div[class*="login"] span, '
            'span:has-text("登录")'
        )
        if login_btn:
            await login_btn.click()
            await self._random_delay(1, 3)
            logger.info("Login dialog opened. Looking for QR code...")

        # Try to switch to QR code login tab if not already showing
        qr_tab = await page.query_selector(
            'div:has-text("扫码登录"), '
            'span:has-text("扫码登录"), '
            'div[class*="qrcode-tab"], '
            'div[class*="qr-tab"]'
        )
        if qr_tab:
            await qr_tab.click()
            await self._random_delay(1, 2)

        # Take a screenshot so the user can see the QR code even in headless mode
        screenshot_path = await self.take_screenshot("login_qrcode")
        logger.info(f"📱 QR code screenshot saved to: {screenshot_path}")
        logger.info(f"⏳ Waiting up to {timeout}s for you to scan the QR code with Xiaohongshu app...")

        # Wait for login to complete by polling login status
        start_time = asyncio.get_event_loop().time()
        while (asyncio.get_event_loop().time() - start_time) < timeout:
            await asyncio.sleep(3)

            if await self._check_login_status():
                logger.info("✅ Login successful!")
                await self._save_cookies_to_file()
                return True

            # Check if QR code expired and needs refresh
            expired = await page.query_selector(
                'div:has-text("二维码已过期"), '
                'span:has-text("已过期"), '
                'div[class*="expired"]'
            )
            if expired:
                logger.warning("QR code expired. Refreshing...")
                refresh_btn = await page.query_selector(
                    'div:has-text("点击刷新"), '
                    'button:has-text("刷新"), '
                    'div[class*="refresh"]'
                )
                if refresh_btn:
                    await refresh_btn.click()
                    await self._random_delay(1, 2)
                    screenshot_path = await self.take_screenshot("login_qrcode_refreshed")
                    logger.info(f"📱 Refreshed QR code screenshot: {screenshot_path}")

        logger.error("❌ Login timed out. QR code was not scanned in time.")
        return False

    async def login_by_cookie(self, cookie_str: Optional[str] = None) -> bool:
        """
        Login using a cookie string.

        Args:
            cookie_str: Cookie string from browser. If None, uses config value.

        Returns:
            True if login was successful (cookie is valid).
        """
        await self._ensure_started()

        cookie = cookie_str or self.config.cookie
        if not cookie:
            logger.error("No cookie string provided.")
            return False

        cookies = self._parse_cookies(cookie)
        await self._context.add_cookies(cookies)
        logger.info("Cookies injected. Verifying login status...")

        # Navigate to verify the cookies work
        await self._page.goto(f"{BASE_URL}/explore", wait_until="networkidle")
        await self._random_delay(1, 2)

        if await self._check_login_status():
            logger.info("✅ Cookie login successful!")
            await self._save_cookies_to_file()
            return True

        logger.error("❌ Cookie login failed - cookies may be invalid or expired.")
        return False

    async def check_login(self) -> bool:
        """
        Check if the current session is logged in.

        Returns:
            True if logged in, False otherwise.
        """
        await self._ensure_started()
        await self._page.goto(f"{BASE_URL}/explore", wait_until="networkidle")
        await self._random_delay(1, 2)
        is_logged_in = await self._check_login_status()
        logger.info(f"Login status: {'✅ Logged in' if is_logged_in else '❌ Not logged in'}")
        if is_logged_in:
            await self._save_cookies_to_file()
        return is_logged_in

    async def _check_login_status(self) -> bool:
        """Check if user is currently logged in by looking for user-specific elements."""
        page = self._page
        try:
            # Check for user avatar or profile elements that only appear when logged in
            logged_in_indicators = [
                'img[class*="avatar"][class*="user"]',
                'div[class*="user-info"]',
                'a[href*="/user/profile/"]',
                'span[class*="user-name"]',
                'div[class*="sidebar"] img[class*="avatar"]',
                'li[class*="user"] img',
            ]
            for selector in logged_in_indicators:
                el = await page.query_selector(selector)
                if el:
                    return True

            # Also check if login button is NOT present (means we are logged in)
            login_btn = await page.query_selector(
                'div.login-btn:has-text("登录"), '
                'button:has-text("登录")'
            )
            # If no login button found and page is loaded, likely logged in
            # But we need at least one positive indicator, so return False
            if login_btn:
                return False

            # Check URL - if redirected to login page, not logged in
            current_url = page.url
            if "login" in current_url.lower():
                return False

            # Check by trying to access creator center
            return False
        except Exception as e:
            logger.debug(f"Login status check error: {e}")
            return False

    async def _handle_creator_login(self, page: Page) -> bool:
        """
        Handle login on the creator center (creator.xiaohongshu.com).

        The creator center uses a separate auth flow. This method tries:
        1. QR code scan login on creator center
        2. Waiting for SSO redirect from main site cookies

        Returns:
            True if login succeeded.
        """
        logger.info("Handling creator center login...")
        await self.take_screenshot("creator_login_page")

        # The creator login page usually shows a QR code or phone login
        # Wait for user to scan QR code or for SSO redirect
        logger.info("━" * 50)
        logger.info("  📱 Creator center requires separate login!")
        logger.info("  Please scan the QR code in the browser window")
        logger.info("  with your Xiaohongshu app.")
        logger.info("  ⏳ Waiting up to 120 seconds...")
        logger.info("━" * 50)

        # Also try clicking QR code login tab if available
        qr_tab = await page.query_selector(
            'div:has-text("扫码登录"), '
            'span:has-text("扫码登录"), '
            'div[class*="qrcode"], '
            'div[class*="qr-login"]'
        )
        if qr_tab:
            await qr_tab.click()
            await self._random_delay(1, 2)

        await self.take_screenshot("creator_qrcode")

        # Poll for successful login (URL changes away from login page)
        start_time = asyncio.get_event_loop().time()
        timeout = 120
        while (asyncio.get_event_loop().time() - start_time) < timeout:
            await asyncio.sleep(3)
            try:
                current_url = page.url
                if "login" not in current_url.lower():
                    logger.info("✅ Creator center login successful!")
                    # Save the new cookies including creator domain
                    await self._save_cookies_to_file()
                    return True

                # Check if QR code expired
                expired = await page.query_selector(
                    'div:has-text("二维码已过期"), '
                    'span:has-text("已过期"), '
                    'div:has-text("已失效")'
                )
                if expired:
                    logger.warning("QR code expired, refreshing...")
                    refresh_btn = await page.query_selector(
                        'div:has-text("点击刷新"), '
                        'button:has-text("刷新"), '
                        'div[class*="refresh"]'
                    )
                    if refresh_btn:
                        await refresh_btn.click()
                        await self._random_delay(1, 2)
                        await self.take_screenshot("creator_qrcode_refreshed")
            except Exception as e:
                # Navigation may destroy execution context - this likely means
                # the page redirected after successful login
                logger.debug(f"Polling interrupted (likely navigation): {e}")
                await asyncio.sleep(2)
                try:
                    current_url = page.url
                    if "login" not in current_url.lower():
                        logger.info("✅ Creator center login successful (detected via navigation)!")
                        await self._save_cookies_to_file()
                        return True
                except Exception:
                    pass

        logger.error("❌ Creator center login timed out.")
        return False

    async def _save_cookies_to_file(self) -> None:
        """Save current browser cookies to a JSON file for later reuse."""
        try:
            cookies = await self._context.cookies()
            cookie_file = self.config.storage.cookie_file
            cookie_file.parent.mkdir(parents=True, exist_ok=True)
            with open(cookie_file, "w", encoding="utf-8") as f:
                json.dump(cookies, f, ensure_ascii=False, indent=2)
            logger.info(f"🍪 Cookies saved to {cookie_file} ({len(cookies)} cookies)")
        except Exception as e:
            logger.error(f"Failed to save cookies: {e}")

    async def _load_cookies_from_file(self) -> bool:
        """Load cookies from a JSON file if it exists."""
        try:
            cookie_file = self.config.storage.cookie_file
            if not cookie_file.exists():
                logger.debug(f"Cookie file not found: {cookie_file}")
                return False

            with open(cookie_file, "r", encoding="utf-8") as f:
                cookies = json.load(f)

            if cookies:
                await self._context.add_cookies(cookies)
                logger.info(f"🍪 Loaded {len(cookies)} cookies from {cookie_file}")
                return True
            return False
        except Exception as e:
            logger.warning(f"Failed to load cookies from file: {e}")
            return False

    # ------------------------------------------------------------------
    # Search
    # ------------------------------------------------------------------

    async def search_notes(
        self,
        keyword: str,
        *,
        sort: str = "general",
        note_type: str = "all",
        limit: int = 20,
    ) -> SearchResult:
        """
        Search notes by keyword.

        Args:
            keyword: Search keyword.
            sort: Sort type - "general" (comprehensive), "time_descending" (latest),
                  "popularity_descending" (most popular).
            note_type: Filter type - "all", "image", "video".
            limit: Maximum number of results to return.

        Returns:
            SearchResult with matching notes.
        """
        await self._ensure_started()
        logger.info(f"Searching notes for keyword: '{keyword}', sort={sort}, type={note_type}")

        page = self._page
        search_params = f"?keyword={keyword}&source=web_search_result_note"
        if sort != "general":
            search_params += f"&sort={sort}"
        if note_type != "all":
            search_params += f"&type={note_type}"

        await page.goto(f"{SEARCH_URL}{search_params}", wait_until="networkidle")
        await self._random_delay()

        notes: list[NoteResult] = []
        seen_ids: set[str] = set()

        # Scroll and collect notes
        scroll_count = 0
        max_scrolls = max(limit // 5, 3)

        while len(notes) < limit and scroll_count < max_scrolls:
            cards = await page.query_selector_all('section.note-item, div[class*="note-item"]')

            for card in cards:
                if len(notes) >= limit:
                    break
                try:
                    note = await self._parse_note_card(card)
                    if note and note.note_id not in seen_ids:
                        seen_ids.add(note.note_id)
                        notes.append(note)
                except Exception as e:
                    logger.debug(f"Failed to parse note card: {e}")

            # Scroll down for more results
            await page.evaluate("window.scrollBy(0, 800)")
            await self._random_delay()
            scroll_count += 1

        logger.info(f"Found {len(notes)} notes for '{keyword}'")
        return SearchResult(
            keyword=keyword,
            notes=notes[:limit],
            total=len(notes),
            has_more=len(notes) >= limit,
        )

    # ------------------------------------------------------------------
    # Note Details
    # ------------------------------------------------------------------

    async def get_note_detail(self, note_id: str) -> NoteResult:
        """
        Fetch detailed information of a specific note.

        Args:
            note_id: The note ID to fetch.

        Returns:
            NoteResult with full note details.
        """
        await self._ensure_started()
        logger.info(f"Fetching note detail: {note_id}")

        page = self._page
        note_url = f"{EXPLORE_URL}/{note_id}"
        await page.goto(note_url, wait_until="networkidle")
        await self._random_delay()

        # Extract note data from the page
        title = await self._safe_text(page, 'div.title, h1[class*="title"], #detail-title')
        content = await self._safe_text(page, 'div.content, div.desc, #detail-desc, div[class*="note-text"]')

        # Extract images
        images: list[NoteImage] = []
        img_elements = await page.query_selector_all(
            'div.swiper-slide img, div[class*="slide"] img, div.note-image img'
        )
        for img in img_elements:
            src = await img.get_attribute("src")
            if src:
                images.append(NoteImage(url=src))

        # Extract video if present
        video: Optional[NoteVideo] = None
        video_el = await page.query_selector("video source, video")
        if video_el:
            video_src = await video_el.get_attribute("src")
            if video_src:
                video = NoteVideo(url=video_src)

        # Extract tags
        tags: list[str] = []
        tag_elements = await page.query_selector_all('a.tag, a[class*="tag"], span[class*="hashtag"]')
        for tag_el in tag_elements:
            tag_text = await tag_el.inner_text()
            if tag_text:
                tags.append(tag_text.strip().lstrip("#"))

        # Extract engagement counts
        liked_count = await self._extract_count(page, '[class*="like"] span, .like-count')
        collected_count = await self._extract_count(page, '[class*="collect"] span, .collect-count')
        comment_count = await self._extract_count(page, '[class*="chat"] span, .comment-count')

        # Extract author info
        author = await self._extract_author(page)

        return NoteResult(
            note_id=note_id,
            title=title,
            content=content,
            author=author,
            images=images,
            video=video,
            tags=tags,
            liked_count=liked_count,
            collected_count=collected_count,
            comment_count=comment_count,
            url=note_url,
        )

    # ------------------------------------------------------------------
    # Publish
    # ------------------------------------------------------------------

    async def publish_image_note(
        self,
        title: str,
        content: str,
        image_paths: list[str | Path],
        *,
        tags: Optional[list[str]] = None,
    ) -> PublishResult:
        """
        Publish an image note to Xiaohongshu.

        Args:
            title: Note title.
            content: Note text content.
            image_paths: List of local image file paths to upload.
            tags: Optional list of hashtags.

        Returns:
            PublishResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Publishing image note: '{title}' with {len(image_paths)} images")

        page = self._page
        try:
            # Navigate to creator publish page with increased timeout
            await page.goto(PUBLISH_URL, wait_until="domcontentloaded", timeout=60000)
            await self._random_delay(3, 5)

            try:
                await page.wait_for_load_state("networkidle", timeout=15000)
            except Exception:
                logger.debug("networkidle timeout on publish page, continuing anyway...")
                await self._random_delay(2, 4)

            # Check if we were redirected to creator login page
            current_url = page.url
            logger.debug(f"Current URL after navigation: {current_url}")

            if "login" in current_url.lower():
                logger.info("Creator center requires login, attempting SSO login...")
                login_success = await self._handle_creator_login(page)
                if not login_success:
                    await self.take_screenshot("creator_login_failed")
                    return PublishResult(
                        success=False,
                        message="Failed to login to creator center. Please login manually first."
                    )
                # Navigate to publish page again after login
                await page.goto(PUBLISH_URL, wait_until="domcontentloaded", timeout=60000)
                await self._random_delay(3, 5)
                try:
                    await page.wait_for_load_state("networkidle", timeout=15000)
                except Exception:
                    await self._random_delay(2, 4)

            # Debug screenshot
            await self.take_screenshot("publish_page_ready")
            logger.debug(f"Publish page URL: {page.url}")

            # Switch to "上传图文" (upload image/text) tab
            # The creator center defaults to "上传视频" tab, we need "上传图文"
            image_tab_switched = False
            try:
                # Use JavaScript to find and click the image tab (avoids viewport issues)
                image_tab_switched = await page.evaluate("""() => {
                    const tabs = document.querySelectorAll('.creator-tab');
                    for (const tab of tabs) {
                        const text = tab.textContent.trim();
                        if (text.includes('图文') && !tab.classList.contains('active')) {
                            tab.scrollIntoView({block: 'center'});
                            tab.click();
                            return true;
                        }
                    }
                    // Fallback: try matching by span.title inside tabs
                    const titles = document.querySelectorAll('.header-tabs .creator-tab .title');
                    for (const title of titles) {
                        if (title.textContent.includes('图文')) {
                            const tab = title.closest('.creator-tab');
                            if (tab && !tab.classList.contains('active')) {
                                tab.scrollIntoView({block: 'center'});
                                tab.click();
                                return true;
                            }
                        }
                    }
                    return false;
                }""")
                if image_tab_switched:
                    logger.info("Switched to '上传图文' tab via JavaScript click")
                    await self._random_delay(2, 4)
                else:
                    logger.debug("JavaScript tab switch returned false, trying Playwright click...")
                    # Fallback: use Playwright force click
                    tab_el = await page.query_selector(
                        '.creator-tab:has-text("上传图文"):not(.active)'
                    )
                    if tab_el:
                        await tab_el.click(force=True)
                        image_tab_switched = True
                        logger.info("Switched to '上传图文' tab via force click")
                        await self._random_delay(2, 4)
            except Exception as e:
                logger.warning(f"Failed to switch to image tab: {e}")

            if image_tab_switched:
                # Wait for the image upload area to render
                await self._random_delay(1, 2)
                await self.take_screenshot("image_tab_active")
            else:
                logger.warning("Could not find '上传图文' tab, trying to upload with current tab")

            # Upload images - file input might be hidden, use set_input_files directly
            # After tab switch, look for the image file input (not video)
            file_input = None

            # Strategy 1: Find file input that accepts image formats
            try:
                all_inputs = await page.query_selector_all('input[type="file"]')
                for inp in all_inputs:
                    accept = await inp.get_attribute("accept") or ""
                    if any(ext in accept.lower() for ext in [".jpg", ".jpeg", ".png", ".gif", ".webp", "image"]):
                        file_input = inp
                        logger.info(f"Found image file input with accept='{accept}'")
                        break
                # If no image-specific input found, use the first available
                if not file_input and all_inputs:
                    accept = await all_inputs[0].get_attribute("accept") or ""
                    logger.debug(f"Using first file input with accept='{accept}'")
                    file_input = all_inputs[0]
            except Exception:
                logger.debug("Could not enumerate file inputs")

            # Strategy 2: Wait for a new file input to appear after tab switch
            if not file_input or (await file_input.get_attribute("accept") or "").startswith(".mp4"):
                logger.debug("No image file input yet, waiting for tab content to load...")
                try:
                    file_input = await page.wait_for_selector(
                        'input[type="file"]', timeout=8000, state="attached"
                    )
                    accept = await file_input.get_attribute("accept") or ""
                    logger.debug(f"Found file input after wait, accept='{accept}'")
                    # Re-check: if this is still a video input, search again
                    if accept.startswith(".mp4"):
                        all_inputs = await page.query_selector_all('input[type="file"]')
                        for inp in all_inputs:
                            a = await inp.get_attribute("accept") or ""
                            if not a.startswith(".mp4"):
                                file_input = inp
                                logger.info(f"Found non-video file input with accept='{a}'")
                                break
                except Exception:
                    logger.debug("Timeout waiting for file input")

            # Strategy 2: Try locating by upload area click + hidden input
            if not file_input:
                try:
                    # Click the upload area to potentially trigger file input
                    upload_area = await page.query_selector(
                        'div[class*="upload"], '
                        'div[class*="drag"], '
                        'div:has-text("拖拽或点击上传"), '
                        'div:has-text("上传图片"), '
                        'div[class*="creator-tab"] div[class*="container"], '
                        'div[class*="publish-upload"]'
                    )
                    if upload_area:
                        logger.debug("Found upload area, looking for hidden file input...")
                    # Try again to find the input (it might be dynamically created)
                    file_input = await page.query_selector('input[type="file"]')
                except Exception:
                    logger.debug("Upload area strategy failed")

            # Strategy 3: Use JavaScript to find hidden file inputs
            if not file_input:
                try:
                    has_input = await page.evaluate(
                        'document.querySelectorAll("input[type=file]").length'
                    )
                    logger.debug(f"Found {has_input} file input(s) via JS")
                    if has_input > 0:
                        file_input = await page.query_selector('input[type="file"]')
                except Exception:
                    pass

            if file_input:
                paths = [str(Path(p).resolve()) for p in image_paths]
                await file_input.set_input_files(paths)
                logger.info(f"Uploaded {len(paths)} images")

                # Wait for upload processing and edit area to appear
                # The creator center renders the title/content editor AFTER image is processed
                logger.info("Waiting for image processing and editor to appear...")
                editor_ready = False
                for wait_attempt in range(10):  # Max ~30 seconds
                    await asyncio.sleep(3)
                    # Check if editor elements have appeared
                    dom_info = await page.evaluate("""() => {
                        const info = {};
                        info.inputs = Array.from(document.querySelectorAll('input')).map(el => ({
                            type: el.type, placeholder: el.placeholder,
                            class: el.className, id: el.id, visible: el.offsetParent !== null,
                            name: el.name
                        }));
                        info.textareas = Array.from(document.querySelectorAll('textarea')).map(el => ({
                            placeholder: el.placeholder, class: el.className, id: el.id
                        }));
                        info.editables = Array.from(
                            document.querySelectorAll('[contenteditable="true"]')
                        ).map(el => ({
                            tag: el.tagName, class: el.className, id: el.id,
                            placeholder: el.getAttribute('data-placeholder') || el.getAttribute('placeholder') || ''
                        }));
                        // Also check for iframes that might contain the editor
                        info.iframes = Array.from(document.querySelectorAll('iframe')).map(el => ({
                            src: el.src, class: el.className, id: el.id
                        }));
                        // Check for shadow DOM elements
                        info.shadowHosts = [];
                        document.querySelectorAll('*').forEach(el => {
                            if (el.shadowRoot) {
                                info.shadowHosts.push({
                                    tag: el.tagName, class: el.className, id: el.id,
                                    childCount: el.shadowRoot.childElementCount
                                });
                            }
                        });
                        // Check for elements with specific data attributes used by editors
                        info.dataAttrs = Array.from(
                            document.querySelectorAll('[data-placeholder], [data-testid], [role="textbox"]')
                        ).map(el => ({
                            tag: el.tagName, class: el.className,
                            placeholder: el.getAttribute('data-placeholder') || '',
                            testid: el.getAttribute('data-testid') || '',
                            role: el.getAttribute('role') || ''
                        }));
                        info.totalInputs = info.inputs.length;
                        info.totalEditables = info.editables.length;
                        info.totalTextareas = info.textareas.length;
                        return info;
                    }""")
                    total = dom_info.get('totalInputs', 0) + dom_info.get('totalEditables', 0) + dom_info.get('totalTextareas', 0)
                    logger.debug(
                        f"Wait #{wait_attempt+1}: {dom_info.get('totalInputs', 0)} inputs, "
                        f"{dom_info.get('totalEditables', 0)} editables, "
                        f"{dom_info.get('totalTextareas', 0)} textareas, "
                        f"{len(dom_info.get('iframes', []))} iframes, "
                        f"{len(dom_info.get('shadowHosts', []))} shadow hosts, "
                        f"{len(dom_info.get('dataAttrs', []))} data-attr elements"
                    )
                    # Editor is ready when we have more than just the file input
                    if total > 1 or dom_info.get('totalEditables', 0) > 0 or dom_info.get('totalTextareas', 0) > 0:
                        editor_ready = True
                        logger.info("Editor area detected!")
                        logger.debug(f"Full DOM info: {json.dumps(dom_info, ensure_ascii=False, indent=2)}")
                        break
                    # Also check for shadow DOM editors or data-attribute based editors
                    if dom_info.get('shadowHosts') or dom_info.get('dataAttrs'):
                        editor_ready = True
                        logger.info("Editor detected via shadow DOM / data attributes!")
                        logger.debug(f"Shadow hosts: {json.dumps(dom_info.get('shadowHosts', []), ensure_ascii=False)}")
                        logger.debug(f"Data attrs: {json.dumps(dom_info.get('dataAttrs', []), ensure_ascii=False)}")
                        break

                if not editor_ready:
                    logger.warning("Editor did not appear after waiting. The upload may still be processing.")

                await self.take_screenshot("after_image_upload")
            else:
                logger.warning("Could not find file input, trying drag-and-drop upload area...")
                # Take a debug screenshot
                await self.take_screenshot("no_file_input_found")
                return PublishResult(
                    success=False,
                    message="Could not find file upload input on the publish page. "
                    "The page may require manual interaction or the selector needs updating."
                )

            # Fill in title - creator center uses various input elements
            title_input = None
            title_selectors = [
                'input[placeholder*="标题"]',
                'div[placeholder*="标题"]',
                '#title-input',
                'input[class*="title"]',
                'div[class*="title"] input',
                'div[class*="c-input_wrapper"] input',
                'input[maxlength]',
                'span[class*="title"] input',
            ]
            for sel in title_selectors:
                try:
                    title_input = await page.wait_for_selector(sel, timeout=3000)
                    if title_input:
                        logger.debug(f"Found title input with selector: {sel}")
                        break
                except Exception:
                    continue

            if title_input:
                await title_input.click()
                await self._random_delay(0.5, 1)
                await title_input.fill(title)
                logger.info(f"Title filled: {title}")
            else:
                logger.warning("Title input not found, trying keyboard input...")
                # Take debug screenshot
                await self.take_screenshot("title_input_not_found")
                # Try to use JS to find and fill the title
                try:
                    filled = await page.evaluate("""(title) => {
                        // Try various strategies to find the title input
                        let el = document.querySelector('input[placeholder*="标题"]');
                        if (!el) el = document.querySelector('input[class*="title"]');
                        if (!el) el = document.querySelector('.title input');
                        if (!el) {
                            // Find by iterating visible inputs
                            const inputs = document.querySelectorAll('input[type="text"], input:not([type])');
                            if (inputs.length > 0) el = inputs[0];
                        }
                        if (el) {
                            el.focus();
                            el.value = title;
                            el.dispatchEvent(new Event('input', {bubbles: true}));
                            el.dispatchEvent(new Event('change', {bubbles: true}));
                            return true;
                        }
                        return false;
                    }""", title)
                    if filled:
                        logger.info("Title filled via JavaScript")
                    else:
                        logger.warning("Could not find title input via any method")
                except Exception as e:
                    logger.warning(f"JS title fill failed: {e}")

            await self._random_delay(0.5, 1)

            # Fill in content - try multiple selectors
            content_input = None
            content_selectors = [
                '#post-textarea',
                'div[contenteditable="true"][class*="ql-editor"]',
                'div.ql-editor[contenteditable="true"]',
                'div[contenteditable="true"]',
                'textarea[placeholder*="正文"]',
                'textarea[placeholder*="输入"]',
                'div[class*="content"] div[contenteditable="true"]',
            ]
            for sel in content_selectors:
                try:
                    content_input = await page.wait_for_selector(sel, timeout=3000)
                    if content_input:
                        logger.debug(f"Found content input with selector: {sel}")
                        break
                except Exception:
                    continue

            if content_input:
                await content_input.click()
                await self._random_delay(0.5, 1)

                # Append tags to content
                full_content = content
                if tags:
                    tag_str = " ".join(f"#{t}" for t in tags)
                    full_content = f"{content}\n\n{tag_str}"

                # For contenteditable divs, use keyboard typing instead of fill
                tag_name = await content_input.evaluate("el => el.tagName.toLowerCase()")
                if tag_name in ("div", "p", "span"):
                    await content_input.evaluate(
                        "(el, text) => { el.innerHTML = text.replace(/\\n/g, '<br>'); }",
                        full_content,
                    )
                    # Trigger input event
                    await content_input.dispatch_event("input")
                else:
                    await content_input.fill(full_content)
                logger.info("Content filled")
            else:
                logger.warning("Content input not found")
                await self.take_screenshot("content_input_not_found")

            await self._random_delay(1, 2)
            await self.take_screenshot("before_publish_click")

            # Click publish button
            publish_btn = None
            publish_selectors = [
                'button:has-text("发布")',
                'button[class*="publish"]',
                'div[class*="publish"] button',
                'button.css-k0omnp',
                'button[class*="submit"]',
                'button.el-button--primary:has-text("发布")',
            ]
            for sel in publish_selectors:
                try:
                    publish_btn = await page.wait_for_selector(sel, timeout=3000)
                    if publish_btn:
                        logger.debug(f"Found publish button with selector: {sel}")
                        break
                except Exception:
                    continue

            if publish_btn:
                await publish_btn.click()
                await self._random_delay(3, 6)
                await self.take_screenshot("after_publish_click")
                logger.info("Publish button clicked")
            else:
                logger.warning("Publish button not found")
                await self.take_screenshot("publish_btn_not_found")

            logger.info("Image note published successfully")
            return PublishResult(success=True, message="Image note published successfully")

        except Exception as e:
            logger.error(f"Failed to publish image note: {e}")
            return PublishResult(success=False, message=f"Failed to publish: {e}")

    async def publish_video_note(
        self,
        title: str,
        content: str,
        video_path: str | Path,
        *,
        tags: Optional[list[str]] = None,
    ) -> PublishResult:
        """
        Publish a video note to Xiaohongshu.

        Args:
            title: Note title.
            content: Note text content.
            video_path: Local video file path to upload.
            tags: Optional list of hashtags.

        Returns:
            PublishResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Publishing video note: '{title}'")

        page = self._page
        try:
            await page.goto(PUBLISH_URL, wait_until="domcontentloaded", timeout=60000)
            await self._random_delay(3, 5)

            try:
                await page.wait_for_load_state("networkidle", timeout=15000)
            except Exception:
                logger.debug("networkidle timeout on publish page, continuing anyway...")
                await self._random_delay(2, 4)

            # Switch to video tab if needed
            video_tab = await page.query_selector('div:has-text("上传视频"), span:has-text("视频")')
            if video_tab:
                await video_tab.click()
                await self._random_delay()

            # Upload video
            file_input = await page.wait_for_selector(
                'input[type="file"]', timeout=10000
            )
            if file_input:
                await file_input.set_input_files(str(Path(video_path).resolve()))
                logger.info("Video uploaded, waiting for processing...")
                # Video processing takes longer
                await self._random_delay(10, 20)

            # Fill in title
            title_input = await page.wait_for_selector(
                'input[placeholder*="标题"], div[placeholder*="标题"], #title-input',
                timeout=10000,
            )
            if title_input:
                await title_input.click()
                await title_input.fill(title)

            # Fill in content
            content_input = await page.wait_for_selector(
                'div[contenteditable="true"], textarea[placeholder*="正文"], #post-textarea',
                timeout=10000,
            )
            if content_input:
                await content_input.click()
                full_content = content
                if tags:
                    tag_str = " ".join(f"#{t}" for t in tags)
                    full_content = f"{content}\n\n{tag_str}"
                await content_input.fill(full_content)

            await self._random_delay()

            # Click publish button
            publish_btn = await page.wait_for_selector(
                'button:has-text("发布"), button[class*="publish"]',
                timeout=10000,
            )
            if publish_btn:
                await publish_btn.click()
                await self._random_delay(3, 6)

            logger.info("Video note published successfully")
            return PublishResult(success=True, message="Video note published successfully")

        except Exception as e:
            logger.error(f"Failed to publish video note: {e}")
            return PublishResult(success=False, message=f"Failed to publish: {e}")

    # ------------------------------------------------------------------
    # Interactions
    # ------------------------------------------------------------------

    async def like_note(self, note_id: str) -> InteractionResult:
        """
        Like a note.

        Args:
            note_id: The note ID to like.

        Returns:
            InteractionResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Liking note: {note_id}")

        page = self._page
        try:
            await page.goto(f"{EXPLORE_URL}/{note_id}", wait_until="networkidle")
            await self._random_delay()

            like_btn = await page.wait_for_selector(
                'span[class*="like-wrapper"] span.like-icon, '
                'div[class*="like"] svg, '
                'button[class*="like"]',
                timeout=10000,
            )
            if like_btn:
                await like_btn.click()
                await self._random_delay()
                logger.info(f"Liked note {note_id}")
                return InteractionResult(
                    success=True, action="like", target_id=note_id, message="Note liked successfully"
                )

            return InteractionResult(
                success=False, action="like", target_id=note_id, message="Like button not found"
            )
        except Exception as e:
            logger.error(f"Failed to like note: {e}")
            return InteractionResult(
                success=False, action="like", target_id=note_id, message=f"Failed: {e}"
            )

    async def collect_note(self, note_id: str) -> InteractionResult:
        """
        Collect (bookmark) a note.

        Args:
            note_id: The note ID to collect.

        Returns:
            InteractionResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Collecting note: {note_id}")

        page = self._page
        try:
            await page.goto(f"{EXPLORE_URL}/{note_id}", wait_until="networkidle")
            await self._random_delay()

            collect_btn = await page.wait_for_selector(
                'span[class*="collect-wrapper"] span.collect-icon, '
                'div[class*="collect"] svg, '
                'button[class*="collect"]',
                timeout=10000,
            )
            if collect_btn:
                await collect_btn.click()
                await self._random_delay()
                logger.info(f"Collected note {note_id}")
                return InteractionResult(
                    success=True, action="collect", target_id=note_id, message="Note collected successfully"
                )

            return InteractionResult(
                success=False, action="collect", target_id=note_id, message="Collect button not found"
            )
        except Exception as e:
            logger.error(f"Failed to collect note: {e}")
            return InteractionResult(
                success=False, action="collect", target_id=note_id, message=f"Failed: {e}"
            )

    async def comment_note(self, note_id: str, comment_text: str) -> InteractionResult:
        """
        Post a comment on a note.

        Args:
            note_id: The note ID to comment on.
            comment_text: The comment content.

        Returns:
            InteractionResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Commenting on note: {note_id}")

        page = self._page
        try:
            await page.goto(f"{EXPLORE_URL}/{note_id}", wait_until="networkidle")
            await self._random_delay()

            # Click on comment input area to activate it
            comment_input = await page.wait_for_selector(
                'div[contenteditable="true"][class*="comment"], '
                'input[placeholder*="评论"], '
                'textarea[placeholder*="评论"], '
                'div.comment-input, '
                'div[class*="reply-container"] input',
                timeout=10000,
            )
            if comment_input:
                await comment_input.click()
                await self._random_delay(0.5, 1.5)
                await comment_input.fill(comment_text)
                await self._random_delay()

                # Click send button
                send_btn = await page.wait_for_selector(
                    'button:has-text("发送"), div.submit-btn, button[class*="submit"]',
                    timeout=5000,
                )
                if send_btn:
                    await send_btn.click()
                    await self._random_delay()
                    logger.info(f"Commented on note {note_id}")
                    return InteractionResult(
                        success=True, action="comment", target_id=note_id,
                        message="Comment posted successfully",
                    )

            return InteractionResult(
                success=False, action="comment", target_id=note_id,
                message="Comment input not found",
            )
        except Exception as e:
            logger.error(f"Failed to comment on note: {e}")
            return InteractionResult(
                success=False, action="comment", target_id=note_id, message=f"Failed: {e}"
            )

    async def follow_user(self, user_id: str) -> InteractionResult:
        """
        Follow a user.

        Args:
            user_id: The user ID to follow.

        Returns:
            InteractionResult indicating success or failure.
        """
        await self._ensure_started()
        logger.info(f"Following user: {user_id}")

        page = self._page
        try:
            await page.goto(f"{BASE_URL}/user/profile/{user_id}", wait_until="networkidle")
            await self._random_delay()

            follow_btn = await page.wait_for_selector(
                'button:has-text("关注"), button[class*="follow"]:not([class*="unfollow"])',
                timeout=10000,
            )
            if follow_btn:
                btn_text = await follow_btn.inner_text()
                if "已关注" in btn_text or "互相关注" in btn_text:
                    return InteractionResult(
                        success=True, action="follow", target_id=user_id,
                        message="Already following this user",
                    )
                await follow_btn.click()
                await self._random_delay()
                logger.info(f"Followed user {user_id}")
                return InteractionResult(
                    success=True, action="follow", target_id=user_id,
                    message="User followed successfully",
                )

            return InteractionResult(
                success=False, action="follow", target_id=user_id,
                message="Follow button not found",
            )
        except Exception as e:
            logger.error(f"Failed to follow user: {e}")
            return InteractionResult(
                success=False, action="follow", target_id=user_id, message=f"Failed: {e}"
            )

    # ------------------------------------------------------------------
    # Comments Retrieval
    # ------------------------------------------------------------------

    async def get_note_comments(self, note_id: str, limit: int = 20) -> list[CommentResult]:
        """
        Get comments from a note.

        Args:
            note_id: The note ID to get comments from.
            limit: Maximum number of comments to retrieve.

        Returns:
            List of CommentResult objects.
        """
        await self._ensure_started()
        logger.info(f"Fetching comments for note: {note_id}")

        page = self._page
        await page.goto(f"{EXPLORE_URL}/{note_id}", wait_until="networkidle")
        await self._random_delay()

        comments: list[CommentResult] = []

        comment_elements = await page.query_selector_all(
            'div[class*="comment-item"], div.comment-inner, div[class*="comment-content"]'
        )

        for el in comment_elements[:limit]:
            try:
                text = await self._safe_inner_text(el, 'span[class*="content"], div.content, p')
                author_name = await self._safe_inner_text(el, 'span[class*="name"], a[class*="name"]')

                author = None
                if author_name:
                    author = UserProfile(user_id="", nickname=author_name)

                if text:
                    comments.append(
                        CommentResult(
                            comment_id="",
                            content=text,
                            author=author,
                        )
                    )
            except Exception as e:
                logger.debug(f"Failed to parse comment: {e}")

        logger.info(f"Fetched {len(comments)} comments for note {note_id}")
        return comments

    # ------------------------------------------------------------------
    # User Profile
    # ------------------------------------------------------------------

    async def get_user_profile(self, user_id: str) -> UserProfile:
        """
        Fetch a user's profile information.

        Args:
            user_id: The user ID to look up.

        Returns:
            UserProfile with the user's information.
        """
        await self._ensure_started()
        logger.info(f"Fetching user profile: {user_id}")

        page = self._page
        await page.goto(f"{BASE_URL}/user/profile/{user_id}", wait_until="networkidle")
        await self._random_delay()

        nickname = await self._safe_text(page, 'div.user-name, span[class*="nickname"]')
        description = await self._safe_text(page, 'div.user-desc, div[class*="desc"], span[class*="bio"]')

        avatar_el = await page.query_selector('img[class*="avatar"], div.avatar img')
        avatar_url = None
        if avatar_el:
            avatar_url = await avatar_el.get_attribute("src")

        followers = await self._extract_count(page, 'span[class*="fans"] span, div.fans-count')
        following = await self._extract_count(page, 'span[class*="follows"] span, div.follow-count')
        likes = await self._extract_count(page, 'span[class*="liked"] span, div.liked-count')

        return UserProfile(
            user_id=user_id,
            nickname=nickname,
            avatar_url=avatar_url,
            description=description,
            followers_count=followers,
            following_count=following,
            liked_count=likes,
        )

    # ------------------------------------------------------------------
    # Screenshot Utility
    # ------------------------------------------------------------------

    async def take_screenshot(self, name: str = "screenshot") -> Path:
        """
        Take a screenshot of the current page.

        Args:
            name: Screenshot file name (without extension).

        Returns:
            Path to the saved screenshot file.
        """
        await self._ensure_started()
        self.config.storage.screenshot_dir.mkdir(parents=True, exist_ok=True)
        path = self.config.storage.screenshot_dir / f"{name}.png"
        try:
            await self._page.screenshot(path=str(path), full_page=False, timeout=15000)
            logger.info(f"Screenshot saved to {path}")
        except Exception as e:
            logger.warning(f"Screenshot failed ({e}), trying viewport-only capture...")
            try:
                await self._page.screenshot(path=str(path), full_page=False, timeout=10000)
                logger.info(f"Screenshot saved to {path} (viewport only)")
            except Exception as e2:
                logger.error(f"Screenshot completely failed: {e2}")
        return path

    # ------------------------------------------------------------------
    # Private Helpers
    # ------------------------------------------------------------------

    async def _ensure_started(self) -> None:
        """Ensure the client is started."""
        if not self._initialized:
            await self.start()

    async def _random_delay(self, min_s: Optional[float] = None, max_s: Optional[float] = None) -> None:
        """Apply a random delay to mimic human behavior."""
        min_delay = min_s if min_s is not None else self.config.rate_limit.min_delay
        max_delay = max_s if max_s is not None else self.config.rate_limit.max_delay
        delay = random.uniform(min_delay, max_delay)
        await asyncio.sleep(delay)

    @staticmethod
    def _parse_cookies(cookie_str: str) -> list[dict]:
        """Parse cookie string into Playwright cookie format."""
        cookies = []
        for item in cookie_str.split(";"):
            item = item.strip()
            if "=" in item:
                name, value = item.split("=", 1)
                cookies.append({
                    "name": name.strip(),
                    "value": value.strip(),
                    "domain": ".xiaohongshu.com",
                    "path": "/",
                })
        return cookies

    async def _safe_text(self, parent, selector: str) -> str:
        """Safely extract text content from a selector."""
        try:
            el = await parent.query_selector(selector)
            if el:
                return (await el.inner_text()).strip()
        except Exception:
            pass
        return ""

    async def _safe_inner_text(self, parent, selector: str) -> str:
        """Safely extract inner text from a child selector."""
        try:
            el = await parent.query_selector(selector)
            if el:
                return (await el.inner_text()).strip()
        except Exception:
            pass
        return ""

    async def _extract_count(self, page: Page, selector: str) -> int:
        """Extract a numeric count from a page element."""
        text = await self._safe_text(page, selector)
        if not text:
            return 0
        # Handle "1.2万" / "1.2w" style counts
        text = text.strip()
        multiplier = 1
        if text.endswith("万") or text.lower().endswith("w"):
            text = text[:-1]
            multiplier = 10000
        try:
            return int(float(text) * multiplier)
        except (ValueError, TypeError):
            # Try extracting digits only
            digits = re.findall(r"[\d.]+", text)
            if digits:
                return int(float(digits[0]) * multiplier)
            return 0

    async def _extract_author(self, page: Page) -> Optional[UserProfile]:
        """Extract author information from a note detail page."""
        try:
            nickname = await self._safe_text(page, 'span.username, a[class*="name"], div.author-name')
            if not nickname:
                return None

            avatar_el = await page.query_selector('a[class*="avatar"] img, img[class*="author-avatar"]')
            avatar_url = None
            if avatar_el:
                avatar_url = await avatar_el.get_attribute("src")

            # Try to get user_id from the author link
            author_link = await page.query_selector('a[class*="author"], a[href*="/user/profile/"]')
            user_id = ""
            if author_link:
                href = await author_link.get_attribute("href")
                if href and "/user/profile/" in href:
                    user_id = href.split("/user/profile/")[-1].split("?")[0]

            return UserProfile(
                user_id=user_id,
                nickname=nickname,
                avatar_url=avatar_url,
            )
        except Exception:
            return None

    async def _parse_note_card(self, card) -> Optional[NoteResult]:
        """Parse a note card element from search/explore page."""
        # Try to extract note ID from the card's link
        link = await card.query_selector("a")
        if not link:
            return None

        href = await link.get_attribute("href")
        if not href:
            return None

        # Extract note ID from URL
        note_id = ""
        if "/explore/" in href:
            note_id = href.split("/explore/")[-1].split("?")[0]
        elif "/discovery/item/" in href:
            note_id = href.split("/discovery/item/")[-1].split("?")[0]

        if not note_id:
            return None

        title = await self._safe_inner_text(card, 'span.title, div.title, a.title')
        author_name = await self._safe_inner_text(card, 'span.name, a.author, div[class*="author"]')

        # Extract cover image
        images: list[NoteImage] = []
        cover_img = await card.query_selector("img")
        if cover_img:
            src = await cover_img.get_attribute("src")
            if src:
                images.append(NoteImage(url=src))

        # Extract like count from the card
        liked_count = 0
        like_text = await self._safe_inner_text(card, 'span[class*="like"] span, span.count')
        if like_text:
            try:
                liked_count = int(like_text.replace("万", "0000").replace("w", "0000"))
            except ValueError:
                pass

        author = None
        if author_name:
            author = UserProfile(user_id="", nickname=author_name)

        return NoteResult(
            note_id=note_id,
            title=title,
            author=author,
            images=images,
            liked_count=liked_count,
            url=f"{EXPLORE_URL}/{note_id}",
        )
