"""
Configuration management for Xiaohongshu Skill.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Optional

from dotenv import load_dotenv
from pydantic import BaseModel, Field

# Load environment variables from .env file
load_dotenv()


class BrowserConfig(BaseModel):
    """Browser configuration."""

    browser_type: str = Field(default="chromium", description="Browser type: chromium, firefox, webkit")
    headless: bool = Field(default=True, description="Run browser in headless mode")
    user_agent: str = Field(
        default=(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
            "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
        ),
        description="Browser user agent string",
    )


class ProxyConfig(BaseModel):
    """Proxy configuration."""

    server: Optional[str] = Field(None, description="Proxy server URL")
    username: Optional[str] = Field(None, description="Proxy username")
    password: Optional[str] = Field(None, description="Proxy password")


class RateLimitConfig(BaseModel):
    """Rate limiting configuration."""

    min_delay: float = Field(default=2.0, description="Minimum delay between requests (seconds)")
    max_delay: float = Field(default=5.0, description="Maximum delay between requests (seconds)")


class StorageConfig(BaseModel):
    """Storage paths configuration."""

    download_dir: Path = Field(default=Path("./downloads"), description="Directory for downloaded media")
    screenshot_dir: Path = Field(default=Path("./screenshots"), description="Directory for screenshots")
    cookie_file: Path = Field(default=Path("./cookies.json"), description="Path to save/load browser cookies")


class XHSConfig(BaseModel):
    """Main configuration for Xiaohongshu Skill."""

    cookie: str = Field(default="", description="Xiaohongshu cookie string")
    browser: BrowserConfig = Field(default_factory=BrowserConfig)
    proxy: ProxyConfig = Field(default_factory=ProxyConfig)
    rate_limit: RateLimitConfig = Field(default_factory=RateLimitConfig)
    storage: StorageConfig = Field(default_factory=StorageConfig)

    @classmethod
    def from_env(cls) -> XHSConfig:
        """Create configuration from environment variables."""
        return cls(
            cookie=os.getenv("XHS_COOKIE", ""),
            browser=BrowserConfig(
                browser_type=os.getenv("BROWSER_TYPE", "chromium"),
                headless=os.getenv("HEADLESS", "true").lower() == "true",
                user_agent=os.getenv(
                    "USER_AGENT",
                    BrowserConfig.model_fields["user_agent"].default,
                ),
            ),
            proxy=ProxyConfig(
                server=os.getenv("PROXY_SERVER"),
                username=os.getenv("PROXY_USERNAME"),
                password=os.getenv("PROXY_PASSWORD"),
            ),
            rate_limit=RateLimitConfig(
                min_delay=float(os.getenv("MIN_REQUEST_DELAY", "2")),
                max_delay=float(os.getenv("MAX_REQUEST_DELAY", "5")),
            ),
            storage=StorageConfig(
                download_dir=Path(os.getenv("DOWNLOAD_DIR", "./downloads")),
                screenshot_dir=Path(os.getenv("SCREENSHOT_DIR", "./screenshots")),
                cookie_file=Path(os.getenv("COOKIE_FILE", "./cookies.json")),
            ),
        )
