use gtk4 as gtk;
use gtk::prelude::*;

pub fn build_graph_widget(history: &[f64], width: i32, height: i32) -> gtk::DrawingArea {
    let values = history.to_vec();
    let area = gtk::DrawingArea::new();
    area.set_content_width(width.max(48));
    area.set_content_height(height.max(24));

    area.set_draw_func(move |_, cr, width, height| {
        let w = width as f64;
        let h = height as f64;
        let left = 34.0;
        let right = 8.0;
        let top = 10.0;
        let bottom = 20.0;
        let plot_w = (w - left - right).max(1.0);
        let plot_h = (h - top - bottom).max(1.0);

        cr.set_source_rgba(0.47, 0.70, 0.96, 0.14);
        let _ = cr.paint();

        cr.set_source_rgba(0.42, 0.62, 0.92, 0.12);
        for i in 0..=4 {
            let y = top + (plot_h / 4.0) * i as f64;
            cr.move_to(left, y);
            cr.line_to(left + plot_w, y);
        }
        for i in 0..=6 {
            let x = left + (plot_w / 6.0) * i as f64;
            cr.move_to(x, top);
            cr.line_to(x, top + plot_h);
        }
        let _ = cr.stroke();

        cr.set_source_rgba(0.85, 0.87, 0.90, 0.8);
        cr.set_line_width(1.2);
        cr.move_to(left, top);
        cr.line_to(left, top + plot_h);
        cr.line_to(left + plot_w, top + plot_h);
        let _ = cr.stroke();

        if values.is_empty() {
            return;
        }

        let mut min = values[0];
        let mut max = values[0];
        for v in &values {
            if *v < min {
                min = *v;
            }
            if *v > max {
                max = *v;
            }
        }

        let spread = (max - min).max(1.0);

        cr.select_font_face(
            "Sans",
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Normal,
        );
        cr.set_font_size(9.0);
        cr.set_source_rgba(0.88, 0.90, 0.93, 0.8);
        cr.move_to(3.0, top + 3.0);
        let _ = cr.show_text(&format!("{:.1}", max));
        cr.move_to(3.0, top + (plot_h / 2.0) + 3.0);
        let _ = cr.show_text(&format!("{:.1}", min + spread / 2.0));
        cr.move_to(3.0, top + plot_h + 3.0);
        let _ = cr.show_text(&format!("{:.1}", min));
        cr.move_to(left, top + plot_h + 14.0);
        let _ = cr.show_text("60s");
        cr.move_to(left + plot_w - 12.0, top + plot_h + 14.0);
        let _ = cr.show_text("0s");

        cr.set_source_rgba(0.37, 0.63, 0.95, 0.25);
        cr.set_line_width(1.0);

        for (i, value) in values.iter().enumerate() {
            let denom = (values.len().saturating_sub(1)).max(1) as f64;
            let x = left + (i as f64 / denom) * plot_w;
            let y = top + plot_h - (((*value - min) / spread) * plot_h);
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        cr.line_to(left + plot_w, top + plot_h);
        cr.line_to(left, top + plot_h);
        cr.close_path();
        let _ = cr.fill();

        cr.set_source_rgba(0.31, 0.56, 0.90, 0.95);
        cr.set_line_width(1.8);
        for (i, value) in values.iter().enumerate() {
            let denom = (values.len().saturating_sub(1)).max(1) as f64;
            let x = left + (i as f64 / denom) * plot_w;
            let y = top + plot_h - (((*value - min) / spread) * plot_h);
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        let _ = cr.stroke();

        if let Some(last) = values.last() {
            let y = top + plot_h - (((*last - min) / spread) * plot_h);
            cr.arc(left + plot_w, y, 2.5, 0.0, std::f64::consts::PI * 2.0);
            let _ = cr.fill();
        }
    });

    area
}
