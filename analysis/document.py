"""ReportLab PDF report builder: paragraph styles, SVG plot embeds, table layout, and full document assembly."""

from __future__ import annotations

import html
import re
from pathlib import Path

from reportlab.graphics import renderPDF
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import cm
from reportlab.platypus import (
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)
from reportlab.platypus.flowables import Flowable
from svglib.svglib import svg2rlg

from .sections import (
    approach_lines,
    asset_block,
    batman_bellman_algorithm_lines,
    comparison_findings_lines,
    engine_section_lines,
    executive_summary_lines,
    field_algorithm_lines,
    head_to_head_findings_lines,
    head_to_head_regime_lines,
    limitations_lines,
    methodology_lines,
    pathway_algorithm_lines,
    profile_recommendation_lines,
    recommendation_rationale_lines,
    regime_assumption_lines,
    regime_characterization_lines,
    section_lines,
    scoring_lines,
    simulation_setup_lines,
)
from .tables import (
    comparison_table_rows,
    head_to_head_table_rows,
    profile_table_rows,
    recommendation_table_rows,
    transition_table_rows,
    boundary_table_rows,
)


def codeify_known_terms(text: str) -> str:
    terms = ["pathway-batman", "batman-bellman", "pathway", "field"]
    pattern = re.compile(r"\b(" + "|".join(re.escape(term) for term in terms) + r")\b")
    parts = re.split(r"(`[^`]+`)", text)
    wrapped: list[str] = []
    for part in parts:
        if part.startswith("`") and part.endswith("`"):
            wrapped.append(part)
            continue
        wrapped.append(pattern.sub(lambda match: f"`{match.group(1)}`", part))
    return "".join(wrapped)


def markup(text: str) -> str:
    escaped = html.escape(codeify_known_terms(text))
    return re.sub(r"`([^`]+)`", r'<font name="Courier">\1</font>', escaped)


class SvgPlot(Flowable):
    def __init__(self, svg_path: Path, max_width: float, max_height: float):
        super().__init__()
        self.drawing = svg2rlg(str(svg_path))
        scale = min(max_width / self.drawing.width, max_height / self.drawing.height)
        self.scale = scale
        self.width = self.drawing.width * scale
        self.height = self.drawing.height * scale

    def wrap(self, available_width: float, available_height: float):
        return self.width, self.height

    def draw(self):
        self.canv.saveState()
        self.canv.scale(self.scale, self.scale)
        renderPDF.draw(self.drawing, self.canv, 0, 0)
        self.canv.restoreState()


def build_styles():
    styles = getSampleStyleSheet()
    styles.add(
        ParagraphStyle(
            name="Body",
            parent=styles["BodyText"],
            fontName="Helvetica",
            fontSize=9.5,
            leading=13,
            spaceAfter=6,
            alignment=TA_LEFT,
        )
    )
    styles.add(
        ParagraphStyle(
            name="BulletBody",
            parent=styles["BodyText"],
            fontName="Helvetica",
            fontSize=9.5,
            leading=13,
            leftIndent=10,
            spaceAfter=4,
        )
    )
    styles.add(
        ParagraphStyle(
            name="Section",
            parent=styles["Heading1"],
            fontName="Helvetica-Bold",
            fontSize=13,
            leading=16,
            textColor=colors.HexColor("#0f172a"),
            spaceBefore=8,
            spaceAfter=8,
        )
    )
    styles.add(
        ParagraphStyle(
            name="Subsection",
            parent=styles["Heading2"],
            fontName="Helvetica-Bold",
            fontSize=11.2,
            leading=14,
            textColor=colors.HexColor("#1e293b"),
            spaceBefore=8,
            spaceAfter=4,
        )
    )
    styles.add(
        ParagraphStyle(
            name="TitleCustom",
            parent=styles["Title"],
            fontName="Helvetica-Bold",
            fontSize=18,
            leading=22,
            alignment=TA_LEFT,
            textColor=colors.HexColor("#0f172a"),
            spaceAfter=14,
        )
    )
    styles.add(
        ParagraphStyle(
            name="Caption",
            parent=styles["BodyText"],
            fontName="Helvetica-Oblique",
            fontSize=8.2,
            leading=11,
            textColor=colors.HexColor("#64748b"),
            spaceBefore=4,
            spaceAfter=10,
            leftIndent=24,
            rightIndent=24,
        )
    )
    styles.add(
        ParagraphStyle(
            name="TableCell",
            parent=styles["BodyText"],
            fontName="Helvetica",
            fontSize=8.5,
            leading=10,
        )
    )
    styles.add(
        ParagraphStyle(
            name="CodeCell",
            parent=styles["BodyText"],
            fontName="Courier",
            fontSize=8.5,
            leading=10,
        )
    )
    styles.add(
        ParagraphStyle(
            name="TableHeader",
            parent=styles["Heading2"],
            fontName="Helvetica-Bold",
            fontSize=9.2,
            leading=11,
            textColor=colors.HexColor("#1e293b"),
            spaceBefore=0,
            spaceAfter=0,
        )
    )
    return styles


def add_paragraphs(story: list, styles, lines: list[str]) -> None:
    for line in lines:
        if line == "":
            story.append(Spacer(1, 0.08 * cm))
        elif line.startswith("- "):
            story.append(Paragraph(markup(line), styles["BulletBody"]))
        else:
            story.append(Paragraph(markup(line), styles["Body"]))


def add_lines(story: list, styles, lines: list[str], style_name: str) -> None:
    for line in lines:
        if line == "":
            story.append(Spacer(1, 0.08 * cm))
        else:
            story.append(Paragraph(markup(line), styles[style_name]))


def add_figure(
    story: list,
    styles,
    report_dir: Path,
    section_name: str,
    figure_title: str,
    max_width: float,
    max_height: float,
) -> None:
    figure = asset_block(section_name, "figure")
    story.append(SvgPlot(report_dir / f"{figure.asset_id}.svg", max_width, max_height))
    caption_lines = list(figure.lines)
    if caption_lines:
        caption_lines[0] = f"{figure_title}. {caption_lines[0]}"
    else:
        caption_lines = [f"{figure_title}."]
    add_lines(story, styles, caption_lines, "Caption")


def figure_flowables(
    styles,
    report_dir: Path,
    section_name: str,
    figure_title: str,
    max_width: float,
    max_height: float,
) -> list:
    figure = asset_block(section_name, "figure")
    caption_lines = list(figure.lines)
    if caption_lines:
        caption_lines[0] = f"{figure_title}. {caption_lines[0]}"
    else:
        caption_lines = [f"{figure_title}."]
    flowables: list = [SvgPlot(report_dir / f"{figure.asset_id}.svg", max_width, max_height)]
    for line in caption_lines:
        if line == "":
            flowables.append(Spacer(1, 0.08 * cm))
        else:
            flowables.append(Paragraph(markup(line), styles["Caption"]))
    return flowables


def make_table(column_labels: list[str], rows: list[list[str]], styles, col_widths: list[float]) -> Table:
    data = [[Paragraph(markup(label), styles["TableHeader"]) for label in column_labels]]
    for row in rows:
        converted = []
        for value in row:
            style = (
                styles["CodeCell"]
                if value.startswith("`") and value.endswith("`")
                else styles["TableCell"]
            )
            text = value[1:-1] if value.startswith("`") and value.endswith("`") else value
            converted.append(Paragraph(markup(text), style))
        data.append(converted)
    table = Table(data, colWidths=col_widths, repeatRows=1, hAlign="LEFT")
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e2e8f0")),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.HexColor("#0f172a")),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("LINEBELOW", (0, 0), (-1, 0), 0.7, colors.HexColor("#94a3b8")),
                ("GRID", (0, 1), (-1, -1), 0.35, colors.HexColor("#cbd5e1")),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#f8fafc")]),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 5),
                ("RIGHTPADDING", (0, 0), (-1, -1), 5),
                ("TOPPADDING", (0, 0), (-1, -1), 5),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
                ("ALIGN", (2, 1), (-1, -1), "RIGHT"),
            ]
        )
    )
    return table


def write_pdf_report(
    artifact_dir: Path,
    report_dir: Path,
    recommendations,
    profile_recommendations,
    transition_metrics,
    boundary_summary,
    aggregates,
    comparison_summary,
    head_to_head_summary,
    baseline_comparison,
    baseline_dir,
) -> None:
    styles = build_styles()
    doc = SimpleDocTemplate(
        str(report_dir / "tuning_report.pdf"),
        pagesize=A4,
        leftMargin=2.2 * cm,
        rightMargin=2.2 * cm,
        topMargin=2.0 * cm,
        bottomMargin=2.0 * cm,
        title="Jacquard Routing: Tuning and Analysis",
    )
    story: list = []

    story.append(Paragraph("Jacquard Routing: Tuning and Analysis", styles["TitleCustom"]))
    add_paragraphs(story, styles, executive_summary_lines(recommendations, aggregates, comparison_summary))
    story.append(Spacer(1, 0.15 * cm))
    story.append(Paragraph("Part I. Tuning", styles["Section"]))

    story.append(Paragraph("1. Recommended Configurations", styles["Section"]))
    recommendation_block = asset_block("Recommendation Overview", "table")
    add_paragraphs(story, styles, recommendation_block.lines)
    story.append(
        make_table(
            ["Engine", "Configuration", "Score", "Activation", "Route Presence", "Max Stress"],
            recommendation_table_rows(recommendations, 2),
            styles,
            [2.0 * cm, 5.3 * cm, 2.0 * cm, 2.3 * cm, 2.7 * cm, 2.1 * cm],
        )
    )
    story.append(Spacer(1, 0.16 * cm))
    story.append(
        KeepTogether(
            [
                Paragraph("2. Transition Behavior", styles["Section"]),
                *(
                    [Paragraph(markup(line), styles["Body"]) if line else Spacer(1, 0.08 * cm)
                     for line in asset_block("Transition Behavior", "table").lines]
                ),
                make_table(
                    ["Engine", "Configuration", "Route Mean", "Route Stddev", "First Mat.", "First Loss", "Recovery", "Churn"],
                    transition_table_rows(transition_metrics),
                    styles,
                    [1.8 * cm, 4.5 * cm, 1.9 * cm, 2.1 * cm, 1.8 * cm, 1.8 * cm, 1.8 * cm, 1.5 * cm],
                ),
            ]
        )
    )
    story.append(Spacer(1, 0.16 * cm))
    story.append(
        KeepTogether(
            [
                Paragraph("3. Failure Boundaries", styles["Section"]),
                *(
                    [Paragraph(markup(line), styles["Body"]) if line else Spacer(1, 0.08 * cm)
                     for line in asset_block("Failure Boundaries", "table").lines]
                ),
                make_table(
                    ["Engine", "Configuration", "Max Stress", "First Failed Family", "Fail Stress", "Reason"],
                    boundary_table_rows(boundary_summary),
                    styles,
                    [1.8 * cm, 4.3 * cm, 1.7 * cm, 4.5 * cm, 1.6 * cm, 2.2 * cm],
                ),
            ]
        )
    )
    story.append(PageBreak())

    story.append(Paragraph("4. Tuning Setup And Scoring", styles["Section"]))
    for heading, lines in [
        ("Simulation Setup", simulation_setup_lines()),
        ("Matrix Design", methodology_lines()),
        ("Regime Assumptions", regime_assumption_lines()),
        ("Regime Characterization", regime_characterization_lines()),
        ("BATMAN Algorithm", batman_bellman_algorithm_lines()),
        ("Pathway Algorithm", pathway_algorithm_lines()),
        ("Field Algorithm", field_algorithm_lines()),
        ("Analytical Approach", approach_lines()),
        ("Recommendation Logic", scoring_lines()),
    ]:
        story.append(Paragraph(heading, styles["Subsection"]))
        add_paragraphs(story, styles, lines)
    story.append(Paragraph("Profile Recommendation Logic", styles["Subsection"]))
    add_paragraphs(story, styles, profile_recommendation_lines(profile_recommendations))
    add_paragraphs(
        story,
        styles,
        asset_block("Profile Recommendations", "table").lines,
    )
    story.append(
        make_table(
            ["Engine", "Profile", "Configuration", "Score", "Activation", "Route", "Max Stress"],
            profile_table_rows(profile_recommendations),
            styles,
            [1.8 * cm, 2.5 * cm, 4.4 * cm, 1.8 * cm, 2.0 * cm, 1.8 * cm, 1.8 * cm],
        )
    )

    story.append(PageBreak())
    story.append(Paragraph("Part II. Analysis", styles["Section"]))

    story.append(Paragraph("5. BATMAN Bellman Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "batman-bellman"))
    story.append(Paragraph("Recommendation Rationale", styles["Subsection"]))
    add_paragraphs(story, styles, recommendation_rationale_lines(recommendations, aggregates, "batman-bellman"))
    story.append(Paragraph("Transition Pressure Analysis", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("BATMAN Bellman Transition Analysis"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 1",
        "Figure 1. BATMAN Bellman stability across transition families",
        16.4 * cm,
        8.2 * cm,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 2",
        "Figure 2. BATMAN Bellman loss timing across transition families",
        16.4 * cm,
        7.4 * cm,
    )
    story.append(PageBreak())

    story.append(Paragraph("6. Pathway Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "pathway"))
    story.append(Paragraph("Recommendation Rationale", styles["Subsection"]))
    add_paragraphs(story, styles, recommendation_rationale_lines(recommendations, aggregates, "pathway"))
    story.append(Paragraph("Budget Figures", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Pathway Budget Figures Intro"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 3",
        "Figure 3. Pathway route presence by search budget",
        16.4 * cm,
        8.0 * cm,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 4",
        "Figure 4. Pathway activation cliffs by search budget",
        16.4 * cm,
        7.2 * cm,
    )
    story.append(PageBreak())

    story.append(Paragraph("7. Field Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "field"))
    story.append(Paragraph("Recommendation Rationale", styles["Subsection"]))
    add_paragraphs(story, styles, recommendation_rationale_lines(recommendations, aggregates, "field"))
    story.append(Paragraph("Corridor Figures", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Field Corridor Figures Intro"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 5",
        "Figure 5. Field route presence by search budget",
        16.4 * cm,
        9.6 * cm,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 6",
        "Figure 6. Field corridor reconfiguration by search budget",
        16.4 * cm,
        9.6 * cm,
    )
    story.append(PageBreak())

    story.append(Paragraph("8. Comparative Analysis", styles["Section"]))
    story.append(Paragraph("Mixed-Engine Comparison", styles["Subsection"]))
    add_paragraphs(story, styles, comparison_findings_lines(comparison_summary))
    story.append(Spacer(1, 0.12 * cm))
    story.append(Paragraph("Head-To-Head Engine Sets", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_findings_lines(head_to_head_summary))
    story.append(Paragraph("Head-To-Head Regimes", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_regime_lines())
    story.append(Paragraph("Limitations And Next Steps", styles["Subsection"]))
    add_paragraphs(story, styles, limitations_lines())
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            [
                Paragraph("Mixed-Engine Regime Split", styles["Subsection"]),
                *(
                    [Paragraph(markup(line), styles["Body"]) if line else Spacer(1, 0.08 * cm)
                     for line in asset_block("Mixed-Engine Regime Split", "table").lines]
                ),
                make_table(
                    ["Family", "Dominant Engine", "Activation", "Route Presence", "Stress"],
                    comparison_table_rows(comparison_summary),
                    styles,
                    [6.4 * cm, 3.1 * cm, 2.2 * cm, 3.0 * cm, 1.8 * cm],
                ),
            ]
        )
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            [
                Paragraph("Head-To-Head Results", styles["Subsection"]),
                *(
                    [Paragraph(markup(line), styles["Body"]) if line else Spacer(1, 0.08 * cm)
                     for line in asset_block("Head-To-Head Results", "table").lines]
                ),
                make_table(
                    ["Regime", "Engine Set", "Activation", "Route", "Dominant", "Stress"],
                    head_to_head_table_rows(head_to_head_summary),
                    styles,
                    [5.6 * cm, 3.2 * cm, 2.0 * cm, 1.8 * cm, 2.1 * cm, 1.4 * cm],
                ),
            ]
        )
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            figure_flowables(
                styles,
                report_dir,
                "Figure 7",
                "Figure 7. Dominant engine by comparison regime",
                14.8 * cm,
                10.2 * cm,
            )
        )
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            figure_flowables(
                styles,
                report_dir,
                "Figure 8",
                "Figure 8. Head-to-head route presence by engine set",
                16.4 * cm,
                10.2 * cm,
            )
        )
    )

    doc.build(story)
