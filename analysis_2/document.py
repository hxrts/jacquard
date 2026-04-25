"""PDF assembly for the active-belief paper report."""

from __future__ import annotations

import re
from pathlib import Path

from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import cm
from reportlab.lib.utils import ImageReader
from reportlab.platypus import (
    Image,
    KeepTogether,
    PageBreak,
    Paragraph,
    Preformatted,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)

from analysis.document import (
    BLOCK_SPACER,
    FIGURE_BLOCK_SPACER,
    INLINE_SPACER,
    PAGE_MARGIN_BOTTOM,
    PAGE_MARGIN_LEFT,
    PAGE_MARGIN_RIGHT,
    PAGE_MARGIN_TOP,
    make_table,
    markup,
)
from analysis.document import (
    build_styles as build_analysis_styles,
)


def write_pdf_report(
    report_dir: Path,
    pdf_path: Path,
    manuscript_text: str,
    figure_specs: list[dict[str, object]],
    figure_rows: list[dict[str, object]],
) -> None:
    styles = build_styles()
    doc = SimpleDocTemplate(
        str(pdf_path),
        pagesize=A4,
        rightMargin=PAGE_MARGIN_RIGHT,
        leftMargin=PAGE_MARGIN_LEFT,
        topMargin=PAGE_MARGIN_TOP,
        bottomMargin=PAGE_MARGIN_BOTTOM,
    )
    story: list[object] = []
    story.append(
        Paragraph(
            paper_markup("Certified Temporal Kernel Transformation In Path-Free Distributed AI"),
            styles["TitleCustom"],
        )
    )
    story.append(Spacer(1, INLINE_SPACER))
    placed_exhibits = add_markdown(
        story,
        styles,
        strip_initial_h1(manuscript_text),
        report_dir,
        figure_specs,
    )
    add_unplaced_figures(story, styles, report_dir, figure_specs, placed_exhibits)
    add_manifest(story, styles, figure_specs)
    doc.build(story)


def build_styles():
    styles = build_analysis_styles()
    styles.add(
        ParagraphStyle(
            name="SmallCode",
            parent=styles["Code"],
            fontSize=7,
            leading=8,
            leftIndent=8,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ListCell",
            parent=styles["Body"],
            leftIndent=0,
            firstLineIndent=0,
            spaceBefore=0,
            spaceAfter=0,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ListMarker",
            parent=styles["Body"],
            leftIndent=0,
            firstLineIndent=0,
            spaceBefore=0,
            spaceAfter=0,
        )
    )
    return styles


def strip_initial_h1(markdown: str) -> str:
    lines = markdown.splitlines()
    if lines and lines[0].startswith("# "):
        return "\n".join(lines[1:]).lstrip()
    return markdown


def add_unplaced_figures(
    story: list[object],
    styles,
    report_dir: Path,
    figure_specs: list[dict[str, object]],
    placed_exhibits: set[str],
) -> None:
    remaining = [
        spec for spec in figure_specs if str(spec["figure_id"]) not in placed_exhibits
    ]
    if not remaining:
        return
    story.append(PageBreak())
    story.append(Paragraph("Supplementary Figures And Tables", styles["Section"]))
    for spec in remaining:
        story.append(KeepTogether(figure_or_table_flowables(styles, report_dir, spec)))
        story.append(Spacer(1, FIGURE_BLOCK_SPACER))


def figure_or_table_flowables(
    styles, report_dir: Path, spec: dict[str, object]
) -> list[object]:
    display_kind = str(spec.get("display_kind", "figure"))
    label = "Table" if display_kind == "table" else "Figure"
    flowables: list[object] = []
    if display_kind in {"figure", "figure-with-table"}:
        flowables.append(
            active_figure_flowable(
                report_dir,
                str(spec["figure_id"]),
                16.4 * cm,
                7.4 * cm,
            )
        )
    table = spec.get("table")
    if isinstance(table, dict):
        if display_kind == "figure-with-table":
            flowables.append(Spacer(1, INLINE_SPACER))
        flowables.append(
            make_table(
                list(table["columns"]),
                [list(row) for row in table["rows"]],
                styles,
                list(table["widths"]),
            )
        )
    caption_text = f"{label} {spec['display_number']}. {spec['figure_name']}. {spec['caption']}"
    flowables.append(Paragraph(paper_markup(caption_text), styles["Caption"]))
    return flowables


def add_manifest(
    story: list[object],
    styles,
    figure_specs: list[dict[str, object]],
) -> None:
    story.append(PageBreak())
    story.append(Paragraph("Data And Reproducibility Notes", styles["Section"]))
    table_rows: list[list[str]] = []
    for spec in figure_specs:
        label = "Table" if str(spec.get("display_kind")) == "table" else "Figure"
        table_rows.append(
            [
                f"{label} {spec['display_number']}",
                str(spec["figure_name"]),
                display_source_artifact(str(spec["source_artifact"])),
                str(spec["artifact_row_count"]),
            ]
        )
    story.append(
        make_table(
            ["Label", "Exhibit", "Dataset", "Rows"],
            table_rows,
            styles,
            [1.6, 4.6, 6.8, 2.0],
        )
    )
    story.append(Spacer(1, BLOCK_SPACER))
    story.append(
        Paragraph(
            paper_markup(
                "Machine-readable data tables, row counts, and deterministic replay checks are included in the companion artifact package for this report build."
            ),
            styles["Body"],
        )
    )


def display_source_artifact(name: str) -> str:
    labels = {
        "active_belief_theorem_assumptions.csv": "theorem boundary audit rows",
        "active_belief_trace_validation.csv": "trace preprocessing and replay audit rows",
        "active_belief_path_validation.csv": "path-free recovery validation rows",
        "active_belief_raw_rounds.csv": "receiver-round belief trajectories",
        "coded_inference_experiment_a2_evidence_modes.csv": "three-mode task-surface rows",
        "active_belief_second_tasks.csv": "task-family comparison rows",
        "active_belief_headline_statistics.csv": "headline paired-delta summaries",
        "active_belief_receiver_runs.csv": "receiver-run outcome summaries",
        "active_belief_demand_ablation.csv": "demand-ablation matched runs",
        "coded_inference_experiment_d_coding_vs_replication.csv": "equal-budget coding comparison rows",
        "active_belief_exact_seed_summary.csv": "stress-boundary seed summaries",
        "active_belief_host_bridge_demand.csv": "host/bridge demand audit rows",
        "active_belief_strong_baselines.csv": "equal-budget baseline comparison rows",
        "active_belief_scale_validation.csv": "large-regime validation rows",
        "coded_inference_experiment_e_observer_frontier.csv": "observer-proxy tradeoff rows",
        "coded_inference_experiment_c_phase_diagram.csv": "control-region sweep rows",
    }
    return labels.get(name, name.replace(".csv", "").replace("_", " "))


def active_figure_flowable(
    report_dir: Path, asset_id: str, max_width: float, max_height: float
):
    png_path = report_dir / f"{asset_id}.png"
    if png_path.exists():
        reader = ImageReader(str(png_path))
        width_px, height_px = reader.getSize()
        scale = min(max_width / width_px, max_height / height_px)
        image = Image(str(png_path))
        image.drawWidth = width_px * scale
        image.drawHeight = height_px * scale
        return image
    svg_path = report_dir / f"{asset_id}.svg"
    if svg_path.exists():
        from analysis.document import figure_flowable

        return figure_flowable(report_dir, asset_id, max_width, max_height)
    from analysis.document import figure_flowable

    return figure_flowable(report_dir, asset_id, max_width, max_height)


def add_markdown(
    story: list[object],
    styles,
    markdown: str,
    report_dir: Path,
    figure_specs: list[dict[str, object]],
) -> set[str]:
    if not markdown.strip():
        return set()
    paragraph_lines: list[str] = []
    table_lines: list[str] = []
    code_lines: list[str] = []
    list_item: tuple[str, list[str]] | None = None
    in_code = False
    pending_table_caption = False
    specs_by_id = {str(spec["figure_id"]): spec for spec in figure_specs}
    placed_exhibits: set[str] = set()

    def flush_paragraph() -> None:
        nonlocal pending_table_caption
        if not paragraph_lines:
            return
        text = " ".join(line.strip() for line in paragraph_lines).strip()
        paragraph_lines.clear()
        if text:
            style_name = "Caption" if pending_table_caption and re.match(r"^Table \d+\.", text) else "Body"
            story.append(Paragraph(paper_markup(text), styles[style_name]))
            story.append(Spacer(1, INLINE_SPACER))
        pending_table_caption = False

    def flush_table() -> None:
        nonlocal pending_table_caption
        if not table_lines:
            return
        rows = markdown_table_rows(table_lines)
        table_lines.clear()
        if rows:
            story.append(markdown_table(rows, styles))
            story.append(Spacer(1, BLOCK_SPACER))
            pending_table_caption = True

    def flush_code() -> None:
        nonlocal pending_table_caption
        if not code_lines:
            return
        story.append(Preformatted("\n".join(code_lines), styles["SmallCode"]))
        code_lines.clear()
        story.append(Spacer(1, BLOCK_SPACER))
        pending_table_caption = False

    def flush_list_item() -> None:
        nonlocal list_item, pending_table_caption
        if list_item is None:
            return
        marker, lines = list_item
        text = " ".join(line.strip() for line in lines).strip()
        list_item = None
        if text:
            story.append(list_item_flowable(marker, text, styles))
            story.append(Spacer(1, 3))
        pending_table_caption = False

    for raw_line in markdown.splitlines():
        line = raw_line.rstrip()
        exhibit_id = exhibit_marker(line)
        if exhibit_id and not in_code:
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            flush_table()
            spec = specs_by_id.get(exhibit_id)
            if spec:
                story.append(
                    KeepTogether(figure_or_table_flowables(styles, report_dir, spec))
                )
                story.append(Spacer(1, FIGURE_BLOCK_SPACER))
                placed_exhibits.add(exhibit_id)
            continue
        if line.startswith("```"):
            if in_code:
                flush_code()
                in_code = False
            else:
                pending_table_caption = False
                flush_list_item()
                flush_paragraph()
                flush_table()
                in_code = True
            continue
        if in_code:
            code_lines.append(line)
            continue
        if line.startswith("|"):
            flush_list_item()
            flush_paragraph()
            table_lines.append(line)
            continue
        flush_table()
        if not line.strip():
            flush_list_item()
            flush_paragraph()
            continue
        if list_item is not None and line.startswith(("  ", "\t")):
            marker, lines = list_item
            lines.append(line.strip())
            list_item = (marker, lines)
            continue
        if line.startswith("# "):
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            story.append(
                Paragraph(paper_markup(line[2:].strip()), styles["TitleCustom"])
            )
        elif line.startswith("## "):
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            story.append(Paragraph(paper_markup(line[3:].strip()), styles["Section"]))
        elif line.startswith("### "):
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            story.append(
                Paragraph(paper_markup(line[4:].strip()), styles["Subsection"])
            )
        elif line.startswith("- "):
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            list_item = ("\u2022", [line[2:].strip()])
        elif numbered_list_line(line):
            pending_table_caption = False
            flush_list_item()
            flush_paragraph()
            marker, _, rest = line.strip().partition(".")
            list_item = (f"{marker}.", [rest.strip()])
        elif line.startswith("> "):
            flush_list_item()
            paragraph_lines.append(line[2:])
        else:
            flush_list_item()
            paragraph_lines.append(line)
    flush_list_item()
    flush_paragraph()
    flush_table()
    flush_code()
    return placed_exhibits


def exhibit_marker(line: str) -> str | None:
    match = re.fullmatch(r"\{\{EXHIBIT:([a-zA-Z0-9_]+)\}\}", line.strip())
    if match:
        return match.group(1)
    return None


def list_item_flowable(marker: str, text: str, styles) -> Table:
    data = [
        [
            Paragraph(paper_markup(marker), styles["ListMarker"]),
            Paragraph(paper_markup(text), styles["ListCell"]),
        ]
    ]
    table = Table(data, colWidths=[0.5 * cm, 15.5 * cm], hAlign="LEFT")
    table.setStyle(
        TableStyle(
            [
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("TEXTCOLOR", (0, 0), (-1, -1), colors.black),
                ("LEFTPADDING", (0, 0), (-1, -1), 0),
                ("RIGHTPADDING", (0, 0), (-1, -1), 3),
                ("TOPPADDING", (0, 0), (-1, -1), 0),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 0),
            ]
        )
    )
    return table


def markdown_table(rows: list[list[str]], styles):
    column_labels = rows[0]
    body_rows = rows[1:]
    return make_table(column_labels, body_rows, styles, [1.0 for _ in column_labels])


def markdown_table_rows(lines: list[str]) -> list[list[str]]:
    rows: list[list[str]] = []
    for line in lines:
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells and all(set(cell) <= {"-", ":", " "} for cell in cells):
            continue
        rows.append(cells)
    return rows


def numbered_list_line(line: str) -> bool:
    prefix, _, rest = line.strip().partition(".")
    return prefix.isdigit() and bool(rest.strip())


def paper_markup(text: str) -> str:
    rendered = markup(text)
    return re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", rendered)
