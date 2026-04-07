/**
 * ReportBuilder Integration - TypeScript wrapper for Python ReportBuilder
 *
 * Provides structured report generation with findings, diagrams, and multiple export formats.
 */

export type SectionLevel = 'CHAPTER' | 'SECTION' | 'SUBSTEP'

export interface Finding {
  title: string
  description: string
  evidence: string[]
  severity: 'info' | 'warning' | 'critical'
  code_refs: string[]
}

export interface Diagram {
  title: string
  type: string
  content: string
  caption: string
}

export interface ReportSection {
  id: string
  title: string
  level: SectionLevel
  content: string
  findings: Finding[]
  diagrams: Diagram[]
  subsections: ReportSection[]
}

export interface ExplorationMetadata {
  session_id: string
  started_at: number
  completed_at: number | null
  scope: string
  depth: number
  files_analyzed: number
  findings_count: number
}

export interface ReportJSON {
  title: string
  metadata: ExplorationMetadata
  sections: ReportSection[]
}

export interface ReportStats {
  chapters: number
  total_findings: number
  total_diagrams: number
  severity_breakdown: {
    critical: number
    warning: number
    info: number
  }
}

const SEVERITY_ICONS = {
  info: 'ℹ️',
  warning: '⚠️',
  critical: '🚨',
} as const

/**
 * TypeScript ReportBuilder wrapper
 */
export class ReportBuilderWrapper {
  private _title: string
  private _rootSections: ReportSection[] = []
  private _metadata: ExplorationMetadata
  private _currentChapter: ReportSection | null = null
  private _currentSection: ReportSection | null = null
  private _sectionCounter = { chapter: 0, section: 0, subsection: 0 }

  constructor(title: string, scope = '') {
    this._title = title
    this._metadata = {
      session_id: `exp-${Date.now()}`,
      started_at: Date.now(),
      completed_at: null,
      scope,
      depth: 1,
      files_analyzed: 0,
      findings_count: 0,
    }
  }

  addChapter(title: string, content = ''): this {
    this._sectionCounter.chapter++
    this._sectionCounter.section = 0
    this._sectionCounter.subsection = 0

    this._currentChapter = {
      id: `ch-${this._sectionCounter.chapter}`,
      title,
      level: 'CHAPTER',
      content,
      findings: [],
      diagrams: [],
      subsections: [],
    }

    this._rootSections.push(this._currentChapter)
    this._currentSection = null
    return this
  }

  addSection(title: string, content = ''): this {
    if (!this._currentChapter) {
      this.addChapter('Overview')
    }

    this._sectionCounter.section++
    this._sectionCounter.subsection = 0

    this._currentSection = {
      id: `sec-${this._sectionCounter.chapter}-${this._sectionCounter.section}`,
      title,
      level: 'SECTION',
      content,
      findings: [],
      diagrams: [],
      subsections: [],
    }

    this._currentChapter!.subsections.push(this._currentSection)
    return this
  }

  addSubsection(title: string, content = ''): this {
    if (!this._currentSection) {
      this.addSection('Details')
    }

    this._sectionCounter.subsection++

    const subsection: ReportSection = {
      id: `sub-${this._sectionCounter.chapter}-${this._sectionCounter.section}-${this._sectionCounter.subsection}`,
      title,
      level: 'SUBSTEP',
      content,
      findings: [],
      diagrams: [],
      subsections: [],
    }

    this._currentSection!.subsections.push(subsection)
    return this
  }

  addFinding(
    title: string,
    description: string,
    options: {
      evidence?: string[]
      severity?: 'info' | 'warning' | 'critical'
      code_refs?: string[]
    } = {}
  ): this {
    if (!this._currentSection) {
      this.addSection('Findings')
    }

    const finding: Finding = {
      title,
      description,
      evidence: options.evidence || [],
      severity: options.severity || 'info',
      code_refs: options.code_refs || [],
    }

    this._currentSection!.findings.push(finding)
    this._metadata.findings_count++
    return this
  }

  addDiagram(title: string, type: string, content: string, caption = ''): this {
    if (!this._currentSection) {
      this.addSection('Diagrams')
    }

    this._currentSection!.diagrams.push({ title, type, content, caption })
    return this
  }

  addArchitectureDiagram(
    title: string,
    components: Array<{ id: string; label: string; type: string }>,
    relationships: Array<[string, string, string]>
  ): this {
    const lines = ['flowchart LR']

    for (const comp of components) {
      const shape = this.getComponentShape(comp.type)
      lines.push(`    ${comp.id}${shape}["${comp.label}"]`)
    }

    for (const [from, to, label] of relationships) {
      if (label) {
        lines.push(`    ${from} -->|${label}| ${to}`)
      } else {
        lines.push(`    ${from} --> ${to}`)
      }
    }

    this.addDiagram(title, 'flowchart', lines.join('\n'))
    return this
  }

  addFlowDiagram(
    title: string,
    steps: Array<{ id: string; label: string; type: string }>
  ): this {
    const lines = ['flowchart TD', '    direction TB']

    for (const step of steps) {
      const shape = this.getFlowShape(step.type)
      lines.push(`    ${step.id}${shape}${step.label}${shape.replace('[', ']').replace('((', '))').replace('{/', '/}').replace('[>', ']>')}`)
    }

    // Connect sequential steps
    for (let i = 0; i < steps.length - 1; i++) {
      lines.push(`    ${steps[i].id} --> ${steps[i + 1].id}`)
    }

    this.addDiagram(title, 'flowchart', lines.join('\n'))
    return this
  }

  addSequenceDiagram(
    title: string,
    participants: string[],
    interactions: Array<{ from: string; to: string; message: string; type?: string }>
  ): this {
    const lines = ['sequenceDiagram']

    for (const p of participants) {
      lines.push(`    participant ${p}`)
    }

    for (const { from, to, message, type = '->' } of interactions) {
      const arrow = type === '-->' ? '-->>' : '->>'
      lines.push(`    ${from}${arrow}${to}: ${message}`)
    }

    this.addDiagram(title, 'sequence', lines.join('\n'))
    return this
  }

  setMetadata(filesAnalyzed: number): this {
    this._metadata.files_analyzed = filesAnalyzed
    return this
  }

  setCompleted(): void {
    this._metadata.completed_at = Date.now()
  }

  buildMarkdown(): string {
    const lines: string[] = []

    // Header
    lines.push(`# ${this._title}\n`)
    lines.push(`**Session ID:** ${this._metadata.session_id}`)
    lines.push(`**Scope:** ${this._metadata.scope}`)
    lines.push(`**Started:** ${new Date(this._metadata.started_at).toISOString()}`)

    if (this._metadata.completed_at) {
      lines.push(`**Completed:** ${new Date(this._metadata.completed_at).toISOString()}`)
      const duration = (this._metadata.completed_at - this._metadata.started_at) / 1000
      lines.push(`**Duration:** ${duration.toFixed(1)}s`)
    }

    lines.push(`**Files Analyzed:** ${this._metadata.files_analyzed}`)
    lines.push(`**Findings:** ${this._metadata.findings_count}\n`)

    // Critical findings summary
    const criticalFindings = this._rootSections
      .flatMap((s) => s.findings)
      .filter((f) => f.severity === 'critical')

    if (criticalFindings.length > 0) {
      lines.push('## Critical Findings\n')
      for (const f of criticalFindings) {
        lines.push(`- **${f.title}**: ${f.description}\n`)
      }
    }

    // Build sections
    for (const section of this._rootSections) {
      lines.push(this.formatSection(section))
    }

    return lines.join('\n')
  }

  buildJSON(): ReportJSON {
    return {
      title: this._title,
      metadata: { ...this._metadata },
      sections: this._rootSections.map((s) => this.cloneSection(s)),
    }
  }

  getStats(): ReportStats {
    let totalFindings = 0
    let totalDiagrams = 0
    const severityBreakdown = { critical: 0, warning: 0, info: 0 }

    const countSection = (section: ReportSection) => {
      totalFindings += section.findings.length
      totalDiagrams += section.diagrams.length

      for (const f of section.findings) {
        if (f.severity in severityBreakdown) {
          severityBreakdown[f.severity as keyof typeof severityBreakdown]++
        }
      }

      for (const sub of section.subsections) {
        countSection(sub)
      }
    }

    for (const section of this._rootSections) {
      countSection(section)
    }

    return {
      chapters: this._rootSections.length,
      total_findings: totalFindings,
      total_diagrams: totalDiagrams,
      severity_breakdown: severityBreakdown,
    }
  }

  private formatSection(section: ReportSection): string {
    const lines: string[] = []
    const prefix = section.level === 'CHAPTER' ? '##' : section.level === 'SECTION' ? '###' : '####'

    lines.push(`\n${prefix} ${section.title}\n`)
    if (section.content) {
      lines.push(`${section.content}\n`)
    }

    for (const diagram of section.diagrams) {
      lines.push(this.formatDiagram(diagram))
    }

    for (const finding of section.findings) {
      lines.push(this.formatFinding(finding))
    }

    for (const subsection of section.subsections) {
      lines.push(this.formatSection(subsection))
    }

    return lines.join('\n')
  }

  private formatDiagram(diagram: Diagram): string {
    const lines: string[] = []
    lines.push(`**${diagram.title}**`)
    if (diagram.caption) {
      lines.push(`*${diagram.caption}*`)
    }
    lines.push(`\`\`\`mermaid`)
    lines.push(diagram.content)
    lines.push('```\n')
    return lines.join('\n')
  }

  private formatFinding(finding: Finding): string {
    const icon = SEVERITY_ICONS[finding.severity]
    const lines: string[] = []

    lines.push(`${icon} **${finding.title}**: ${finding.description}\n`)

    if (finding.evidence.length > 0) {
      lines.push('Evidence:')
      for (const e of finding.evidence) {
        lines.push(`- ${e}`)
      }
      lines.push('')
    }

    if (finding.code_refs.length > 0) {
      lines.push('Code references:')
      for (const ref of finding.code_refs) {
        lines.push(`- \`${ref}\``)
      }
      lines.push('')
    }

    return lines.join('\n')
  }

  private cloneSection(section: ReportSection): ReportSection {
    return {
      ...section,
      findings: [...section.findings],
      diagrams: [...section.diagrams],
      subsections: section.subsections.map((s) => this.cloneSection(s)),
    }
  }

  private getComponentShape(type: string): string {
    const shapes: Record<string, string> = {
      service: '[',
      database: '[(',
      cache: '[',
      queue: '[(',
      component: '[',
    }
    return shapes[type] || '['
  }

  private getFlowShape(type: string): string {
    const shapes: Record<string, string> = {
      start: '((',
      end: '((',
      decision: '{',
      step: '[',
    }
    return shapes[type] || '['
  }
}
