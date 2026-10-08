#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod cloud;
mod dipole;
mod gaussian;
mod harmonics;
mod inferno;
mod kepler;
mod knots;
mod lorenz;
mod science;
mod spherical;
mod spirals;
mod waves;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let example = std::env::args().nth(1).unwrap_or_default();
    eframe::run_native(
        "egui_plot3d demo",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 900.0])
                .with_min_inner_size([360.0, 400.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(
                app::Gallery::new(&cc.egui_ctx).with_example(&example),
            ))
        }),
    )
}
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id("plot_canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::Gallery::new(&cc.egui_ctx)))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match result {
                Ok(()) => loading.remove(),
                Err(error) => {
                    loading.set_text_content(Some(
                        "The gallery could not start. Please enable WebGL or try another browser.",
                    ));
                    web_sys::console::error_1(&error);
                }
            }
        }
    });
}
