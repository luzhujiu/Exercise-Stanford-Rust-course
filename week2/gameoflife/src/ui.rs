use nannou::prelude::*;
use nannou::ui::prelude::*;
use nannou::prelude::Rect;
use crate::Model;
use crate::Grid;

fn slider(val: f32, min: f32, max: f32) -> widget::Slider<'static, f32> {
    widget::Slider::new(val, min, max)
        .w_h(200.0, 30.0)
        .label_font_size(15)
        .rgb(0.3, 0.3, 0.3)
        .label_rgb(1.0, 1.0, 1.0)
        .border(0.0)
}

pub fn update_ui(_app: &App, _model: &mut Model, _update: Update) {

    let ui = &mut _model.ui.set_widgets();

    let label = if _model.is_active { "stop" } else { "start" };

    let win = _app.window_rect().pad(25.0);
    let r = Rect::from_w_h(200.0,60.0).top_right_of(win);
    
    for _click in widget::Button::new()
        .x_y(r.x_y().0.into(), r.x_y().1.into())
        .w_h(r.w_h().0.into(), r.w_h().1.into())
        .label(label)
        .label_font_size(15)
        .rgb(0.3, 0.3, 0.3)
        .label_rgb(1.0, 1.0, 1.0)
        .border(0.0)
    .set(_model.ids.start, ui)
    {
        _model.is_active = !_model.is_active;
    }
    
    let r2 = Rect::from_w_h(200.0,60.0).bottom_left_of(r).shift_y(-80.0);

    for _click in widget::Button::new()
        .x_y(r2.x_y().0.into(), r2.x_y().1.into())
        .w_h(r2.w_h().0.into(), r2.w_h().1.into())
        .label("reset")
        .label_font_size(15)
        .rgb(0.3, 0.3, 0.3)
        .label_rgb(1.0, 1.0, 1.0)
        .border(0.0)
        .set(_model.ids.reset, ui)
    {
        if !_model.is_active {
            _model.grid.clear();
        }
    }

    let r3 = Rect::from_w_h(200.0, 30.0).bottom_left_of(r2).shift_y(-50.0);

    for value in slider(_model.duration as f32, 0.1, 3.0)
        .x_y(r3.x_y().0.into(), r3.x_y().1.into())
        .label("duration")
        .set(_model.ids.duration, ui)
    {
        _model.duration = value as f32;
    }
    
    let r4 = Rect::from_w_h(200.0, 30.0).bottom_left_of(r3).shift_y(-50.0);

    for value in slider(_model.num_of_rows as f32, 5.0, 20.0)
        .x_y(r4.x_y().0.into(), r4.x_y().1.into())
        .label("num_of_rows")
        .set(_model.ids.num_of_rows, ui)
    {
        if ! _model.is_active {
            let new_value = value.trunc() as usize;
            if _model.num_of_rows != new_value {
                _model.num_of_rows = new_value;
                _model.grid = Grid::new(_model.num_of_rows, _model.num_of_rows);
            }
        }
    }
    
}

