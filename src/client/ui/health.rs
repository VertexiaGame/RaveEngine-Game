use bevy_egui::{EguiContexts, egui};


//notice: partly vibecoded.
fn rounded_vertical_gradient(
    rect: egui::Rect,
    corner_radius: f32,
    top: egui::Color32,
    bottom: egui::Color32,
) -> egui::Shape {
    let h = rect.height();
    let w = rect.width();
    if w <= 0.0 || h <= 0.0 {
        return egui::Shape::Noop;
    }
    let r = corner_radius.clamp(0.0, w * 0.5).clamp(0.0, h * 0.5);
    if r <= 0.5 {
        return egui::Shape::gradient_rect(rect, egui::Direction::TopDown, [top, bottom]);
    }
    let color_at = |y: f32| {
        let t = ((y - rect.min.y) / h).clamp(0.0, 1.0);
        top.lerp_to_gamma(bottom, t)
    };

    let segs = 8;
    let mut pts: Vec<egui::Pos2> = Vec::with_capacity(segs * 4 + 4);
    pts.push(egui::pos2(rect.min.x + r, rect.min.y));
    pts.push(egui::pos2(rect.max.x - r, rect.min.y));
    let tr_c = egui::pos2(rect.max.x - r, rect.min.y + r);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = -std::f32::consts::FRAC_PI_2 + t * std::f32::consts::FRAC_PI_2;
        pts.push(egui::pos2(tr_c.x + r * a.cos(), tr_c.y + r * a.sin()));
    }
    pts.push(egui::pos2(rect.max.x, rect.max.y - r));
    let br_c = egui::pos2(rect.max.x - r, rect.max.y - r);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = t * std::f32::consts::FRAC_PI_2;
        pts.push(egui::pos2(br_c.x + r * a.cos(), br_c.y + r * a.sin()));
    }
    pts.push(egui::pos2(rect.min.x + r, rect.max.y));
    let bl_c = egui::pos2(rect.min.x + r, rect.max.y - r);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = std::f32::consts::FRAC_PI_2 + t * std::f32::consts::FRAC_PI_2;
        pts.push(egui::pos2(bl_c.x + r * a.cos(), bl_c.y + r * a.sin()));
    }
    pts.push(egui::pos2(rect.min.x, rect.min.y + r));
    let tl_c = egui::pos2(rect.min.x + r, rect.min.y + r);
    for i in 1..segs {
        let t = i as f32 / segs as f32;
        let a = std::f32::consts::PI + t * std::f32::consts::FRAC_PI_2;
        pts.push(egui::pos2(tl_c.x + r * a.cos(), tl_c.y + r * a.sin()));
    }

    let center = rect.center();
    let mut mesh = egui::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex::untextured(center, color_at(center.y)));
    for p in &pts {
        mesh.vertices.push(egui::epaint::Vertex::untextured(*p, color_at(p.y)));
    }
    for i in 1..=pts.len() {
        let a = i as u32;
        let b = if i == pts.len() { 1 } else { i as u32 + 1 };
        mesh.indices.extend_from_slice(&[0, a, b]);
    }
    egui::Shape::from(mesh)
}

pub fn draw_health_bar(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let screen_rect = ctx.content_rect();
    let screen_width = screen_rect.width();

    let scale_factor = (screen_width / 1280.0).clamp(0.7, 1.2);

    egui::Area::new(egui::Id::new("client_health_bar_area"))
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -12.0 * scale_factor),
        )
        .show(ctx, |ui| {
            let width = 300.0 * scale_factor;
            let height = 28.0 * scale_factor;
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            let painter = ui.painter();

            let outer_radius = 6.0 * scale_factor;
            let stroke_width = 2.0 * scale_factor;
            let inner_rect = rect.shrink(stroke_width);
            let inner_radius = (outer_radius - stroke_width).max(0.0);
            let gradient = rounded_vertical_gradient(
                inner_rect,
                inner_radius,
                egui::Color32::from_rgb(74, 184, 80),
                egui::Color32::from_rgb(48, 120, 52),
            );
            painter.add(gradient);

            painter.rect_stroke(
                rect,
                outer_radius,
                egui::Stroke::new(stroke_width, egui::Color32::from_rgb(61, 61, 61)),
                egui::StrokeKind::Inside,
            );

            let font_size = 13.0 * scale_factor;
            let is_medium_loaded = ctx.fonts(|f| {
                f.families()
                    .contains(&egui::FontFamily::Name("Medium".into()))
            });
            let font = if is_medium_loaded {
                egui::FontId::new(font_size, egui::FontFamily::Name("Medium".into()))
            } else {
                egui::FontId::new(font_size, egui::FontFamily::Proportional)
            };

            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Health",
                font,
                egui::Color32::WHITE,
            );
        });
}
