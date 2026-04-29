use gtk4 as gtk;
use gtk::prelude::*;

pub struct GaugeWidget {
    pub widget: gtk::Frame,
}

impl GaugeWidget {
    pub fn new(title: &str) -> Self {
        let frame = gtk::Frame::new(Some(title));
        frame.set_hexpand(true);
        frame.set_vexpand(false);

        let da = gtk::DrawingArea::new();
        da.set_content_width(180);
        da.set_content_height(120);
        da.set_draw_func(|_, cr, width, height| {
            let cx = width as f64 / 2.0;
            let cy = height as f64 * 0.9;
            let r = (height as f64 * 0.75).min(width as f64 * 0.45);

            cr.set_source_rgb(0.85, 0.85, 0.85);
            cr.set_line_width(10.0);
            let _ = cr.arc(cx, cy, r, std::f64::consts::PI, 2.0 * std::f64::consts::PI);
            let _ = cr.stroke();

            cr.set_source_rgb(0.1, 0.6, 0.25);
            let _ = cr.arc(
                cx,
                cy,
                r,
                std::f64::consts::PI,
                std::f64::consts::PI + std::f64::consts::PI * 0.55,
            );
            let _ = cr.stroke();
        });

        frame.set_child(Some(&da));
        Self { widget: frame }
    }
}
