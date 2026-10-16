//! Explanatory UI for the network. Diagram positions are schematic, not data axes.
use crate::neural::{Model, label_color};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, Ui, pos2, vec2};

pub fn inspector(ui: &mut Ui, model: &Model, input: &mut [f64; 2]) -> bool {
    let before = *input;
    ui.strong("1. The examples");
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(vec2(140.0, 140.0), Sense::click_and_drag());
        let area = rect.shrink(7.0);
        let project = |p: [f64; 2]| {
            pos2(
                area.left() + (p[0] as f32 + 1.0) * 0.5 * area.width(),
                area.bottom() - (p[1] as f32 + 1.0) * 0.5 * area.height(),
            )
        };
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        for &(p, label) in model.samples() {
            ui.painter()
                .circle_filled(project(p), 2.3, label_color(label));
        }
        if (response.clicked() || response.dragged())
            && let Some(p) = response.interact_pointer_pos()
        {
            *input = [
                ((p.x - area.left()) / area.width() * 2.0 - 1.0).clamp(-1.0, 1.0) as f64,
                ((area.bottom() - p.y) / area.height() * 2.0 - 1.0).clamp(-1.0, 1.0) as f64,
            ];
        }
        ui.painter().circle_stroke(
            project(*input),
            5.0,
            Stroke::new(2.0, Color32::from_rgb(245, 65, 145)),
        );
        ui.vertical(|ui| {
            ui.colored_label(label_color(1.0), "Orange: inside = 1");
            ui.colored_label(label_color(0.0), "Blue: outside = 0");
            ui.label("These answers stay fixed.");
            ui.label("Click the map to inspect a point.");
        });
    });
    ui.add_space(8.0);
    ui.strong("2. Follow one prediction, left to right");
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Slider::new(&mut input[0], -1.0..=1.0).text("x"));
        ui.add(egui::Slider::new(&mut input[1], -1.0..=1.0).text("y"));
    });
    let (a, b, probability) = model.forward(*input);
    ui.label(format!(
        "Network: {:.1}% inside  |  Correct answer: {}",
        probability * 100.0,
        if model.answer(*input) {
            "inside"
        } else {
            "outside"
        },
    ));
    let activations = [input.to_vec(), a.to_vec(), b.to_vec(), vec![probability]];
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 260.0), Sense::hover());
    let foreground = ui.visuals().text_color();
    let location = |layer: usize, index: usize| {
        let n = activations[layer].len();
        pos2(
            rect.left() + rect.width() * (0.10 + layer as f32 * 0.265),
            rect.top()
                + if n == 1 {
                    141.0
                } else if n == 2 {
                    115.0 + 52.0 * index as f32
                } else {
                    50.0 + index as f32 * 26.0
                },
        )
    };
    let weights = model.weights();
    for layer in 0..3 {
        for j in 0..activations[layer + 1].len() {
            for k in 0..activations[layer].len() {
                let index = match layer {
                    0 => j * 2 + k,
                    1 => 24 + j * 8 + k,
                    _ => 96 + k,
                };
                let w = weights[index];
                let color = signed_color(w.tanh()).gamma_multiply(0.5);
                ui.painter().line_segment(
                    [location(layer, k), location(layer + 1, j)],
                    Stroke::new(0.5 + w.abs().tanh() as f32, color),
                );
            }
        }
    }
    for (layer, values) in activations.iter().enumerate() {
        ui.painter().text(
            pos2(location(layer, 0).x, rect.top() + 8.0),
            Align2::CENTER_TOP,
            ["Inputs", "Hidden 1", "Hidden 2", "Output"][layer],
            FontId::proportional(12.0),
            foreground,
        );
        for (j, &value) in values.iter().enumerate() {
            let position = location(layer, j);
            let color = if layer == 3 {
                egui_plot3d::ColorMap::Viridis.color(value)
            } else {
                signed_color(value)
            };
            ui.painter().circle_filled(position, 11.0, color);
            ui.painter().circle_stroke(
                position,
                11.0,
                Stroke::new(1.0, foreground.gamma_multiply(0.4)),
            );
            let text_color = if layer == 3 && value < 0.5 {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            ui.painter().text(
                position,
                Align2::CENTER_CENTER,
                format!("{value:.1}"),
                FontId::monospace(9.0),
                text_color,
            );
            if layer == 0 {
                ui.painter().text(
                    position - vec2(17.0, 0.0),
                    Align2::RIGHT_CENTER,
                    if j == 0 { "x" } else { "y" },
                    FontId::proportional(12.0),
                    foreground,
                );
            }
            let response = ui.interact(
                egui::Rect::from_center_size(position, vec2(24.0, 24.0)),
                ui.id().with(("neuron", layer, j)),
                Sense::hover(),
            );
            response.on_hover_ui(|ui| {
                ui.label(format!("Value: {value:.5}"));
                match layer {
                    0 => {
                        ui.label(if j == 0 {
                            "Input x coordinate"
                        } else {
                            "Input y coordinate"
                        });
                    }
                    1 | 2 => {
                        let bias = weights[if layer == 1 { 16 + j } else { 88 + j }];
                        ui.label(format!("Bias: {bias:.5}"));
                        ui.label("tanh(sum of input × weight + bias)");
                    }
                    _ => {
                        ui.label(format!("Bias: {:.5}", weights[104]));
                        ui.label("sigmoid(sum of input × weight + bias)");
                    }
                }
            });
        }
    }
    ui.label("Circles show values for this point. Hidden values and weights: blue = negative, orange = positive. Thicker lines mean stronger weights.");
    ui.add_space(8.0);
    ui.strong("3. Training changes the weights");
    ui.label("Each update predicts all 256 examples, measures the errors, and adjusts the 105 weights and biases to reduce them.");
    ui.label("Moving the pink point only asks for a prediction. It does not train the network.");
    ui.collapsing("The math", |ui| {
        ui.label("2 inputs, two layers of 8 tanh neurons, and 1 sigmoid output. Binary cross-entropy loss; full-batch backpropagation and Adam updates.");
        ui.label("Accuracy counts training examples on the correct side of 50%. It is not a test-set score.");
        ui.hyperlink_to("Adam optimizer paper", "https://arxiv.org/abs/1412.6980");
    });
    before != *input
}

fn signed_color(value: f64) -> Color32 {
    let end = if value >= 0.0 {
        [255.0, 169.0, 60.0]
    } else {
        [75.0, 171.0, 230.0]
    };
    let t = value.abs().clamp(0.0, 1.0);
    let c: [u8; 3] = std::array::from_fn(|i| (215.0 + (end[i] - 215.0) * t) as u8);
    Color32::from_rgb(c[0], c[1], c[2])
}
