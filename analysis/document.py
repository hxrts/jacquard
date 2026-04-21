"""ReportLab PDF report builder: paragraph styles, SVG plot embeds, table layout, and full document assembly."""

from __future__ import annotations

import html
import re
from dataclasses import dataclass
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
    analysis_takeaway_lines,
    asset_block,
    asset_block_by_id,
    babel_algorithm_lines,
    batman_bellman_algorithm_lines,
    batman_classic_algorithm_lines,
    comparison_findings_lines,
    document_title,
    diffusion_field_posture_lines,
    diffusion_takeaway_lines,
    engine_section_lines,
    executive_summary_lines,
    field_algorithm_lines,
    head_to_head_findings_lines,
    head_to_head_regime_lines,
    head_to_head_takeaway_lines,
    limitations_lines,
    large_population_takeaway_lines,
    methodology_lines,
    olsrv2_algorithm_lines,
    pathway_algorithm_lines,
    regime_assumption_lines,
    regime_characterization_lines,
    routing_fitness_takeaway_lines,
    scatter_algorithm_lines,
    section_lines,
    scoring_lines,
    simulation_setup_lines,
)
from .tables import (
    benchmark_profile_audit_table_rows,
    comparison_config_sensitivity_table_rows,
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
    large_population_diffusion_transition_table_rows,
    large_population_route_summary_table_rows,
    profile_table_rows,
    recommendation_table_rows,
    routing_fitness_crossover_table_rows,
    routing_fitness_multiflow_table_rows,
    routing_fitness_stale_repair_table_rows,
    transition_table_rows,
    boundary_table_rows,
)

REPORT_TEXT_COLOR = colors.HexColor("#000000")
REPORT_CAPTION_COLOR = colors.HexColor("#6b6b6b")
REPORT_TABLE_HEADER_COLOR = colors.HexColor("#7a7a7a")
REPORT_TABLE_STRIPE_COLOR = colors.HexColor("#f0f0f0")
REPORT_INVERSE_TEXT_COLOR = colors.HexColor("#ffffff")


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
    svg_path = report_dir / f"{asset_id}.svg"
    if svg_path.exists():
        return SvgPlot(svg_path, max_width, max_height)
    png_path = report_dir / f"{asset_id}.png"
    reader = ImageReader(str(png_path))
    width_px, height_px = reader.getSize()
    scale = min(max_width / width_px, max_height / height_px)
    image = Image(str(png_path))
    image.drawWidth = width_px * scale
    image.drawHeight = height_px * scale
    return image


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
            textColor=REPORT_TEXT_COLOR,
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
            textColor=REPORT_TEXT_COLOR,
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
            textColor=REPORT_TEXT_COLOR,
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
            textColor=REPORT_CAPTION_COLOR,
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
            story.append(Spacer(1, INLINE_SPACER))
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

PAGE_MARGIN_LEFT = 2.5 * cm
PAGE_MARGIN_RIGHT = 2.5 * cm
PAGE_MARGIN_TOP = 2.2 * cm
PAGE_MARGIN_BOTTOM = 2.2 * cm
TABLE_WIDTH = 16.6 * cm
INLINE_SPACER = 0.08 * cm
TITLE_SPACER = 0.15 * cm
SECTION_SPACER = 0.12 * cm
BLOCK_SPACER = 0.16 * cm
FIGURE_BLOCK_SPACER = 0.18 * cm
TABLE_CAPTION_SPACER = 0.14 * cm


@dataclass(frozen=True)
class FigureLayout:
    width: float
    height: float
    keep_together: bool = False


FIGURE_LAYOUT_STANDARD = FigureLayout(16.4 * cm, 7.4 * cm)
FIGURE_LAYOUT_TALL = FigureLayout(16.4 * cm, 8.6 * cm)
FIGURE_LAYOUT_SCATTER = FigureLayout(16.4 * cm, 9.4 * cm)
FIGURE_LAYOUT_FIELD = FigureLayout(16.4 * cm, 10.2 * cm)
FIGURE_LAYOUT_COMPARISON_TILE = FigureLayout(14.8 * cm, 10.2 * cm, keep_together=True)
FIGURE_LAYOUT_COMPARISON_BAR = FigureLayout(16.4 * cm, 10.2 * cm, keep_together=True)
FIGURE_LAYOUT_COMPARISON_HEATMAP = FigureLayout(17.2 * cm, 9.6 * cm, keep_together=True)
FIGURE_LAYOUT_COMPARISON_SCATTER = FigureLayout(15.4 * cm, 9.0 * cm, keep_together=True)
FIGURE_LAYOUT_COMPARISON_DIVERGENCE = FigureLayout(16.8 * cm, 9.2 * cm, keep_together=True)
FIGURE_LAYOUT_DIFFUSION = FigureLayout(18.0 * cm, 22.0 * cm)
FIGURE_LAYOUT_LARGE_POP_ROUTE = FigureLayout(16.6 * cm, 9.8 * cm, keep_together=True)
FIGURE_LAYOUT_LARGE_POP_DIFFUSION = FigureLayout(17.2 * cm, 10.8 * cm, keep_together=True)
FIGURE_LAYOUT_ROUTING_FITNESS = FigureLayout(16.6 * cm, 9.6 * cm, keep_together=True)

FIGURE_LAYOUTS: dict[str, FigureLayout] = {
    "batman_classic_transition_stability": FIGURE_LAYOUT_STANDARD,
    "batman_classic_transition_loss": FIGURE_LAYOUT_STANDARD,
    "batman_bellman_transition_stability": FIGURE_LAYOUT_STANDARD,
    "batman_bellman_transition_loss": FIGURE_LAYOUT_STANDARD,
    "babel_decay_stability": FIGURE_LAYOUT_STANDARD,
    "babel_decay_loss": FIGURE_LAYOUT_STANDARD,
    "olsrv2_decay_stability": FIGURE_LAYOUT_TALL,
    "olsrv2_decay_loss": FIGURE_LAYOUT_TALL,
    "scatter_profile_route_presence": FIGURE_LAYOUT_SCATTER,
    "scatter_profile_runtime": FIGURE_LAYOUT_SCATTER,
    "pathway_budget_route_presence": FIGURE_LAYOUT_STANDARD,
    "pathway_budget_activation": FIGURE_LAYOUT_STANDARD,
    "field_budget_route_presence": FIGURE_LAYOUT_FIELD,
    "field_budget_reconfiguration": FIGURE_LAYOUT_FIELD,
    "comparison_dominant_engine": FIGURE_LAYOUT_COMPARISON_TILE,
    "head_to_head_route_presence": FIGURE_LAYOUT_COMPARISON_BAR,
    "head_to_head_timing_profile": FIGURE_LAYOUT_COMPARISON_HEATMAP,
    "recommended_engine_robustness": FIGURE_LAYOUT_COMPARISON_SCATTER,
    "mixed_vs_standalone_divergence": FIGURE_LAYOUT_COMPARISON_DIVERGENCE,
    "diffusion_delivery_coverage": FIGURE_LAYOUT_DIFFUSION,
    "diffusion_resource_boundedness": FIGURE_LAYOUT_DIFFUSION,
    "large_population_route_scaling": FIGURE_LAYOUT_LARGE_POP_ROUTE,
    "large_population_route_fragility": FIGURE_LAYOUT_LARGE_POP_ROUTE,
    "large_population_diffusion_transitions": FIGURE_LAYOUT_LARGE_POP_DIFFUSION,
    "routing_fitness_crossover": FIGURE_LAYOUT_ROUTING_FITNESS,
    "routing_fitness_multiflow": FIGURE_LAYOUT_ROUTING_FITNESS,
    "routing_fitness_stale_repair": FIGURE_LAYOUT_ROUTING_FITNESS,
}

PART_I_SETUP_SECTIONS = [
    ("Simulation Setup", simulation_setup_lines),
    ("Matrix Design", methodology_lines),
    ("Regime Assumptions", regime_assumption_lines),
    ("Regime Characterization", regime_characterization_lines),
    ("BATMAN Classic Algorithm", batman_classic_algorithm_lines),
    ("BATMAN Bellman Algorithm", batman_bellman_algorithm_lines),
    ("Babel Algorithm", babel_algorithm_lines),
    ("OLSRv2 Algorithm", olsrv2_algorithm_lines),
    ("Scatter Algorithm", scatter_algorithm_lines),
    ("Pathway Algorithm", pathway_algorithm_lines),
    ("Field Algorithm", field_algorithm_lines),
    ("Recommendation Logic", scoring_lines),
]

ENGINE_ANALYSIS_SECTIONS = [
    {
        "title": "3. BATMAN Classic Analysis",
        "engine_family": "batman-classic",
        "context_heading": "Transition Pressure Analysis",
        "context_section": "BATMAN Classic Transition Analysis",
        "figure_ids": (
            "batman_classic_transition_stability",
            "batman_classic_transition_loss",
        ),
    },
    {
        "title": "4. BATMAN Bellman Analysis",
        "engine_family": "batman-bellman",
        "context_heading": "Transition Pressure Analysis",
        "context_section": "BATMAN Bellman Transition Analysis",
        "figure_ids": (
            "batman_bellman_transition_stability",
            "batman_bellman_transition_loss",
        ),
    },
    {
        "title": "5. Babel Analysis",
        "engine_family": "babel",
        "context_heading": "Decay Window And Feasibility Analysis",
        "context_section": "Babel Decay Analysis",
        "figure_ids": ("babel_decay_stability", "babel_decay_loss"),
    },
    {
        "title": "6. OLSRv2 Analysis",
        "engine_family": "olsrv2",
        "context_heading": "Topology Propagation And Churn Analysis",
        "context_section": "OLSRv2 Decay Analysis",
        "figure_ids": ("olsrv2_decay_stability", "olsrv2_decay_loss"),
    },
    {
        "title": "7. Scatter Analysis",
        "engine_family": "scatter",
        "context_heading": None,
        "context_section": "Scatter Profile Figures Intro",
        "figure_ids": ("scatter_profile_route_presence", "scatter_profile_runtime"),
    },
    {
        "title": "8. Pathway Analysis",
        "engine_family": "pathway",
        "context_heading": "Budget Figures",
        "context_section": "Pathway Budget Figures Intro",
        "figure_ids": ("pathway_budget_route_presence", "pathway_budget_activation"),
    },
    {
        "title": "9. Field Analysis",
        "engine_family": "field",
        "context_heading": "Corridor Figures",
        "context_section": "Field Corridor Figures Intro",
        "figure_ids": ("field_budget_route_presence", "field_budget_reconfiguration"),
    },
]

COMPARISON_FIGURE_IDS = [
    "comparison_dominant_engine",
    "head_to_head_route_presence",
    "head_to_head_timing_profile",
    "recommended_engine_robustness",
    "mixed_vs_standalone_divergence",
]

DIFFUSION_FIGURE_IDS = [
    "diffusion_delivery_coverage",
    "diffusion_resource_boundedness",
]

LARGE_POPULATION_FIGURE_IDS = [
    "large_population_route_scaling",
    "large_population_route_fragility",
    "large_population_diffusion_transitions",
]

ROUTING_FITNESS_FIGURE_IDS = [
    "routing_fitness_crossover",
    "routing_fitness_multiflow",
    "routing_fitness_stale_repair",
]


def has_figure_asset(report_dir: Path, asset_id: str) -> bool:
    return (report_dir / f"{asset_id}.svg").exists() or (report_dir / f"{asset_id}.png").exists()


def available_figure_ids(report_dir: Path, asset_ids: list[str] | tuple[str, ...]) -> list[str]:
    return [asset_id for asset_id in asset_ids if has_figure_asset(report_dir, asset_id)]


def caption_lines_with_label(label: str, lines: list[str]) -> list[str]:
    caption_lines = list(lines) if lines else []
    while caption_lines and caption_lines[0] == "":
        caption_lines.pop(0)
    if caption_lines:
        caption_lines[0] = f"{label}. {caption_lines[0]}"
        return caption_lines
    return [f"{label}."]


def caption_lines_with_inline_label(label: str, lines: list[str]) -> list[str]:
    caption_lines = list(lines) if lines else []
    while caption_lines and caption_lines[0] == "":
        caption_lines.pop(0)
    if caption_lines:
        caption_lines[0] = f"{label}.{chr(160)}{caption_lines[0]}"
        return caption_lines
    return [f"{label}."]


def paragraph_flowables(styles, lines: list[str], style_name: str) -> list:
    flowables: list = []
    for line in lines:
        if line == "":
            flowables.append(Spacer(1, INLINE_SPACER))
        else:
            flowables.append(Paragraph(markup(line), styles[style_name]))
    return flowables


def figure_caption_flowables(styles, asset_id: str) -> list:
    block = asset_block_by_id(asset_id, "figure")
    return paragraph_flowables(
        styles,
        caption_lines_with_inline_label(block.section_title, block.lines),
        "Caption",
    )


def figure_asset_flowables(
    styles,
    report_dir: Path,
    asset_id: str,
    layout: FigureLayout,
) -> list:
    flowables: list = [
        figure_flowable(report_dir, asset_id, layout.width, layout.height),
        *figure_caption_flowables(styles, asset_id),
    ]
    return flowables


def add_figure_asset(
    story: list,
    styles,
    report_dir: Path,
    asset_id: str,
) -> None:
    layout = FIGURE_LAYOUTS[asset_id]
    flowables = figure_asset_flowables(styles, report_dir, asset_id, layout)
    if layout.keep_together:
        story.append(KeepTogether(flowables))
    else:
        story.extend(flowables)


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
                ("BACKGROUND", (0, 0), (-1, 0), REPORT_TABLE_HEADER_COLOR),
                ("TEXTCOLOR", (0, 0), (-1, 0), REPORT_INVERSE_TEXT_COLOR),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, REPORT_TABLE_STRIPE_COLOR]),
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
    return [
        Spacer(1, TABLE_CAPTION_SPACER),
        *paragraph_flowables(
            styles,
            caption_lines_with_label(f"Table {table_number}", description_lines),
            "Caption",
        ),
    ]


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
    story.append(Spacer(1, BLOCK_SPACER))


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
    comparison_config_sensitivity,
    head_to_head_summary,
    diffusion_engine_summary,
    diffusion_baseline_audit,
    diffusion_weight_sensitivity,
    diffusion_regime_engine_summary,
    diffusion_engine_comparison,
    diffusion_boundary_summary,
    large_population_route_summary,
    routing_fitness_crossover_summary,
    routing_fitness_multiflow_summary,
    routing_fitness_stale_repair_summary,
    large_population_diffusion_transitions,
    field_diffusion_regime_calibration,
    field_vs_best_diffusion_alternative,
    baseline_comparison,
    baseline_dir,
) -> None:
    styles = build_styles()
    report_title = document_title()
    doc = SimpleDocTemplate(
        str(pdf_path),
        pagesize=A4,
        leftMargin=PAGE_MARGIN_LEFT,
        rightMargin=PAGE_MARGIN_RIGHT,
        topMargin=PAGE_MARGIN_TOP,
        bottomMargin=PAGE_MARGIN_BOTTOM,
        title=report_title,
    )
    story: list = []
    table_counter = [0]

    story.append(Paragraph(report_title, styles["TitleCustom"]))
    add_paragraphs(story, styles, executive_summary_lines(recommendations, aggregates, comparison_summary))
    story.append(Paragraph("Design Setting", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Design Setting"))
    story.append(Spacer(1, TITLE_SPACER))
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
    add_paragraphs(story, styles, section_lines("Recommendation Detail Note"))

    story.append(Paragraph("2. Tuning Setup And Scoring", styles["Section"]))
    for heading, lines_fn in PART_I_SETUP_SECTIONS:
        story.append(Paragraph(heading, styles["Subsection"]))
        add_paragraphs(story, styles, lines_fn())
    story.append(Paragraph("Reference Material", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Tuning Reference Material"))

    story.append(Paragraph("Part II. Analysis", styles["Section"]))
    story.append(Paragraph("Reading Guide", styles["Subsection"]))
    add_paragraphs(story, styles, section_lines("Part II Reading Guide"))

    for section_spec in ENGINE_ANALYSIS_SECTIONS:
        story.append(Paragraph(section_spec["title"], styles["Section"]))
        story.append(Paragraph("Findings", styles["Subsection"]))
        add_paragraphs(
            story,
            styles,
            engine_section_lines(
                recommendations,
                aggregates,
                section_spec["engine_family"],
            ),
        )
        if section_spec["context_heading"] is not None:
            story.append(Paragraph(section_spec["context_heading"], styles["Subsection"]))
        add_paragraphs(story, styles, section_lines(section_spec["context_section"]))
        for asset_id in available_figure_ids(report_dir, section_spec["figure_ids"]):
            add_figure_asset(story, styles, report_dir, asset_id)

    story.append(Paragraph("10. Comparative Analysis", styles["Section"]))
    story.append(Paragraph("Mixed-Engine Comparison", styles["Subsection"]))
    add_paragraphs(story, styles, comparison_findings_lines(comparison_summary))
    story.append(Spacer(1, SECTION_SPACER))
    story.append(Paragraph("Head-To-Head Engine Sets", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_findings_lines(head_to_head_summary))
    story.append(Paragraph("Head-To-Head Regimes", styles["Subsection"]))
    add_paragraphs(story, styles, head_to_head_regime_lines())
    story.append(Paragraph("Limitations And Next Steps", styles["Subsection"]))
    add_paragraphs(story, styles, limitations_lines())
    add_paragraphs(story, styles, section_lines("Comparison Detail Note"))
    story.append(Spacer(1, FIGURE_BLOCK_SPACER))
    for asset_id in available_figure_ids(report_dir, COMPARISON_FIGURE_IDS):
        add_figure_asset(story, styles, report_dir, asset_id)
        story.append(Spacer(1, FIGURE_BLOCK_SPACER))
    add_paragraphs(story, styles, head_to_head_takeaway_lines(head_to_head_summary))
    story.append(Paragraph("Part II Takeaways", styles["Subsection"]))
    add_paragraphs(
        story,
        styles,
        analysis_takeaway_lines(recommendations, comparison_summary, head_to_head_summary),
    )

    if not large_population_route_summary.is_empty() or not large_population_diffusion_transitions.is_empty():
        story.append(Paragraph("11. Large-Population Findings", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Large-Population Introduction"))
        if not large_population_route_summary.is_empty():
            large_route_block = asset_block("Large-Population Route Summary", "table")
            add_table_section(
                story,
                styles,
                "Large-Population Route Summary",
                large_route_block.lines,
                ["Topology", "Engine Set", "Small", "Moderate", "High", "dHigh", "High Loss"],
                large_population_route_summary_table_rows(large_population_route_summary),
                [4.2 * cm, 3.5 * cm, 1.5 * cm, 1.8 * cm, 1.5 * cm, 1.7 * cm, 1.6 * cm],
                table_counter,
                large_route_block.description_lines,
            )
        if not large_population_diffusion_transitions.is_empty():
            large_diffusion_block = asset_block("Large-Population Diffusion Transitions", "table")
            add_table_section(
                story,
                styles,
                "Large-Population Diffusion Transitions",
                large_diffusion_block.lines,
                ["Question", "Size", "Collapse", "Viable", "Explosive"],
                large_population_diffusion_transition_table_rows(
                    large_population_diffusion_transitions
                ),
                [4.0 * cm, 1.7 * cm, 3.4 * cm, 3.4 * cm, 3.4 * cm],
                table_counter,
                large_diffusion_block.description_lines,
            )
        story.append(Paragraph("Large-Population Figure Context", styles["Subsection"]))
        add_paragraphs(story, styles, section_lines("Large-Population Figure Context"))
        for asset_id in available_figure_ids(report_dir, LARGE_POPULATION_FIGURE_IDS):
            add_figure_asset(story, styles, report_dir, asset_id)
        story.append(Paragraph("Large-Population Takeaways", styles["Subsection"]))
        add_paragraphs(
            story,
            styles,
            large_population_takeaway_lines(
                large_population_route_summary, large_population_diffusion_transitions
            ),
        )

    if (
        not routing_fitness_crossover_summary.is_empty()
        or not routing_fitness_multiflow_summary.is_empty()
        or not routing_fitness_stale_repair_summary.is_empty()
    ):
        story.append(Paragraph("12. Routing-Fitness Remaining Questions", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Routing-Fitness Introduction"))
        if not routing_fitness_crossover_summary.is_empty():
            crossover_block = asset_block("Routing-Fitness Crossover Summary", "table")
            add_table_section(
                story,
                styles,
                "Routing-Fitness Crossover Summary",
                crossover_block.lines,
                ["Question", "Band", "Engine Set", "Route", "Loss", "Churn", "Hop"],
                routing_fitness_crossover_table_rows(routing_fitness_crossover_summary),
                [3.2 * cm, 1.8 * cm, 3.0 * cm, 1.5 * cm, 1.3 * cm, 1.4 * cm, 1.3 * cm],
                table_counter,
                crossover_block.description_lines,
            )
        if not routing_fitness_multiflow_summary.is_empty():
            multiflow_block = asset_block("Routing-Fitness Multi-Flow Summary", "table")
            add_table_section(
                story,
                styles,
                "Routing-Fitness Multi-Flow Summary",
                multiflow_block.lines,
                ["Family", "Engine Set", "Min", "Max", "Spread", "Starved", "Broker P/C/S", "Live", "Churn"],
                routing_fitness_multiflow_table_rows(routing_fitness_multiflow_summary),
                [2.8 * cm, 2.6 * cm, 1.2 * cm, 1.2 * cm, 1.3 * cm, 1.4 * cm, 1.5 * cm, 1.3 * cm, 1.3 * cm],
                table_counter,
                multiflow_block.description_lines,
            )
        if not routing_fitness_stale_repair_summary.is_empty():
            stale_block = asset_block("Routing-Fitness Stale Repair Summary", "table")
            add_table_section(
                story,
                styles,
                "Routing-Fitness Stale Repair Summary",
                stale_block.lines,
                ["Family", "Engine Set", "Persist", "Route", "Unrec.", "Status", "Loss", "Churn"],
                routing_fitness_stale_repair_table_rows(routing_fitness_stale_repair_summary),
                [2.8 * cm, 2.5 * cm, 1.2 * cm, 1.2 * cm, 1.2 * cm, 2.5 * cm, 1.1 * cm, 1.2 * cm],
                table_counter,
                stale_block.description_lines,
            )
        story.append(Paragraph("Routing-Fitness Figure Context", styles["Subsection"]))
        add_paragraphs(story, styles, section_lines("Routing-Fitness Figure Context"))
        for asset_id in available_figure_ids(report_dir, ROUTING_FITNESS_FIGURE_IDS):
            add_figure_asset(story, styles, report_dir, asset_id)
        story.append(Paragraph("Routing-Fitness Takeaways", styles["Subsection"]))
        add_paragraphs(
            story,
            styles,
            routing_fitness_takeaway_lines(
                routing_fitness_crossover_summary,
                routing_fitness_multiflow_summary,
                routing_fitness_stale_repair_summary,
            ),
        )

    if not diffusion_engine_summary.is_empty():
        story.append(Paragraph("Part III. Diffusion Calibration", styles["Section"]))
        story.append(Paragraph("13. Diffusion Calibration", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Diffusion Calibration Introduction"))
        add_paragraphs(story, styles, section_lines("Diffusion Calibration Detail Note"))
        story.append(Paragraph("Part IV. Diffusion Engine Comparison", styles["Section"]))
        story.append(Paragraph("14. Diffusion Engine Comparison", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Diffusion Analysis Introduction"))
        story.append(Spacer(1, FIGURE_BLOCK_SPACER))
        diffusion_regime_block = asset_block("Diffusion Regime Comparison", "table")
        table_counter[0] += 1
        story.append(
            KeepTogether(
                [
                    Paragraph("Diffusion Regime Comparison", styles["Subsection"]),
                    *paragraph_flowables(styles, diffusion_regime_block.lines, "Body"),
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
        story.append(Spacer(1, FIGURE_BLOCK_SPACER))
        add_paragraphs(story, styles, section_lines("Diffusion Appendix Note"))
        story.append(Spacer(1, FIGURE_BLOCK_SPACER))
        story.append(Paragraph("Diffusion Figure Context", styles["Subsection"]))
        add_paragraphs(story, styles, section_lines("Diffusion Figure Context"))
        for asset_id in available_figure_ids(report_dir, DIFFUSION_FIGURE_IDS):
            add_figure_asset(story, styles, report_dir, asset_id)
        story.append(Paragraph("Diffusion Takeaways", styles["Subsection"]))
        add_paragraphs(
            story,
            styles,
            diffusion_takeaway_lines(
                diffusion_regime_engine_summary,
                field_vs_best_diffusion_alternative,
            ),
        )
        add_paragraphs(story, styles, diffusion_field_posture_lines(diffusion_engine_comparison))

    story.append(Paragraph("Appendix A. Tuning Reference Tables", styles["Section"]))
    add_paragraphs(story, styles, section_lines("Tuning Reference Tables Intro"))
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
    add_paragraphs(story, styles, section_lines("Route-Visible Reference Tables Intro"))
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
    comparison_breakdown_block = asset_block("Mixed-Engine Selected-Round Breakdown", "table")
    add_table_section(
        story,
        styles,
        "Mixed-Engine Selected-Round Breakdown",
        comparison_breakdown_block.lines,
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
        comparison_breakdown_block.description_lines,
    )
    sensitivity_block = asset_block("Comparison Config Sensitivity Audit", "table")
    add_table_section(
        story,
        styles,
        "Comparison Config Sensitivity Audit",
        sensitivity_block.lines,
        ["Surface", "Family", "Class", "Configs", "Topline Sigs", "Selection Sigs"],
        comparison_config_sensitivity_table_rows(comparison_config_sensitivity),
        [2.0 * cm, 5.4 * cm, 3.0 * cm, 1.3 * cm, 2.0 * cm, 2.0 * cm],
        table_counter,
        sensitivity_block.description_lines,
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
    crossover_ref_block = asset_block("Routing-Fitness Crossover Summary", "table")
    add_table_section(
        story,
        styles,
        "Routing-Fitness Crossover Summary",
        crossover_ref_block.lines,
        ["Question", "Band", "Engine Set", "Route", "Loss", "Churn", "Hop"],
        routing_fitness_crossover_table_rows(routing_fitness_crossover_summary),
        [3.2 * cm, 1.8 * cm, 3.0 * cm, 1.5 * cm, 1.3 * cm, 1.4 * cm, 1.3 * cm],
        table_counter,
        crossover_ref_block.description_lines,
    )
    multiflow_ref_block = asset_block("Routing-Fitness Multi-Flow Summary", "table")
    add_table_section(
        story,
        styles,
        "Routing-Fitness Multi-Flow Summary",
        multiflow_ref_block.lines,
        ["Family", "Engine Set", "Min", "Max", "Spread", "Starved", "Broker P/C/S", "Live", "Churn"],
        routing_fitness_multiflow_table_rows(routing_fitness_multiflow_summary),
        [3.0 * cm, 2.8 * cm, 1.3 * cm, 1.3 * cm, 1.4 * cm, 1.5 * cm, 1.4 * cm, 1.4 * cm, 1.2 * cm],
        table_counter,
        multiflow_ref_block.description_lines,
    )
    stale_ref_block = asset_block("Routing-Fitness Stale Repair Summary", "table")
    add_table_section(
        story,
        styles,
        "Routing-Fitness Stale Repair Summary",
        stale_ref_block.lines,
        ["Family", "Engine Set", "Persist", "Route", "Unrec.", "Status", "Loss", "Churn"],
        routing_fitness_stale_repair_table_rows(routing_fitness_stale_repair_summary),
        [2.8 * cm, 2.5 * cm, 1.2 * cm, 1.2 * cm, 1.2 * cm, 2.5 * cm, 1.1 * cm, 1.2 * cm],
        table_counter,
        stale_ref_block.description_lines,
    )

    if not diffusion_engine_summary.is_empty():
        story.append(Paragraph("Appendix C. Diffusion Reference Tables", styles["Section"]))
        add_paragraphs(story, styles, section_lines("Diffusion Reference Tables Intro"))
        diffusion_cal_block = asset_block("Diffusion Calibration Summary", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Calibration Summary",
            diffusion_cal_block.lines,
            ["Family", "Engine Set", "Delivery", "Coverage", "Latency", "State", "Leak", "Max Leak", "Stress"],
            diffusion_engine_summary_table_rows(diffusion_engine_summary),
            [3.2 * cm, 3.7 * cm, 1.3 * cm, 1.3 * cm, 1.2 * cm, 1.6 * cm, 1.1 * cm, 2.5 * cm, 1.0 * cm],
            table_counter,
            diffusion_cal_block.description_lines,
        )
        diffusion_baseline_block = asset_block("Diffusion Baseline Audit", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Baseline Audit",
            diffusion_baseline_block.lines,
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
            diffusion_baseline_block.description_lines,
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
        diffusion_sensitivity_block = asset_block("Diffusion Winner Sensitivity", "table")
        add_table_section(
            story,
            styles,
            "Diffusion Winner Sensitivity",
            diffusion_sensitivity_block.lines,
            ["Family", "Balanced", "Delivery-Heavy", "Boundedness-Heavy", "Stable"],
            diffusion_weight_sensitivity_table_rows(diffusion_weight_sensitivity),
            [4.3 * cm, 3.3 * cm, 3.3 * cm, 3.5 * cm, 1.2 * cm],
            table_counter,
            diffusion_sensitivity_block.description_lines,
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
