#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui_inspect::EguiInspect;

use eframe::egui;

#[derive(EguiInspect)]
struct MyApp {
    #[inspect(no_edit)]
    string: String,
    #[inspect(multiline)]
    code: String,
    #[inspect(min = 12.0, max = 53.0)]
    unsigned32: u32,
    #[inspect(hide)]
    _skipped: bool,
    #[inspect(no_edit)]
    strings: Vec<String>,
    #[inspect(no_edit)]
    raw_string: &'static str,
    #[inspect(slider, min = -43.0, max = 125.0)]
    float64: f64,
    #[inspect(name = "A proper field name")]
    ugly_internal_field_name: u16,
    #[inspect(name = "A tuple struct")]
    ugly_internal_field_name_2: Foo,
    #[inspect(name = "A struct with three floats")]
    vector_struct: Vector,
    my_enum: MyEnum,
    #[inspect(no_edit)]
    my_enum_readonly: MyEnum,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            string: "I am a single line string".to_owned(),
            code: "Hello\nI\nam\na\nmultiline\nstring".to_owned(),
            _skipped: true,
            unsigned32: 42,
            strings: vec![
                "Bonjour".to_string(),
                "Voici une liste de string".to_string(),
                "Avec plusieurs strings".to_string(),
            ],
            raw_string: "YetAnotherString",
            float64: 6.0,
            ugly_internal_field_name: 16,
            ugly_internal_field_name_2: Foo(50, 123.45),
            vector_struct: Vector {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            my_enum: MyEnum::StructData {
                vec: Default::default(),
                foo: Default::default(),
            },
            my_enum_readonly: MyEnum::Unnamed(0, 10),
        }
    }
}

#[derive(EguiInspect, PartialEq, Default)]
struct Foo(i32, f32);

#[derive(EguiInspect, PartialEq, Default)]
struct Vector {
    #[inspect(name = "X axis")]
    x: f32,
    #[inspect(name = "Y axis")]
    y: f32,
    #[inspect(name = "Z axis")]
    z: f32,
}

#[derive(EguiInspect, PartialEq)]
enum MyEnum {
    NoData,
    StructData { vec: Vector, foo: Foo },
    Unnamed(usize, usize),
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.inspect_mut("Test App", ui);
            // self.inspect("Test App", ui);

            let foo = Foo(1, 2.0);
            foo.inspect("label for tuple struct", ui);
        });

        // Resize the native window to be just the size we need it to be:
        frame.set_window_size(ctx.used_size());
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}
