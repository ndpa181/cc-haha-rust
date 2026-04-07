"""
ReportBuilder: Exploration results output with Mermaid diagrams.
Generates structured reports with hierarchical structure and visualizations.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum, auto
from typing import Any


class SectionLevel(Enum):
    """Hierarchy levels for report sections."""
    CHAPTER = auto()   # Top-level major sections
    SECTION = auto()    # Second-level sections
    SUBSTEP = auto()    # Third-level details


@dataclass
class Finding:
    """A single finding from exploration."""
    title: str
    description: str
    evidence: list[str] = field(default_factory=list)
    severity: str = "info"  # info, warning, critical
    code_refs: list[str] = field(default_factory=list)


@dataclass
class Diagram:
    """A Mermaid diagram to include in the report."""
    title: str
    diagram_type: str  # flowchart, sequence, class, etc.
    content: str
    caption: str = ""


@dataclass
class ReportSection:
    """A section of the exploration report."""
    id: str
    title: str
    level: SectionLevel
    content: str = ""
    findings: list[Finding] = field(default_factory=list)
    subsections: list[ReportSection] = field(default_factory=list)
    diagrams: list[Diagram] = field(default_factory=list)


@dataclass
class ExplorationMetadata:
    """Metadata about the exploration session."""
    session_id: str
    started_at: float = field(default_factory=time.time)
    completed_at: float | None = None
    scope: str = ""
    depth: int = 1
    files_analyzed: int = 0
    findings_count: int = 0


class ReportBuilder:
    """
    Builds structured exploration reports with findings and diagrams.

    Features:
    - Hierarchical section structure
    - Auto-generated Mermaid diagrams from relationships
    - Finding aggregation and severity ranking
    - Multiple export formats (Markdown, JSON)
    - Code reference linking
    """

    def __init__(self, title: str, scope: str = "") -> None:
        self.title = title
        self.root_sections: list[ReportSection] = []
        self.metadata = ExplorationMetadata(
            session_id=f"exp-{int(time.time())}",
            scope=scope,
        )
        self._current_chapter: ReportSection | None = None
        self._current_section: ReportSection | None = None

    def add_chapter(self, title: str, content: str = "") -> ReportBuilder:
        """Add a top-level chapter section."""
        chapter = ReportSection(
            id=f"ch-{len(self.root_sections)}",
            title=title,
            level=SectionLevel.CHAPTER,
            content=content,
        )
        self.root_sections.append(chapter)
        self._current_chapter = chapter
        self._current_section = None
        return self

    def add_section(self, title: str, content: str = "") -> ReportBuilder:
        """Add a second-level section under the current chapter."""
        if not self._current_chapter:
            self.add_chapter("Overview")

        section = ReportSection(
            id=f"sec-{len(self._current_chapter.subsections)}",
            title=title,
            level=SectionLevel.SECTION,
            content=content,
        )
        self._current_chapter.subsections.append(section)
        self._current_section = section
        return self

    def add_subsection(self, title: str, content: str = "") -> ReportBuilder:
        """Add a third-level subsection under the current section."""
        if not self._current_section:
            if not self._current_chapter:
                self.add_chapter("Overview")
            # Promote to section if needed
            self._current_section = ReportSection(
                id=f"sub-{len(self._current_chapter.subsections)}",
                title="Details",
                level=SectionLevel.SECTION,
            )
            self._current_chapter.subsections.append(self._current_section)

        subsection = ReportSection(
            id=f"sub-{len(self._current_section.subsections)}",
            title=title,
            level=SectionLevel.SUBSTEP,
            content=content,
        )
        self._current_section.subsections.append(subsection)
        return self

    def add_finding(
        self,
        title: str,
        description: str,
        evidence: list[str] | None = None,
        severity: str = "info",
        code_refs: list[str] | None = None,
    ) -> ReportBuilder:
        """Add a finding to the current section."""
        if not self._current_section:
            self.add_section("Findings")

        finding = Finding(
            title=title,
            description=description,
            evidence=evidence or [],
            severity=severity,
            code_refs=code_refs or [],
        )
        self._current_section.findings.append(finding)
        self.metadata.findings_count += 1
        return self

    def add_diagram(
        self,
        title: str,
        diagram_type: str,
        content: str,
        caption: str = "",
    ) -> ReportBuilder:
        """Add a Mermaid diagram to the current section."""
        if not self._current_section:
            self.add_section("Diagrams")

        diagram = Diagram(
            title=title,
            diagram_type=diagram_type,
            content=content,
            caption=caption,
        )
        self._current_section.diagrams.append(diagram)
        return self

    def add_architecture_diagram(
        self,
        title: str,
        components: list[dict[str, Any]],
        relationships: list[tuple[str, str, str]],
    ) -> ReportBuilder:
        """
        Add a component architecture diagram.

        Args:
            title: Diagram title
            components: List of {id, label, type} dicts
            relationships: List of (from_id, to_id, label) tuples
        """
        lines = ["flowchart LR"]

        # Add nodes with shapes based on type
        for comp in components:
            comp_type = comp.get("type", "component")
            shape = {
                "service": f'["{comp["label"]}"]',
                "database": f'[("{comp["label"]}")]',
                "cache": f'["{comp["label"]}"]',
                "queue": f'[("{comp["label"]}")]',
                "component": f'["{comp["label"]}"]',
            }.get(comp_type, f'["{comp["label"]}"]')

            lines.append(f'    {comp["id"]}{shape}')

        # Add relationships
        for from_id, to_id, label in relationships:
            if label:
                lines.append(f'    {from_id} -->|{label}| {to_id}')
            else:
                lines.append(f'    {from_id} --> {to_id}')

        self.add_diagram(title, "flowchart", "\n".join(lines))
        return self

    def add_flow_diagram(
        self,
        title: str,
        steps: list[dict[str, Any]],
    ) -> ReportBuilder:
        """
        Add a flow diagram from step sequence.

        Args:
            title: Diagram title
            steps: List of {id, label, type} dicts
        """
        lines = ["flowchart TD"]
        lines.append("    direction TB")

        for i, step in enumerate(steps):
            node_id = step.get("id", f"step{i}")
            label = step.get("label", f"Step {i}")
            step_type = step.get("type", "step")

            style = {
                "start": "((",  # Circle
                "end": "(())",  # Double circle
                "decision": "{}",  # Diamond
                "step": "[",      # Rectangle
            }.get(step_type, "[")

            close = {
                "start": "))",
                "end": "))",
                "decision": "}",
                "step": "]",
            }.get(step_type, "]")

            lines.append(f'    {node_id}{style}{label}{close}')

        # Connect sequential steps
        for i in range(len(steps) - 1):
            curr_id = steps[i].get("id", f"step{i}")
            next_id = steps[i + 1].get("id", f"step{i+1}")
            lines.append(f'    {curr_id} --> {next_id}')

        self.add_diagram(title, "flowchart", "\n".join(lines))
        return self

    def add_sequence_diagram(
        self,
        title: str,
        participants: list[str],
        interactions: list[dict[str, str]],
    ) -> ReportBuilder:
        """
        Add a sequence diagram.

        Args:
            title: Diagram title
            participants: List of participant names
            interactions: List of {from, to, message, type} dicts
        """
        lines = ["sequenceDiagram"]

        # Add participants
        for p in participants:
            lines.append(f"    participant {p}")

        # Add interactions
        for interaction in interactions:
            inter_type = interaction.get("type", "->")
            msg_from = interaction["from"]
            msg_to = interaction["to"]
            message = interaction.get("message", "")

            if inter_type == "->":
                lines.append(f"    {msg_from}->>{msg_to}: {message}")
            elif inter_type == "-->":
                lines.append(f"    {msg_from}-->>{msg_to}: {message}")
            elif inter_type == ">>":
                lines.append(f"    {msg_from}>>{msg_to}>>: {message}")

        self.add_diagram(title, "sequence", "\n".join(lines))
        return self

    def set_completed(self) -> None:
        """Mark exploration as completed."""
        self.metadata.completed_at = time.time()

    def build_markdown(self) -> str:
        """Build the report as Markdown string."""
        lines: list[str] = []

        # Header
        lines.append(f"# {self.title}\n")
        lines.append(f"**Session ID:** {self.metadata.session_id}")
        lines.append(f"**Scope:** {self.metadata.scope}")
        started = datetime.fromtimestamp(self.metadata.started_at).isoformat()
        lines.append(f"**Started:** {started}")

        if self.metadata.completed_at:
            completed = datetime.fromtimestamp(self.metadata.completed_at).isoformat()
            lines.append(f"**Completed:** {completed}")
            duration = self.metadata.completed_at - self.metadata.started_at
            lines.append(f"**Duration:** {duration:.1f}s")

        lines.append(f"**Files Analyzed:** {self.metadata.files_analyzed}")
        lines.append(f"**Findings:** {self.metadata.findings_count}\n")

        # Summary of critical findings
        critical_findings = []
        for section in self.root_sections:
            for subsection in section.subsections:
                for finding in subsection.findings:
                    if finding.severity == "critical":
                        critical_findings.append(finding)

        if critical_findings:
            lines.append("## Critical Findings\n")
            for f in critical_findings:
                lines.append(f"- **{f.title}**: {f.description}\n")

        # Build sections
        for chapter in self.root_sections:
            lines.append(self._format_chapter(chapter))

        return "\n".join(lines)

    def _format_chapter(self, chapter: ReportSection) -> str:
        """Format a chapter section."""
        lines = [f"\n## {chapter.title}\n"]
        if chapter.content:
            lines.append(f"{chapter.content}\n")

        for diagram in chapter.diagrams:
            lines.append(self._format_diagram(diagram))

        for finding in chapter.findings:
            lines.append(self._format_finding(finding))

        for subsection in chapter.subsections:
            lines.append(self._format_subsection(subsection))

        return "\n".join(lines)

    def _format_subsection(self, subsection: ReportSection) -> str:
        """Format a subsection."""
        lines = [f"### {subsection.title}\n"]
        if subsection.content:
            lines.append(f"{subsection.content}\n")

        for diagram in subsection.diagrams:
            lines.append(self._format_diagram(diagram))

        for finding in subsection.findings:
            lines.append(self._format_finding(finding))

        for sub in subsection.subsections:
            lines.append(self._format_subsection(sub))

        return "\n".join(lines)

    def _format_diagram(self, diagram: Diagram) -> str:
        """Format a Mermaid diagram."""
        lines = [f"**{diagram.title}**"]
        if diagram.caption:
            lines.append(f'*{diagram.caption}*')
        lines.append('```mermaid')
        lines.append(diagram.content)
        lines.append('```\n')
        return "\n".join(lines)

    def _format_finding(self, finding: Finding) -> str:
        """Format a finding."""
        severity_icon = {
            "info": "ℹ️",
            "warning": "⚠️",
            "critical": "🚨",
        }.get(finding.severity, "ℹ️")

        lines = [f"{severity_icon} **{finding.title}**: {finding.description}\n"]

        if finding.evidence:
            lines.append("Evidence:")
            for e in finding.evidence:
                lines.append(f"- {e}")
            lines.append("")

        if finding.code_refs:
            lines.append("Code references:")
            for ref in finding.code_refs:
                lines.append(f"- `{ref}`")
            lines.append("")

        return "\n".join(lines)

    def build_json(self) -> dict[str, Any]:
        """Build the report as JSON-serializable dict."""
        return {
            "title": self.title,
            "metadata": {
                "session_id": self.metadata.session_id,
                "started_at": self.metadata.started_at,
                "completed_at": self.metadata.completed_at,
                "scope": self.metadata.scope,
                "depth": self.metadata.depth,
                "files_analyzed": self.metadata.files_analyzed,
                "findings_count": self.metadata.findings_count,
            },
            "sections": [self._section_to_dict(s) for s in self.root_sections],
        }

    def _section_to_dict(self, section: ReportSection) -> dict[str, Any]:
        """Convert a section to dict."""
        return {
            "id": section.id,
            "title": section.title,
            "level": section.level.name,
            "content": section.content,
            "findings": [
                {
                    "title": f.title,
                    "description": f.description,
                    "evidence": f.evidence,
                    "severity": f.severity,
                    "code_refs": f.code_refs,
                }
                for f in section.findings
            ],
            "diagrams": [
                {
                    "title": d.title,
                    "type": d.diagram_type,
                    "content": d.content,
                    "caption": d.caption,
                }
                for d in section.diagrams
            ],
            "subsections": [self._section_to_dict(s) for s in section.subsections],
        }

    def _collect_all_findings(self, sections: list[ReportSection]) -> list[Finding]:
        """Recursively collect all findings from sections and subsections."""
        findings = []
        for section in sections:
            findings.extend(section.findings)
            findings.extend(self._collect_all_findings(section.subsections))
        return findings

    def get_stats(self) -> dict[str, Any]:
        """Get report statistics."""
        all_findings = self._collect_all_findings(self.root_sections)

        total_findings = len(all_findings)
        total_diagrams = sum(
            len(s.diagrams) for s in self.root_sections
            for ss in s.subsections
            for ss2 in ss.subsections
            if hasattr(ss2, 'diagrams')
        ) + sum(len(s.diagrams) for s in self.root_sections)

        return {
            "chapters": len(self.root_sections),
            "total_findings": total_findings,
            "total_diagrams": total_diagrams,
            "severity_breakdown": {
                "critical": sum(1 for f in all_findings if f.severity == "critical"),
                "warning": sum(1 for f in all_findings if f.severity == "warning"),
                "info": sum(1 for f in all_findings if f.severity == "info"),
            },
        }
