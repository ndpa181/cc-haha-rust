/**
 * KnowledgeGraph Integration - TypeScript wrapper for Python KnowledgeGraph
 *
 * Provides persistent memory with typed entries, relationships, and search.
 */

import { pythonCall } from './pythonBridge'

export type MemoryType = 'user' | 'project' | 'reference' | 'feedback' | 'fact'

export interface MemoryEntry {
  id: string
  name: string
  description: string
  content: string
  memory_type: MemoryType
  created_at: number
  updated_at: number
  importance: number
  tags: string[]
  relationships: string[]
}

export interface SearchResult {
  entry: MemoryEntry
  score: number
  matched_on: string[]
}

export interface KnowledgeGraphStats {
  total_entries: number
  by_type: Record<string, number>
  total_tags: number
  avg_importance: number
}

/**
 * TypeScript KnowledgeGraph wrapper
 */
export class KnowledgeGraphWrapper {
  private entries = new Map<string, MemoryEntry>()
  private tagsIndex = new Map<string, Set<string>>()
  private typeIndex = new Map<MemoryType, Set<string>>()
  private storagePath: string | null = null

  constructor(storagePath?: string) {
    this.storagePath = storagePath || null
    this.typeIndex.set('user', new Set())
    this.typeIndex.set('project', new Set())
    this.typeIndex.set('reference', new Set())
    this.typeIndex.set('feedback', new Set())
    this.typeIndex.set('fact', new Set())
  }

  add(
    name: string,
    content: string,
    memoryType: MemoryType,
    options: {
      description?: string
      importance?: number
      tags?: string[]
    } = {}
  ): MemoryEntry {
    const entry: MemoryEntry = {
      id: this.generateId(),
      name,
      description: options.description || '',
      content,
      memory_type: memoryType,
      created_at: Date.now(),
      updated_at: Date.now(),
      importance: options.importance ?? 0.5,
      tags: options.tags || [],
      relationships: [],
    }

    this.entries.set(entry.id, entry)

    // Update indices
    for (const tag of entry.tags) {
      if (!this.tagsIndex.has(tag)) {
        this.tagsIndex.set(tag, new Set())
      }
      this.tagsIndex.get(tag)!.add(entry.id)
    }

    this.typeIndex.get(memoryType)!.add(entry.id)

    // Persist if path set
    if (this.storagePath) {
      this.save()
    }

    return entry
  }

  update(
    entryId: string,
    updates: Partial<Pick<MemoryEntry, 'name' | 'content' | 'importance' | 'tags'>>
  ): MemoryEntry | null {
    const entry = this.entries.get(entryId)
    if (!entry) return null

    Object.assign(entry, updates, { updated_at: Date.now() })
    return entry
  }

  addRelationship(entryId: string, relatedId: string): boolean {
    const entry = this.entries.get(entryId)
    const related = this.entries.get(relatedId)
    if (!entry || !related) return false

    if (!entry.relationships.includes(relatedId)) {
      entry.relationships.push(relatedId)
    }
    if (!related.relationships.includes(entryId)) {
      related.relationships.push(entryId)
    }

    return true
  }

  getRelated(entryId: string, depth = 1): MemoryEntry[] {
    const entry = this.entries.get(entryId)
    if (!entry) return []

    const related: MemoryEntry[] = []
    const visited = new Set<string>([entryId])
    const queue = [...entry.relationships]

    let currentDepth = 0
    while (queue.length > 0 && currentDepth < depth) {
      const nextQueue: string[] = []
      for (const relId of queue) {
        if (visited.has(relId)) continue
        visited.add(relId)
        const relEntry = this.entries.get(relId)
        if (relEntry) {
          related.push(relEntry)
          nextQueue.push(...relEntry.relationships)
        }
      }
      queue.length = 0
      queue.push(...nextQueue)
      currentDepth++
    }

    return related
  }

  search(
    query: string,
    options: {
      memoryTypes?: MemoryType[]
      tags?: string[]
      limit?: number
    } = {}
  ): SearchResult[] {
    const { memoryTypes, tags, limit = 10 } = options
    const queryLower = query.toLowerCase()
    const queryTerms = queryLower.split(/\s+/)

    let candidateIds = new Set(this.entries.keys())

    // Filter by type
    if (memoryTypes && memoryTypes.length > 0) {
      const typeIds = new Set<string>()
      for (const mt of memoryTypes) {
        for (const id of this.typeIndex.get(mt) || []) {
          typeIds.add(id)
        }
      }
      candidateIds = new Set([...candidateIds].filter((id) => typeIds.has(id)))
    }

    // Filter by tags
    if (tags && tags.length > 0) {
      const tagIds = new Set<string>()
      for (const tag of tags) {
        for (const id of this.tagsIndex.get(tag) || []) {
          tagIds.add(id)
        }
      }
      candidateIds = new Set([...candidateIds].filter((id) => tagIds.has(id)))
    }

    const results: SearchResult[] = []

    for (const entryId of candidateIds) {
      const entry = this.entries.get(entryId)!
      let score = 0
      const matchedOn: string[] = []

      // Name match (3x weight)
      if (entry.name.toLowerCase().includes(queryLower)) {
        score += 3.0 * (queryLower.length / entry.name.length)
        matchedOn.push('name')
      }

      // Description match (2x weight)
      if (entry.description.toLowerCase().includes(queryLower)) {
        score += 2.0 * (queryLower.length / entry.description.length)
        matchedOn.push('description')
      }

      // Content match (1x weight)
      const contentLower = entry.content.toLowerCase()
      const contentMatches = queryTerms.filter((term) => contentLower.includes(term)).length
      if (contentMatches > 0) {
        score += contentMatches / queryTerms.length
        matchedOn.push('content')
      }

      // Tag match bonus
      if (tags) {
        const matchingTags = tags.filter((t) => entry.tags.includes(t)).length
        score += matchingTags * 0.5
      }

      // Importance boost
      score *= 0.5 + entry.importance

      if (score > 0) {
        results.push({ entry, score, matched_on: matchedOn })
      }
    }

    // Sort by score descending
    results.sort((a, b) => b.score - a.score)

    return results.slice(0, limit)
  }

  remove(entryId: string): boolean {
    const entry = this.entries.get(entryId)
    if (!entry) return false

    // Remove from indices
    for (const tag of entry.tags) {
      this.tagsIndex.get(tag)?.delete(entryId)
    }
    this.typeIndex.get(entry.memory_type)?.delete(entryId)

    // Remove from related entries
    for (const relId of entry.relationships) {
      const rel = this.entries.get(relId)
      if (rel) {
        rel.relationships = rel.relationships.filter((id) => id !== entryId)
      }
    }

    this.entries.delete(entryId)
    return true
  }

  getStats(): KnowledgeGraphStats {
    const byType: Record<string, number> = {}
    for (const [type, ids] of this.typeIndex) {
      byType[type] = ids.size
    }

    const totalImportance = Array.from(this.entries.values()).reduce((sum, e) => sum + e.importance, 0)

    return {
      total_entries: this.entries.size,
      by_type: byType,
      total_tags: this.tagsIndex.size,
      avg_importance: this.entries.size > 0 ? totalImportance / this.entries.size : 0,
    }
  }

  save(): void {
    if (!this.storagePath) return
    // In a real implementation, would write to file
  }

  static async fromPython<T extends KnowledgeGraphWrapper>(
    this: new (path?: string) => T,
    storagePath: string
  ): Promise<T> {
    // For future Python interop - currently using pure TypeScript implementation
    return new this(storagePath)
  }

  private generateId(): string {
    return `mem_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
  }
}

/**
 * Helper to create memory entries with proper typing
 */
export function createMemoryEntry(
  type: MemoryType,
  name: string,
  content: string,
  importance = 0.5
): Omit<MemoryEntry, 'id' | 'created_at' | 'updated_at'> {
  return {
    name,
    description: '',
    content,
    memory_type: type,
    importance,
    tags: [],
    relationships: [],
  }
}
