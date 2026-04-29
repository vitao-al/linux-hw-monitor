use gtk4 as gtk;
use gtk::prelude::*;

pub fn build_graph_widget(history: &[f64]) -> gtk::DrawingArea {
    let values = history.to_vec();
    let area = gtk::DrawingArea::new();
    area.set_content_width(420);
    area.set_content_height(180);

    area.set_draw_func(move |_, cr, width, height| {
        cr.set_source_rgb(0.11, 0.11, 0.11);
        let _ = cr.paint();

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.1);
        for i in 0..6 {
            let y = (height as f64 / 6.0) * i as f64;
            cr.move_to(0.0, y);
            cr.line_to(width as f64, y);
        }
        let _ = cr.stroke();

        if values.len() < 2 {
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
        cr.set_source_rgb(0.2, 0.8, 0.55);
        cr.set_line_width(2.0);

        for (i, value) in values.iter().enumerate() {
            let x = (i as f64 / (values.len() - 1) as f64) * width as f64;
            let y = height as f64 - (((*value - min) / spread) * height as f64);
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        let _ = cr.stroke();
    });

    area
}
