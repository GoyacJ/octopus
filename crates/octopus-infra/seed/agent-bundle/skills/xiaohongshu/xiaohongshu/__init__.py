"""
Xiaohongshu Skill - Full-pipeline operations for Xiaohongshu (Little Red Book).
"""

from xiaohongshu.client import XHSClient
from xiaohongshu.models import NoteResult, UserProfile, SearchResult, CommentResult

__version__ = "1.0.0"
__all__ = ["XHSClient", "NoteResult", "UserProfile", "SearchResult", "CommentResult"]
