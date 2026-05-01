use gtk4 as gtk;
use gtk::prelude::*;

/// RGB color palette for sensor graphs, keyed by group id.
pub fn group_color(group_id: &str) -> (f64, f64, f64) {
    match group_id {
        id if id.contains("cpu")     => (0.31, 0.56, 0.90), // blue
        id if id.contains("gpu")     => (0.29, 0.75, 0.48), // green
        id if id.contains("mem")     => (0.72, 0.42, 0.95), // purple
        id if id.contains("disk")
           | id.contains("storage")  => (0.95, 0.70, 0.20), // amber
        id if id.contains("net")     => (0.22, 0.78, 0.78), // teal
        id if id.contains("temp")
           | id.contains("thermal")  => (0.95, 0.36, 0.22), // red-orange
        id if id.contains("fan")     => (0.45, 0.85, 0.85), // cyan
        id if id.contains("power")
           | id.contains("energy")   => (0.95, 0.55, 0.15), // orange
        _                            => (0.50, 0.60, 0.80), // grey-blue
    }
}

pub fn build_graph_widget(
    history: &[f64],
    width: i32,
    height: i32,
    color: (f64, f64, f64),
) -> gtk::DrawingArea {
    let values = history.to_vec();
    let (cr, cg, cb) = color;
    let area = gtk::DrawingArea::new();
    area.set_content_width(width.max(48));
    area.set_content_height(height.max(24));

    area.set_draw_func(move |_, cr_ctx, width, height| {
        let w = width as f64;
        let h = height as f64;
        let left = 34.0;
        let right = 8.0;
        let top = 10.0;
        let bottom = 20.0;
        let plot_w = (w - left - right).max(1.0);
        let plot_h = (h - top - bottom).max(1.0);

        // Background tint from graph color.
        cr_ctx.set_source_rgba(cr, cg, cb, 0.10);
        let _ = cr_ctx.paint();

        // Grid lines.
        cr_ctx.set_source_rgba(cr, cg, cb, 0.12);
        for i in 0..=4 {
            let y = top + (plot_h / 4.0) * i as f64;
            cr_ctx.move_to(left, y);
            cr_ctx.line_to(left + plot_w, y);
        }
        for i in 0..=6 {
            let x = left + (plot_w / 6.0) * i as f64;
            cr_ctx.move_to(x, top);
            cr_ctx.line_to(x, top + plot_h);
        }
        let _ = cr_ctx.stroke();

        // Axes.
        cr_ctx.set_source_rgba(0.85, 0.87, 0.90, 0.8);
        cr_ctx.set_line_width(1.2);
        cr_ctx.move_to(left, top);
        cr_ctx.line_to(left, top + plot_h);
        cr_ctx.line_to(left + plot_w, top + plot_h);
        let _ = cr_ctx.stroke();

        if values.is_empty() {
            return;
        }

        let mut min = values[0];
        let mut max = values[0];
        for v in &values {
            if *v < min { min = *v; }
            if *v > max { max = *v; }
        }
        let spread = (max - min).max(1.0);

        // Y-axis labels.
        cr_ctx.select_font_face(
            "Sans",
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Normal,
        );
        cr_ctx.set_font_size(9.0);
        cr_ctx.set_source_rgba(0.88, 0.90, 0.93, 0.8);
        cr_ctx.move_to(3.0, top + 3.0);
        let _ = cr_ctx.show_text(&format!("{:.1}", max));
        cr_ctx.move_to(3.0, top + (plot_h / 2.0) + 3.0);
        let _ = cr_ctx.show_text(&format!("{:.1}", min + spread / 2.0));
        cr_ctx.move_to(3.0, top + plot_h + 3.0);
        let _ = cr_ctx.show_text(&format!("{:.1}", min));
        cr_ctx.move_to(left, top + plot_h + 14.0);
        let _ = cr_ctx.show_text("60s");
        cr_ctx.move_to(left + plot_w - 12.0, top + plot_h + 14.0);
        let _ = cr_ctx.show_text("0s");

        // Fill under curve.
        cr_ctx.set_source_rgba(cr, cg, cb, 0.22);
        cr_ctx.set_line_width(1.0);
        for (i, value) in values.iter().enumerate() {
            let denom = (values.len().saturating_sub(1)).max(1) as f64;
            let x = left + (i as f64 / denom) * plot_w;
            let y = top + plot_h - (((*value - min) / spread) * plot_h);
            if i == 0 { cr_ctx.move_to(x, y); } else { cr_ctx.line_to(x, y); }
        }
        cr_ctx.line_to(left + plot_w, top + plot_h);
        cr_ctx.line_to(left, top + plot_h);
        cr_ctx.close_path();
        let _ = cr_ctx.fill();

        // Line.
        cr_ctx.set_source_rgba(cr, cg, cb, 0.95);
        cr_ctx.set_line_width(1.8);
        for (i, value) in values.iter().enumerate() {
            let denom = (values.len().saturating_sub(1)).max(1) as f64;
            let x = left + (i as f64 / denom) * plot_w;
            let y = top + plot_h - (((*value - min) / spread) * plot_h);
            if i == 0 { cr_ctx.move_to(x, y); } else { cr_ctx.line_to(x, y); }
        }
        let _ = cr_ctx.stroke();

        // Latest value dot.
        if let Some(last) = values.last() {
            let y = top + plot_h - (((*last - min) / spread) * plot_h);
            cr_ctx.arc(left + plot_w, y, 2.5, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr_ctx.fill();
        }
    });

    area
}
