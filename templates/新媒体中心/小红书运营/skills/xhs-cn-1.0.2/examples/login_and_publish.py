#!/usr/bin/env python3
"""
Login to Xiaohongshu and publish a test note.

This script demonstrates the complete flow:
1. Launch browser (non-headless so you can see the QR code)
2. Login via QR code scanning
3. Verify login status
4. Generate a test image
5. Publish a test note with the image
6. Save cookies for future sessions

Usage:
    pip install -e .
    playwright install chromium
    python examples/login_and_publish.py
"""

import asyncio
import sys
from datetime import datetime
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from xiaohongshu.client import XHSClient
from xiaohongshu.config import XHSConfig, BrowserConfig, StorageConfig


def create_test_image(output_path: str = "./test_images/test_note.png") -> str:
    """
    Generate a simple test image for publishing.

    Args:
        output_path: Path to save the generated image.

    Returns:
        The path to the created image file.
    """
    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError:
        print("❌ Pillow is required. Install with: pip install Pillow")
        sys.exit(1)

    # Create a colorful gradient image
    width, height = 1080, 1440
    img = Image.new("RGB", (width, height))
    draw = ImageDraw.Draw(img)

    # Draw a nice gradient background
    for y in range(height):
        r = int(255 * (1 - y / height) * 0.6 + 100)
        g = int(180 * (y / height) + 50)
        b = int(255 * (y / height) * 0.8 + 80)
        r = min(255, max(0, r))
        g = min(255, max(0, g))
        b = min(255, max(0, b))
        draw.line([(0, y), (width, y)], fill=(r, g, b))

    # Add decorative elements
    for i in range(5):
        x = 100 + i * 200
        y = 400 + (i % 3) * 100
        radius = 30 + i * 10
        draw.ellipse(
            [x - radius, y - radius, x + radius, y + radius],
            fill=(255, 255, 255, 128),
            outline=(255, 255, 255),
        )

    # Add text
    now = datetime.now().strftime("%Y-%m-%d %H:%M")

    # Try to use a nice font, fall back to default
    try:
        font_large = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 60)
        font_medium = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 36)
        font_small = ImageFont.truetype("/System/Library/Fonts/PingFang.ttc", 28)
    except (OSError, IOError):
        try:
            font_large = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 60)
            font_medium = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 36)
            font_small = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 28)
        except (OSError, IOError):
            font_large = ImageFont.load_default()
            font_medium = font_large
            font_small = font_large

    # Title
    draw.text(
        (width // 2, 200),
        "🌟 Xiaohongshu Skill Test 🌟",
        fill=(255, 255, 255),
        font=font_large,
        anchor="mm",
    )

    # Subtitle
    draw.text(
        (width // 2, 300),
        "Automated Publishing Test",
        fill=(255, 255, 255),
        font=font_medium,
        anchor="mm",
    )

    # Timestamp
    draw.text(
        (width // 2, height - 200),
        f"Generated at {now}",
        fill=(255, 255, 240),
        font=font_small,
        anchor="mm",
    )

    # Watermark
    draw.text(
        (width // 2, height - 120),
        "xiaohongshu-skill by AI",
        fill=(255, 255, 255),
        font=font_small,
        anchor="mm",
    )

    # Save the image
    output = Path(output_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(str(output), "PNG")
    print(f"✅ Test image created: {output.resolve()}")
    return str(output.resolve())


async def main():
    """Main flow: login via QR code, then publish a test note."""

    print("=" * 60)
    print("  🌟 Xiaohongshu Skill - Login & Publish Test")
    print("=" * 60)
    print()

    # ------------------------------------------------------------------
    # Step 0: Configuration
    # ------------------------------------------------------------------
    # Use non-headless mode so you can see and scan the QR code
    config = XHSConfig(
        cookie="",  # We will login via QR code
        browser=BrowserConfig(
            browser_type="chromium",
            headless=False,  # IMPORTANT: must be False to see QR code
            user_agent=(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            ),
        ),
        storage=StorageConfig(
            cookie_file=Path("./cookies.json"),
            screenshot_dir=Path("./screenshots"),
            download_dir=Path("./downloads"),
        ),
    )

    # ------------------------------------------------------------------
    # Step 1: Generate test image
    # ------------------------------------------------------------------
    print("📸 Step 1: Generating test image...")
    image_path = create_test_image()
    print()

    # ------------------------------------------------------------------
    # Step 2: Login
    # ------------------------------------------------------------------
    async with XHSClient(config) as client:

        print("🔐 Step 2: Checking login status...")

        # First try to use saved cookies
        is_logged_in = await client.check_login()

        if not is_logged_in:
            print()
            print("━" * 50)
            print("  📱 Please scan the QR code with your")
            print("     Xiaohongshu (小红书) mobile app!")
            print()
            print("  The browser window should now show the")
            print("  login page with a QR code.")
            print()
            print("  ⏳ You have 120 seconds to scan...")
            print("━" * 50)
            print()

            login_ok = await client.login_by_qrcode(timeout=120)
            if not login_ok:
                print("❌ Login failed! Please try again.")
                print("   Tip: Make sure your Xiaohongshu app is updated")
                print("   and you have a stable internet connection.")
                return

        print("✅ Login successful!")
        print()

        # ------------------------------------------------------------------
        # Step 3: Publish test note
        # ------------------------------------------------------------------
        print("📤 Step 3: Publishing test note...")
        print()

        now_str = datetime.now().strftime("%Y年%m月%d日 %H:%M")

        result = await client.publish_image_note(
            title=f"🌟 AI自动化测试笔记 - {now_str}",
            content=(
                f"这是一条由 Xiaohongshu Skill 自动发布的测试笔记 ✨\n\n"
                f"📅 发布时间：{now_str}\n"
                f"🤖 发布方式：Python + Playwright 自动化\n"
                f"📌 项目地址：github.com/xiaohongshu-skill\n\n"
                f"如果你看到这条笔记，说明自动化发布功能正常工作！🎉\n\n"
                f"这个测试完成后会手动删除，请忽略～"
            ),
            image_paths=[image_path],
            tags=["AI自动化", "技术测试", "Python"],
        )

        print()
        print("━" * 50)
        if result.success:
            print("  ✅ 发布成功!")
            if result.note_id:
                print(f"  📝 Note ID: {result.note_id}")
            if result.url:
                print(f"  🔗 URL: {result.url}")
        else:
            print(f"  ❌ 发布失败: {result.message}")
        print("━" * 50)
        print()

        # Take a final screenshot
        try:
            screenshot = await client.take_screenshot("publish_result")
            print(f"📸 Result screenshot: {screenshot}")
        except Exception:
            print("📸 (Screenshot skipped due to timeout)")

        # ------------------------------------------------------------------
        # Step 4: Verify
        # ------------------------------------------------------------------
        print()
        print("✅ All done! Summary:")
        print(f"   🍪 Cookies saved to: {config.storage.cookie_file}")
        print(f"   📸 Screenshots in: {config.storage.screenshot_dir}")
        print(f"   ℹ️  Next time you run this script, it will auto-login using saved cookies.")
        print()
        print("⚠️  Remember to manually delete the test note from your account!")


if __name__ == "__main__":
    asyncio.run(main())
