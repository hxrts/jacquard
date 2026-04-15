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
    Image,
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)
from reportlab.platypus.flowables import Flowable
from reportlab.lib.utils import ImageReader
from svglib.svglib import svg2rlg

from .sections import (
    approach_lines,
    analysis_takeaway_lines,
    asset_block,
    babel_algorithm_lines,
    batman_bellman_algorithm_lines,
    batman_classic_algorithm_lines,
    comparison_findings_lines,
    diffusion_field_posture_lines,
    engine_section_lines,
    executive_summary_lines,
    field_algorithm_lines,
    head_to_head_findings_lines,
    head_to_head_regime_lines,
    head_to_head_takeaway_lines,
    limitations_lines,
    methodology_lines,
    olsrv2_algorithm_lines,
    pathway_algorithm_lines,
    profile_recommendation_lines,
    recommendation_rationale_lines,
    regime_assumption_lines,
    regime_characterization_lines,
    scatter_algorithm_lines,
    section_lines,
    scoring_lines,
    simulation_setup_lines,
)
from .tables import (
    benchmark_profile_audit_table_rows,
    comparison_engine_round_breakdown_table_rows,
    comparison_table_rows,
    diffusion_baseline_audit_table_rows,
    diffusion_boundary_table_rows,
    diffusion_engine_comparison_table_rows,
    diffusion_regime_engine_summary_table_rows,
    diffusion_engine_summary_table_rows,
    diffusion_weight_sensitivity_table_rows,
    field_vs_best_diffusion_alternative_table_rows,
    field_diffusion_regime_table_rows,
    field_profile_table_rows,
    field_routing_regime_table_rows,
    head_to_head_table_rows,
    profile_table_rows,
    recommendation_table_rows,
    transition_table_rows,
    boundary_table_rows,
)


def codeify_known_terms(text: str) -> str:
    terms = [
        "pathway-batman-bellman",
        "batman-classic",
        "batman-bellman",
        "babel",
        "olsrv2",
        "scatter",
        "pathway",
        "field",
    ]
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


def figure_flowable(report_dir: Path, asset_id: str, max_width: float, max_height: float):
    png_path = report_dir / f"{asset_id}.png"
    if png_path.exists():
        reader = ImageReader(str(png_path))
        width_px, height_px = reader.getSize()
        scale = min(max_width / width_px, max_height / height_px)
        image = Image(str(png_path))
        image.drawWidth = width_px * scale
        image.drawHeight = height_px * scale
        return image
    return SvgPlot(report_dir / f"{asset_id}.svg", max_width, max_height)


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
            leftIndent=9,
            bulletIndent=0,
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
            textColor=colors.HexColor("#000000"),
            spaceBefore=16,
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
            textColor=colors.HexColor("#000000"),
            spaceBefore=12,
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
            textColor=colors.HexColor("#000000"),
            spaceAfter=14,
        )
    )
    styles.add(
        ParagraphStyle(
            name="Caption",
            parent=styles["BodyText"],
            fontName="Helvetica",
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
            fontSize=8.5,
            leading=10,
            textColor=colors.HexColor("#ffffff"),
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
            bullet_text = line[2:]
            story.append(
                Paragraph(
                    markup(bullet_text),
                    styles["BulletBody"],
                    bulletText="\u2022",
                )
            )
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
    story.append(figure_flowable(report_dir, figure.asset_id, max_width, max_height))
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
    flowables: list = [figure_flowable(report_dir, figure.asset_id, max_width, max_height)]
    for line in caption_lines:
        if line == "":
            flowables.append(Spacer(1, 0.08 * cm))
        else:
            flowables.append(Paragraph(markup(line), styles["Caption"]))
    return flowables


TABLE_WIDTH = 16.6 * cm
PART_II_FIGURE_WIDTH = 16.4 * cm
PART_II_TRIPTYCH_HEIGHT = 7.4 * cm
PART_II_QUAD_HEIGHT = 8.6 * cm
PART_II_HEPTAD_HEIGHT = 9.4 * cm
PART_II_NINE_PANEL_HEIGHT = 10.2 * cm


def make_table(column_labels: list[str], rows: list[list[str]], styles, col_widths: list[float]) -> Table:
    total = sum(col_widths)
    if total > 0:
        col_widths = [w * TABLE_WIDTH / total for w in col_widths]
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
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#7a7a7a")),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.HexColor("#ffffff")),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#f0f0f0")]),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 4),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
                ("ALIGN", (2, 1), (-1, -1), "RIGHT"),
            ]
        )
    )
    return table


def table_caption_flowables(
    styles,
    table_number: int,
    description_lines: list[str],
) -> list:
    table_title = f"Table {table_number}"
    caption_lines = list(description_lines) if description_lines else []
    if caption_lines:
        caption_lines[0] = f"{table_title}. {caption_lines[0]}"
    else:
        caption_lines = [f"{table_title}."]
    flowables: list = []
    for line in caption_lines:
        if line == "":
            flowables.append(Spacer(1, 0.08 * cm))
        else:
            flowables.append(Paragraph(markup(line), styles["Caption"]))
    return flowables


def add_table_caption(
    story: list,
    styles,
    table_number: int,
    description_lines: list[str],
) -> None:
    story.extend(table_caption_flowables(styles, table_number, description_lines))


def add_table_section(
    story: list,
    styles,
    title: str,
    intro_lines: list[str],
    column_labels: list[str],
    rows: list[list[str]],
    col_widths: list[float],
    table_counter: list[int],
    description_lines: list[str] | None = None,
) -> None:
    story.append(Paragraph(title, styles["Subsection"]))
    add_paragraphs(story, styles, intro_lines)
    story.append(make_table(column_labels, rows, styles, col_widths))
    table_counter[0] += 1
    add_table_caption(story, styles, table_counter[0], description_lines or [])
    story.append(Spacer(1, 0.16 * cm))


def write_pdf_report(
    artifact_dir: Path,
    report_dir: Path,
    pdf_path: Path,
    recommendations,
    profile_recommendations,
    field_profile_recommendations,
    benchmark_profile_audit,
    field_routing_regime_calibration,
    transition_metrics,
    boundary_summary,
    aggregates,
    comparison_summary,
    comparison_engine_round_breakdown,
    head_to_head_summary,
    diffusion_engine_summary,
    diffusion_baseline_audit,
    diffusion_weight_sensitivity,
    diffusion_regime_engine_summary,
    diffusion_engine_comparison,
    diffusion_boundary_summary,
    field_diffusion_regime_calibration,
    field_vs_best_diffusion_alternative,
    baseline_comparison,
    baseline_dir,
) -> None:
    styles = build_styles()
    doc = SimpleDocTemplate(
        str(pdf_path),
        pagesize=A4,
        leftMargin=2.2 * cm,
        rightMargin=2.2 * cm,
        topMargin=2.0 * cm,
        bottomMargin=2.0 * cm,
        title="Jacquard Routing: Tuning and Analysis",
    )
    story: list = []
    table_counter = [0]

    story.append(Paragraph("Jacquard Routing: Tuning and Analysis", styles["TitleCustom"]))
    add_paragraphs(story, styles, executive_summary_lines(recommendations, aggregates, comparison_summary))
    story.append(Spacer(1, 0.15 * cm))
    story.append(Paragraph("Part I. Tuning", styles["Section"]))

    story.append(Paragraph("1. Recommended Configurations", styles["Section"]))
    recommendation_block = asset_block("Recommendation Overview", "table")
    add_paragraphs(story, styles, recommendation_block.lines)
    story.append(
        make_table(
            ["Engine Configuration", "Score", "Activation", "Route Presence", "Max Stress"],
            recommendation_table_rows(recommendations, 2),
            styles,
            [6.8 * cm, 1.8 * cm, 2.0 * cm, 2.4 * cm, 1.8 * cm],
        )
    )
    table_counter[0] += 1
    add_table_caption(story, styles, table_counter[0], recommendation_block.description_lines)
    add_paragraphs(
        story,
        styles,
        [
            "Detailed transition, failure-boundary, profile, and field-regime tables are collected in Appendix A so the main report can stay focused on the key recommendations and figures.",
        ],
    )

    story.append(Paragraph("2. Tuning Setup And Scoring", styles["Section"]))
    for heading, lines in [
        ("Simulation Setup", simulation_setup_lines()),
        ("Matrix Design", methodology_lines()),
        ("Regime Assumptions", regime_assumption_lines()),
        ("Regime Characterization", regime_characterization_lines()),
        ("BATMAN Classic Algorithm", batman_classic_algorithm_lines()),
        ("BATMAN Bellman Algorithm", batman_bellman_algorithm_lines()),
        ("Babel Algorithm", babel_algorithm_lines()),
        ("OLSRv2 Algorithm", olsrv2_algorithm_lines()),
        ("Scatter Algorithm", scatter_algorithm_lines()),
        ("Pathway Algorithm", pathway_algorithm_lines()),
        ("Field Algorithm", field_algorithm_lines()),
        ("Recommendation Logic", scoring_lines()),
    ]:
        story.append(Paragraph(heading, styles["Subsection"]))
        add_paragraphs(story, styles, lines)
    story.append(Paragraph("Reference Material", styles["Subsection"]))
    add_paragraphs(
        story,
        styles,
        [
            "Appendix A contains the detailed transition, failure-boundary, profile, and field-specific calibration tables that support the main tuning recommendation.",
        ],
    )

    story.append(Paragraph("Part II. Analysis", styles["Section"]))
    story.append(Paragraph("Reading Guide", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Part II Reading Guide"))

    story.append(Paragraph("3. BATMAN Classic Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "batman-classic"))
    story.append(Paragraph("Transition Pressure Analysis", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("BATMAN Classic Transition Analysis"))
    add_figure(
        story, styles, report_dir, "Figure 1", "Figure 1", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )
    add_figure(
        story, styles, report_dir, "Figure 2", "Figure 2", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )

    story.append(Paragraph("4. BATMAN Bellman Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "batman-bellman"))
    story.append(Paragraph("Transition Pressure Analysis", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("BATMAN Bellman Transition Analysis"))
    add_figure(
        story, styles, report_dir, "Figure 3", "Figure 3", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )
    add_figure(
        story, styles, report_dir, "Figure 4", "Figure 4", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )

    story.append(Paragraph("5. Babel Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "babel"))
    story.append(Paragraph("Decay Window And Feasibility Analysis", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Babel Decay Analysis"))
    add_figure(
        story, styles, report_dir, "Figure 5", "Figure 5", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )
    add_figure(
        story, styles, report_dir, "Figure 6", "Figure 6", PART_II_FIGURE_WIDTH, PART_II_TRIPTYCH_HEIGHT
    )

    story.append(Paragraph("6. OLSRv2 Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "olsrv2"))
    story.append(Paragraph("Topology Propagation And Churn Analysis", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("OLSRv2 Decay Analysis"))
    add_figure(
        story, styles, report_dir, "Figure 7", "Figure 7", PART_II_FIGURE_WIDTH, PART_II_QUAD_HEIGHT
    )
    add_figure(
        story, styles, report_dir, "Figure 8", "Figure 8", PART_II_FIGURE_WIDTH, PART_II_QUAD_HEIGHT
    )

    story.append(Paragraph("7. Scatter Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "scatter"))
    add_paragraphs(story, styles, section_lines("Scatter Profile Figures Intro"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 9",
        "Figure 9. Scatter active route presence by maintained profile. Higher values are better because they indicate the objective-visible route stays present for more of the active window. The y-axis is shown as a percentage so the outcome sweep reads like the other route-visible figures in Part II.",
        PART_II_FIGURE_WIDTH,
        PART_II_HEPTAD_HEIGHT,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 10",
        "Figure 10. Scatter startup timing by maintained profile. Lower values are better because routes materialize earlier. The dashed family lines reuse the same profile sweep as Figure 9 so startup cost can be compared directly against the route-visible outcome view.",
        PART_II_FIGURE_WIDTH,
        PART_II_HEPTAD_HEIGHT,
    )

    story.append(Paragraph("8. Pathway Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "pathway"))
    story.append(Paragraph("Budget Figures", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Pathway Budget Figures Intro"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 11",
        "Figure 11. Pathway active route presence by search budget. Higher values are better: they indicate the route is present for more of the objective-active window. Upward trends show budgets where additional search is still buying useful coverage; plateaus show diminishing returns.",
        PART_II_FIGURE_WIDTH,
        PART_II_TRIPTYCH_HEIGHT,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 12",
        "Figure 12. Pathway activation cliffs by search budget. Higher values are better: they indicate objectives activate successfully more often. Step changes reveal the budget threshold where Pathway moves from under-search to reliable activation.",
        PART_II_FIGURE_WIDTH,
        PART_II_TRIPTYCH_HEIGHT,
    )

    story.append(Paragraph("9. Field Analysis", styles["Section"]))
    story.append(Paragraph("Findings", styles["Subsection"]))
    add_paragraphs(story, styles, engine_section_lines(recommendations, aggregates, "field"))
    story.append(Paragraph("Corridor Figures", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Field Corridor Figures Intro"))
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 13",
        "Figure 13. Field active route presence by search budget. Higher values are better: they indicate the admitted corridor stays available for more of the active window. Rising curves show where additional search or heuristic guidance is still improving continuity; flat curves show the stable operating floor.",
        PART_II_FIGURE_WIDTH,
        PART_II_NINE_PANEL_HEIGHT,
    )
    add_figure(
        story,
        styles,
        report_dir,
        "Figure 14",
        "Figure 14. Field corridor reconfiguration by search budget. Lower values are generally better because they indicate less continuation churn and fewer search-driven reconfigurations. Rising lines mean the engine is paying more control-motion cost to maintain continuity.",
        PART_II_FIGURE_WIDTH,
        PART_II_NINE_PANEL_HEIGHT,
    )

    story.append(Paragraph("10. Comparative Analysis", styles["Section"]))
    story.append(Paragraph("Mixed-Engine Comparison", styles["Subsection"]))
    add_paragraphs(story, styles, comparison_findings_lines(comparison_summary))
    story.append(Spacer(1, 0.12 * cm))
    story.append(Paragraph("Head-To-Head Engine Sets", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_findings_lines(head_to_head_summary))
    story.append(Paragraph("Head-To-Head Regimes", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_regime_lines())
    story.append(Paragraph("Limitations And Next Steps", styles["Subsection"]))
    add_paragraphs(story, styles, limitations_lines())
    add_paragraphs(
        story,
        styles,
        [
            "The full mixed-engine and head-to-head tables are collected in Appendix B. The main body keeps the figures and takeaways.",
        ],
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            figure_flowables(
                styles,
                report_dir,
                "Figure 15",
                "Figure 15. Mixed-engine router arbitration by comparison regime. Tile color marks the engine the deterministic router selected most often in the mixed stack, while the overlaid percentage shows how dominant that choice was. This is an arbitration view, not a standalone performance comparison: close to 100% means the router effectively stuck with one engine for that regime, while lower percentages mean arbitration was more split.",
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
                "Figure 16",
                "Figure 16. Head-to-head standalone capability by comparison regime. Longer bars are better: they mark the engine with the highest total-window route presence when run alone, and bar width encodes that route-presence level directly. This is the standalone capability view for the same regime families, so a small `next lower gap` means the engines cluster tightly at the top while a large gap means the scenario cleanly separates the leading tier from the rest.",
                16.4 * cm,
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
                "Figure 17",
                "Figure 17. Head-to-head timing profile by regime and engine set. Earlier materialization is better in the left panel, while later first-loss rounds are better in the right panel. Blank first-loss cells mean no loss was observed in the maintained scenario window.",
                17.2 * cm,
                9.6 * cm,
            )
        )
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            figure_flowables(
                styles,
                report_dir,
                "Figure 18",
                "Figure 18. Recommended-engine robustness frontier. Farther right is better because route presence is higher, while lower is better because variability is lower. The best defaults sit toward the lower-right, where they combine strong coverage with less regime-to-regime spread.",
                15.4 * cm,
                9.0 * cm,
            )
        )
    )
    story.append(Spacer(1, 0.18 * cm))
    story.append(
        KeepTogether(
            figure_flowables(
                styles,
                report_dir,
                "Figure 19",
                "Figure 19. Mixed-vs-standalone route-presence divergence by regime. Longer bars are larger gaps: they show how much more total-window route presence the best standalone engine achieved than the mixed-stack outcome in the same regime. Bar color marks the standalone winner for that gap, while the overlaid label shows the mixed-engine choice versus the standalone winner.",
                16.8 * cm,
                9.2 * cm,
            )
        )
    )
    story.append(Spacer(1, 0.16 * cm))
    add_paragraphs(story, styles, head_to_head_takeaway_lines(head_to_head_summary))
    story.append(Paragraph("Part II Takeaways", styles["Subsection"]))
    add_paragraphs(
        story,
        styles,
        analysis_takeaway_lines(recommendations, comparison_summary, head_to_head_summary),
    )

    if not diffusion_engine_summary.is_empty():
        story.append(Paragraph("Part III. Diffusion Calibration", styles["Section"]))
        story.append(Paragraph("10. Diffusion Calibration", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Diffusion Calibration Introduction"))
        add_paragraphs(
            story,
            styles,
            [
                "Detailed diffusion calibration, field-calibration, and boundary tables are collected in Appendix C so the main comparison can stay focused on regime winners and the figure-level differences.",
            ],
        )
        story.append(Paragraph("Part IV. Diffusion Engine Comparison", styles["Section"]))
        story.append(Paragraph("11. Diffusion Engine Comparison", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Diffusion Analysis Introduction"))
        story.append(Spacer(1, 0.18 * cm))
        diffusion_regime_block = asset_block("Diffusion Regime Comparison", "table")
        table_counter[0] += 1
        story.append(
            KeepTogether(
                [
                    Paragraph("Diffusion Regime Comparison", styles["Subsection"]),
                    *(
                        [Paragraph(markup(line), styles["Body"]) if line else Spacer(1, 0.08 * cm)
                         for line in diffusion_regime_block.lines]
                    ),
                    make_table(
                        ["Regime", "Engine Set", "Delivery", "Coverage", "Cluster Cov.", "Tx", "State", "Score"],
                        diffusion_regime_engine_summary_table_rows(diffusion_regime_engine_summary),
                        styles,
                        [1.6 * cm, 4.0 * cm, 1.4 * cm, 1.4 * cm, 1.9 * cm, 1.1 * cm, 1.6 * cm, 1.4 * cm],
                    ),
                    *table_caption_flowables(styles, table_counter[0], diffusion_regime_block.description_lines),
                ]
            )
        )
        story.append(Spacer(1, 0.18 * cm))
        add_paragraphs(
            story,
            styles,
            [
                "Appendix C contains the full diffusion family matrix and the field-versus-best-alternative regime table.",
            ],
        )
        story.append(Spacer(1, 0.18 * cm))
        story.append(Paragraph("Diffusion Figure Context", styles["Subsection"]))
        add_paragraphs(story, styles, section_lines("Diffusion Figure Context"))
        add_figure(
            story,
            styles,
            report_dir,
            "Figure 20",
            "Figure 20. Diffusion delivery and coverage by scenario family. Longer bars are better because they indicate more successful delivery; the dot shows coverage, so a wider gap between bar and dot means delivery is lagging behind spread. Strong performers keep both high rather than trading one off against the other.",
            18.0 * cm,
            22.0 * cm,
        )
        add_figure(
            story,
            styles,
            report_dir,
            "Figure 21",
            "Figure 21. Diffusion transmission load and boundedness by scenario family. Lower transmission bars are better when delivery remains competitive because they indicate cheaper diffusion. The `R=` and bounded-state annotations show whether that load is staying inside the intended bounded operating regime or drifting toward over-spread.",
            18.0 * cm,
            22.0 * cm,
        )
        story.append(Paragraph("Diffusion Takeaways", styles["Subsection"]))
        add_paragraphs(story, styles, section_lines("Diffusion Takeaways"))
        add_paragraphs(story, styles, diffusion_field_posture_lines(diffusion_engine_comparison))

    story.append(Paragraph("Appendix A. Tuning Reference Tables", styles["Section"]))
    add_paragraphs(
        story,
        styles,
        [
            "These tables provide the detailed tuning reference material behind the main recommendation and analysis sections.",
        ],
    )
    transition_block = asset_block("Transition Behavior", "table")
    add_table_section(
        story,
        styles,
        "Transition Behavior",
        transition_block.lines,
        ["Engine", "Configuration", "Route Mean", "Route Stddev", "First Mat.", "First Loss", "Recov.", "Churn"],
        transition_table_rows(transition_metrics),
        [1.8 * cm, 4.5 * cm, 1.9 * cm, 2.1 * cm, 1.8 * cm, 1.8 * cm, 1.8 * cm, 1.5 * cm],
        table_counter,
        transition_block.description_lines,
    )
    boundary_block = asset_block("Failure Boundaries", "table")
    add_table_section(
        story,
        styles,
        "Failure Boundaries",
        boundary_block.lines,
        ["Engine", "Configuration", "Max Stress", "First Failed Family", "Fail Stress", "Reason"],
        boundary_table_rows(boundary_summary),
        [1.8 * cm, 4.3 * cm, 1.7 * cm, 4.5 * cm, 1.6 * cm, 2.2 * cm],
        table_counter,
        boundary_block.description_lines,
    )
    profile_block = asset_block("Profile Recommendations", "table")
    add_table_section(
        story,
        styles,
        "Profile Recommendations",
        profile_block.lines,
        ["Engine", "Profile", "Configuration", "Score", "Activation", "Route", "Max Stress"],
        profile_table_rows(profile_recommendations),
        [1.8 * cm, 2.5 * cm, 4.4 * cm, 1.8 * cm, 2.0 * cm, 1.8 * cm, 1.8 * cm],
        table_counter,
        profile_block.description_lines,
    )
    field_profile_block = asset_block("Field Continuity Profiles", "table")
    add_table_section(
        story,
        styles,
        "Field Continuity Profiles",
        field_profile_block.lines,
        ["Profile", "Configuration", "Score", "Route", "Shifts", "Carry", "Narrow", "Degraded"],
        field_profile_table_rows(field_profile_recommendations),
        [3.0 * cm, 4.6 * cm, 1.6 * cm, 1.5 * cm, 1.5 * cm, 1.8 * cm, 1.4 * cm, 1.6 * cm],
        table_counter,
        field_profile_block.description_lines,
    )
    field_regime_block = asset_block("Field Regime Calibration", "table")
    add_table_section(
        story,
        styles,
        "Field Regime Calibration",
        field_regime_block.lines,
        ["Regime", "Success Criteria", "Configuration", "Route", "Transition", "Shifts", "Carry", "Stress"],
        field_routing_regime_table_rows(field_routing_regime_calibration),
        [2.2 * cm, 5.1 * cm, 3.6 * cm, 1.2 * cm, 1.4 * cm, 1.2 * cm, 1.4 * cm, 1.1 * cm],
        table_counter,
        field_regime_block.description_lines,
    )

    story.append(Paragraph("Appendix B. Route-Visible Reference Tables", styles["Section"]))
    add_paragraphs(
        story,
        styles,
        [
            "These tables collect the exhaustive mixed-engine, mixed-engine selected-round breakdown, and head-to-head route-visible results referenced by the main comparison section.",
        ],
    )
    mixed_regime_block = asset_block("Mixed-Engine Regime Split", "table")
    add_table_section(
        story,
        styles,
        "Mixed-Engine Regime Split",
        mixed_regime_block.lines,
        ["Family", "Selected-Round Leader", "Activation", "Active Route", "Stress"],
        comparison_table_rows(comparison_summary),
        [6.4 * cm, 3.1 * cm, 2.2 * cm, 3.0 * cm, 1.8 * cm],
        table_counter,
        mixed_regime_block.description_lines,
    )
    add_table_section(
        story,
        styles,
        "Mixed-Engine Selected-Round Breakdown",
        [
            "Each row reports the best maintained mixed comparison config for that family. The per-engine columns show average selected-route rounds under one shared router policy, so this table explains why the mixed stack leader is not an oracle best-of-engines result.",
        ],
        [
            "Family",
            "Leader",
            "Active Route",
            "Handoffs",
            "Batman Classic",
            "Batman Bellman",
            "Babel",
            "OLSRv2",
            "Scatter",
            "Pathway",
            "Field",
        ],
        comparison_engine_round_breakdown_table_rows(comparison_engine_round_breakdown),
        [3.0 * cm, 1.5 * cm, 1.2 * cm, 1.0 * cm, 1.1 * cm, 1.1 * cm, 0.9 * cm, 0.9 * cm, 0.9 * cm, 0.9 * cm, 0.9 * cm],
        table_counter,
        [
            "Family is the comparison regime. Leader is the selected-round leader. Active Route is active-window route presence. Handoffs is mean engine handoff count. The remaining columns show mean selected-route rounds per engine under the shared router policy.",
        ],
    )
    benchmark_block = asset_block("Benchmark Profile Audit", "table")
    add_table_section(
        story,
        styles,
        "Benchmark Profile Audit",
        benchmark_block.lines,
        ["Engine Set", "Representative", "Benchmark Config", "Calibrated Profile", "Calibrated Config", "Match"],
        benchmark_profile_audit_table_rows(benchmark_profile_audit),
        [2.6 * cm, 2.8 * cm, 4.0 * cm, 2.8 * cm, 4.0 * cm, 1.4 * cm],
        table_counter,
        benchmark_block.description_lines,
    )
    h2h_block = asset_block("Head-To-Head Results", "table")
    add_table_section(
        story,
        styles,
        "Head-To-Head Results",
        h2h_block.lines,
        ["Regime", "Engine Set", "Activation", "Active Route", "Selected Leader", "Stress"],
        head_to_head_table_rows(head_to_head_summary),
        [5.6 * cm, 3.2 * cm, 2.0 * cm, 1.8 * cm, 2.1 * cm, 1.4 * cm],
        table_counter,
        h2h_block.description_lines,
    )

    if not diffusion_engine_summary.is_empty():
        story.append(Paragraph("Appendix C. Diffusion Reference Tables", styles["Section"]))
        add_paragraphs(
            story,
            styles,
            [
                "These tables hold the exhaustive diffusion calibration and comparison material that supports the shorter regime-level discussion in the main body.",
            ],
        )
        diffusion_cal_block = asset_block("Diffusion Calibration Summary", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Calibration Summary",
            diffusion_cal_block.lines,
            ["Family", "Engine Set", "Delivery", "Coverage", "Latency", "State", "Stress"],
            diffusion_engine_summary_table_rows(diffusion_engine_summary),
            [4.0 * cm, 4.6 * cm, 1.6 * cm, 1.6 * cm, 1.5 * cm, 2.0 * cm, 1.4 * cm],
            table_counter,
            diffusion_cal_block.description_lines,
        )
        add_table_section(
            story,
            styles,
            "Diffusion Baseline Audit",
            [
                "These rows summarize the maintained non-field diffusion baselines. They are representative benchmark configs, not a calibrated-best sweep, so the generic winner tables should be read with that scope in mind.",
            ],
            [
                "Config",
                "Rep",
                "TTL",
                "Forward",
                "Bridge",
                "Delivery",
                "Coverage",
                "Cluster",
                "State",
            ],
            diffusion_baseline_audit_table_rows(diffusion_baseline_audit),
            [3.6 * cm, 0.9 * cm, 0.9 * cm, 1.2 * cm, 1.1 * cm, 1.2 * cm, 1.2 * cm, 1.2 * cm, 1.3 * cm],
            table_counter,
            [
                "Config is the baseline configuration. Rep is the replication budget. TTL is the time-to-live in rounds. Forward is the forward probability. Bridge is the bridge bias. Delivery, Coverage, and Cluster are mean delivery, coverage, and cluster-coverage scores. State is the boundedness classification.",
            ],
        )
        diffusion_boundary_block = asset_block("Diffusion Calibration Boundaries", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Calibration Boundaries",
            diffusion_boundary_block.lines,
            ["Engine Set", "Viable Families", "First Collapse", "Collapse Stress", "First Explosive", "Explosive Stress"],
            diffusion_boundary_table_rows(diffusion_boundary_summary),
            [4.6 * cm, 2.0 * cm, 3.4 * cm, 1.8 * cm, 3.4 * cm, 1.8 * cm],
            table_counter,
            diffusion_boundary_block.description_lines,
        )
        field_diff_regime_block = asset_block("Field Diffusion Regime Calibration", "table")
        add_table_section(
            story,
            styles,
            "Field Diffusion Regime Calibration",
            field_diff_regime_block.lines,
            ["Regime", "Success Criteria", "Configuration", "Posture", "State", "Transition", "Delivery", "Tx", "Fit"],
            field_diffusion_regime_table_rows(field_diffusion_regime_calibration),
            [1.6 * cm, 4.8 * cm, 3.0 * cm, 1.9 * cm, 1.3 * cm, 1.7 * cm, 1.3 * cm, 1.0 * cm, 1.2 * cm],
            table_counter,
            field_diff_regime_block.description_lines,
        )
        field_alt_block = asset_block("Field Vs Best Alternative", "table")
        add_table_section(
            story,
            styles,
            "Field Vs Best Alternative",
            field_alt_block.lines,
            ["Regime", "Field", "OK", "State", "Alternative", "Alt State", "dDel", "dCov", "dClus", "dTx", "dScore"],
            field_vs_best_diffusion_alternative_table_rows(
                field_vs_best_diffusion_alternative
            ),
            [1.5 * cm, 3.0 * cm, 0.8 * cm, 1.3 * cm, 3.0 * cm, 1.5 * cm, 1.0 * cm, 1.0 * cm, 1.1 * cm, 0.9 * cm, 1.1 * cm],
            table_counter,
            field_alt_block.description_lines,
        )
        add_table_section(
            story,
            styles,
            "Diffusion Winner Sensitivity",
            [
                "This table re-scores the generic Part IV family winners under delivery-heavy and boundedness-heavy weights. A `no` in Stable means the family-level winner is sensitive to generic weighting and should be read as provisional relative to the regime-specific tables.",
            ],
            ["Family", "Balanced", "Delivery-Heavy", "Boundedness-Heavy", "Stable"],
            diffusion_weight_sensitivity_table_rows(diffusion_weight_sensitivity),
            [4.3 * cm, 3.3 * cm, 3.3 * cm, 3.5 * cm, 1.2 * cm],
            table_counter,
            [
                "Family is the diffusion scenario. Balanced, Delivery-Heavy, and Boundedness-Heavy show the winning configuration under each weighting. Stable indicates whether the winner is consistent across all three weightings.",
            ],
        )
        diffusion_comp_block = asset_block("Diffusion Engine Comparison", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Engine Comparison",
            diffusion_comp_block.lines,
            ["Family", "Engine Set", "Delivery", "Coverage", "Tx", "R_est", "State"],
            diffusion_engine_comparison_table_rows(diffusion_engine_comparison),
            [3.7 * cm, 4.8 * cm, 1.5 * cm, 1.5 * cm, 1.2 * cm, 1.5 * cm, 1.7 * cm],
            table_counter,
            diffusion_comp_block.description_lines,
        )

    doc.build(story)
