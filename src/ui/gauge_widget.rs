use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;

pub struct GaugeWidget {
    pub widget: gtk::Frame,
    value: Rc<Cell<f64>>,
    drawing: gtk::DrawingArea,
}

impl GaugeWidget {
    pub fn new(title: &str) -> Self {
        let frame = gtk::Frame::new(Some(title));
        frame.set_hexpand(true);
        frame.set_vexpand(false);

        let da = gtk::DrawingArea::new();
        da.set_content_width(180);
        da.set_content_height(120);
        let value = Rc::new(Cell::new(0.0_f64));
        let value_ref = Rc::clone(&value);
        da.set_draw_func(move |_, cr, width, height| {
            let is_dark = adw::StyleManager::default().is_dark();
            let (text_r, text_g, text_b) = if is_dark {
                (0.94, 0.94, 0.94)
            } else {
                (0.13, 0.14, 0.16)
            };

            let cx = width as f64 / 2.0;
            let cy = height as f64 * 0.9;
            let r = (height as f64 * 0.75).min(width as f64 * 0.45);
            let level = value_ref.get().clamp(0.0_f64, 1.0_f64);
            let percent = level * 100.0;

            let (pr, pg, pb) = if level < 0.65 {
                (0.10, 0.62, 0.30)
            } else if level < 0.85 {
                (0.90, 0.62, 0.10)
            } else {
                (0.84, 0.20, 0.18)
            };

            cr.set_source_rgb(0.85, 0.85, 0.85);
            cr.set_line_width(10.0);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            let _ = cr.arc(cx, cy, r, std::f64::consts::PI, 2.0 * std::f64::consts::PI);
            let _ = cr.stroke();

            cr.set_source_rgb(pr, pg, pb);
            let _ = cr.arc(
                cx,
                cy,
                r,
                std::f64::consts::PI,
                std::f64::consts::PI + std::f64::consts::PI * level,
            );
            let _ = cr.stroke();

            let value_text = format!("{percent:.1}%");
            cr.set_source_rgb(text_r, text_g, text_b);
            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );
            cr.set_font_size((height as f64 * 0.17).max(16.0));
            if let Ok(ext) = cr.text_extents(&value_text) {
                cr.move_to(
                    cx - ext.width() / 2.0 - ext.x_bearing(),
                    cy - r * 0.22,
                );
                let _ = cr.show_text(&value_text);
            }

            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Normal,
            );
            cr.set_font_size((height as f64 * 0.09).max(11.0));
            let context_text = "Utilization";
            if let Ok(ext) = cr.text_extents(context_text) {
                cr.move_to(
                    cx - ext.width() / 2.0 - ext.x_bearing(),
                    cy - r * 0.08,
                );
                let _ = cr.show_text(context_text);
            }

            cr.set_source_rgba(text_r, text_g, text_b, 0.85);
            cr.set_font_size((height as f64 * 0.08).max(10.0));
            let _ = cr.move_to(cx - r - 8.0, cy + 2.0);
            let _ = cr.show_text("0%");
            let _ = cr.move_to(cx + r - 28.0, cy + 2.0);
            let _ = cr.show_text("100%");
        });

        frame.set_child(Some(&da));
        Self {
            widget: frame,
            value,
            drawing: da,
        }
    }

    pub fn set_value_percent(&self, percent: f64) {
        self.value.set((percent / 100.0_f64).clamp(0.0_f64, 1.0_f64));
        self.drawing.queue_draw();
    }
}
