"""
Convenience wrapper providing synchronous API for XHSClient.

Usage:
    from xiaohongshu.api import search, get_note, publish_image, like, comment, follow

    results = search("春季穿搭")
    note = get_note("note_id_here")
    publish_image("标题", "内容", ["image1.jpg", "image2.jpg"])
    like("note_id_here")
    comment("note_id_here", "好棒！")
    follow("user_id_here")
"""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Optional

from xiaohongshu.client import XHSClient
from xiaohongshu.config import XHSConfig
from xiaohongshu.models import (
    CommentResult,
    InteractionResult,
    NoteResult,
    PublishResult,
    SearchResult,
    UserProfile,
)


def _run(coro):
    """Run an async coroutine synchronously."""
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None

    if loop and loop.is_running():
        # If we're already in an async context, create a new event loop in a thread
        import concurrent.futures
        with concurrent.futures.ThreadPoolExecutor() as pool:
            return pool.submit(asyncio.run, coro).result()
    else:
        return asyncio.run(coro)


def search(
    keyword: str,
    *,
    sort: str = "general",
    note_type: str = "all",
    limit: int = 20,
    config: Optional[XHSConfig] = None,
) -> SearchResult:
    """
    Search notes by keyword.

    Args:
        keyword: Search keyword.
        sort: Sort type - "general", "time_descending", "popularity_descending".
        note_type: Filter type - "all", "image", "video".
        limit: Maximum number of results.
        config: Optional XHSConfig override.

    Returns:
        SearchResult with matching notes.
    """
    async def _search():
        async with XHSClient(config) as client:
            return await client.search_notes(keyword, sort=sort, note_type=note_type, limit=limit)
    return _run(_search())


def get_note(note_id: str, *, config: Optional[XHSConfig] = None) -> NoteResult:
    """
    Get detailed information of a specific note.

    Args:
        note_id: The note ID.
        config: Optional XHSConfig override.

    Returns:
        NoteResult with full details.
    """
    async def _get():
        async with XHSClient(config) as client:
            return await client.get_note_detail(note_id)
    return _run(_get())


def publish_image(
    title: str,
    content: str,
    image_paths: list[str | Path],
    *,
    tags: Optional[list[str]] = None,
    config: Optional[XHSConfig] = None,
) -> PublishResult:
    """
    Publish an image note.

    Args:
        title: Note title.
        content: Note content.
        image_paths: List of image file paths.
        tags: Optional hashtags.
        config: Optional XHSConfig override.

    Returns:
        PublishResult.
    """
    async def _publish():
        async with XHSClient(config) as client:
            return await client.publish_image_note(title, content, image_paths, tags=tags)
    return _run(_publish())


def publish_video(
    title: str,
    content: str,
    video_path: str | Path,
    *,
    tags: Optional[list[str]] = None,
    config: Optional[XHSConfig] = None,
) -> PublishResult:
    """
    Publish a video note.

    Args:
        title: Note title.
        content: Note content.
        video_path: Video file path.
        tags: Optional hashtags.
        config: Optional XHSConfig override.

    Returns:
        PublishResult.
    """
    async def _publish():
        async with XHSClient(config) as client:
            return await client.publish_video_note(title, content, video_path, tags=tags)
    return _run(_publish())


def like(note_id: str, *, config: Optional[XHSConfig] = None) -> InteractionResult:
    """
    Like a note.

    Args:
        note_id: The note ID.
        config: Optional XHSConfig override.

    Returns:
        InteractionResult.
    """
    async def _like():
        async with XHSClient(config) as client:
            return await client.like_note(note_id)
    return _run(_like())


def collect(note_id: str, *, config: Optional[XHSConfig] = None) -> InteractionResult:
    """
    Collect (bookmark) a note.

    Args:
        note_id: The note ID.
        config: Optional XHSConfig override.

    Returns:
        InteractionResult.
    """
    async def _collect():
        async with XHSClient(config) as client:
            return await client.collect_note(note_id)
    return _run(_collect())


def comment(
    note_id: str,
    comment_text: str,
    *,
    config: Optional[XHSConfig] = None,
) -> InteractionResult:
    """
    Comment on a note.

    Args:
        note_id: The note ID.
        comment_text: Comment content.
        config: Optional XHSConfig override.

    Returns:
        InteractionResult.
    """
    async def _comment():
        async with XHSClient(config) as client:
            return await client.comment_note(note_id, comment_text)
    return _run(_comment())


def follow(user_id: str, *, config: Optional[XHSConfig] = None) -> InteractionResult:
    """
    Follow a user.

    Args:
        user_id: The user ID.
        config: Optional XHSConfig override.

    Returns:
        InteractionResult.
    """
    async def _follow():
        async with XHSClient(config) as client:
            return await client.follow_user(user_id)
    return _run(_follow())


def get_comments(
    note_id: str,
    *,
    limit: int = 20,
    config: Optional[XHSConfig] = None,
) -> list[CommentResult]:
    """
    Get comments from a note.

    Args:
        note_id: The note ID.
        limit: Maximum number of comments.
        config: Optional XHSConfig override.

    Returns:
        List of CommentResult.
    """
    async def _comments():
        async with XHSClient(config) as client:
            return await client.get_note_comments(note_id, limit=limit)
    return _run(_comments())


def get_user(user_id: str, *, config: Optional[XHSConfig] = None) -> UserProfile:
    """
    Get user profile.

    Args:
        user_id: The user ID.
        config: Optional XHSConfig override.

    Returns:
        UserProfile.
    """
    async def _user():
        async with XHSClient(config) as client:
            return await client.get_user_profile(user_id)
    return _run(_user())
