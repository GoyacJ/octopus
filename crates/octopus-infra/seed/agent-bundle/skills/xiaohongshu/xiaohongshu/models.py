"""
Data models for Xiaohongshu Skill.
"""

from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, Field


class NoteImage(BaseModel):
    """Image associated with a note."""

    url: str = Field(..., description="Image URL")
    width: Optional[int] = Field(None, description="Image width in pixels")
    height: Optional[int] = Field(None, description="Image height in pixels")


class NoteVideo(BaseModel):
    """Video associated with a note."""

    url: str = Field(..., description="Video URL")
    duration: Optional[int] = Field(None, description="Video duration in seconds")
    cover_url: Optional[str] = Field(None, description="Video cover image URL")


class UserProfile(BaseModel):
    """Xiaohongshu user profile."""

    user_id: str = Field(..., description="User ID")
    nickname: str = Field("", description="User nickname")
    avatar_url: Optional[str] = Field(None, description="Avatar image URL")
    description: Optional[str] = Field(None, description="User bio")
    followers_count: Optional[int] = Field(None, description="Number of followers")
    following_count: Optional[int] = Field(None, description="Number of following")
    notes_count: Optional[int] = Field(None, description="Number of notes")
    liked_count: Optional[int] = Field(None, description="Total likes received")


class NoteResult(BaseModel):
    """A single Xiaohongshu note."""

    note_id: str = Field(..., description="Note ID")
    title: str = Field("", description="Note title")
    content: str = Field("", description="Note text content")
    author: Optional[UserProfile] = Field(None, description="Note author")
    images: list[NoteImage] = Field(default_factory=list, description="Note images")
    video: Optional[NoteVideo] = Field(None, description="Note video (if video note)")
    tags: list[str] = Field(default_factory=list, description="Note hashtags")
    liked_count: int = Field(0, description="Number of likes")
    collected_count: int = Field(0, description="Number of collections/bookmarks")
    comment_count: int = Field(0, description="Number of comments")
    share_count: int = Field(0, description="Number of shares")
    created_at: Optional[datetime] = Field(None, description="Note creation time")
    url: Optional[str] = Field(None, description="Note URL")


class SearchResult(BaseModel):
    """Search results from Xiaohongshu."""

    keyword: str = Field(..., description="Search keyword used")
    notes: list[NoteResult] = Field(default_factory=list, description="List of matching notes")
    total: Optional[int] = Field(None, description="Total number of results")
    has_more: bool = Field(False, description="Whether more results are available")


class CommentResult(BaseModel):
    """A single comment on a note."""

    comment_id: str = Field(..., description="Comment ID")
    content: str = Field("", description="Comment text")
    author: Optional[UserProfile] = Field(None, description="Comment author")
    liked_count: int = Field(0, description="Number of likes on this comment")
    sub_comment_count: int = Field(0, description="Number of sub-comments/replies")
    created_at: Optional[datetime] = Field(None, description="Comment creation time")


class PublishResult(BaseModel):
    """Result of publishing a note."""

    success: bool = Field(..., description="Whether the publish was successful")
    note_id: Optional[str] = Field(None, description="Published note ID")
    url: Optional[str] = Field(None, description="Published note URL")
    message: str = Field("", description="Status message")


class InteractionResult(BaseModel):
    """Result of an interaction action (like, collect, follow, etc.)."""

    success: bool = Field(..., description="Whether the action was successful")
    action: str = Field(..., description="Action type performed")
    target_id: str = Field(..., description="Target ID (note_id or user_id)")
    message: str = Field("", description="Status message")
