#!/usr/bin/env python3
"""
Example usage of the Xiaohongshu Skill.

Prerequisites:
    1. pip install -e .
    2. playwright install chromium
    3. Copy .env.example to .env and fill in XHS_COOKIE
"""

import asyncio
from xiaohongshu.client import XHSClient


async def example_search():
    """Example: Search for notes about a topic."""
    async with XHSClient() as client:
        results = await client.search_notes("春季穿搭", limit=5)
        print(f"\n🔍 Search Results for '{results.keyword}':")
        print(f"   Found {results.total} notes\n")

        for i, note in enumerate(results.notes, 1):
            print(f"  {i}. {note.title or '(Untitled)'}")
            print(f"     ❤️ {note.liked_count} likes")
            if note.author:
                print(f"     👤 {note.author.nickname}")
            print(f"     🔗 {note.url}")
            print()


async def example_get_note():
    """Example: Get detailed info of a specific note."""
    async with XHSClient() as client:
        # Replace with an actual note ID
        note = await client.get_note_detail("YOUR_NOTE_ID")
        print(f"\n📝 Note Detail:")
        print(f"   Title: {note.title}")
        print(f"   Content: {note.content[:200]}...")
        print(f"   ❤️ {note.liked_count} | ⭐ {note.collected_count} | 💬 {note.comment_count}")
        print(f"   Tags: {', '.join(note.tags)}")
        print()


async def example_publish_image():
    """Example: Publish an image note."""
    async with XHSClient() as client:
        result = await client.publish_image_note(
            title="我的春日穿搭分享 🌸",
            content=(
                "今天分享一套超适合春天的穿搭～\n\n"
                "上衣：白色针织开衫\n"
                "下装：高腰牛仔裤\n"
                "鞋子：小白鞋\n\n"
                "整体风格清新自然，上班通勤都可以！\n"
                "姐妹们觉得怎么样？评论区告诉我～"
            ),
            image_paths=["./images/outfit1.jpg", "./images/outfit2.jpg"],
            tags=["春季穿搭", "通勤穿搭", "OOTD", "穿搭分享"],
        )
        print(f"\n📤 Publish Result: {'✅ Success' if result.success else '❌ Failed'}")
        print(f"   Message: {result.message}")
        print()


async def example_interactions():
    """Example: Interact with a note (like, collect, comment)."""
    async with XHSClient() as client:
        note_id = "YOUR_NOTE_ID"

        # Like the note
        like_result = await client.like_note(note_id)
        print(f"❤️ Like: {'✅' if like_result.success else '❌'} {like_result.message}")

        # Collect the note
        collect_result = await client.collect_note(note_id)
        print(f"⭐ Collect: {'✅' if collect_result.success else '❌'} {collect_result.message}")

        # Comment on the note
        comment_result = await client.comment_note(note_id, "好棒！已收藏～请问在哪里买的呀？")
        print(f"💬 Comment: {'✅' if comment_result.success else '❌'} {comment_result.message}")
        print()


async def example_user_profile():
    """Example: Get a user's profile."""
    async with XHSClient() as client:
        profile = await client.get_user_profile("YOUR_USER_ID")
        print(f"\n👤 User Profile:")
        print(f"   Nickname: {profile.nickname}")
        print(f"   Bio: {profile.description}")
        print(f"   Followers: {profile.followers_count}")
        print(f"   Likes: {profile.liked_count}")
        print()


async def main():
    """Run all examples."""
    print("=" * 60)
    print("  🌟 Xiaohongshu Skill - Example Usage")
    print("=" * 60)

    # Only search example works without a specific note/user ID
    await example_search()

    # Uncomment these to test with real IDs:
    # await example_get_note()
    # await example_publish_image()
    # await example_interactions()
    # await example_user_profile()


if __name__ == "__main__":
    asyncio.run(main())
