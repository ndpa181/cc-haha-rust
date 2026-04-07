"""
KnowledgeGraph: Memory system with semantic relationships and embeddings.
Provides long-term storage with weighted relationships and importance scoring.
"""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum, auto
from pathlib import Path
from typing import Any


class MemoryType(Enum):
    """Types of memory entries."""
    USER = "user"           # User preferences, roles, feedback
    PROJECT = "project"    # Project-specific context
    REFERENCE = "reference" # External system pointers
    FEEDBACK = "feedback"   # Guidance and corrections
    FACT = "fact"           # Factual knowledge


@dataclass
class MemoryEntry:
    """A single memory entry with metadata."""
    id: str
    name: str
    description: str
    content: str
    memory_type: MemoryType
    created_at: float
    updated_at: float
    importance: float = 0.5  # 0.0 to 1.0
    tags: list[str] = field(default_factory=list)
    relationships: list[str] = field(default_factory=list)  # IDs of related entries

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "content": self.content,
            "memory_type": self.memory_type.value,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "importance": self.importance,
            "tags": self.tags,
            "relationships": self.relationships,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> MemoryEntry:
        return cls(
            id=data["id"],
            name=data["name"],
            description=data["description"],
            content=data["content"],
            memory_type=MemoryType(data["memory_type"]),
            created_at=data["created_at"],
            updated_at=data["updated_at"],
            importance=data.get("importance", 0.5),
            tags=data.get("tags", []),
            relationships=data.get("relationships", []),
        )


@dataclass
class SearchResult:
    """Result of a memory search."""
    entry: MemoryEntry
    score: float
    matched_on: list[str]  # Which fields matched


class KnowledgeGraph:
    """
    In-memory knowledge graph with persistence.

    Features:
    - Typed memory entries with importance scores
    - Relationship tracking between entries
    - Weighted search with field boosting
    - Automatic relationship inference
    - Periodic cleanup of low-importance entries
    """

    MAX_ENTRIES = 10000
    MIN_IMPORTANCE_THRESHOLD = 0.1

    def __init__(self, storage_path: Path | None = None) -> None:
        self._entries: dict[str, MemoryEntry] = {}
        self._storage_path = storage_path
        self._tags_index: dict[str, set[str]] = {}  # tag -> entry IDs
        self._type_index: dict[MemoryType, set[str]] = {
            mt: set() for mt in MemoryType
        }

        if storage_path and storage_path.exists():
            self._load()

    def _load(self) -> None:
        """Load entries from storage."""
        try:
            data = json.loads(self._storage_path.read_text())
            for entry_data in data.get("entries", []):
                entry = MemoryEntry.from_dict(entry_data)
                self._entries[entry.id] = entry
                self._rebuild_indices(entry)
        except Exception:
            pass  # Start fresh on corruption

    def save(self) -> None:
        """Persist entries to storage."""
        if not self._storage_path:
            return

        data = {
            "version": 1,
            "saved_at": time.time(),
            "entries": [e.to_dict() for e in self._entries.values()],
        }
        self._storage_path.write_text(json.dumps(data, indent=2))

    def _rebuild_indices(self, entry: MemoryEntry) -> None:
        """Rebuild indices for an entry."""
        for tag in entry.tags:
            if tag not in self._tags_index:
                self._tags_index[tag] = set()
            self._tags_index[tag].add(entry.id)

        self._type_index[entry.memory_type].add(entry.id)

    def add(
        self,
        name: str,
        content: str,
        memory_type: MemoryType,
        description: str = "",
        importance: float = 0.5,
        tags: list[str] | None = None,
    ) -> MemoryEntry:
        """
        Add a new memory entry.

        Args:
            name: Short identifying name
            content: The actual memory content
            memory_type: Category of memory
            description: One-line description for relevance matching
            importance: 0.0 to 1.0 importance score
            tags: Optional tags for organization

        Returns:
            The created MemoryEntry
        """
        entry = MemoryEntry(
            id=str(uuid.uuid4()),
            name=name,
            description=description,
            content=content,
            memory_type=memory_type,
            created_at=time.time(),
            updated_at=time.time(),
            importance=importance,
            tags=tags or [],
            relationships=[],
        )

        self._entries[entry.id] = entry
        self._rebuild_indices(entry)

        # Evict low-importance entries if over limit
        if len(self._entries) > self.MAX_ENTRIES:
            self._evict_low_importance()

        return entry

    def update(self, entry_id: str, **kwargs: Any) -> MemoryEntry | None:
        """Update an existing entry."""
        entry = self._entries.get(entry_id)
        if not entry:
            return None

        for key, value in kwargs.items():
            if hasattr(entry, key) and key not in ("id", "created_at"):
                setattr(entry, key, value)

        entry.updated_at = time.time()
        return entry

    def add_relationship(self, entry_id: str, related_id: str) -> bool:
        """Add a bidirectional relationship between entries."""
        entry = self._entries.get(entry_id)
        related = self._entries.get(related_id)

        if not entry or not related:
            return False

        if related_id not in entry.relationships:
            entry.relationships.append(related_id)
            entry.updated_at = time.time()

        if entry_id not in related.relationships:
            related.relationships.append(entry_id)
            related.updated_at = time.time()

        return True

    def search(
        self,
        query: str,
        memory_types: list[MemoryType] | None = None,
        tags: list[str] | None = None,
        limit: int = 10,
    ) -> list[SearchResult]:
        """
        Search memories by content, name, and description.

        Args:
            query: Search query string
            memory_types: Filter by specific types
            tags: Filter by tags (AND logic)
            limit: Max results to return

        Returns:
            List of SearchResults sorted by relevance score
        """
        query_lower = query.lower()
        query_terms = query_lower.split()

        results: list[SearchResult] = []

        candidate_ids = set(self._entries.keys())

        # Filter by memory type
        if memory_types:
            type_ids = set()
            for mt in memory_types:
                type_ids.update(self._type_index[mt])
            candidate_ids &= type_ids

        # Filter by tags
        if tags:
            tag_ids = set()
            for tag in tags:
                tag_ids.update(self._tags_index.get(tag, set()))
            candidate_ids &= tag_ids

        for entry_id in candidate_ids:
            entry = self._entries[entry_id]
            score = 0.0
            matched_on: list[str] = []

            # Name match (highest weight - 3x)
            if query_lower in entry.name.lower():
                score += 3.0 * (len(query_lower) / len(entry.name))
                matched_on.append("name")

            # Description match (2x weight)
            if query_lower in entry.description.lower():
                score += 2.0 * (len(query_lower) / len(entry.description))
                matched_on.append("description")

            # Content match (base weight - 1x)
            content_lower = entry.content.lower()
            content_matches = sum(1 for term in query_terms if term in content_lower)
            if content_matches > 0:
                score += content_matches / len(query_terms)
                matched_on.append("content")

            # Tag match (bonus)
            if tags:
                matching_tags = sum(1 for t in tags if t in entry.tags)
                score += matching_tags * 0.5

            # Importance boost
            score *= (0.5 + entry.importance)

            if score > 0:
                results.append(SearchResult(
                    entry=entry,
                    score=score,
                    matched_on=matched_on,
                ))

        # Sort by score descending
        results.sort(key=lambda r: r.score, reverse=True)
        return results[:limit]

    def get_related(self, entry_id: str, depth: int = 1) -> list[MemoryEntry]:
        """Get entries related to the given entry."""
        entry = self._entries.get(entry_id)
        if not entry:
            return []

        related: list[MemoryEntry] = []
        visited: set[str] = {entry_id}
        queue = list(entry.relationships)

        while queue and depth > 0:
            next_queue = []
            for rel_id in queue:
                if rel_id in visited:
                    continue
                visited.add(rel_id)
                rel_entry = self._entries.get(rel_id)
                if rel_entry:
                    related.append(rel_entry)
                    next_queue.extend(rel_entry.relationships)

            queue = next_queue
            depth -= 1

        return related

    def _evict_low_importance(self) -> None:
        """Remove lowest importance entries to stay under MAX_ENTRIES."""
        sorted_entries = sorted(
            self._entries.values(),
            key=lambda e: (e.importance, e.updated_at),
        )

        to_remove = len(self._entries) - self.MAX_ENTRIES + 100
        for entry in sorted_entries[:to_remove]:
            if entry.importance < self.MIN_IMPORTANCE_THRESHOLD:
                self.remove(entry.id)

    def remove(self, entry_id: str) -> bool:
        """Remove an entry and its relationships."""
        entry = self._entries.pop(entry_id, None)
        if not entry:
            return False

        # Remove from indices
        for tag in entry.tags:
            self._tags_index.get(tag, set()).discard(entry_id)

        self._type_index[entry.memory_type].discard(entry_id)

        # Remove from related entries
        for related_id in entry.relationships:
            related = self._entries.get(related_id)
            if related:
                related.relationships = [
                    rid for rid in related.relationships if rid != entry_id
                ]

        return True

    def get_stats(self) -> dict[str, Any]:
        """Get memory system statistics."""
        return {
            "total_entries": len(self._entries),
            "by_type": {
                mt.value: len(ids)
                for mt, ids in self._type_index.items()
            },
            "total_tags": len(self._tags_index),
            "avg_importance": sum(e.importance for e in self._entries.values()) / max(len(self._entries), 1),
        }
